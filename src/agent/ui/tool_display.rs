//! Tool call display for visible tool execution feedback
//!
//! Shows tool calls with status indicators, names, descriptions, and results.
//! Includes forge-style output with formatted arguments and tree-like status.

use crate::agent::ui::colors::{ansi, icons};
use colored::Colorize;
use std::io::{self, Write};

/// Status of a tool call
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallStatus {
    Pending,
    Executing,
    Success,
    Error,
    Canceled,
}

impl ToolCallStatus {
    /// Get the icon for this status
    pub fn icon(&self) -> &'static str {
        match self {
            ToolCallStatus::Pending => icons::PENDING,
            ToolCallStatus::Executing => icons::EXECUTING,
            ToolCallStatus::Success => icons::SUCCESS,
            ToolCallStatus::Error => icons::ERROR,
            ToolCallStatus::Canceled => icons::CANCELED,
        }
    }

    /// Get the color code for this status
    pub fn color(&self) -> &'static str {
        match self {
            ToolCallStatus::Pending => ansi::GRAY,
            ToolCallStatus::Executing => ansi::CYAN,
            ToolCallStatus::Success => "\x1b[32m", // Green
            ToolCallStatus::Error => "\x1b[31m",   // Red
            ToolCallStatus::Canceled => ansi::GRAY,
        }
    }
}

/// Represents a tool call for display
#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    pub name: String,
    pub description: String,
    pub status: ToolCallStatus,
    pub result: Option<String>,
    pub error: Option<String>,
}

impl ToolCallInfo {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            status: ToolCallStatus::Pending,
            result: None,
            error: None,
        }
    }

    pub fn executing(mut self) -> Self {
        self.status = ToolCallStatus::Executing;
        self
    }

    pub fn success(mut self, result: Option<String>) -> Self {
        self.status = ToolCallStatus::Success;
        self.result = result;
        self
    }

    pub fn error(mut self, error: String) -> Self {
        self.status = ToolCallStatus::Error;
        self.error = Some(error);
        self
    }
}

/// Display manager for tool calls
pub struct ToolCallDisplay;

impl ToolCallDisplay {
    /// Print a tool call start message
    pub fn print_start(name: &str, description: &str) {
        println!(
            "\n{} {} {}",
            icons::TOOL.cyan(),
            name.cyan().bold(),
            description.dimmed()
        );
        let _ = io::stdout().flush();
    }

    /// Print a tool call with status
    pub fn print_status(info: &ToolCallInfo) {
        let status_icon = info.status.icon();
        let color = info.status.color();

        print!(
            "{}{}{} {} {} {}{}",
            ansi::CLEAR_LINE,
            color,
            status_icon,
            ansi::RESET,
            info.name.cyan().bold(),
            info.description.dimmed(),
            ansi::RESET
        );

        match info.status {
            ToolCallStatus::Success => {
                println!(" {}", "[done]".green());
            }
            ToolCallStatus::Error => {
                if let Some(ref err) = info.error {
                    println!(" {} {}", "[error]".red(), err.red());
                } else {
                    println!(" {}", "[error]".red());
                }
            }
            ToolCallStatus::Canceled => {
                println!(" {}", "[canceled]".yellow());
            }
            _ => {
                println!();
            }
        }

        let _ = io::stdout().flush();
    }

    /// Print a tool call result (for verbose output)
    pub fn print_result(name: &str, result: &str, truncate: bool) {
        let display_result = if truncate && result.len() > 200 {
            format!("{}... (truncated)", &result[..200])
        } else {
            result.to_string()
        };

        println!(
            "  {} {} {}",
            icons::ARROW.dimmed(),
            name.cyan(),
            display_result.dimmed()
        );
        let _ = io::stdout().flush();
    }

