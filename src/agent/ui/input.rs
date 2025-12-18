//! Custom input handler with Claude Code-style @ file picker
//!
//! Provides:
//! - Real-time inline file suggestions when typing @
//! - Arrow key navigation in dropdown
//! - **Enter to SELECT suggestion** (not submit)
//! - Enter to SUBMIT only when no suggestions are active
//! - Support for multiple @ file references

use crate::agent::commands::SLASH_COMMANDS;
use crate::agent::ui::colors::ansi;
use crossterm::{
    cursor::{self, MoveToColumn, MoveUp},
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};
use std::path::PathBuf;

/// Result of reading user input
pub enum InputResult {
    /// User submitted text (Enter with no picker open)
    Submit(String),
    /// User cancelled (Ctrl+C or Escape with no picker)
    Cancel,
    /// User wants to exit
    Exit,
}

/// Suggestion item
#[derive(Clone)]
struct Suggestion {
    display: String,
    value: String,
    is_dir: bool,
}

/// Input state
struct InputState {
    /// Current input text
    text: String,
    /// Cursor position in text (character index)
    cursor: usize,
    /// Current suggestions
    suggestions: Vec<Suggestion>,
    /// Selected suggestion index (-1 = none selected)
    selected: i32,
    /// Whether suggestions dropdown is visible
    showing_suggestions: bool,
    /// Start position of current completion (@ position)
    completion_start: Option<usize>,
    /// Project path for file searches
    project_path: PathBuf,
    /// Number of lines rendered for suggestions (for cleanup)
    rendered_lines: usize,
}

