//! Animated spinner for terminal UI
//!
//! Provides a Gemini-style spinner that updates in place with elapsed time
//! and cycles through witty/informative phrases.

use crate::agent::ui::colors::{ansi, format_elapsed};
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Spinner animation frames (dots pattern like Gemini CLI)
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Animation interval in milliseconds
const ANIMATION_INTERVAL_MS: u64 = 80;

/// Phrase change interval in seconds (like Gemini's 15 seconds)
const PHRASE_CHANGE_INTERVAL_SECS: u64 = 8;

/// Witty loading phrases inspired by Gemini CLI
const WITTY_PHRASES: &[&str] = &[
    "Analyzing your codebase...",
    "Consulting the digital spirits...",
    "Warming up the AI hamsters...",
    "Polishing the algorithms...",
    "Brewing fresh bytes...",
    "Engaging cognitive processors...",
    "Compiling brilliance...",
    "Untangling neural nets...",
    "Converting coffee into insights...",
    "Scanning for patterns...",
    "Traversing the AST...",
    "Checking dependencies...",
    "Looking for security issues...",
    "Mapping the architecture...",
    "Detecting frameworks...",
    "Parsing configurations...",
    "Analyzing code patterns...",
    "Deep diving into your code...",
    "Searching for vulnerabilities...",
    "Exploring the codebase...",
    "Processing your request...",
    "Thinking deeply about this...",
    "Gathering context...",
    "Reading documentation...",
    "Inspecting files...",
];

/// Informative tips shown occasionally
const TIPS: &[&str] = &[
    "Tip: Use /model to switch AI models...",
    "Tip: Use /provider to change providers...",
    "Tip: Type /help for available commands...",
    "Tip: Use /clear to reset conversation...",
    "Tip: Try 'sync-ctl analyze' for full analysis...",
    "Tip: Security scans support 5 modes (lightning to paranoid)...",
];

/// Message types for spinner control
#[derive(Debug)]
pub enum SpinnerMessage {
    /// Update the spinner text
    UpdateText(String),
    /// Update to show a tool is executing
    ToolExecuting { name: String, description: String },
    /// Tool completed successfully
    ToolComplete { name: String },
    /// Show thinking/reasoning
    Thinking(String),
    /// Stop the spinner
    Stop,
}

/// An animated spinner that runs in the background
pub struct Spinner {
    sender: mpsc::Sender<SpinnerMessage>,
    is_running: Arc<AtomicBool>,
}

impl Spinner {
    /// Create and start a new spinner with initial text
    pub fn new(initial_text: &str) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_clone = is_running.clone();
        let initial = initial_text.to_string();

        tokio::spawn(async move {
            run_spinner(receiver, is_running_clone, initial).await;
        });

