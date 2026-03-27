//! Dockerfile instruction AST types.
//!
//! These types represent the parsed structure of a Dockerfile,
//! matching the Haskell `language-docker` library for compatibility.

use std::collections::HashSet;

/// A positioned instruction with source location information.
#[derive(Debug, Clone, PartialEq)]
pub struct InstructionPos {
    /// The parsed instruction.
    pub instruction: Instruction,
    /// Line number (1-indexed).
    pub line_number: u32,
    /// Original source text of the instruction.
    pub source_text: String,
}

impl InstructionPos {
    /// Create a new positioned instruction.
    pub fn new(instruction: Instruction, line_number: u32, source_text: String) -> Self {
        Self {
            instruction,
            line_number,
            source_text,
        }
    }
}

/// Dockerfile instructions.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// FROM instruction
    From(BaseImage),
    /// RUN instruction
    Run(RunArgs),
    /// COPY instruction
    Copy(CopyArgs, CopyFlags),
    /// ADD instruction
    Add(AddArgs, AddFlags),
    /// ENV instruction
    Env(Vec<(String, String)>),
    /// LABEL instruction
    Label(Vec<(String, String)>),
    /// EXPOSE instruction
    Expose(Vec<Port>),
    /// ARG instruction
    Arg(String, Option<String>),
    /// ENTRYPOINT instruction
    Entrypoint(Arguments),
    /// CMD instruction
    Cmd(Arguments),
    /// SHELL instruction
    Shell(Arguments),
    /// USER instruction
    User(String),
    /// WORKDIR instruction
    Workdir(String),
    /// VOLUME instruction
    Volume(String),
    /// MAINTAINER instruction (deprecated)
    Maintainer(String),
    /// HEALTHCHECK instruction
    Healthcheck(HealthCheck),
    /// ONBUILD instruction (wraps another instruction)
    OnBuild(Box<Instruction>),
    /// STOPSIGNAL instruction
    Stopsignal(String),
    /// Comment line
    Comment(String),
}

impl Instruction {
    /// Check if this is a FROM instruction.
    pub fn is_from(&self) -> bool {
        matches!(self, Self::From(_))
    }

    /// Check if this is a RUN instruction.
    pub fn is_run(&self) -> bool {
        matches!(self, Self::Run(_))
    }

    /// Check if this is a COPY instruction.
    pub fn is_copy(&self) -> bool {
        matches!(self, Self::Copy(_, _))
    }

    /// Check if this is an ONBUILD instruction.
    pub fn is_onbuild(&self) -> bool {
        matches!(self, Self::OnBuild(_))
    }

    /// Get the wrapped instruction if this is ONBUILD.
    pub fn unwrap_onbuild(&self) -> Option<&Instruction> {
        match self {
            Self::OnBuild(inner) => Some(inner.as_ref()),
            _ => None,
        }
    }
}

/// Base image in FROM instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct BaseImage {
    /// The image reference.
    pub image: Image,
    /// Image tag (e.g., "latest", "3.9").
    pub tag: Option<String>,
    /// Image digest (e.g., "sha256:...").
    pub digest: Option<String>,
    /// Stage alias (AS name).
    pub alias: Option<ImageAlias>,
    /// Target platform (--platform=...).
    pub platform: Option<String>,
}

impl BaseImage {
    /// Create a new base image with just a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            image: Image::new(name),
            tag: None,
            digest: None,
            alias: None,
            platform: None,
        }
    }

    /// Check if the image uses a variable reference.
    pub fn is_variable(&self) -> bool {
        self.image.name.starts_with('$')
    }

    /// Check if this is the scratch image.
    pub fn is_scratch(&self) -> bool {
        self.image.name.eq_ignore_ascii_case("scratch")
    }

    /// Check if the image has an explicit tag or digest.
    pub fn has_version(&self) -> bool {
        self.tag.is_some() || self.digest.is_some()
    }
}

/// Docker image reference.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// Optional registry (e.g., "docker.io", "gcr.io").
    pub registry: Option<String>,
    /// Image name (e.g., "ubuntu", "library/ubuntu").
    pub name: String,
}

