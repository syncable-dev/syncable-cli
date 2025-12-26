//! Shell parsing module for hadolint-rs.
//!
//! Provides:
//! - Shell command extraction from RUN instructions
//! - ShellCheck integration for deeper analysis
//!
//! This module handles parsing shell commands from Dockerfile RUN instructions
//! and provides utilities for rule implementations to analyze them.

pub mod shellcheck;

use crate::analyzer::hadolint::parser::instruction::{Arguments, RunArgs};

/// Parsed shell command information.
#[derive(Debug, Clone, Default)]
pub struct ParsedShell {
    /// Original shell script text.
    pub original: String,
    /// Extracted commands.
    pub commands: Vec<Command>,
    /// Whether the shell has pipes.
    pub has_pipes: bool,
}

impl ParsedShell {
    /// Parse a shell command string.
    pub fn parse(script: &str) -> Self {
        let original = script.to_string();
        let commands = extract_commands(script);
        let has_pipes = script.contains('|');

        Self {
            original,
            commands,
            has_pipes,
        }
    }

    /// Parse from RUN instruction arguments.
    pub fn from_run_args(args: &RunArgs) -> Self {
        match &args.arguments {
            Arguments::Text(text) => Self::parse(text),
            Arguments::List(list) => {
                // Exec form - join for analysis
                let script = list.join(" ");
                Self::parse(&script)
            }
        }
    }

    /// Check if any command matches the predicate.
    pub fn any_command<F>(&self, pred: F) -> bool
    where
        F: Fn(&Command) -> bool,
    {
        self.commands.iter().any(pred)
    }

    /// Check if all commands match the predicate.
    pub fn all_commands<F>(&self, pred: F) -> bool
    where
        F: Fn(&Command) -> bool,
    {
        self.commands.iter().all(pred)
    }

    /// Check if no commands match the predicate.
    pub fn no_commands<F>(&self, pred: F) -> bool
    where
        F: Fn(&Command) -> bool,
    {
        !self.any_command(pred)
    }

    /// Find command names in the script.
    pub fn find_command_names(&self) -> Vec<&str> {
        self.commands.iter().map(|c| c.name.as_str()).collect()
    }

    /// Check if using a specific program.
    pub fn using_program(&self, prog: &str) -> bool {
        self.commands.iter().any(|c| c.name == prog)
    }

    /// Check if any command is a pip install.
    pub fn is_pip_install(&self, cmd: &Command) -> bool {
        cmd.is_pip_install()
    }
}

/// A single command extracted from a shell script.
#[derive(Debug, Clone)]
pub struct Command {
    /// Command name (e.g., "apt-get", "pip").
    pub name: String,
    /// All arguments including flags.
    pub arguments: Vec<String>,
    /// Extracted flags (e.g., ["-y", "--no-cache"]).
    pub flags: Vec<String>,
}