    /// Print a summary of tool calls
    pub fn print_summary(tools: &[ToolCallInfo]) {
        if tools.is_empty() {
            return;
        }

        let success_count = tools
            .iter()
            .filter(|t| t.status == ToolCallStatus::Success)
            .count();
        let error_count = tools
            .iter()
            .filter(|t| t.status == ToolCallStatus::Error)
            .count();

        println!();
        if error_count == 0 {
            println!(
                "{} {} tool{} executed successfully",
                icons::SUCCESS.green(),
                success_count,
                if success_count == 1 { "" } else { "s" }
            );
        } else {
            println!(
                "{} {}/{} tools succeeded, {} failed",
                icons::ERROR.red(),
                success_count,
                tools.len(),
                error_count
            );
        }
    }
}

/// Print a tool call inline (single line, updating)
pub fn print_tool_inline(status: ToolCallStatus, name: &str, description: &str) {
    let icon = status.icon();
    let color = status.color();

    print!(
        "{}{}{} {} {} {}{}",
        ansi::CLEAR_LINE,
        color,
        icon,
        ansi::RESET,
        name,
        description,
        ansi::RESET
    );
    let _ = io::stdout().flush();
}

/// Print a tool group header
pub fn print_tool_group_header(count: usize) {
    println!(
        "\n{} {} tool{}:",
        icons::TOOL,
        count,
        if count == 1 { "" } else { "s" }
    );
}

// ============================================================================
// Forge-style tool display
// ============================================================================

/// Forge-style tool display that shows:
/// ```text
/// ● tool_name(arg1=value1, arg2=value2)
///   └ Running...
/// ```
pub struct ForgeToolDisplay;

impl ForgeToolDisplay {
    /// Format tool arguments in a readable way
    /// - Truncates long strings
    /// - Shows line counts for multi-line content
    /// - Uses key=value format
    pub fn format_args(args: &serde_json::Value) -> String {
        match args {
            serde_json::Value::Object(map) => {
                let formatted: Vec<String> = map
                    .iter()
                    .map(|(key, value)| {
                        let val_str = Self::format_value(value);
                        format!("{}={}", key, val_str)
                    })
                    .collect();
                formatted.join(", ")
            }
            _ => args.to_string(),
        }
    }

    /// Format a single value for display
    fn format_value(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => {
                let line_count = s.lines().count();
                if line_count > 1 {
                    format!("<{} lines>", line_count)
                } else if s.len() > 50 {
                    format!("{}...", &s[..47])
                } else {
                    s.clone()
                }
            }
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Array(arr) => {
                format!("[{} items]", arr.len())
            }
            serde_json::Value::Object(map) => {
                format!("{{{} keys}}", map.len())
            }
            serde_json::Value::Null => "null".to_string(),
        }
    }

    /// Print tool start in forge style
    /// ```text
    /// ● tool_name(args)
    ///   └ Running...
    /// ```
    pub fn start(name: &str, args: &serde_json::Value) {
        let formatted_args = Self::format_args(args);
        println!(
            "{} {}({})",
            "●".cyan(),
            name.cyan().bold(),
            formatted_args.dimmed()
        );
        println!("  {} Running...", "└".dimmed());
        let _ = io::stdout().flush();
    }

    /// Update the status line (overwrites "Running...")
    pub fn update_status(status: &str) {
        // Move up one line and clear
        print!("\x1b[1A\x1b[2K");
        println!("  {} {}", "└".dimmed(), status);
        let _ = io::stdout().flush();
    }

    /// Complete the tool with a result summary
    pub fn complete(result_summary: &str) {
        // Move up one line and clear
        print!("\x1b[1A\x1b[2K");
        println!("  {} {}", "└".green(), result_summary.green());
        let _ = io::stdout().flush();
    }

    /// Complete with error
    pub fn error(error_msg: &str) {
        // Move up one line and clear
        print!("\x1b[1A\x1b[2K");
        println!("  {} {}", "└".red(), error_msg.red());
        let _ = io::stdout().flush();
    }

    /// Print tool inline without the tree structure (for simpler display)
    pub fn print_inline(name: &str, args: &serde_json::Value) {
        let formatted_args = Self::format_args(args);
        println!(
            "{} {}({})",
            "●".cyan(),
            name.cyan().bold(),
            formatted_args.dimmed()
        );
        let _ = io::stdout().flush();
    }

    /// Summarize tool result for display
    /// Takes the raw result and extracts a short summary
    pub fn summarize_result(name: &str, result: &str) -> String {
        // Try to parse as JSON and extract summary
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(result) {
            // Handle common patterns
            if let Some(success) = json.get("success").and_then(|v| v.as_bool())
                && !success
            {
                if let Some(err) = json.get("error").and_then(|v| v.as_str()) {
                    return format!("Error: {}", truncate_str(err, 50));
                }
                return "Failed".to_string();
            }

            // Check for issues/errors count
            if let Some(issues) = json.get("issues").and_then(|v| v.as_array()) {
                return format!("{} issues found", issues.len());
            }

            // Check for files written
            if let Some(files) = json.get("files_written").and_then(|v| v.as_u64()) {
                let lines = json
                    .get("total_lines")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                return format!("wrote {} file(s) ({} lines)", files, lines);
            }

            // Check for lines in file
            if let Some(lines) = json.get("total_lines").and_then(|v| v.as_u64()) {
                return format!("read {} lines", lines);
            }

            // Check for entries (directory listing)
            if let Some(count) = json.get("total_count").and_then(|v| v.as_u64()) {
                return format!("{} entries", count);
            }

            // Default: show action if available
            if let Some(action) = json.get("action").and_then(|v| v.as_str()) {
                if let Some(path) = json.get("path").and_then(|v| v.as_str()) {
                    return format!("{} {}", action.to_lowercase(), path);
                }
                return action.to_lowercase();
            }
        }

        // Fallback: truncate raw result
        format!("{} completed", name)
    }
}

