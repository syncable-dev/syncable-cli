//! Dockerfile parser using nom.
//!
//! Parses Dockerfile content into an AST of `InstructionPos` elements.

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_till, take_while},
    character::complete::{char, space0, space1},
    combinator::opt,
    multi::separated_list0,
    sequence::{pair, preceded, tuple},
    IResult,
};

use super::instruction::*;

/// Parse error information.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Error message.
    pub message: String,
    /// Line number where the error occurred (1-indexed).
    pub line: u32,
    /// Column number (1-indexed, if available).
    pub column: Option<u32>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.column {
            Some(col) => write!(f, "line {}:{}: {}", self.line, col, self.message),
            None => write!(f, "line {}: {}", self.line, self.message),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a Dockerfile string into a list of positioned instructions.
pub fn parse_dockerfile(input: &str) -> Result<Vec<InstructionPos>, ParseError> {
    let mut instructions = Vec::new();
    let mut line_number = 1u32;

    // Process line by line, handling line continuations
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let start_line = line_number;
        let mut combined_line = String::new();
        let mut source_text = String::new();

        // Collect lines with continuations
        loop {
            let line = lines.get(i).unwrap_or(&"");
            source_text.push_str(line);
            source_text.push('\n');

            let trimmed = line.trim_end();
            if trimmed.ends_with('\\') {
                // Line continuation - remove backslash and continue
                combined_line.push_str(&trimmed[..trimmed.len() - 1]);
                combined_line.push(' ');
                i += 1;
                line_number += 1;
                if i >= lines.len() {
                    break;
                }
            } else {
                combined_line.push_str(trimmed);
                i += 1;
                line_number += 1;
                break;
            }
        }

        let trimmed = combined_line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Parse the instruction
        match parse_instruction(trimmed) {
            Ok((_, instruction)) => {
                instructions.push(InstructionPos::new(
                    instruction,
                    start_line,
                    source_text.trim_end().to_string(),
                ));
            }
            Err(_) => {
                // Try to parse as comment
                if trimmed.starts_with('#') {
                    let comment = trimmed[1..].trim().to_string();
                    instructions.push(InstructionPos::new(
                        Instruction::Comment(comment),
                        start_line,
                        source_text.trim_end().to_string(),
                    ));
                }
                // Skip unparseable lines (parser directives, empty lines after continuation, etc.)
            }
        }
    }

    Ok(instructions)
}

/// Parse a single instruction.
fn parse_instruction(input: &str) -> IResult<&str, Instruction> {
    alt((
        parse_from,
        parse_run,
        parse_copy,
        parse_add,
        parse_env,
        parse_label,
        parse_expose,
        parse_arg,
        parse_entrypoint,
        parse_cmd,
        parse_shell,
        parse_user,
        parse_workdir,
        parse_volume,
        parse_maintainer,
        parse_healthcheck,
        parse_onbuild,
        parse_stopsignal,
        parse_comment,
    ))(input)
}

/// Parse FROM instruction.
fn parse_from(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("FROM")(input)?;
    let (input, _) = space1(input)?;

    // Parse optional --platform flag
    let (input, platform) = opt(preceded(
        pair(tag("--platform="), space0),
        take_till(|c: char| c.is_whitespace()),
    ))(input)?;
    let (input, _) = space0(input)?;

    // Parse platform with space separator
    let (input, platform) = if platform.is_none() {
        opt(preceded(
            pair(tag("--platform"), space0),
            preceded(char('='), take_till(|c: char| c.is_whitespace())),
        ))(input)?
    } else {
        (input, platform)
    };
    let (input, _) = space0(input)?;

    // Parse image reference
    let (input, image_ref) = take_till(|c: char| c.is_whitespace())(input)?;
    let (input, _) = space0(input)?;

    // Parse optional AS alias
    let (input, alias) = opt(preceded(
        pair(tag_no_case("AS"), space1),
        take_while(|c: char| c.is_alphanumeric() || c == '_' || c == '-'),
    ))(input)?;

    // Parse image reference into components
    let base_image = parse_image_reference(image_ref, platform.map(|s| s.to_string()), alias.map(|s| ImageAlias::new(s)));

    Ok((input, Instruction::From(base_image)))
}

