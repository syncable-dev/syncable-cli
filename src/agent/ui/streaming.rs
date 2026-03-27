//! Streaming response display for real-time AI output
//!
//! Handles streaming tokens from the AI and displaying them in real-time.

use crate::agent::ui::colors::{ansi, icons};
use crate::agent::ui::spinner::Spinner;
use crate::agent::ui::tool_display::{ToolCallDisplay, ToolCallInfo, ToolCallStatus};
use colored::Colorize;
use std::io::{self, Write};
use std::time::Instant;

/// State of the streaming response
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingState {
    /// Ready for input
    Idle,
    /// AI is generating response
    Responding,
    /// Waiting for tool confirmation
    WaitingForConfirmation,
    /// Tools are executing
    ExecutingTools,
}

/// Manages the display of streaming AI responses
pub struct StreamingDisplay {
    state: StreamingState,
    start_time: Option<Instant>,
    current_text: String,
    tool_calls: Vec<ToolCallInfo>,
    chars_displayed: usize,
}

impl StreamingDisplay {
    pub fn new() -> Self {
        Self {
            state: StreamingState::Idle,
            start_time: None,
            current_text: String::new(),
            tool_calls: Vec::new(),
            chars_displayed: 0,
        }
    }

    /// Start a new response
    pub fn start_response(&mut self) {
        self.state = StreamingState::Responding;
        self.start_time = Some(Instant::now());
        self.current_text.clear();
        self.tool_calls.clear();
        self.chars_displayed = 0;

        // Print AI label
        print!("\n{} ", "AI:".blue().bold());
        let _ = io::stdout().flush();
    }

    /// Append text chunk to the response
    pub fn append_text(&mut self, text: &str) {
        self.current_text.push_str(text);

        // Print new text directly (streaming effect)
        print!("{}", text);
        let _ = io::stdout().flush();
        self.chars_displayed += text.len();
    }

    /// Record a tool call starting
    pub fn tool_call_started(&mut self, name: &str, description: &str) {
        self.state = StreamingState::ExecutingTools;

        let info = ToolCallInfo::new(name, description).executing();
        self.tool_calls.push(info.clone());

        // Print tool call notification
        ToolCallDisplay::print_start(name, description);
    }

    /// Record a tool call completed
    pub fn tool_call_completed(&mut self, name: &str, result: Option<String>) {
        if let Some(info) = self.tool_calls.iter_mut().find(|t| t.name == name) {
            *info = info.clone().success(result);
            ToolCallDisplay::print_status(info);
        }

        // Check if all tools are done
        if self.tool_calls.iter().all(|t| {
            matches!(
                t.status,
                ToolCallStatus::Success | ToolCallStatus::Error | ToolCallStatus::Canceled
            )
        }) {
            self.state = StreamingState::Responding;
        }
    }

    /// Record a tool call failed
    pub fn tool_call_failed(&mut self, name: &str, error: String) {
        // Clean up nested error messages (e.g., "ToolCallError: ToolCallError: actual error")
        let clean_error = error
            .replace("Toolset error: ", "")
            .replace("ToolCallError: ", "");

        if let Some(info) = self.tool_calls.iter_mut().find(|t| t.name == name) {
            *info = info.clone().error(clean_error);
            ToolCallDisplay::print_status(info);
        }
    }

    /// Show thinking/reasoning indicator
    pub fn show_thinking(&self, subject: &str) {
        print!(
            "{}{} {} {}{}",
            ansi::CLEAR_LINE,
            icons::THINKING,
            "Thinking:".cyan(),
            subject.dimmed(),
            ansi::RESET
        );
        let _ = io::stdout().flush();
    }

    /// End the current response
    pub fn end_response(&mut self) {
        self.state = StreamingState::Idle;

        // Ensure newline after response
        if !self.current_text.is_empty() && !self.current_text.ends_with('\n') {
            println!();
        }

        // Print summary if there were tool calls
        if !self.tool_calls.is_empty() {
            ToolCallDisplay::print_summary(&self.tool_calls);
        }

        // Print elapsed time if significant
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            if elapsed.as_secs() >= 2 {
                println!(
                    "\n{} {:.1}s",
                    "Response time:".dimmed(),
                    elapsed.as_secs_f64()
                );
            }
        }

        println!();
        let _ = io::stdout().flush();
    }

    /// Handle an error during streaming
    pub fn handle_error(&mut self, error: &str) {
        self.state = StreamingState::Idle;
        println!("\n{} {}", icons::ERROR.red(), error.red());
        let _ = io::stdout().flush();
    }

    /// Get the current state
    pub fn state(&self) -> StreamingState {
        self.state
    }

    /// Get elapsed time since start
    pub fn elapsed_secs(&self) -> u64 {
        self.start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0)
    }

    /// Get the accumulated text
    pub fn text(&self) -> &str {
        &self.current_text
    }

    /// Get tool calls
    pub fn tool_calls(&self) -> &[ToolCallInfo] {
        &self.tool_calls
    }
}

impl Default for StreamingDisplay {
    fn default() -> Self {
        Self::new()
    }
}

/// A simpler streaming helper for basic use cases
pub struct SimpleStreamer {
    started: bool,
}

impl SimpleStreamer {
    pub fn new() -> Self {
        Self { started: false }
    }

    /// Print the AI label (call once at start)
    pub fn start(&mut self) {
        if !self.started {
            print!("\n{} ", "AI:".blue().bold());
            let _ = io::stdout().flush();
            self.started = true;
        }
    }

    /// Stream a text chunk
    pub fn stream(&mut self, text: &str) {
        self.start();
        print!("{}", text);
        let _ = io::stdout().flush();
    }

    /// End the stream
    pub fn end(&mut self) {
        if self.started {
            println!();
            println!();
            self.started = false;
        }
    }

    /// Print a tool call notification
    pub fn tool_call(&self, name: &str, description: &str) {
        println!();
        ToolCallDisplay::print_start(name, description);
    }

    /// Print tool call completed
    pub fn tool_complete(&self, name: &str) {
        let info = ToolCallInfo::new(name, "").success(None);
        ToolCallDisplay::print_status(&info);
    }
}

impl Default for SimpleStreamer {
    fn default() -> Self {
        Self::new()
    }
}

/// Print a "thinking" indicator with optional spinner
pub async fn show_thinking_with_spinner(message: &str) -> Spinner {
    Spinner::new(&format!("💭 {}", message))
}

/// Print a static thinking message
pub fn print_thinking(subject: &str) {
    println!(
        "{} {} {}",
        icons::THINKING,
        "Thinking about:".cyan(),
        subject.white()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_display_state() {
        let mut display = StreamingDisplay::new();
        assert_eq!(display.state(), StreamingState::Idle);

        display.start_response();
        assert_eq!(display.state(), StreamingState::Responding);

        display.tool_call_started("test", "testing");
        assert_eq!(display.state(), StreamingState::ExecutingTools);
    }

    #[test]
    fn test_append_text() {
        let mut display = StreamingDisplay::new();
        display.start_response();
        display.append_text("Hello ");
        display.append_text("World");
        assert_eq!(display.text(), "Hello World");
    }
}