impl Command {
    /// Create a new command.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: Vec::new(),
            flags: Vec::new(),
        }
    }

    /// Check if the command has specific arguments.
    pub fn has_args(&self, expected_name: &str, expected_args: &[&str]) -> bool {
        if self.name != expected_name {
            return false;
        }
        expected_args
            .iter()
            .all(|arg| self.arguments.iter().any(|a| a == *arg))
    }

    /// Check if the command has any of the specified arguments.
    pub fn has_any_arg(&self, args: &[&str]) -> bool {
        args.iter()
            .any(|arg| self.arguments.iter().any(|a| a == *arg))
    }

    /// Check if the command has a specific flag.
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }

    /// Check if the command has any of the specified flags.
    pub fn has_any_flag(&self, flags: &[&str]) -> bool {
        flags.iter().any(|f| self.has_flag(f))
    }

    /// Get arguments without flags.
    pub fn args_no_flags(&self) -> Vec<&str> {
        self.arguments
            .iter()
            .filter(|a| !a.starts_with('-'))
            .map(|s| s.as_str())
            .collect()
    }

    /// Get the value for a flag (e.g., "-t" returns "release" for "-t=release").
    pub fn get_flag_value(&self, flag: &str) -> Option<&str> {
        // Check for --flag=value format
        for arg in &self.arguments {
            if let Some(stripped) = arg.strip_prefix(&format!("--{}=", flag)) {
                return Some(stripped);
            }
            if let Some(stripped) = arg.strip_prefix(&format!("-{}=", flag)) {
                return Some(stripped);
            }
        }

        // Check for --flag value format
        let mut iter = self.arguments.iter();
        while let Some(arg) = iter.next() {
            if arg == &format!("--{}", flag) || arg == &format!("-{}", flag) {
                return iter.next().map(|s| s.as_str());
            }
        }

        None
    }

    /// Check if this is a pip install command.
    pub fn is_pip_install(&self) -> bool {
        // Standard pip install
        if (self.name.starts_with("pip") && !self.name.starts_with("pipenv"))
            && self.arguments.iter().any(|a| a == "install")
        {
            return true;
        }

        // python -m pip install
        if self.name.starts_with("python") {
            let args: Vec<&str> = self.arguments.iter().map(|s| s.as_str()).collect();
            if args.windows(3).any(|w| w == ["-m", "pip", "install"]) {
                return true;
            }
        }

        false
    }

    /// Check if this is an apt-get install command.
    pub fn is_apt_get_install(&self) -> bool {
        self.name == "apt-get" && self.arguments.iter().any(|a| a == "install")
    }

    /// Check if this is an apk add command.
    pub fn is_apk_add(&self) -> bool {
        self.name == "apk" && self.arguments.iter().any(|a| a == "add")
    }
}

/// Extract commands from a shell script.
fn extract_commands(script: &str) -> Vec<Command> {
    let mut commands = Vec::new();

    // Simple tokenization: split by command separators
    let separators = ["&&", "||", ";", "|", "\n"];

    let mut remaining = script.trim();

    while !remaining.is_empty() {
        // Find the next separator
        let next_sep = separators
            .iter()
            .filter_map(|sep| remaining.find(sep).map(|pos| (pos, sep.len())))
            .min_by_key(|(pos, _)| *pos);

        let cmd_str = match next_sep {
            Some((pos, len)) => {
                let cmd = &remaining[..pos];
                remaining = &remaining[pos + len..];
                cmd
            }
            None => {
                let cmd = remaining;
                remaining = "";
                cmd
            }
        };

        // Parse the command
        if let Some(cmd) = parse_single_command(cmd_str.trim()) {
            commands.push(cmd);
        }

        remaining = remaining.trim_start();
    }

    commands
}

/// Parse a single command string into a Command.
fn parse_single_command(cmd_str: &str) -> Option<Command> {
    let cmd_str = cmd_str.trim();
    if cmd_str.is_empty() {
        return None;
    }

    // Handle subshells and command substitution
    let cmd_str = cmd_str.trim_start_matches('(').trim_end_matches(')').trim();

    // Simple word splitting
    let words: Vec<&str> = shell_words(cmd_str);

    if words.is_empty() {
        return None;
    }

    let name = words[0].to_string();
    let arguments: Vec<String> = words[1..].iter().map(|s| s.to_string()).collect();
    let flags = extract_flags(&arguments);

    Some(Command {
        name,
        arguments,
        flags,
    })
}