/// Parse image reference into BaseImage.
fn parse_image_reference(
    image_ref: &str,
    platform: Option<String>,
    alias: Option<ImageAlias>,
) -> BaseImage {
    // Handle digest
    if let Some(at_pos) = image_ref.find('@') {
        let (image_part, digest) = image_ref.split_at(at_pos);
        let digest = &digest[1..]; // Remove @

        let (image, tag) = parse_image_tag(image_part);
        return BaseImage {
            image,
            tag,
            digest: Some(digest.to_string()),
            alias,
            platform,
        };
    }

    // Handle tag
    let (image, tag) = parse_image_tag(image_ref);

    BaseImage {
        image,
        tag,
        digest: None,
        alias,
        platform,
    }
}

/// Parse image:tag into Image and optional tag.
fn parse_image_tag(image_ref: &str) -> (Image, Option<String>) {
    // Find the last colon that's not part of a port or registry
    // Registry format: host:port/name or host/name
    // Tag format: name:tag

    let parts: Vec<&str> = image_ref.split('/').collect();

    if parts.len() == 1 {
        // Simple name or name:tag
        if let Some(colon_pos) = image_ref.rfind(':') {
            let name = &image_ref[..colon_pos];
            let tag = &image_ref[colon_pos + 1..];
            (Image::new(name), Some(tag.to_string()))
        } else {
            (Image::new(image_ref), None)
        }
    } else {
        // Has path separators - might have registry
        let last_part = parts.last().unwrap();

        // Check if last part has a tag
        if let Some(colon_pos) = last_part.rfind(':') {
            // Check if it looks like a tag (not a port)
            let potential_tag = &last_part[colon_pos + 1..];
            if !potential_tag.chars().all(|c| c.is_ascii_digit()) || potential_tag.len() > 5 {
                // It's a tag, not a port
                let full_name = image_ref[..image_ref.len() - potential_tag.len() - 1].to_string();
                let (registry, name) = split_registry(&full_name);
                return (
                    match registry {
                        Some(r) => Image::with_registry(r, name),
                        None => Image::new(name),
                    },
                    Some(potential_tag.to_string()),
                );
            }
        }

        // No tag
        let (registry, name) = split_registry(image_ref);
        (
            match registry {
                Some(r) => Image::with_registry(r, name),
                None => Image::new(name),
            },
            None,
        )
    }
}

/// Split registry from image name.
fn split_registry(name: &str) -> (Option<String>, String) {
    // Registry indicators: contains '.', ':', or is 'localhost'
    if let Some(slash_pos) = name.find('/') {
        let potential_registry = &name[..slash_pos];
        if potential_registry.contains('.')
            || potential_registry.contains(':')
            || potential_registry == "localhost"
        {
            return (
                Some(potential_registry.to_string()),
                name[slash_pos + 1..].to_string(),
            );
        }
    }
    (None, name.to_string())
}

/// Parse RUN instruction.
fn parse_run(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("RUN")(input)?;
    let (input, _) = space0(input)?;

    // Parse flags (--mount, --network, --security)
    let (input, flags) = parse_run_flags(input)?;
    let (input, _) = space0(input)?;

    // Parse arguments (exec form or shell form)
    let (input, arguments) = parse_arguments(input)?;

    Ok((input, Instruction::Run(RunArgs { arguments, flags })))
}

/// Parse RUN flags.
fn parse_run_flags(input: &str) -> IResult<&str, RunFlags> {
    let mut flags = RunFlags::default();
    let mut remaining = input;

    loop {
        let (input, _) = space0(remaining)?;

        // Check for --mount
        if let Ok((input, mount)) = parse_mount_flag(input) {
            flags.mount.insert(mount);
            remaining = input;
            continue;
        }

        // Check for --network
        if let Ok((input, network)) = parse_flag_value(input, "--network") {
            flags.network = Some(network.to_string());
            remaining = input;
            continue;
        }

        // Check for --security
        if let Ok((input, security)) = parse_flag_value(input, "--security") {
            flags.security = Some(security.to_string());
            remaining = input;
            continue;
        }

        break;
    }

    Ok((remaining, flags))
}

/// Parse --flag=value.
fn parse_flag_value<'a>(input: &'a str, flag: &str) -> IResult<&'a str, &'a str> {
    let (input, _) = tag(flag)(input)?;
    let (input, _) = char('=')(input)?;
    take_till(|c: char| c.is_whitespace())(input)
}

