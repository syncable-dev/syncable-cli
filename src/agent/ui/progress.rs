//! Generation progress indicator - Claude Code style
//!
//! Shows a clean status line with current action during AI response generation.
//! Format: ✱ Action… (esc to interrupt)
//!
//! Inspired by Claude Code's elegant minimal approach.

use crate::agent::ui::colors::ansi;
use parking_lot::RwLock;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Animation frames for the indicator asterisk (subtle pulse)
const INDICATOR_FRAMES: &[&str] = &["✱", "✳", "✱", "✴", "✱", "✳"];

/// Animation interval - slower for subtle effect
const ANIMATION_INTERVAL_MS: u64 = 300;

/// Messages for controlling the progress indicator
#[derive(Debug, Clone)]
pub enum ProgressMessage {
    /// Update token counts (input, output)
    UpdateTokens { input: u64, output: u64 },
    /// Update the current action being performed
    Action(String),
    /// Update the detail/focus text (shown below main line)
    Focus(String),
    /// Clear the focus text
    ClearFocus,
    /// Stop the indicator
    Stop,
}

/// Shared state for progress tracking
#[derive(Debug)]
pub struct ProgressState {
    pub input_tokens: AtomicU64,
    pub output_tokens: AtomicU64,
    pub is_running: AtomicBool,
    /// Whether the indicator is paused (for coordinating with other output)
    pub is_paused: AtomicBool,
    /// Whether an interrupt has been requested (ESC pressed)
    pub interrupt_requested: AtomicBool,
    /// Current action being performed (e.g., "Generating response")
    pub action: RwLock<String>,
    /// Current focus/detail (e.g., "Reading config.yaml")
    pub focus: RwLock<Option<String>>,
    /// Start time for elapsed tracking
    pub start_time: std::time::Instant,
    /// Optional layout state for fixed status line rendering
    pub layout_state: RwLock<Option<std::sync::Arc<super::layout::LayoutState>>>,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self {
            input_tokens: AtomicU64::new(0),
            output_tokens: AtomicU64::new(0),
            is_running: AtomicBool::new(true),
            is_paused: AtomicBool::new(false),
            interrupt_requested: AtomicBool::new(false),
            action: RwLock::new("Generating".to_string()),
            focus: RwLock::new(None),
            start_time: std::time::Instant::now(),
            layout_state: RwLock::new(None),
        }
    }
}

impl ProgressState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn update_tokens(&self, input: u64, output: u64) {
        self.input_tokens.fetch_add(input, Ordering::SeqCst);
        self.output_tokens.fetch_add(output, Ordering::SeqCst);
    }

    pub fn get_tokens(&self) -> (u64, u64) {
        (
            self.input_tokens.load(Ordering::SeqCst),
            self.output_tokens.load(Ordering::SeqCst),
        )
    }

    pub fn set_action(&self, action: &str) {
        *self.action.write() = action.to_string();
    }

    pub fn get_action(&self) -> String {
        self.action.read().clone()
    }

    pub fn set_focus(&self, focus: &str) {
        *self.focus.write() = Some(focus.to_string());
    }

    pub fn clear_focus(&self) {
        *self.focus.write() = None;
    }

    pub fn get_focus(&self) -> Option<String> {
        self.focus.read().clone()
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Pause the indicator (stops rendering but keeps state)
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
    }

    /// Resume the indicator after pause
    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    /// Get elapsed time since start
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Set the layout state for fixed status line rendering
    pub fn set_layout(&self, layout: std::sync::Arc<super::layout::LayoutState>) {
        *self.layout_state.write() = Some(layout);
    }

    /// Check if layout is active (for choosing render mode)
    pub fn has_layout(&self) -> bool {
        self.layout_state
            .read()
            .as_ref()
            .map(|l| l.is_active())
            .unwrap_or(false)
    }

    /// Get layout state if available
    pub fn get_layout(&self) -> Option<std::sync::Arc<super::layout::LayoutState>> {
        self.layout_state.read().clone()
    }

    /// Request an interrupt (called when ESC is pressed)
    pub fn request_interrupt(&self) {
        self.interrupt_requested.store(true, Ordering::SeqCst);
    }

    /// Check if an interrupt has been requested
    pub fn is_interrupted(&self) -> bool {
        self.interrupt_requested.load(Ordering::SeqCst)
    }

    /// Clear the interrupt flag
    pub fn clear_interrupt(&self) {
        self.interrupt_requested.store(false, Ordering::SeqCst);
    }
}

/// Progress indicator with Claude Code style display
pub struct GenerationIndicator {
    sender: mpsc::Sender<ProgressMessage>,
    state: Arc<ProgressState>,
}

impl GenerationIndicator {
    /// Create and start a new progress indicator
    pub fn new() -> Self {
        Self::with_action("Generating")
    }