/// Simple shell word splitting.
fn shell_words(input: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut word_start = None;
    let mut escaped = false;

    for (i, c) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if c == '\\' && !in_single_quote {
            escaped = true;
            if word_start.is_none() {
                word_start = Some(i);
            }
            continue;
        }

        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            if word_start.is_none() {
                word_start = Some(i);
            }
            continue;
        }

        if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            if word_start.is_none() {
                word_start = Some(i);
            }
            continue;
        }

        if c.is_whitespace() && !in_single_quote && !in_double_quote {
            if let Some(start) = word_start {
                let word = &input[start..i];
                let word = word.trim_matches(|c| c == '\'' || c == '"');
                if !word.is_empty() {
                    words.push(word);
                }
                word_start = None;
            }
        } else if word_start.is_none() {
            word_start = Some(i);
        }
    }

    // Don't forget the last word
    if let Some(start) = word_start {
        let word = &input[start..];
        let word = word.trim_matches(|c| c == '\'' || c == '"');
        if !word.is_empty() {
            words.push(word);
        }
    }

    words
}

/// Extract flags from arguments.
fn extract_flags(arguments: &[String]) -> Vec<String> {
    let mut flags = Vec::new();

    for arg in arguments {
        if arg == "--" || arg == "-" {
            continue;
        }

        if let Some(stripped) = arg.strip_prefix("--") {
            // Long flag
            let flag = stripped.split('=').next().unwrap_or(stripped);
            flags.push(flag.to_string());
        } else if let Some(stripped) = arg.strip_prefix('-') {
            // Short flag(s)
            for c in stripped.chars() {
                if c == '=' {
                    break;
                }
                flags.push(c.to_string());
            }
        }
    }

    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let shell = ParsedShell::parse("apt-get update");
        assert_eq!(shell.commands.len(), 1);
        assert_eq!(shell.commands[0].name, "apt-get");
        assert_eq!(shell.commands[0].arguments, vec!["update"]);
    }

    #[test]
    fn test_parse_chained_commands() {
        let shell = ParsedShell::parse("apt-get update && apt-get install -y nginx");
        assert_eq!(shell.commands.len(), 2);
        assert_eq!(shell.commands[0].name, "apt-get");
        assert_eq!(shell.commands[1].name, "apt-get");
        assert!(shell.commands[1].has_flag("y"));
    }

    #[test]
    fn test_parse_pipe() {
        let shell = ParsedShell::parse("cat file | grep pattern");
        assert!(shell.has_pipes);
        assert_eq!(shell.commands.len(), 2);
    }

    #[test]
    fn test_command_has_args() {
        let cmd = Command {
            name: "apt-get".to_string(),
            arguments: vec!["install".to_string(), "-y".to_string(), "nginx".to_string()],
            flags: vec!["y".to_string()],
        };

        assert!(cmd.has_args("apt-get", &["install"]));
        assert!(cmd.has_flag("y"));
        assert!(!cmd.has_flag("q"));
    }

    #[test]
    fn test_is_pip_install() {
        let cmd = Command {
            name: "pip".to_string(),
            arguments: vec!["install".to_string(), "requests".to_string()],
            flags: vec![],
        };
        assert!(cmd.is_pip_install());

        let cmd2 = Command {
            name: "pipenv".to_string(),
            arguments: vec!["install".to_string()],
            flags: vec![],
        };
        assert!(!cmd2.is_pip_install());
    }

    #[test]
    fn test_is_apt_get_install() {
        let cmd = Command {
            name: "apt-get".to_string(),
            arguments: vec!["install".to_string(), "-y".to_string(), "nginx".to_string()],
            flags: vec!["y".to_string()],
        };
        assert!(cmd.is_apt_get_install());
    }

    #[test]
    fn test_args_no_flags() {
        let cmd = Command {
            name: "apt-get".to_string(),
            arguments: vec![
                "install".to_string(),
                "-y".to_string(),
                "nginx".to_string(),
                "curl".to_string(),
            ],
            flags: vec!["y".to_string()],
        };

        let args = cmd.args_no_flags();
        assert_eq!(args, vec!["install", "nginx", "curl"]);
    }

    #[test]
    fn test_using_program() {
        let shell = ParsedShell::parse("apt-get update && curl -O http://example.com/file");
        assert!(shell.using_program("apt-get"));
        assert!(shell.using_program("curl"));
        assert!(!shell.using_program("wget"));
    }
}
