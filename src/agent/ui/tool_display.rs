//! Tool call display for visible tool execution feedback
//!
//! Shows tool calls with status indicators, names, descriptions, and results.

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

        let success_count = tools.iter().filter(|t| t.status == ToolCallStatus::Success).count();
        let error_count = tools.iter().filter(|t| t.status == ToolCallStatus::Error).count();

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
    println!("\n{} {} tool{}:", icons::TOOL, count, if count == 1 { "" } else { "s" });
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