/// Parse --mount flag.
fn parse_mount_flag(input: &str) -> IResult<&str, RunMount> {
    let (input, _) = tag("--mount=")(input)?;
    let (input, mount_str) = take_till(|c: char| c.is_whitespace())(input)?;

    // Parse mount options
    let mount = parse_mount_options(mount_str);
    Ok((input, mount))
}

/// Parse mount options string.
fn parse_mount_options(s: &str) -> RunMount {
    let opts: std::collections::HashMap<&str, &str> = s
        .split(',')
        .filter_map(|part| {
            let mut parts = part.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            Some((key, value))
        })
        .collect();

    let mount_type = opts.get("type").copied().unwrap_or("bind");

    match mount_type {
        "cache" => RunMount::Cache(CacheOpts {
            target: opts.get("target").map(|s| s.to_string()),
            id: opts.get("id").map(|s| s.to_string()),
            sharing: opts.get("sharing").map(|s| s.to_string()),
            from: opts.get("from").map(|s| s.to_string()),
            source: opts.get("source").map(|s| s.to_string()),
            mode: opts.get("mode").map(|s| s.to_string()),
            uid: opts.get("uid").and_then(|s| s.parse().ok()),
            gid: opts.get("gid").and_then(|s| s.parse().ok()),
            read_only: opts.get("ro").is_some() || opts.get("readonly").is_some(),
        }),
        "tmpfs" => RunMount::Tmpfs(TmpOpts {
            target: opts.get("target").map(|s| s.to_string()),
            size: opts.get("size").map(|s| s.to_string()),
        }),
        "secret" => RunMount::Secret(SecretOpts {
            id: opts.get("id").map(|s| s.to_string()),
            target: opts.get("target").map(|s| s.to_string()),
            required: opts.get("required").map(|s| *s == "true").unwrap_or(false),
            mode: opts.get("mode").map(|s| s.to_string()),
            uid: opts.get("uid").and_then(|s| s.parse().ok()),
            gid: opts.get("gid").and_then(|s| s.parse().ok()),
        }),
        "ssh" => RunMount::Ssh(SshOpts {
            id: opts.get("id").map(|s| s.to_string()),
            target: opts.get("target").map(|s| s.to_string()),
            required: opts.get("required").map(|s| *s == "true").unwrap_or(false),
            mode: opts.get("mode").map(|s| s.to_string()),
            uid: opts.get("uid").and_then(|s| s.parse().ok()),
            gid: opts.get("gid").and_then(|s| s.parse().ok()),
        }),
        _ => RunMount::Bind(BindOpts {
            target: opts.get("target").map(|s| s.to_string()),
            source: opts.get("source").map(|s| s.to_string()),
            from: opts.get("from").map(|s| s.to_string()),
            read_only: opts.get("ro").is_some() || opts.get("readonly").is_some(),
        }),
    }
}

/// Parse arguments (exec form or shell form).
fn parse_arguments(input: &str) -> IResult<&str, Arguments> {
    // Try exec form first
    if let Ok((remaining, list)) = parse_json_array(input) {
        return Ok((remaining, Arguments::List(list)));
    }

    // Fall back to shell form
    Ok(("", Arguments::Text(input.trim().to_string())))
}

