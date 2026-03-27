//! Terminal layout with ANSI scrolling regions
//!
//! Provides a split terminal layout:
//! - Scrollable content area (top) - for tool output, thinking, responses
//! - Fixed status line - for progress indicator
//! - Fixed input line - always visible prompt
//!
//! Uses ANSI escape codes for scroll regions, compatible with most terminals.

use crossterm::{cursor::MoveTo, execute, terminal};
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};

use super::colors::ansi;

/// Number of lines reserved at bottom (status + focus + input + mode indicator)
const RESERVED_LINES: u16 = 4;

/// ANSI escape codes for scroll region control
pub mod escape {
    /// Set scroll region from line `top` to line `bottom` (1-indexed)
    pub fn set_scroll_region(top: u16, bottom: u16) -> String {
        format!("\x1b[{};{}r", top, bottom)
    }

    /// Reset scroll region to full screen
    pub const RESET_SCROLL_REGION: &str = "\x1b[r";

    /// Save cursor position
    pub const SAVE_CURSOR: &str = "\x1b[s";

    /// Restore cursor position
    pub const RESTORE_CURSOR: &str = "\x1b[u";

    /// Move cursor to line (1-indexed), column 1
    pub fn move_to_line(line: u16) -> String {
        format!("\x1b[{};1H", line)
    }
}

/// Shared state for terminal layout
#[derive(Debug)]
pub struct LayoutState {
    /// Whether layout is active
    pub active: AtomicBool,
    /// Terminal height when layout was set up
    pub term_height: AtomicU16,
    /// Terminal width
    pub term_width: AtomicU16,
}

impl Default for LayoutState {
    fn default() -> Self {
        let (width, height) = terminal::size().unwrap_or((80, 24));
        Self {
            active: AtomicBool::new(false),
            term_height: AtomicU16::new(height),
            term_width: AtomicU16::new(width),
        }
    }
}

impl LayoutState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    pub fn height(&self) -> u16 {
        self.term_height.load(Ordering::SeqCst)
    }

    pub fn width(&self) -> u16 {
        self.term_width.load(Ordering::SeqCst)
    }

    /// Get the line number for status (1-indexed)
    pub fn status_line(&self) -> u16 {
        self.height().saturating_sub(3)
    }

    /// Get the line number for focus/detail (1-indexed)
    pub fn focus_line(&self) -> u16 {
        self.height().saturating_sub(2)
    }

    /// Get the line number for input (1-indexed)
    pub fn input_line(&self) -> u16 {
        self.height().saturating_sub(1)
    }

    /// Get the line number for mode indicator (1-indexed)
    pub fn mode_line(&self) -> u16 {
        self.height()
    }
}

/// Terminal layout manager with scroll regions
pub struct TerminalLayout {
    state: Arc<LayoutState>,
}

impl TerminalLayout {
    /// Create a new layout manager
    pub fn new() -> Self {
        Self {
            state: LayoutState::new(),
        }
    }

    /// Get shared state for external access
    pub fn state(&self) -> Arc<LayoutState> {
        self.state.clone()
    }

    /// Initialize the layout - sets up scroll region and fixed lines
    pub fn init(&self) -> io::Result<()> {
        let mut stdout = io::stdout();

        // Get current terminal size
        let (width, height) = terminal::size()?;
        self.state.term_width.store(width, Ordering::SeqCst);
        self.state.term_height.store(height, Ordering::SeqCst);

        // Calculate scroll region (leave RESERVED_LINES at bottom)
        let scroll_bottom = height.saturating_sub(RESERVED_LINES);

        // Move to bottom and create space for reserved lines
        execute!(stdout, MoveTo(0, height - 1))?;
        for _ in 0..RESERVED_LINES {
            println!();
        }

        // Set scroll region (top to scroll_bottom)
        print!("{}", escape::set_scroll_region(1, scroll_bottom));

        // Move cursor to top of scroll region
        execute!(stdout, MoveTo(0, 0))?;

        // Draw initial fixed lines (status, focus, input, mode)
        self.draw_status_line("")?;
        self.draw_focus_line(None)?;
        self.draw_input_line(false)?;
        self.draw_mode_line(false)?;

        // Move back to scroll region
        execute!(stdout, MoveTo(0, 0))?;

        self.state.active.store(true, Ordering::SeqCst);
        stdout.flush()?;

        Ok(())
    }

    /// Update the status line (progress indicator area)
    pub fn update_status(&self, content: &str) -> io::Result<()> {
        if !self.state.is_active() {
            return Ok(());
        }

        let mut stdout = io::stdout();
        let status_line = self.state.status_line();

        // Save cursor, move to status line, clear and print, restore
        print!("{}", escape::SAVE_CURSOR);
        print!("{}", escape::move_to_line(status_line));
        print!("{}", ansi::CLEAR_LINE);
        print!("{}", content);
        print!("{}", escape::RESTORE_CURSOR);
        stdout.flush()?;

        Ok(())
    }

    /// Draw the status line with optional content
    fn draw_status_line(&self, content: &str) -> io::Result<()> {
        let mut stdout = io::stdout();
        let status_line = self.state.status_line();

        print!("{}", escape::move_to_line(status_line));
        print!("{}", ansi::CLEAR_LINE);
        if !content.is_empty() {
            print!("{}", content);
        }
        stdout.flush()?;

        Ok(())
    }

