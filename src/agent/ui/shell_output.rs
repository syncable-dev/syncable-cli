//! Streaming shell output display
//!
//! Shows the last N lines of shell command output, overwriting previous
//! lines as new output arrives. Creates a compact, live-updating view.

use colored::Colorize;
use crossterm::{cursor, execute, terminal};
use std::collections::VecDeque;
use std::io::{self, Write};
use std::time::{Duration, Instant};

/// Default number of lines to display
const DEFAULT_MAX_LINES: usize = 5;

/// Streaming output buffer that overwrites previous display
pub struct StreamingShellOutput {
    lines: VecDeque<String>,
    max_lines: usize,
    command: String,
    start_time: Instant,
    lines_rendered: usize,
    timeout_secs: u64,
}

impl StreamingShellOutput {
    /// Create a new streaming output buffer
    pub fn new(command: &str, timeout_secs: u64) -> Self {
        Self {
            lines: VecDeque::with_capacity(DEFAULT_MAX_LINES + 1),
            max_lines: DEFAULT_MAX_LINES,
            command: command.to_string(),
            start_time: Instant::now(),
            lines_rendered: 0,
            timeout_secs,
        }
    }

    /// Create with custom max lines
    pub fn with_max_lines(command: &str, timeout_secs: u64, max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines + 1),
            max_lines,
            command: command.to_string(),
            start_time: Instant::now(),
            lines_rendered: 0,
            timeout_secs,
        }
    }

    /// Format elapsed time display
    fn format_elapsed(&self) -> String {
        let elapsed = self.start_time.elapsed();
        let secs = elapsed.as_secs();
        if secs >= 60 {
            let mins = secs / 60;
            let remaining_secs = secs % 60;
            format!("{}m {}s", mins, remaining_secs)
        } else {
            format!("{}s", secs)
        }
    }

    /// Format timeout display
    fn format_timeout(&self) -> String {
        let mins = self.timeout_secs / 60;
        let secs = self.timeout_secs % 60;
        if mins > 0 {
            format!("timeout: {}m {}s", mins, secs)
        } else {
            format!("timeout: {}s", secs)
        }
    }

    /// Render the header line
    fn render_header(&self) {
        let elapsed = self.format_elapsed();
        let timeout = self.format_timeout();

        // Truncate command if needed (using safe UTF-8 truncation)
        let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
        let prefix_len = 2 + timeout.len() + elapsed.len() + 10; // "● Bash(" + ") " + times
        let max_cmd_len = term_width.saturating_sub(prefix_len);
        let cmd_display = truncate_safe(&self.command, max_cmd_len);

        print!(
            "{} {}({}) {} ({})",
            "●".cyan().bold(),
            "Bash".cyan(),
            cmd_display.cyan(),
            timeout.dimmed(),
            elapsed.yellow()
        );
    }

    /// Render the output box with lines
    fn render_output(&self) {
        let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
        let content_width = term_width.saturating_sub(5); // "  │ " prefix

        for (i, line) in self.lines.iter().enumerate() {
            let is_last = i == self.lines.len() - 1;
            let prefix = if is_last { "└" } else { "│" };

            // Truncate line if needed (using safe UTF-8 truncation)
            let display = truncate_safe(line, content_width);

            println!("  {} {}", prefix.dimmed(), display);
        }
        // Note: Removed the "Running..." status line - elapsed time is shown in header
    }

    /// Clear previously rendered lines
    fn clear_previous(&mut self) {
        if self.lines_rendered > 0 {
            let mut stdout = io::stdout();
            // Move cursor up and clear lines
            for _ in 0..self.lines_rendered {
                let _ = execute!(
                    stdout,
                    cursor::MoveUp(1),
                    terminal::Clear(terminal::ClearType::CurrentLine)
                );
            }
        }
    }

    /// Push a new line of output
    pub fn push_line(&mut self, line: &str) {
        // Skip empty lines at the start
        if self.lines.is_empty() && line.trim().is_empty() {
            return;
        }

        // Clean the line - remove ANSI codes for storage but keep content
        let cleaned = strip_ansi_codes(line);

        // Add line to buffer
        self.lines.push_back(cleaned);

        // Keep only max_lines
        while self.lines.len() > self.max_lines {
            self.lines.pop_front();
        }

        // Re-render
        self.render();
    }

    /// Push multiple lines (e.g., from splitting on newlines)
    pub fn push_lines(&mut self, text: &str) {
        for line in text.lines() {
            self.push_line(line);
        }
    }

    /// Full render with header and output
    pub fn render(&mut self) {
        self.clear_previous();

        let mut stdout = io::stdout();

        // Render header
        self.render_header();
        println!();

        // Render output lines
        let lines_count = self.lines.len();
        self.render_output();

        // Calculate total lines rendered (header + output lines)
        self.lines_rendered = 1 + lines_count;

        let _ = stdout.flush();
    }

    /// Finish rendering - show final state
    pub fn finish(&mut self, success: bool, exit_code: Option<i32>) {
        self.clear_previous();

        let elapsed = self.format_elapsed();
        let status_icon = if success { "✓" } else { "✗" };

        // Final header (using safe UTF-8 truncation)
        let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
        let max_cmd_len = term_width.saturating_sub(30);
        let cmd_display = truncate_safe(&self.command, max_cmd_len);

        let exit_info = match exit_code {
            Some(code) if code != 0 => format!(" (exit {})", code),
            _ => String::new(),
        };

        if success {
            println!(
                "{} {}({}) {} {}{}",
                status_icon.green().bold(),
                "Bash".green(),
                cmd_display.dimmed(),
                "completed".green(),
                elapsed.dimmed(),
                exit_info.red()
            );
        } else {
            println!(
                "{} {}({}) {} {}{}",
                status_icon.red().bold(),
                "Bash".red(),
                cmd_display.dimmed(),
                "failed".red(),
                elapsed.dimmed(),
                exit_info.red()
            );
        }

        // Show last few lines of output on failure
        if !success && !self.lines.is_empty() {
            for line in self.lines.iter().take(3) {
                println!("  {} {}", "│".dimmed(), line.dimmed());
            }
        }

        let _ = io::stdout().flush();
        self.lines_rendered = 0;
    }

    /// Get elapsed duration
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Simple ANSI code stripping (basic implementation)
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we hit a letter
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Safely truncate a string to a maximum visual width, handling UTF-8 properly.
/// Adds "..." suffix when truncation occurs.
/// This prevents panics from slicing multi-byte UTF-8 characters.
fn truncate_safe(s: &str, max_width: usize) -> String {
    // Strip ANSI codes first to get accurate visual width
    let stripped = strip_ansi_codes(s);
    
    // Calculate visual width (count characters, not bytes)
    let visual_len: usize = stripped.chars().count();
    
    if visual_len <= max_width {
        return s.to_string();
    }
    
    // Need to truncate - work with stripped version
    // Reserve space for "..."
    let truncate_to = max_width.saturating_sub(3);
    
    let mut result = String::new();
    let mut char_count = 0;
    
    for ch in stripped.chars() {
        if char_count >= truncate_to {
            result.push_str("...");
            break;
        }
        result.push(ch);
        char_count += 1;
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_codes() {
        let input = "\x1b[32mgreen\x1b[0m text";
        assert_eq!(strip_ansi_codes(input), "green text");
    }

    #[test]
    fn test_truncate_safe_ascii() {
        // Basic ASCII truncation
        assert_eq!(truncate_safe("hello world", 8), "hello...");
        assert_eq!(truncate_safe("short", 10), "short");
        assert_eq!(truncate_safe("exactly10!", 10), "exactly10!");
    }

    #[test]
    fn test_truncate_safe_utf8_box_drawing() {
        // Box drawing characters (multi-byte UTF-8) - the exact case that caused the panic
        let box_line = "╭ Warning ──────────────────────────────────╮";
        // Should NOT panic and should truncate properly
        let result = truncate_safe(box_line, 20);
        assert!(result.ends_with("..."));
        assert!(result.chars().count() <= 20);
    }

    #[test]
    fn test_truncate_safe_utf8_emoji() {
        // Emoji (multi-byte UTF-8)
        let emoji_str = "🚀 Building project 📦 with dependencies 🔧";
        let result = truncate_safe(emoji_str, 15);
        assert!(result.ends_with("..."));
        // Should not panic
    }

    #[test]
    fn test_truncate_safe_mixed_content() {
        // Mixed ASCII and multi-byte characters
        let mixed = "#9 3.304 ╭ Warning ───";
        let result = truncate_safe(mixed, 15);
        assert!(result.ends_with("..."));
        assert!(result.chars().count() <= 15);
    }

    #[test]
    fn test_truncate_safe_no_truncation_needed() {
        let short = "hello";
        assert_eq!(truncate_safe(short, 100), "hello");
        
        let exact = "12345";
        assert_eq!(truncate_safe(exact, 5), "12345");
    }

    #[test]
    fn test_streaming_output_buffer() {
        let mut stream = StreamingShellOutput::new("test", 60);
        stream.push_line("line 1");
        stream.push_line("line 2");
        assert_eq!(stream.lines.len(), 2);

        // Fill beyond max
        for i in 0..10 {
            stream.push_line(&format!("line {}", i));
        }
        assert_eq!(stream.lines.len(), DEFAULT_MAX_LINES);
    }

    #[test]
    fn test_streaming_output_with_utf8_content() {
        // Ensure the buffer doesn't panic with UTF-8 content
        let mut stream = StreamingShellOutput::new("docker build", 60);
        stream.push_line("╭ Warning ────────────────╮");
        stream.push_line("│ This is a warning message │");
        stream.push_line("╰────────────────────────────╯");
        assert_eq!(stream.lines.len(), 3);
    }
}