/// Parse JSON array for exec form.
fn parse_json_array(input: &str) -> IResult<&str, Vec<String>> {
    let (input, _) = char('[')(input)?;
    let (input, _) = space0(input)?;
    let (input, items) = separated_list0(
        tuple((space0, char(','), space0)),
        parse_json_string,
    )(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(']')(input)?;
    Ok((input, items))
}

/// Parse a JSON string.
fn parse_json_string(input: &str) -> IResult<&str, String> {
    let (input, _) = char('"')(input)?;
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    let mut consumed = 0;

    while let Some(c) = chars.next() {
        consumed += c.len_utf8();
        if c == '"' {
            return Ok((&input[consumed..], result));
        } else if c == '\\' {
            if let Some(next) = chars.next() {
                consumed += next.len_utf8();
                match next {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    _ => {
                        result.push('\\');
                        result.push(next);
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Char)))
}

/// Parse COPY instruction.
fn parse_copy(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("COPY")(input)?;
    let (input, _) = space0(input)?;

    // Parse flags
    let (input, flags) = parse_copy_flags(input)?;
    let (input, _) = space0(input)?;

    // Parse sources and destination
    let (input, args) = parse_copy_args(input)?;

    Ok((input, Instruction::Copy(args, flags)))
}

/// Parse COPY flags.
fn parse_copy_flags(input: &str) -> IResult<&str, CopyFlags> {
    let mut flags = CopyFlags::default();
    let mut remaining = input;

    loop {
        let (input, _) = space0(remaining)?;

        if let Ok((input, from)) = parse_flag_value(input, "--from") {
            flags.from = Some(from.to_string());
            remaining = input;
            continue;
        }
        if let Ok((input, chown)) = parse_flag_value(input, "--chown") {
            flags.chown = Some(chown.to_string());
            remaining = input;
            continue;
        }
        if let Ok((input, chmod)) = parse_flag_value(input, "--chmod") {
            flags.chmod = Some(chmod.to_string());
            remaining = input;
            continue;
        }
        if let Ok((input, _)) = tag::<&str, &str, nom::error::Error<&str>>("--link")(input) {
            flags.link = true;
            remaining = input;
            continue;
        }

        break;
    }

    Ok((remaining, flags))
}

/// Parse COPY arguments.
fn parse_copy_args(input: &str) -> IResult<&str, CopyArgs> {
    // Try exec form first
    if let Ok((remaining, items)) = parse_json_array(input) {
        if items.len() >= 2 {
            let dest = items.last().unwrap().clone();
            let sources = items[..items.len() - 1].to_vec();
            return Ok((remaining, CopyArgs::new(sources, dest)));
        }
    }

    // Shell form: space-separated paths
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() >= 2 {
        let dest = parts.last().unwrap().to_string();
        let sources: Vec<String> = parts[..parts.len() - 1].iter().map(|s| s.to_string()).collect();
        Ok(("", CopyArgs::new(sources, dest)))
    } else if parts.len() == 1 {
        // Single argument - treat as both source and dest
        Ok(("", CopyArgs::new(vec![parts[0].to_string()], parts[0])))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Space)))
    }
}

/// Parse ADD instruction.
fn parse_add(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("ADD")(input)?;
    let (input, _) = space0(input)?;

    // Parse flags
    let (input, flags) = parse_add_flags(input)?;
    let (input, _) = space0(input)?;

    // Parse sources and destination (same as COPY)
    let (input, copy_args) = parse_copy_args(input)?;
    let args = AddArgs::new(copy_args.sources, copy_args.dest);

    Ok((input, Instruction::Add(args, flags)))
}

/// Parse ADD flags.
fn parse_add_flags(input: &str) -> IResult<&str, AddFlags> {
    let mut flags = AddFlags::default();
    let mut remaining = input;

    loop {
        let (input, _) = space0(remaining)?;

        if let Ok((input, chown)) = parse_flag_value(input, "--chown") {
            flags.chown = Some(chown.to_string());
            remaining = input;
            continue;
        }
        if let Ok((input, chmod)) = parse_flag_value(input, "--chmod") {
            flags.chmod = Some(chmod.to_string());
            remaining = input;
            continue;
        }
        if let Ok((input, checksum)) = parse_flag_value(input, "--checksum") {
            flags.checksum = Some(checksum.to_string());
            remaining = input;
            continue;
        }
        if let Ok((input, _)) = tag::<&str, &str, nom::error::Error<&str>>("--link")(input) {
            flags.link = true;
            remaining = input;
            continue;
        }

        break;
    }

    Ok((remaining, flags))
}

/// Parse ENV instruction.
fn parse_env(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("ENV")(input)?;
    let (input, _) = space1(input)?;

    // ENV can be KEY=VALUE or KEY VALUE
    let pairs = parse_key_value_pairs(input);
    Ok(("", Instruction::Env(pairs)))
}

/// Parse LABEL instruction.
fn parse_label(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("LABEL")(input)?;
    let (input, _) = space1(input)?;

    let pairs = parse_key_value_pairs(input);
    Ok(("", Instruction::Label(pairs)))
}