/// Truncate a string to max length
fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_call_info() {
        let info = ToolCallInfo::new("read_file", "reading src/main.rs");
        assert_eq!(info.status, ToolCallStatus::Pending);

        let info = info.executing();
        assert_eq!(info.status, ToolCallStatus::Executing);

        let info = info.success(Some("file contents".to_string()));
        assert_eq!(info.status, ToolCallStatus::Success);
        assert!(info.result.is_some());
    }

    #[test]
    fn test_status_icons() {
        assert_eq!(ToolCallStatus::Pending.icon(), icons::PENDING);
        assert_eq!(ToolCallStatus::Success.icon(), icons::SUCCESS);
        assert_eq!(ToolCallStatus::Error.icon(), icons::ERROR);
    }

    #[test]
    fn test_forge_format_args() {
        // Simple args
        let args = json!({"path": "src/main.rs", "check": true});
        let formatted = ForgeToolDisplay::format_args(&args);
        assert!(formatted.contains("path=src/main.rs"));
        assert!(formatted.contains("check=true"));

        // Multi-line content should show line count
        let args = json!({"content": "line1\nline2\nline3"});
        let formatted = ForgeToolDisplay::format_args(&args);
        assert!(formatted.contains("<3 lines>"));

        // Long string should be truncated
        let long_str = "a".repeat(100);
        let args = json!({"data": long_str});
        let formatted = ForgeToolDisplay::format_args(&args);
        assert!(formatted.contains("..."));
    }

    #[test]
    fn test_forge_summarize_result() {
        // Files written
        let result = r#"{"success": true, "files_written": 3, "total_lines": 150}"#;
        let summary = ForgeToolDisplay::summarize_result("write_files", result);
        assert!(summary.contains("3 file"));
        assert!(summary.contains("150 lines"));

        // Issues found
        let result = r#"{"issues": [1, 2, 3]}"#;
        let summary = ForgeToolDisplay::summarize_result("hadolint", result);
        assert!(summary.contains("3 issues"));

        // Directory listing
        let result = r#"{"total_count": 25}"#;
        let summary = ForgeToolDisplay::summarize_result("list_directory", result);
        assert!(summary.contains("25 entries"));
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("short", 10), "short");
        assert_eq!(truncate_str("this is a longer string", 10), "this is...");
    }
}