impl Image {
    /// Create a new image with just a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            registry: None,
            name: name.into(),
        }
    }

    /// Create a new image with registry.
    pub fn with_registry(registry: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            registry: Some(registry.into()),
            name: name.into(),
        }
    }

    /// Get the full image reference.
    pub fn full_name(&self) -> String {
        match &self.registry {
            Some(reg) => format!("{}/{}", reg, self.name),
            None => self.name.clone(),
        }
    }
}

/// Image alias (AS name in FROM).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageAlias(pub String);

impl ImageAlias {
    /// Create a new image alias.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the alias name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// RUN instruction arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct RunArgs {
    /// The command arguments.
    pub arguments: Arguments,
    /// RUN flags (--mount, --network, etc.).
    pub flags: RunFlags,
}

impl RunArgs {
    /// Create a new RUN with shell form.
    pub fn shell(cmd: impl Into<String>) -> Self {
        Self {
            arguments: Arguments::Text(cmd.into()),
            flags: RunFlags::default(),
        }
    }

    /// Create a new RUN with exec form.
    pub fn exec(args: Vec<String>) -> Self {
        Self {
            arguments: Arguments::List(args),
            flags: RunFlags::default(),
        }
    }
}

/// RUN instruction flags.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RunFlags {
    /// Mount options (--mount=...).
    pub mount: HashSet<RunMount>,
    /// Network mode (--network=...).
    pub network: Option<String>,
    /// Security mode (--security=...).
    pub security: Option<String>,
}

/// RUN mount types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RunMount {
    /// Bind mount
    Bind(BindOpts),
    /// Cache mount
    Cache(CacheOpts),
    /// Tmpfs mount
    Tmpfs(TmpOpts),
    /// Secret mount
    Secret(SecretOpts),
    /// SSH mount
    Ssh(SshOpts),
}

/// Bind mount options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct BindOpts {
    pub target: Option<String>,
    pub source: Option<String>,
    pub from: Option<String>,
    pub read_only: bool,
}

/// Cache mount options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct CacheOpts {
    pub target: Option<String>,
    pub id: Option<String>,
    pub sharing: Option<String>,
    pub from: Option<String>,
    pub source: Option<String>,
    pub mode: Option<String>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub read_only: bool,
}

/// Tmpfs mount options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TmpOpts {
    pub target: Option<String>,
    pub size: Option<String>,
}

/// Secret mount options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SecretOpts {
    pub id: Option<String>,
    pub target: Option<String>,
    pub required: bool,
    pub mode: Option<String>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
}

/// SSH mount options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SshOpts {
    pub id: Option<String>,
    pub target: Option<String>,
    pub required: bool,
    pub mode: Option<String>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
}

/// COPY instruction arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyArgs {
    /// Source paths.
    pub sources: Vec<String>,
    /// Destination path.
    pub dest: String,
}

impl CopyArgs {
    /// Create new copy args.
    pub fn new(sources: Vec<String>, dest: impl Into<String>) -> Self {
        Self {
            sources,
            dest: dest.into(),
        }
    }
}

/// COPY instruction flags.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CopyFlags {
    /// --from=<stage>
    pub from: Option<String>,
    /// --chown=<user:group>
    pub chown: Option<String>,
    /// --chmod=<mode>
    pub chmod: Option<String>,
    /// --link
    pub link: bool,
}

/// ADD instruction arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct AddArgs {
    /// Source paths/URLs.
    pub sources: Vec<String>,
    /// Destination path.
    pub dest: String,
}

impl AddArgs {
    /// Create new add args.
    pub fn new(sources: Vec<String>, dest: impl Into<String>) -> Self {
        Self {
            sources,
            dest: dest.into(),
        }
    }

    /// Check if any source is a URL.
    pub fn has_url(&self) -> bool {
        self.sources
            .iter()
            .any(|s| s.starts_with("http://") || s.starts_with("https://"))
    }

    /// Check if any source appears to be an archive.
    pub fn has_archive(&self) -> bool {
        const ARCHIVE_EXTENSIONS: &[&str] = &[
            ".tar", ".tar.gz", ".tgz", ".tar.bz2", ".tbz2", ".tar.xz", ".txz", ".zip", ".gz",
            ".bz2", ".xz", ".Z", ".lz", ".lzma",
        ];
        self.sources
            .iter()
            .any(|s| ARCHIVE_EXTENSIONS.iter().any(|ext| s.ends_with(ext)))
    }
}

