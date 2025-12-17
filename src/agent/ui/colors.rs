//! Color theme and styling utilities for terminal UI
//!
//! Provides semantic colors and ANSI escape codes for consistent styling.

use colored::Colorize;

/// Status icons for different states
pub mod icons {
    pub const PENDING: &str = "○";
    pub const EXECUTING: &str = "◐";
    pub const SUCCESS: &str = "✓";
    pub const ERROR: &str = "✗";
    pub const WARNING: &str = "⚠";
    pub const CANCELED: &str = "⊘";
    pub const CONFIRMING: &str = "⏳";
    pub const ARROW: &str = "→";
    pub const THINKING: &str = "💭";
    pub const ROBOT: &str = "🤖";
    pub const TOOL: &str = "🔧";
    pub const SHELL: &str = "🐚";
    pub const EDIT: &str = "✏️";
    pub const FILE: &str = "📄";
    pub const FOLDER: &str = "📁";
    pub const SECURITY: &str = "🔒";
    pub const SEARCH: &str = "🔍";
}

/// ANSI escape codes for direct terminal control
pub mod ansi {
    /// Clear current line
    pub const CLEAR_LINE: &str = "\x1b[2K\r";
    /// Move cursor up one line
    pub const CURSOR_UP: &str = "\x1b[1A";
    /// Hide cursor
    pub const HIDE_CURSOR: &str = "\x1b[?25l";
    /// Show cursor
    pub const SHOW_CURSOR: &str = "\x1b[?25h";
    /// Reset all styles
    pub const RESET: &str = "\x1b[0m";
    /// Bold
    pub const BOLD: &str = "\x1b[1m";
    /// Dim
    pub const DIM: &str = "\x1b[2m";

    // 256-color codes for Syncable brand
    pub const PURPLE: &str = "\x1b[38;5;141m";
    pub const ORANGE: &str = "\x1b[38;5;216m";
    pub const PINK: &str = "\x1b[38;5;212m";
    pub const MAGENTA: &str = "\x1b[38;5;207m";
    pub const CYAN: &str = "\x1b[38;5;51m";
    pub const GRAY: &str = "\x1b[38;5;245m";
    pub const SUCCESS: &str = "\x1b[38;5;114m"; // Green for success
}

/// Format a tool name for display
pub fn format_tool_name(name: &str) -> String {
    name.cyan().bold().to_string()
}

/// Format a status message based on success/failure
pub fn format_status(success: bool, message: &str) -> String {
    if success {
        format!("{} {}", icons::SUCCESS.green(), message.green())
    } else {
        format!("{} {}", icons::ERROR.red(), message.red())
    }
}

/// Format elapsed time for display
pub fn format_elapsed(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else {
        let mins = seconds / 60;
        let secs = seconds % 60;
        format!("{}m {}s", mins, secs)
    }
}

/// Format a thinking/reasoning message
pub fn format_thinking(subject: &str) -> String {
    format!(
        "{} {}",
        icons::THINKING,
        subject.cyan().italic()
    )
}

/// Format an info message
pub fn format_info(message: &str) -> String {
    format!("{} {}", icons::ARROW.cyan(), message)
}

/// Format a warning message
pub fn format_warning(message: &str) -> String {
    format!("⚠ {}", message.yellow())
}

/// Format an error message
pub fn format_error(message: &str) -> String {
    format!("{} {}", icons::ERROR.red(), message.red())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_elapsed() {
        assert_eq!(format_elapsed(5), "5s");
        assert_eq!(format_elapsed(30), "30s");
        assert_eq!(format_elapsed(65), "1m 5s");
        assert_eq!(format_elapsed(125), "2m 5s");
    }
}
