//! Interactive confirmation UI for shell commands and file operations
//!
//! Provides Claude Code-style confirmation prompts before executing
//! potentially destructive operations.

use colored::Colorize;
use inquire::ui::{Color, IndexPrefix, RenderConfig, StyleSheet, Styled};
use inquire::{InquireError, Select, Text};
use std::collections::HashSet;
use std::sync::Mutex;

/// Get custom render config for confirmation prompts
fn get_confirmation_render_config() -> RenderConfig<'static> {
    RenderConfig::default()
        .with_highlighted_option_prefix(Styled::new("> ").with_fg(Color::LightCyan))
        .with_option_index_prefix(IndexPrefix::Simple)
        .with_selected_option(Some(StyleSheet::new().with_fg(Color::LightCyan)))
        .with_scroll_up_prefix(Styled::new("▲ "))
        .with_scroll_down_prefix(Styled::new("▼ "))
}

/// Result of a user confirmation prompt
#[derive(Debug, Clone)]
pub enum ConfirmationResult {
    /// User approved, proceed with the operation
    Proceed,
    /// User approved and wants to skip future prompts for similar commands
    ProceedAlways(String), // The command prefix to allow always
    /// User wants to provide alternative instructions
    Modify(String),
    /// User cancelled (Esc or Ctrl+C)
    Cancel,
}

/// Session-level tracking of always-allowed commands
#[derive(Debug)]
pub struct AllowedCommands {
    prefixes: Mutex<HashSet<String>>,
}

impl AllowedCommands {
    pub fn new() -> Self {
        Self {
            prefixes: Mutex::new(HashSet::new()),
        }
    }

    /// Check if a command prefix is already allowed
    pub fn is_allowed(&self, command: &str) -> bool {
        let prefixes = self.prefixes.lock().unwrap();
        prefixes.iter().any(|prefix| command.starts_with(prefix))
    }

    /// Add a command prefix to the allowed list
    pub fn allow(&self, prefix: String) {
        let mut prefixes = self.prefixes.lock().unwrap();
        prefixes.insert(prefix);
    }
}

impl Default for AllowedCommands {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract the command prefix (first word or first two words for compound commands)
fn extract_command_prefix(command: &str) -> String {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return command.to_string();
    }

    // For compound commands like "docker build", "npm run", use first two words
    let compound_commands = ["docker", "terraform", "helm", "kubectl", "npm", "cargo", "go"];
    if parts.len() >= 2 && compound_commands.contains(&parts[0]) {
        format!("{} {}", parts[0], parts[1])
    } else {
        parts[0].to_string()
    }
}

/// Display a command confirmation box
fn display_command_box(command: &str, working_dir: &str) {
    let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
    let box_width = term_width.min(70);
    let inner_width = box_width - 4; // Account for borders and padding

    // Top border
    println!(
        "{}",
        format!(
            "{}{}{}",
            "┌─ Bash command ".dimmed(),
            "─".repeat(inner_width.saturating_sub(15)).dimmed(),
            "┐".dimmed()
        )
    );

    // Command content (may wrap)
    let command_lines = textwrap::wrap(command, inner_width - 2);
    for line in &command_lines {
        println!(
            "{}  {}{}",
            "│".dimmed(),
            line.cyan().bold(),
            " ".repeat(inner_width.saturating_sub(line.len() + 2))
        );
    }

    // Working directory
    let dir_display = format!("in {}", working_dir);
    println!(
        "{}  {}{}{}",
        "│".dimmed(),
        dir_display.dimmed(),
        " ".repeat(inner_width.saturating_sub(dir_display.len() + 2)),
        "│".dimmed()
    );

    // Bottom border
    println!(
        "{}",
        format!(
            "{}{}{}",
            "└".dimmed(),
            "─".repeat(box_width - 2).dimmed(),
            "┘".dimmed()
        )
    );
    println!();
}

/// Confirm shell command execution with the user
///
/// Shows the command in a box and presents options:
/// 1. Yes - proceed once
/// 2. Yes, and don't ask again for this command type
/// 3. Type feedback to tell the agent what to do differently
pub fn confirm_shell_command(
    command: &str,
    working_dir: &str,
) -> ConfirmationResult {
    display_command_box(command, working_dir);

    let prefix = extract_command_prefix(command);
    let short_dir = std::path::Path::new(working_dir)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| working_dir.to_string());

    let options = vec![
        format!("Yes"),
        format!("Yes, and don't ask again for `{}` commands in {}", prefix, short_dir),
        format!("Type here to tell Syncable Agent what to do differently"),
    ];

    println!("{}", "Do you want to proceed?".white());

    let selection = Select::new("", options.clone())
        .with_render_config(get_confirmation_render_config())
        .with_page_size(3)  // Show all 3 options
        .with_help_message("↑↓ to move, Enter to select, Esc to cancel")
        .prompt();

    match selection {
        Ok(answer) => {
            if answer == options[0] {
                ConfirmationResult::Proceed
            } else if answer == options[1] {
                ConfirmationResult::ProceedAlways(prefix)
            } else {
                // User wants to type feedback
                println!();
                match Text::new("What should I do instead?")
                    .with_help_message("Press Enter to submit, Esc to cancel")
                    .prompt()
                {
                    Ok(feedback) if !feedback.trim().is_empty() => {
                        ConfirmationResult::Modify(feedback)
                    }
                    _ => ConfirmationResult::Cancel,
                }
            }
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            ConfirmationResult::Cancel
        }
        Err(_) => ConfirmationResult::Cancel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_command_prefix() {
        assert_eq!(extract_command_prefix("docker build -t test ."), "docker build");
        assert_eq!(extract_command_prefix("npm run test"), "npm run");
        assert_eq!(extract_command_prefix("cargo build"), "cargo build");
        assert_eq!(extract_command_prefix("make"), "make");
        assert_eq!(extract_command_prefix("hadolint Dockerfile"), "hadolint");
    }

    #[test]
    fn test_allowed_commands() {
        let allowed = AllowedCommands::new();
        assert!(!allowed.is_allowed("docker build -t test ."));

        allowed.allow("docker build".to_string());
        assert!(allowed.is_allowed("docker build -t test ."));
        assert!(allowed.is_allowed("docker build --no-cache ."));
        assert!(!allowed.is_allowed("docker run test"));
    }
}