    /// Create with a specific initial action
    pub fn with_action(action: &str) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let state = ProgressState::new();
        state.set_action(action);
        let state_clone = state.clone();

        tokio::spawn(async move {
            run_progress_indicator(receiver, state_clone).await;
        });

        Self { sender, state }
    }

    /// Update token counts
    pub async fn update_tokens(&self, input: u64, output: u64) {
        self.state.update_tokens(input, output);
        let _ = self
            .sender
            .send(ProgressMessage::UpdateTokens { input, output })
            .await;
    }

    /// Set the current action (e.g., "Analyzing", "Reading files")
    pub async fn set_action(&self, action: &str) {
        self.state.set_action(action);
        let _ = self
            .sender
            .send(ProgressMessage::Action(action.to_string()))
            .await;
    }

    /// Set focus/detail text shown below the main status
    pub async fn set_focus(&self, focus: &str) {
        self.state.set_focus(focus);
        let _ = self
            .sender
            .send(ProgressMessage::Focus(focus.to_string()))
            .await;
    }

    /// Clear the focus text
    pub async fn clear_focus(&self) {
        self.state.clear_focus();
        let _ = self.sender.send(ProgressMessage::ClearFocus).await;
    }

    /// Stop the indicator
    pub async fn stop(&self) {
        self.state.stop();
        let _ = self.sender.send(ProgressMessage::Stop).await;
        // Give the indicator task time to clean up
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    /// Pause the indicator (clears line and shows cursor for other output)
    pub async fn pause(&self) {
        self.state.pause();
        // Clear current lines to make room for other output
        print!("\r{}", ansi::CLEAR_LINE);
        print!("{}", ansi::SHOW_CURSOR);
        let _ = io::stdout().flush();
    }

    /// Resume the indicator after pause
    pub async fn resume(&self) {
        self.state.resume();
        print!("{}", ansi::HIDE_CURSOR);
        let _ = io::stdout().flush();
    }

    /// Get the shared state for external updates
    pub fn state(&self) -> Arc<ProgressState> {
        self.state.clone()
    }
}

impl Default for GenerationIndicator {
    fn default() -> Self {
        Self::new()
    }
}

/// Format token count with K suffix for large numbers
fn format_tokens(tokens: u64) -> String {
    if tokens >= 100_000 {
        format!("{:.1}k", tokens as f64 / 1000.0)
    } else if tokens >= 10_000 {
        format!("{:.0}k", tokens as f64 / 1000.0)
    } else {
        tokens.to_string()
    }
}

/// Coral/orange color for the indicator (matches Claude Code)
const CORAL: &str = "\x1b[38;5;209m";