/// Parse key=value pairs.
fn parse_key_value_pairs(input: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut remaining = input.trim();

    while !remaining.is_empty() {
        // Find key
        let key_end = remaining.find(|c: char| c == '=' || c.is_whitespace()).unwrap_or(remaining.len());
        if key_end == 0 {
            remaining = remaining.trim_start();
            continue;
        }

        let key = &remaining[..key_end];
        remaining = &remaining[key_end..];

        // Check for = sign
        if remaining.starts_with('=') {
            remaining = &remaining[1..];
            // Parse value
            let value = if remaining.starts_with('"') {
                // Quoted value
                let end = find_closing_quote(remaining);
                let val = &remaining[1..end];
                remaining = &remaining[end + 1..];
                val.to_string()
            } else {
                // Unquoted value
                let end = remaining.find(|c: char| c.is_whitespace()).unwrap_or(remaining.len());
                let val = &remaining[..end];
                remaining = &remaining[end..];
                val.to_string()
            };
            pairs.push((key.to_string(), value));
        } else {
            // Legacy format: KEY VALUE (no =)
            remaining = remaining.trim_start();
            if !remaining.is_empty() {
                let value = if remaining.starts_with('"') {
                    let end = find_closing_quote(remaining);
                    let val = &remaining[1..end];
                    remaining = &remaining[end + 1..];
                    val.to_string()
                } else {
                    remaining.to_string()
                };
                pairs.push((key.to_string(), value.trim().to_string()));
                break;
            }
        }

        remaining = remaining.trim_start();
    }

    pairs
}

/// Find closing quote position.
fn find_closing_quote(s: &str) -> usize {
    let mut escaped = false;
    for (i, c) in s.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return i;
        }
    }
    s.len() - 1
}

/// Parse EXPOSE instruction.
fn parse_expose(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("EXPOSE")(input)?;
    let (input, _) = space1(input)?;

    let mut ports = Vec::new();
    for part in input.split_whitespace() {
        if let Some(port) = parse_port_spec(part) {
            ports.push(port);
        }
    }

    Ok(("", Instruction::Expose(ports)))
}

/// Parse a port specification like "80", "80/tcp", "53/udp".
fn parse_port_spec(s: &str) -> Option<Port> {
    let parts: Vec<&str> = s.split('/').collect();
    let port_num: u16 = parts[0].parse().ok()?;
    let protocol = parts.get(1).map(|p| {
        if p.eq_ignore_ascii_case("udp") {
            PortProtocol::Udp
        } else {
            PortProtocol::Tcp
        }
    }).unwrap_or(PortProtocol::Tcp);

    Some(Port { number: port_num, protocol })
}

/// Parse ARG instruction.
fn parse_arg(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("ARG")(input)?;
    let (input, _) = space1(input)?;

    let content = input.trim();
    if let Some(eq_pos) = content.find('=') {
        let name = content[..eq_pos].to_string();
        let default = content[eq_pos + 1..].to_string();
        Ok(("", Instruction::Arg(name, Some(default))))
    } else {
        Ok(("", Instruction::Arg(content.to_string(), None)))
    }
}

/// Parse ENTRYPOINT instruction.
fn parse_entrypoint(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("ENTRYPOINT")(input)?;
    let (input, _) = space0(input)?;

    let (input, arguments) = parse_arguments(input)?;
    Ok((input, Instruction::Entrypoint(arguments)))
}

/// Parse CMD instruction.
fn parse_cmd(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("CMD")(input)?;
    let (input, _) = space0(input)?;

    let (input, arguments) = parse_arguments(input)?;
    Ok((input, Instruction::Cmd(arguments)))
}

/// Parse SHELL instruction.
fn parse_shell(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("SHELL")(input)?;
    let (input, _) = space0(input)?;

    let (input, arguments) = parse_arguments(input)?;
    Ok((input, Instruction::Shell(arguments)))
}

/// Parse USER instruction.
fn parse_user(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("USER")(input)?;
    let (input, _) = space1(input)?;

    Ok(("", Instruction::User(input.trim().to_string())))
}

/// Parse WORKDIR instruction.
fn parse_workdir(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("WORKDIR")(input)?;
    let (input, _) = space1(input)?;

    Ok(("", Instruction::Workdir(input.trim().to_string())))
}

/// Parse VOLUME instruction.
fn parse_volume(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("VOLUME")(input)?;
    let (input, _) = space1(input)?;

    // VOLUME can be JSON array or space-separated
    // For simplicity, store as single string
    Ok(("", Instruction::Volume(input.trim().to_string())))
}

/// Parse MAINTAINER instruction (deprecated).
fn parse_maintainer(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("MAINTAINER")(input)?;
    let (input, _) = space1(input)?;

    Ok(("", Instruction::Maintainer(input.trim().to_string())))
}