/// ADD instruction flags.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AddFlags {
    /// --chown=<user:group>
    pub chown: Option<String>,
    /// --chmod=<mode>
    pub chmod: Option<String>,
    /// --link
    pub link: bool,
    /// --checksum=<checksum>
    pub checksum: Option<String>,
}

/// Port specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Port {
    /// Port number.
    pub number: u16,
    /// Protocol (tcp/udp).
    pub protocol: PortProtocol,
}

impl Port {
    /// Create a TCP port.
    pub fn tcp(number: u16) -> Self {
        Self {
            number,
            protocol: PortProtocol::Tcp,
        }
    }

    /// Create a UDP port.
    pub fn udp(number: u16) -> Self {
        Self {
            number,
            protocol: PortProtocol::Udp,
        }
    }
}

/// Port protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PortProtocol {
    #[default]
    Tcp,
    Udp,
}

/// Arguments in shell form or exec form.
#[derive(Debug, Clone, PartialEq)]
pub enum Arguments {
    /// Shell form: RUN apt-get update
    Text(String),
    /// Exec form: RUN ["apt-get", "update"]
    List(Vec<String>),
}

impl Arguments {
    /// Check if this is shell form.
    pub fn is_shell_form(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Check if this is exec form.
    pub fn is_exec_form(&self) -> bool {
        matches!(self, Self::List(_))
    }

    /// Get the text if shell form.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Get the list if exec form.
    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            Self::List(v) => Some(v),
            _ => None,
        }
    }

    /// Convert to a single string (for shell form, returns as-is; for exec form, joins with spaces).
    pub fn to_string_lossy(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::List(v) => v.join(" "),
        }
    }
}

/// HEALTHCHECK instruction.
#[derive(Debug, Clone, PartialEq)]
pub enum HealthCheck {
    /// HEALTHCHECK NONE
    None,
    /// HEALTHCHECK CMD ...
    Cmd {
        /// The command to run.
        cmd: Arguments,
        /// Interval between checks.
        interval: Option<String>,
        /// Timeout for each check.
        timeout: Option<String>,
        /// Start period before checks begin.
        start_period: Option<String>,
        /// Number of retries before unhealthy.
        retries: Option<u32>,
    },
}

impl HealthCheck {
    /// Create a HEALTHCHECK CMD with defaults.
    pub fn cmd(cmd: Arguments) -> Self {
        Self::Cmd {
            cmd,
            interval: None,
            timeout: None,
            start_period: None,
            retries: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_image() {
        let img = BaseImage::new("ubuntu");
        assert!(!img.is_scratch());
        assert!(!img.is_variable());
        assert!(!img.has_version());

        let scratch = BaseImage::new("scratch");
        assert!(scratch.is_scratch());

        let var = BaseImage::new("${BASE_IMAGE}");
        assert!(var.is_variable());

        let tagged = BaseImage {
            tag: Some("20.04".to_string()),
            ..BaseImage::new("ubuntu")
        };
        assert!(tagged.has_version());
    }

    #[test]
    fn test_image() {
        let img = Image::new("ubuntu");
        assert_eq!(img.full_name(), "ubuntu");

        let img_with_reg = Image::with_registry("gcr.io", "my-project/my-image");
        assert_eq!(img_with_reg.full_name(), "gcr.io/my-project/my-image");
    }

    #[test]
    fn test_arguments() {
        let shell = Arguments::Text("apt-get update".to_string());
        assert!(shell.is_shell_form());
        assert_eq!(shell.as_text(), Some("apt-get update"));

        let exec = Arguments::List(vec!["apt-get".to_string(), "update".to_string()]);
        assert!(exec.is_exec_form());
        assert_eq!(
            exec.as_list(),
            Some(&["apt-get".to_string(), "update".to_string()][..])
        );
    }

    #[test]
    fn test_add_args() {
        let add = AddArgs::new(vec!["app.tar.gz".to_string()], "/app");
        assert!(add.has_archive());
        assert!(!add.has_url());

        let add_url = AddArgs::new(vec!["https://example.com/file.txt".to_string()], "/app");
        assert!(add_url.has_url());
        assert!(!add_url.has_archive());
    }
}