/// Internal progress indicator loop - Claude Code style
///
/// Note: ESC key detection is handled by a separate dedicated listener (spawn_esc_listener)
/// which runs continuously with its own raw mode, independent of this animation loop.
async fn run_progress_indicator(
    mut receiver: mpsc::Receiver<ProgressMessage>,
    state: Arc<ProgressState>,
) {
    let start_time = Instant::now();
    let mut frame_index = 0;
    let mut had_focus = false;
    let mut interval = tokio::time::interval(Duration::from_millis(ANIMATION_INTERVAL_MS));

    // Hide cursor during animation (only if not using layout)
    if !state.has_layout() {
        print!("{}", ansi::HIDE_CURSOR);
        let _ = io::stdout().flush();
    }

    // Track if we need to clear display on pause
    let mut was_rendering = false;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if !state.is_running() {
                    break;
                }

                let use_layout = state.has_layout();

                // Handle pause - clear display when transitioning to paused
                if state.is_paused() {
                    if was_rendering && !use_layout {
                        // Clear our display before yielding to other output (only for non-layout mode)
                        if had_focus {
                            print!("{}{}", ansi::CURSOR_UP, ansi::CLEAR_LINE);
                        }
                        print!("\r{}", ansi::CLEAR_LINE);
                        print!("{}", ansi::SHOW_CURSOR);
                        let _ = io::stdout().flush();
                        was_rendering = false;
                        had_focus = false;
                    }
                    continue;
                }

                // We're about to render - hide cursor if we just resumed
                if !was_rendering && !use_layout {
                    print!("{}", ansi::HIDE_CURSOR);
                    let _ = io::stdout().flush();
                }
                was_rendering = true;

                let elapsed = start_time.elapsed();
                let indicator = INDICATOR_FRAMES[frame_index % INDICATOR_FRAMES.len()];
                frame_index += 1;

                let action = state.get_action();
                let focus = state.get_focus();
                let (input_tokens, output_tokens) = state.get_tokens();
                let total_tokens = input_tokens + output_tokens;

                // Build stats string: (^C to stop · 12.3s · ↓ 28k tokens)
                let elapsed_secs = elapsed.as_secs_f64();
                let elapsed_str = if elapsed_secs >= 60.0 {
                    format!("{:.0}m {:.0}s", elapsed_secs / 60.0, elapsed_secs % 60.0)
                } else {
                    format!("{:.1}s", elapsed_secs)
                };

                let stats = if total_tokens > 0 {
                    format!(
                        "{}(^C to stop · {} · ↓ {} tokens){}",
                        ansi::DIM,
                        elapsed_str,
                        format_tokens(total_tokens),
                        ansi::RESET
                    )
                } else {
                    format!(
                        "{}(^C to stop · {}){}",
                        ansi::DIM,
                        elapsed_str,
                        ansi::RESET
                    )
                };

                // Format the status content
                let status_content = format!(
                    "{}{}{} {}{}…{} {}",
                    CORAL,
                    indicator,
                    ansi::RESET,
                    CORAL,
                    action,
                    ansi::RESET,
                    stats,
                );

                // Render using layout or fallback to inline mode
                if use_layout {
                    if let Some(layout_state) = state.get_layout() {
                        // Use fixed status line rendering
                        render_to_layout(&layout_state, &status_content, focus.as_deref());
                    }
                } else {
                    // Fallback: inline rendering with \r
                    // Clear previous lines if we had focus
                    if had_focus {
                        print!("{}{}", ansi::CURSOR_UP, ansi::CLEAR_LINE);
                    }
                    print!("\r{}", ansi::CLEAR_LINE);

                    // Main status line
                    print!("{}", status_content);

                    // Focus line below (if set): └ detail
                    if let Some(ref focus_text) = focus {
                        print!(
                            "\n{}└{} {}{}{}",
                            ansi::DIM,
                            ansi::RESET,
                            ansi::GRAY,
                            focus_text,
                            ansi::RESET
                        );
                        had_focus = true;
                    } else {
                        had_focus = false;
                    }

                    let _ = io::stdout().flush();
                }
            }
            Some(msg) = receiver.recv() => {
                match msg {
                    ProgressMessage::UpdateTokens { .. } => {
                        // Handled via shared state
                    }
                    ProgressMessage::Action(action) => {
                        state.set_action(&action);
                    }
                    ProgressMessage::Focus(focus) => {
                        state.set_focus(&focus);
                    }
                    ProgressMessage::ClearFocus => {
                        state.clear_focus();
                    }
                    ProgressMessage::Stop => {
                        state.stop();
                        break;
                    }
                }
            }
        }
    }

    // Clean up - clear the status lines (raw mode is handled by spawn_esc_listener)
    if !state.has_layout() {
        if had_focus {
            print!("{}{}", ansi::CURSOR_UP, ansi::CLEAR_LINE);
        }
        print!("\r{}", ansi::CLEAR_LINE);
        print!("{}", ansi::SHOW_CURSOR);
        let _ = io::stdout().flush();
    }
}

/// Render progress to the fixed status line using layout
fn render_to_layout(layout_state: &super::layout::LayoutState, status: &str, focus: Option<&str>) {
    use super::layout::escape;

    if !layout_state.is_active() {
        return;
    }

    let mut stdout = io::stdout();
    let status_line = layout_state.status_line();
    let focus_line = layout_state.focus_line();

    // Save cursor, move to status line, render
    print!("{}", escape::SAVE_CURSOR);
    print!("{}", escape::move_to_line(status_line));
    print!("{}", ansi::CLEAR_LINE);
    print!("{}", status);

    // Focus on dedicated focus line (not relative \n)
    print!("{}", escape::move_to_line(focus_line));
    print!("{}", ansi::CLEAR_LINE);
    if let Some(focus_text) = focus {
        print!(
            "{}└{} {}{}{}",
            ansi::DIM,
            ansi::RESET,
            ansi::GRAY,
            focus_text,
            ansi::RESET
        );
    }

    print!("{}", escape::RESTORE_CURSOR);
    let _ = stdout.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(0), "0");
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(1000), "1000");
        assert_eq!(format_tokens(9999), "9999");
        assert_eq!(format_tokens(10000), "10k");
        assert_eq!(format_tokens(10499), "10k");
        assert_eq!(format_tokens(10999), "11k");
        assert_eq!(format_tokens(100000), "100.0k");
        assert_eq!(format_tokens(150000), "150.0k");
    }

    #[test]
    fn test_progress_state() {
        let state = ProgressState::new();
        assert!(state.is_running());
        assert_eq!(state.get_tokens(), (0, 0));
        assert_eq!(state.get_action(), "Generating");
        assert!(state.get_focus().is_none());

        state.update_tokens(100, 50);
        assert_eq!(state.get_tokens(), (100, 50));

        state.set_action("Analyzing");
        assert_eq!(state.get_action(), "Analyzing");

        state.set_focus("Reading file.rs");
        assert_eq!(state.get_focus(), Some("Reading file.rs".to_string()));

        state.clear_focus();
        assert!(state.get_focus().is_none());

        state.stop();
        assert!(!state.is_running());
    }
}