/// Parse HEALTHCHECK instruction.
fn parse_healthcheck(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("HEALTHCHECK")(input)?;
    let (input, _) = space1(input)?;

    let content = input.trim();

    // Check for NONE
    if content.eq_ignore_ascii_case("NONE") {
        return Ok(("", Instruction::Healthcheck(HealthCheck::None)));
    }

    // Parse options
    let mut interval = None;
    let mut timeout = None;
    let mut start_period = None;
    let mut retries = None;
    let mut remaining = content;

    loop {
        remaining = remaining.trim_start();
        if remaining.starts_with("--interval=") {
            let value_start = 11;
            let value_end = remaining[value_start..].find(' ').map(|i| value_start + i).unwrap_or(remaining.len());
            interval = Some(remaining[value_start..value_end].to_string());
            remaining = &remaining[value_end..];
        } else if remaining.starts_with("--timeout=") {
            let value_start = 10;
            let value_end = remaining[value_start..].find(' ').map(|i| value_start + i).unwrap_or(remaining.len());
            timeout = Some(remaining[value_start..value_end].to_string());
            remaining = &remaining[value_end..];
        } else if remaining.starts_with("--start-period=") {
            let value_start = 15;
            let value_end = remaining[value_start..].find(' ').map(|i| value_start + i).unwrap_or(remaining.len());
            start_period = Some(remaining[value_start..value_end].to_string());
            remaining = &remaining[value_end..];
        } else if remaining.starts_with("--retries=") {
            let value_start = 10;
            let value_end = remaining[value_start..].find(' ').map(|i| value_start + i).unwrap_or(remaining.len());
            retries = remaining[value_start..value_end].parse().ok();
            remaining = &remaining[value_end..];
        } else {
            break;
        }
    }

    // Parse CMD
    remaining = remaining.trim_start();
    if remaining.to_uppercase().starts_with("CMD") {
        remaining = &remaining[3..].trim_start();
    }

    let (_, arguments) = parse_arguments(remaining)?;

    Ok(("", Instruction::Healthcheck(HealthCheck::Cmd {
        cmd: arguments,
        interval,
        timeout,
        start_period,
        retries,
    })))
}

/// Parse ONBUILD instruction.
fn parse_onbuild(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("ONBUILD")(input)?;
    let (input, _) = space1(input)?;

    let (remaining, inner) = parse_instruction(input)?;
    Ok((remaining, Instruction::OnBuild(Box::new(inner))))
}

/// Parse STOPSIGNAL instruction.
fn parse_stopsignal(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = tag_no_case("STOPSIGNAL")(input)?;
    let (input, _) = space1(input)?;

    Ok(("", Instruction::Stopsignal(input.trim().to_string())))
}