    /// Draw the focus/detail line
    fn draw_focus_line(&self, content: Option<&str>) -> io::Result<()> {
        let mut stdout = io::stdout();
        let focus_line = self.state.focus_line();

        print!("{}", escape::move_to_line(focus_line));
        print!("{}", ansi::CLEAR_LINE);
        if let Some(text) = content {
            print!(
                "{}└{} {}{}{}",
                ansi::DIM,
                ansi::RESET,
                ansi::GRAY,
                text,
                ansi::RESET
            );
        }
        stdout.flush()?;

        Ok(())
    }

    /// Draw the input line
    fn draw_input_line(&self, _has_text: bool) -> io::Result<()> {
        let mut stdout = io::stdout();
        let input_line = self.state.input_line();

        print!("{}", escape::move_to_line(input_line));
        print!("{}", ansi::CLEAR_LINE);
        // Input prompt will be drawn by input handler
        stdout.flush()?;

        Ok(())
    }

    /// Draw the mode indicator line
    fn draw_mode_line(&self, plan_mode: bool) -> io::Result<()> {
        let mut stdout = io::stdout();
        let mode_line = self.state.mode_line();

        print!("{}", escape::move_to_line(mode_line));
        print!("{}", ansi::CLEAR_LINE);

        if plan_mode {
            print!(
                "{}⏸ plan mode on (shift+tab to switch){}",
                ansi::DIM,
                ansi::RESET
            );
        } else {
            print!(
                "{}▶ standard mode (shift+tab to switch){}",
                ansi::DIM,
                ansi::RESET
            );
        }
        stdout.flush()?;

        Ok(())
    }

    /// Update the mode indicator
    pub fn update_mode(&self, plan_mode: bool) -> io::Result<()> {
        if !self.state.is_active() {
            return Ok(());
        }

        let mut stdout = io::stdout();

        print!("{}", escape::SAVE_CURSOR);
        self.draw_mode_line(plan_mode)?;
        print!("{}", escape::RESTORE_CURSOR);
        stdout.flush()?;

        Ok(())
    }

    /// Position cursor at the input line for user input
    pub fn position_for_input(&self) -> io::Result<()> {
        if !self.state.is_active() {
            return Ok(());
        }

        let mut stdout = io::stdout();
        let input_line = self.state.input_line();

        print!("{}", escape::move_to_line(input_line));
        print!("{}", ansi::CLEAR_LINE);
        stdout.flush()?;

        Ok(())
    }

    /// Return cursor to scroll region (for output)
    pub fn position_for_output(&self) -> io::Result<()> {
        if !self.state.is_active() {
            return Ok(());
        }

        // Restore saved cursor position (in scroll region)
        print!("{}", escape::RESTORE_CURSOR);
        io::stdout().flush()?;

        Ok(())
    }

    /// Clean up - reset scroll region and restore terminal
    pub fn cleanup(&self) -> io::Result<()> {
        if !self.state.is_active() {
            return Ok(());
        }

        let mut stdout = io::stdout();

        // Reset scroll region
        print!("{}", escape::RESET_SCROLL_REGION);

        // Clear the fixed lines
        let height = self.state.height();
        for line in (height - RESERVED_LINES + 1)..=height {
            print!("{}", escape::move_to_line(line));
            print!("{}", ansi::CLEAR_LINE);
        }

        // Move to bottom
        execute!(stdout, MoveTo(0, height - 1))?;
        print!("{}", ansi::SHOW_CURSOR);

        self.state.active.store(false, Ordering::SeqCst);
        stdout.flush()?;

        Ok(())
    }

    /// Handle terminal resize
    pub fn handle_resize(&self) -> io::Result<()> {
        if !self.state.is_active() {
            return Ok(());
        }

        // Get new size
        let (width, height) = terminal::size()?;
        self.state.term_width.store(width, Ordering::SeqCst);
        self.state.term_height.store(height, Ordering::SeqCst);

        // Recalculate and set new scroll region
        let scroll_bottom = height.saturating_sub(RESERVED_LINES);
        print!("{}", escape::set_scroll_region(1, scroll_bottom));

        // Redraw fixed lines
        self.draw_status_line("")?;
        self.draw_focus_line(None)?;
        self.draw_input_line(false)?;
        self.draw_mode_line(false)?;

        io::stdout().flush()?;
        Ok(())
    }
}

impl Default for TerminalLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TerminalLayout {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Print content to the scroll region (normal output area)
/// This ensures output goes to the right place when layout is active
pub fn print_to_scroll_region(content: &str) {
    // Just print normally - the scroll region handles it
    print!("{}", content);
    let _ = io::stdout().flush();
}

/// Println to the scroll region
pub fn println_to_scroll_region(content: &str) {
    println!("{}", content);
    let _ = io::stdout().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_state_defaults() {
        let state = LayoutState::default();
        assert!(!state.is_active());
        assert!(state.height() > 0);
        assert!(state.width() > 0);
    }

    #[test]
    fn test_scroll_region_escape() {
        assert_eq!(escape::set_scroll_region(1, 20), "\x1b[1;20r");
        assert_eq!(escape::move_to_line(5), "\x1b[5;1H");
    }
}