impl InputState {
    fn new(project_path: PathBuf) -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            suggestions: Vec::new(),
            selected: -1,
            showing_suggestions: false,
            completion_start: None,
            project_path,
            rendered_lines: 0,
        }
    }

    /// Insert character at cursor
    fn insert_char(&mut self, c: char) {
        // Insert at cursor position
        let byte_pos = self.char_to_byte_pos(self.cursor);
        self.text.insert(byte_pos, c);
        self.cursor += 1;

        // Check if we should trigger completion
        if c == '@' {
            let valid_trigger = self.cursor == 1 ||
                self.text.chars().nth(self.cursor - 2).map(|c| c.is_whitespace()).unwrap_or(false);
            if valid_trigger {
                self.completion_start = Some(self.cursor - 1);
                self.refresh_suggestions();
            }
        } else if c == '/' && self.cursor == 1 {
            // Slash command at start
            self.completion_start = Some(0);
            self.refresh_suggestions();
        } else if c.is_whitespace() {
            // Space closes completion
            self.close_suggestions();
        } else if self.completion_start.is_some() {
            // Continue filtering
            self.refresh_suggestions();
        }
    }

    /// Delete character before cursor
    fn backspace(&mut self) {
        if self.cursor > 0 {
            let byte_pos = self.char_to_byte_pos(self.cursor - 1);
            let next_byte_pos = self.char_to_byte_pos(self.cursor);
            self.text.replace_range(byte_pos..next_byte_pos, "");
            self.cursor -= 1;

            // Check if we deleted the @ trigger
            if let Some(start) = self.completion_start {
                if self.cursor <= start {
                    self.close_suggestions();
                } else {
                    self.refresh_suggestions();
                }
            }
        }
    }

    /// Convert character position to byte position
    fn char_to_byte_pos(&self, char_pos: usize) -> usize {
        self.text.char_indices()
            .nth(char_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }

    /// Get the current filter text (after @ or /)
    fn get_filter(&self) -> Option<String> {
        self.completion_start.map(|start| {
            let filter_start = start + 1; // Skip the @ or /
            if filter_start <= self.cursor {
                self.text.chars().skip(filter_start).take(self.cursor - filter_start).collect()
            } else {
                String::new()
            }
        })
    }

    /// Refresh suggestions based on current filter
    fn refresh_suggestions(&mut self) {
        let filter = self.get_filter().unwrap_or_default();
        let trigger = self.completion_start
            .and_then(|pos| self.text.chars().nth(pos));

        self.suggestions = match trigger {
            Some('@') => self.search_files(&filter),
            Some('/') => self.search_commands(&filter),
            _ => Vec::new(),
        };

        self.showing_suggestions = !self.suggestions.is_empty();
        self.selected = if self.showing_suggestions { 0 } else { -1 };
    }

    /// Search for files matching filter
    fn search_files(&self, filter: &str) -> Vec<Suggestion> {
        let mut results = Vec::new();
        let filter_lower = filter.to_lowercase();

        self.walk_dir(&self.project_path.clone(), &filter_lower, &mut results, 0, 4);

        // Sort: directories first, then by path length
        results.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.value.len().cmp(&b.value.len()),
            }
        });

        results.truncate(8);
        results
    }

    /// Walk directory tree for matching files
    fn walk_dir(&self, dir: &PathBuf, filter: &str, results: &mut Vec<Suggestion>, depth: usize, max_depth: usize) {
        if depth > max_depth || results.len() >= 20 {
            return;
        }

        let skip_dirs = ["node_modules", ".git", "target", "__pycache__", ".venv", "venv", "dist", "build", ".next"];

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files (except some)
            if file_name.starts_with('.') && !file_name.starts_with(".env") && file_name != ".gitignore" {
                continue;
            }

            let rel_path = path.strip_prefix(&self.project_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| file_name.clone());

            let is_dir = path.is_dir();

            if filter.is_empty() || rel_path.to_lowercase().contains(filter) || file_name.to_lowercase().contains(filter) {
                let display = if is_dir {
                    format!("{}/", rel_path)
                } else {
                    rel_path.clone()
                };
                results.push(Suggestion {
                    display: display.clone(),
                    value: display,
                    is_dir,
                });
            }

            if is_dir && !skip_dirs.contains(&file_name.as_str()) {
                self.walk_dir(&path, filter, results, depth + 1, max_depth);
            }
        }
    }

    /// Search for slash commands matching filter
    fn search_commands(&self, filter: &str) -> Vec<Suggestion> {
        let filter_lower = filter.to_lowercase();

        SLASH_COMMANDS.iter()
            .filter(|cmd| {
                cmd.name.to_lowercase().starts_with(&filter_lower) ||
                cmd.alias.map(|a| a.to_lowercase().starts_with(&filter_lower)).unwrap_or(false)
            })
            .take(8)
            .map(|cmd| Suggestion {
                display: format!("/{:<12} {}", cmd.name, cmd.description),
                value: format!("/{}", cmd.name),
                is_dir: false,
            })
            .collect()
    }

    /// Close suggestions dropdown
    fn close_suggestions(&mut self) {
        self.showing_suggestions = false;
        self.suggestions.clear();
        self.selected = -1;
        self.completion_start = None;
    }

    /// Move selection up
    fn select_up(&mut self) {
        if self.showing_suggestions && !self.suggestions.is_empty() {
            if self.selected > 0 {
                self.selected -= 1;
            }
        }
    }

    /// Move selection down
    fn select_down(&mut self) {
        if self.showing_suggestions && !self.suggestions.is_empty() {
            if self.selected < self.suggestions.len() as i32 - 1 {
                self.selected += 1;
            }
        }
    }

    /// Accept the current selection
    fn accept_selection(&mut self) -> bool {
        if self.showing_suggestions && self.selected >= 0 {
            if let Some(suggestion) = self.suggestions.get(self.selected as usize) {
                if let Some(start) = self.completion_start {
                    // Replace @filter with @value
                    let before = self.text.chars().take(start).collect::<String>();
                    let after = self.text.chars().skip(self.cursor).collect::<String>();

                    // For files, use @path format; for commands, use /command
                    let replacement = if suggestion.value.starts_with('/') {
                        format!("{} ", suggestion.value)
                    } else {
                        format!("@{} ", suggestion.value)
                    };

                    self.text = format!("{}{}{}", before, replacement, after);
                    self.cursor = before.len() + replacement.len();
                }
                self.close_suggestions();
                return true;
            }
        }
        false
    }

    /// Move cursor left
    fn cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor right
    fn cursor_right(&mut self) {
        if self.cursor < self.text.chars().count() {
            self.cursor += 1;
        }
    }

    /// Move cursor to start
    fn cursor_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    fn cursor_end(&mut self) {
        self.cursor = self.text.chars().count();
    }
}

/// Render the input UI
fn render(state: &InputState, prompt: &str, stdout: &mut io::Stdout) -> io::Result<usize> {
    // Clear current line and render input
    execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;

    // Print prompt and input text
    print!("{}{}{} {}", ansi::SUCCESS, prompt, ansi::RESET, state.text);

    // Render suggestions below if active
    let mut lines_rendered = 0;
    if state.showing_suggestions && !state.suggestions.is_empty() {
        println!(); // Move to next line
        lines_rendered += 1;

        for (i, suggestion) in state.suggestions.iter().enumerate() {
            let is_selected = i as i32 == state.selected;
            let prefix = if is_selected { "▸" } else { " " };

            if is_selected {
                if suggestion.is_dir {
                    println!("\r  {}{} {}{}", ansi::CYAN, prefix, suggestion.display, ansi::RESET);
                } else {
                    println!("\r  {}{} {}{}", ansi::WHITE, prefix, suggestion.display, ansi::RESET);
                }
            } else {
                println!("\r  {}{} {}{}", ansi::DIM, prefix, suggestion.display, ansi::RESET);
            }
            lines_rendered += 1;
        }

        // Print hint
        println!("\r  {}[↑↓ navigate, Enter select, Esc cancel]{}", ansi::DIM, ansi::RESET);
        lines_rendered += 1;

        // Move cursor back up to input line
        execute!(stdout, MoveUp(lines_rendered as u16))?;
    }

    // Position cursor correctly within input
    let prompt_visual_len = prompt.len() + 1; // +1 for space
    let cursor_col = prompt_visual_len + state.text.chars().take(state.cursor).count();
    execute!(stdout, MoveToColumn(cursor_col as u16))?;

    stdout.flush()?;
    Ok(lines_rendered)
}