/// Parse comment.
fn parse_comment(input: &str) -> IResult<&str, Instruction> {
    let (input, _) = char('#')(input)?;
    Ok(("", Instruction::Comment(input.trim().to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_from_simple() {
        let result = parse_dockerfile("FROM ubuntu").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0].instruction {
            Instruction::From(base) => {
                assert_eq!(base.image.name, "ubuntu");
                assert!(base.tag.is_none());
            }
            _ => panic!("Expected FROM instruction"),
        }
    }

    #[test]
    fn test_parse_from_with_tag() {
        let result = parse_dockerfile("FROM ubuntu:20.04").unwrap();
        match &result[0].instruction {
            Instruction::From(base) => {
                assert_eq!(base.image.name, "ubuntu");
                assert_eq!(base.tag, Some("20.04".to_string()));
            }
            _ => panic!("Expected FROM instruction"),
        }
    }

    #[test]
    fn test_parse_from_with_alias() {
        let result = parse_dockerfile("FROM ubuntu:20.04 AS builder").unwrap();
        match &result[0].instruction {
            Instruction::From(base) => {
                assert_eq!(base.image.name, "ubuntu");
                assert_eq!(base.alias.as_ref().map(|a| a.as_str()), Some("builder"));
            }
            _ => panic!("Expected FROM instruction"),
        }
    }

    #[test]
    fn test_parse_run_shell() {
        let result = parse_dockerfile("RUN apt-get update && apt-get install -y nginx").unwrap();
        match &result[0].instruction {
            Instruction::Run(args) => {
                assert!(args.arguments.is_shell_form());
                assert!(args.arguments.as_text().unwrap().contains("apt-get"));
            }
            _ => panic!("Expected RUN instruction"),
        }
    }

    #[test]
    fn test_parse_run_exec() {
        let result = parse_dockerfile(r#"RUN ["apt-get", "update"]"#).unwrap();
        match &result[0].instruction {
            Instruction::Run(args) => {
                assert!(args.arguments.is_exec_form());
                let list = args.arguments.as_list().unwrap();
                assert_eq!(list[0], "apt-get");
                assert_eq!(list[1], "update");
            }
            _ => panic!("Expected RUN instruction"),
        }
    }

    #[test]
    fn test_parse_copy() {
        let result = parse_dockerfile("COPY src/ /app/").unwrap();
        match &result[0].instruction {
            Instruction::Copy(args, _) => {
                assert_eq!(args.sources, vec!["src/"]);
                assert_eq!(args.dest, "/app/");
            }
            _ => panic!("Expected COPY instruction"),
        }
    }

    #[test]
    fn test_parse_copy_with_from() {
        let result = parse_dockerfile("COPY --from=builder /app/dist /app/").unwrap();
        match &result[0].instruction {
            Instruction::Copy(args, flags) => {
                assert_eq!(flags.from, Some("builder".to_string()));
                assert_eq!(args.sources, vec!["/app/dist"]);
                assert_eq!(args.dest, "/app/");
            }
            _ => panic!("Expected COPY instruction"),
        }
    }

    #[test]
    fn test_parse_env() {
        let result = parse_dockerfile("ENV NODE_ENV=production").unwrap();
        match &result[0].instruction {
            Instruction::Env(pairs) => {
                assert_eq!(pairs.len(), 1);
                assert_eq!(pairs[0].0, "NODE_ENV");
                assert_eq!(pairs[0].1, "production");
            }
            _ => panic!("Expected ENV instruction"),
        }
    }

    #[test]
    fn test_parse_expose() {
        let result = parse_dockerfile("EXPOSE 80 443/tcp 53/udp").unwrap();
        match &result[0].instruction {
            Instruction::Expose(ports) => {
                assert_eq!(ports.len(), 3);
                assert_eq!(ports[0].number, 80);
                assert_eq!(ports[1].number, 443);
                assert_eq!(ports[2].number, 53);
                assert_eq!(ports[2].protocol, PortProtocol::Udp);
            }
            _ => panic!("Expected EXPOSE instruction"),
        }
    }

    #[test]
    fn test_parse_workdir() {
        let result = parse_dockerfile("WORKDIR /app").unwrap();
        match &result[0].instruction {
            Instruction::Workdir(path) => {
                assert_eq!(path, "/app");
            }
            _ => panic!("Expected WORKDIR instruction"),
        }
    }

    #[test]
    fn test_parse_user() {
        let result = parse_dockerfile("USER node").unwrap();
        match &result[0].instruction {
            Instruction::User(user) => {
                assert_eq!(user, "node");
            }
            _ => panic!("Expected USER instruction"),
        }
    }

    #[test]
    fn test_parse_comment() {
        let result = parse_dockerfile("# This is a comment").unwrap();
        match &result[0].instruction {
            Instruction::Comment(text) => {
                assert_eq!(text, "This is a comment");
            }
            _ => panic!("Expected Comment"),
        }
    }

    #[test]
    fn test_parse_full_dockerfile() {
        let dockerfile = r#"
FROM node:18-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:18-alpine
WORKDIR /app
COPY --from=builder /app/dist ./dist
EXPOSE 3000
CMD ["node", "dist/index.js"]
"#;

        let result = parse_dockerfile(dockerfile).unwrap();
        // Should have multiple instructions
        assert!(result.len() >= 10);
    }

    #[test]
    fn test_line_continuation() {
        let dockerfile = r#"RUN apt-get update && \
    apt-get install -y nginx"#;

        let result = parse_dockerfile(dockerfile).unwrap();
        assert_eq!(result.len(), 1);
        match &result[0].instruction {
            Instruction::Run(args) => {
                let text = args.arguments.as_text().unwrap();
                assert!(text.contains("apt-get update"));
                assert!(text.contains("apt-get install"));
            }
            _ => panic!("Expected RUN instruction"),
        }
    }

    #[test]
    fn test_image_with_registry() {
        let result = parse_dockerfile("FROM gcr.io/my-project/my-image:latest").unwrap();
        match &result[0].instruction {
            Instruction::From(base) => {
                assert_eq!(base.image.registry, Some("gcr.io".to_string()));
                assert_eq!(base.image.name, "my-project/my-image");
                assert_eq!(base.tag, Some("latest".to_string()));
            }
            _ => panic!("Expected FROM instruction"),
        }
    }
}