        Self { sender, is_running }
    }

    /// Update the spinner text
    pub async fn set_text(&self, text: &str) {
        let _ = self
            .sender
            .send(SpinnerMessage::UpdateText(text.to_string()))
            .await;
    }

    /// Show tool executing status
    pub async fn tool_executing(&self, name: &str, description: &str) {
        let _ = self
            .sender
            .send(SpinnerMessage::ToolExecuting {
                name: name.to_string(),
                description: description.to_string(),
            })
            .await;
    }

    /// Mark a tool as complete (will be shown in the completed list)
    pub async fn tool_complete(&self, name: &str) {
        let _ = self
            .sender
            .send(SpinnerMessage::ToolComplete {
                name: name.to_string(),
            })
            .await;
    }

    /// Show thinking status
    pub async fn thinking(&self, subject: &str) {
        let _ = self
            .sender
            .send(SpinnerMessage::Thinking(subject.to_string()))
            .await;
    }

    /// Stop the spinner and clear the line
    pub async fn stop(&self) {
        let _ = self.sender.send(SpinnerMessage::Stop).await;
        // Give the spinner task time to clean up
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    /// Check if spinner is still running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

/// Internal spinner loop with phrase cycling
async fn run_spinner(
    mut receiver: mpsc::Receiver<SpinnerMessage>,
    is_running: Arc<AtomicBool>,
    initial_text: String,
) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    let start_time = Instant::now();
    let mut frame_index = 0;
    let mut current_text = initial_text;
    let mut last_phrase_change = Instant::now();
    let mut phrase_index = 0;
    let mut current_tool: Option<String> = None;
    let mut tools_completed: usize = 0;
    let mut has_printed_tool_line = false;
    let mut interval = tokio::time::interval(Duration::from_millis(ANIMATION_INTERVAL_MS));
    let mut rng = StdRng::from_entropy();

    // Hide cursor during spinner
    print!("{}", ansi::HIDE_CURSOR);
    let _ = io::stdout().flush();

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if !is_running.load(Ordering::SeqCst) {
                    break;
                }

                let elapsed = start_time.elapsed().as_secs();
                let frame = SPINNER_FRAMES[frame_index % SPINNER_FRAMES.len()];
                frame_index += 1;

                // Cycle phrases if idle
                if current_tool.is_none() && last_phrase_change.elapsed().as_secs() >= PHRASE_CHANGE_INTERVAL_SECS {
                    if rng.gen_bool(0.25) && !TIPS.is_empty() {
                        let tip_idx = rng.gen_range(0..TIPS.len());
                        current_text = TIPS[tip_idx].to_string();
                    } else {
                        phrase_index = (phrase_index + 1) % WITTY_PHRASES.len();
                        current_text = WITTY_PHRASES[phrase_index].to_string();
                    }
                    last_phrase_change = Instant::now();
                }

                if has_printed_tool_line {
                    // Move up to tool line, update it, move back down to spinner line
                    if let Some(ref tool) = current_tool {
                        print!("{}{}  {}🔧 {}{}{}",
                            ansi::CURSOR_UP,
                            ansi::CLEAR_LINE,
                            ansi::PURPLE,
                            tool,
                            ansi::RESET,
                            "\n" // Move back down
                        );
                    }
                    // Now update spinner line
                    print!("\r{}  {}{}{} {} {}{}({}){}",
                        ansi::CLEAR_LINE,
                        ansi::CYAN,
                        frame,
                        ansi::RESET,
                        current_text,
                        ansi::GRAY,
                        ansi::DIM,
                        format_elapsed(elapsed),
                        ansi::RESET
                    );
                } else {
                    // Single line mode (no tool yet)
                    print!("\r{}  {}{}{} {} {}{}({}){}",
                        ansi::CLEAR_LINE,
                        ansi::CYAN,
                        frame,
                        ansi::RESET,
                        current_text,
                        ansi::GRAY,
                        ansi::DIM,
                        format_elapsed(elapsed),
                        ansi::RESET
                    );
                }
                let _ = io::stdout().flush();
            }
            Some(msg) = receiver.recv() => {
                match msg {
                    SpinnerMessage::UpdateText(text) => {
                        current_text = text;
                    }
                    SpinnerMessage::ToolExecuting { name, description } => {
                        if !has_printed_tool_line {
                            // First tool - print tool line then newline for spinner
                            print!("\r{}  {}🔧 {}{}{}\n",
                                ansi::CLEAR_LINE,
                                ansi::PURPLE,
                                name,
                                ansi::RESET,
                                "" // Spinner will be on next line
                            );
                            has_printed_tool_line = true;
                        }
                        // Tool line will be updated on next tick
                        current_tool = Some(name);
                        current_text = description;
                        last_phrase_change = Instant::now();
                    }
                    SpinnerMessage::ToolComplete { name: _ } => {
                        tools_completed += 1;
                        current_tool = None;
                        phrase_index = (phrase_index + 1) % WITTY_PHRASES.len();
                        current_text = WITTY_PHRASES[phrase_index].to_string();
                    }
                    SpinnerMessage::Thinking(subject) => {
                        current_text = format!("💭 {}", subject);
                    }
                    SpinnerMessage::Stop => {
                        is_running.store(false, Ordering::SeqCst);
                        break;
                    }
                }
            }
        }
    }

    // Clear both lines and show summary
    if has_printed_tool_line {
        // Clear spinner line
        print!("\r{}", ansi::CLEAR_LINE);
        // Move up and clear tool line
        print!("{}{}", ansi::CURSOR_UP, ansi::CLEAR_LINE);
    } else {
        print!("\r{}", ansi::CLEAR_LINE);
    }

    // Print summary
    if tools_completed > 0 {
        println!(
            "  {}✓{} {} tool{} used",
            ansi::SUCCESS,
            ansi::RESET,
            tools_completed,
            if tools_completed == 1 { "" } else { "s" }
        );
    }
    print!("{}", ansi::SHOW_CURSOR);
    let _ = io::stdout().flush();
}

/// A simple inline spinner for synchronous contexts
pub struct InlineSpinner {
    frames: Vec<&'static str>,
    current: usize,
}

impl InlineSpinner {
    pub fn new() -> Self {
        Self {
            frames: SPINNER_FRAMES.to_vec(),
            current: 0,
        }
    }

    /// Get the next frame
    pub fn next_frame(&mut self) -> &'static str {
        let frame = self.frames[self.current % self.frames.len()];
        self.current += 1;
        frame
    }

    /// Print a spinner update inline (clears and rewrites)
    pub fn print(&mut self, message: &str) {
        let frame = self.next_frame();
        print!("{}{} {}", ansi::CLEAR_LINE, frame, message);
        let _ = io::stdout().flush();
    }
}

impl Default for InlineSpinner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_spinner() {
        let mut spinner = InlineSpinner::new();
        assert_eq!(spinner.next_frame(), "⠋");
        assert_eq!(spinner.next_frame(), "⠙");
        assert_eq!(spinner.next_frame(), "⠹");
    }
}