/// Clear rendered suggestion lines
fn clear_suggestions(num_lines: usize, stdout: &mut io::Stdout) -> io::Result<()> {
    if num_lines > 0 {
        // Save position, clear lines below, restore
        for _ in 0..num_lines {
            execute!(stdout,
                cursor::MoveDown(1),
                Clear(ClearType::CurrentLine)
            )?;
        }
        execute!(stdout, MoveUp(num_lines as u16))?;
    }
    Ok(())
}

/// Read user input with Claude Code-style @ file picker
pub fn read_input_with_file_picker(prompt: &str, project_path: &PathBuf) -> InputResult {
    let mut stdout = io::stdout();
    let mut state = InputState::new(project_path.clone());

    // Enable raw mode
    if terminal::enable_raw_mode().is_err() {
        return read_simple_input(prompt);
    }

    // Initial render
    print!("{}{}{} ", ansi::SUCCESS, prompt, ansi::RESET);
    let _ = stdout.flush();

    let result = loop {
        match event::read() {
            Ok(Event::Key(key_event)) => {
                // Clear previous suggestions before processing
                if state.rendered_lines > 0 {
                    let _ = clear_suggestions(state.rendered_lines, &mut stdout);
                }

                match key_event.code {
                    KeyCode::Enter => {
                        if state.showing_suggestions && state.selected >= 0 {
                            // Accept selection, don't submit
                            state.accept_selection();
                        } else if !state.text.trim().is_empty() {
                            // Submit
                            print!("\r\n");
                            let _ = stdout.flush();
                            break InputResult::Submit(state.text.clone());
                        }
                    }
                    KeyCode::Tab => {
                        // Tab also accepts selection
                        if state.showing_suggestions && state.selected >= 0 {
                            state.accept_selection();
                        }
                    }
                    KeyCode::Esc => {
                        if state.showing_suggestions {
                            state.close_suggestions();
                        } else {
                            print!("\r\n");
                            let _ = stdout.flush();
                            break InputResult::Cancel;
                        }
                    }
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !state.text.is_empty() {
                            // Clear input
                            state.text.clear();
                            state.cursor = 0;
                            state.close_suggestions();
                        } else {
                            print!("\r\n");
                            let _ = stdout.flush();
                            break InputResult::Cancel;
                        }
                    }
                    KeyCode::Char('d') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        print!("\r\n");
                        let _ = stdout.flush();
                        break InputResult::Exit;
                    }
                    KeyCode::Up => {
                        if state.showing_suggestions {
                            state.select_up();
                        }
                    }
                    KeyCode::Down => {
                        if state.showing_suggestions {
                            state.select_down();
                        }
                    }
                    KeyCode::Left => {
                        state.cursor_left();
                        // Close suggestions if cursor moves before @
                        if let Some(start) = state.completion_start {
                            if state.cursor <= start {
                                state.close_suggestions();
                            }
                        }
                    }
                    KeyCode::Right => {
                        state.cursor_right();
                    }
                    KeyCode::Home | KeyCode::Char('a') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.cursor_home();
                        state.close_suggestions();
                    }
                    KeyCode::End | KeyCode::Char('e') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.cursor_end();
                    }
                    KeyCode::Backspace => {
                        state.backspace();
                    }
                    KeyCode::Char(c) => {
                        state.insert_char(c);
                    }
                    _ => {}
                }

                // Re-render
                state.rendered_lines = render(&state, prompt, &mut stdout).unwrap_or(0);
            }
            Ok(Event::Resize(_, _)) => {
                // Redraw on resize
                state.rendered_lines = render(&state, prompt, &mut stdout).unwrap_or(0);
            }
            Err(_) => {
                break InputResult::Cancel;
            }
            _ => {}
        }
    };

    // Disable raw mode
    let _ = terminal::disable_raw_mode();

    // Clean up any remaining rendered lines
    if state.rendered_lines > 0 {
        let _ = clear_suggestions(state.rendered_lines, &mut stdout);
    }

    result
}

/// Simple fallback input without raw mode
fn read_simple_input(prompt: &str) -> InputResult {
    print!("{}{}{} ", ansi::SUCCESS, prompt, ansi::RESET);
    let _ = io::stdout().flush();

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            let trimmed = input.trim();
            if trimmed.eq_ignore_ascii_case("exit") || trimmed == "/exit" || trimmed == "/quit" {
                InputResult::Exit
            } else {
                InputResult::Submit(trimmed.to_string())
            }
        }
        Err(_) => InputResult::Cancel,
    }
}
