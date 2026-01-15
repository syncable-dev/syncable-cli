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
    cursor::{self, MoveUp},
    event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyModifiers},
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
    /// User toggled planning mode (Shift+Tab)
    TogglePlanMode,
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
    /// Number of wrapped lines the input text occupied in last render
    prev_wrapped_lines: usize,
    /// Whether in plan mode (shows ★ indicator)
    plan_mode: bool,
}

impl InputState {
    fn new(project_path: PathBuf, plan_mode: bool) -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            suggestions: Vec::new(),
            selected: -1,
            showing_suggestions: false,
            completion_start: None,
            project_path,
            rendered_lines: 0,
            prev_wrapped_lines: 1,
            plan_mode,
        }
    }

    /// Insert character at cursor
    fn insert_char(&mut self, c: char) {
        // Skip carriage returns, keep newlines for multi-line support
        if c == '\r' {
            return;
        }

        // Insert at cursor position
        let byte_pos = self.char_to_byte_pos(self.cursor);
        self.text.insert(byte_pos, c);
        self.cursor += 1;

        // Check if we should trigger completion
        if c == '@' {
            let valid_trigger = self.cursor == 1
                || self
                    .text
                    .chars()
                    .nth(self.cursor - 2)
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false);
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

    /// Delete word before cursor (Ctrl+Backspace / Cmd+Delete / Alt+Backspace)
    fn delete_word_left(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let chars: Vec<char> = self.text.chars().collect();
        let mut new_cursor = self.cursor;

        // Skip whitespace going backwards
        while new_cursor > 0 && chars[new_cursor - 1].is_whitespace() {
            new_cursor -= 1;
        }

        // Skip word characters going backwards
        while new_cursor > 0 && !chars[new_cursor - 1].is_whitespace() {
            new_cursor -= 1;
        }

        // Delete from new_cursor to current cursor
        let start_byte = self.char_to_byte_pos(new_cursor);
        let end_byte = self.char_to_byte_pos(self.cursor);
        self.text.replace_range(start_byte..end_byte, "");
        self.cursor = new_cursor;

        // Update suggestions if active
        if let Some(start) = self.completion_start {
            if self.cursor <= start {
                self.close_suggestions();
            } else {
                self.refresh_suggestions();
            }
        }
    }

    /// Clear entire input (Ctrl+U)
    fn clear_all(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.close_suggestions();
    }

    /// Delete from cursor to beginning of current line (Cmd+Backspace)
    fn delete_to_line_start(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let chars: Vec<char> = self.text.chars().collect();

        // Find the previous newline or start of text
        let mut line_start = self.cursor;
        while line_start > 0 && chars[line_start - 1] != '\n' {
            line_start -= 1;
        }

        // If cursor is at line start, delete the newline to join with previous line
        if line_start == self.cursor && self.cursor > 0 {
            line_start -= 1;
        }

        // Delete from line_start to cursor
        let start_byte = self.char_to_byte_pos(line_start);
        let end_byte = self.char_to_byte_pos(self.cursor);
        self.text.replace_range(start_byte..end_byte, "");
        self.cursor = line_start;

        self.close_suggestions();
    }

    /// Convert character position to byte position
    fn char_to_byte_pos(&self, char_pos: usize) -> usize {
        self.text
            .char_indices()
            .nth(char_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }

    /// Get the current filter text (after @ or /)
    fn get_filter(&self) -> Option<String> {
        self.completion_start.map(|start| {
            let filter_start = start + 1; // Skip the @ or /
            if filter_start <= self.cursor {
                self.text
                    .chars()
                    .skip(filter_start)
                    .take(self.cursor - filter_start)
                    .collect()
            } else {
                String::new()
            }
        })
    }

    /// Refresh suggestions based on current filter
    fn refresh_suggestions(&mut self) {
        let filter = self.get_filter().unwrap_or_default();
        let trigger = self
            .completion_start
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

        self.walk_dir(
            &self.project_path.clone(),
            &filter_lower,
            &mut results,
            0,
            4,
        );

        // Sort: directories first, then by path length
        results.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.value.len().cmp(&b.value.len()),
        });

        results.truncate(8);
        results
    }

    /// Walk directory tree for matching files
    fn walk_dir(
        &self,
        dir: &PathBuf,
        filter: &str,
        results: &mut Vec<Suggestion>,
        depth: usize,
        max_depth: usize,
    ) {
        if depth > max_depth || results.len() >= 20 {
            return;
        }

        let skip_dirs = [
            "node_modules",
            ".git",
            "target",
            "__pycache__",
            ".venv",
            "venv",
            "dist",
            "build",
            ".next",
        ];

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files (except some)
            if file_name.starts_with('.')
                && !file_name.starts_with(".env")
                && file_name != ".gitignore"
            {
                continue;
            }

            let rel_path = path
                .strip_prefix(&self.project_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| file_name.clone());

            let is_dir = path.is_dir();

            if filter.is_empty()
                || rel_path.to_lowercase().contains(filter)
                || file_name.to_lowercase().contains(filter)
            {
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

        SLASH_COMMANDS
            .iter()
            .filter(|cmd| {
                cmd.name.to_lowercase().starts_with(&filter_lower)
                    || cmd
                        .alias
                        .map(|a| a.to_lowercase().starts_with(&filter_lower))
                        .unwrap_or(false)
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
        if self.showing_suggestions && !self.suggestions.is_empty() && self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    fn select_down(&mut self) {
        if self.showing_suggestions
            && !self.suggestions.is_empty()
            && self.selected < self.suggestions.len() as i32 - 1
        {
            self.selected += 1;
        }
    }

    /// Accept the current selection
    fn accept_selection(&mut self) -> bool {
        if self.showing_suggestions
            && self.selected >= 0
            && let Some(suggestion) = self.suggestions.get(self.selected as usize)
        {
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

    /// Move cursor up one line
    fn cursor_up(&mut self) {
        let chars: Vec<char> = self.text.chars().collect();
        if self.cursor == 0 {
            return;
        }

        // Find the start of the current line
        let mut current_line_start = self.cursor;
        while current_line_start > 0 && chars[current_line_start - 1] != '\n' {
            current_line_start -= 1;
        }

        // If we're on the first line, can't go up
        if current_line_start == 0 {
            return;
        }

        // Find the column position on current line
        let col = self.cursor - current_line_start;

        // Find the start of the previous line
        let prev_line_end = current_line_start - 1; // Position of the \n
        let mut prev_line_start = prev_line_end;
        while prev_line_start > 0 && chars[prev_line_start - 1] != '\n' {
            prev_line_start -= 1;
        }

        // Calculate the length of the previous line
        let prev_line_len = prev_line_end - prev_line_start;

        // Move cursor to same column on previous line (or end if line is shorter)
        self.cursor = prev_line_start + col.min(prev_line_len);
    }

    /// Move cursor down one line
    fn cursor_down(&mut self) {
        let chars: Vec<char> = self.text.chars().collect();
        let text_len = chars.len();

        // Find the start of the current line
        let mut current_line_start = self.cursor;
        while current_line_start > 0 && chars[current_line_start - 1] != '\n' {
            current_line_start -= 1;
        }

        // Find the column position on current line
        let col = self.cursor - current_line_start;

        // Find the end of the current line (the \n or end of text)
        let mut current_line_end = self.cursor;
        while current_line_end < text_len && chars[current_line_end] != '\n' {
            current_line_end += 1;
        }

        // If we're on the last line, can't go down
        if current_line_end >= text_len {
            return;
        }

        // Find the start of the next line
        let next_line_start = current_line_end + 1;

        // Find the end of the next line
        let mut next_line_end = next_line_start;
        while next_line_end < text_len && chars[next_line_end] != '\n' {
            next_line_end += 1;
        }

        // Calculate the length of the next line
        let next_line_len = next_line_end - next_line_start;

        // Move cursor to same column on next line (or end if line is shorter)
        self.cursor = next_line_start + col.min(next_line_len);
    }
}

/// Render the input UI with multi-line support
fn render(state: &mut InputState, prompt: &str, stdout: &mut io::Stdout) -> io::Result<usize> {
    // Get terminal width
    let (term_width, _) = terminal::size().unwrap_or((80, 24));
    let term_width = term_width as usize;

    // Calculate prompt length (include ★ prefix if in plan mode)
    let mode_prefix_len = if state.plan_mode { 2 } else { 0 }; // "★ " = 2 chars
    let prompt_len = prompt.len() + 1 + mode_prefix_len; // +1 for space after prompt

    // Move up to clear previous rendered lines, then to column 0
    if state.prev_wrapped_lines > 1 {
        execute!(
            stdout,
            cursor::MoveUp((state.prev_wrapped_lines - 1) as u16)
        )?;
    }
    execute!(stdout, cursor::MoveToColumn(0))?;

    // Clear from cursor to end of screen
    execute!(stdout, Clear(ClearType::FromCursorDown))?;

    // Print prompt and input text with mode indicator if in plan mode
    // In raw mode, \n doesn't return to column 0, so we need \r\n
    let display_text = state.text.replace('\n', "\r\n");
    if state.plan_mode {
        print!(
            "{}★{} {}{}{} {}",
            ansi::ORANGE,
            ansi::RESET,
            ansi::SUCCESS,
            prompt,
            ansi::RESET,
            display_text
        );
    } else {
        print!(
            "{}{}{} {}",
            ansi::SUCCESS,
            prompt,
            ansi::RESET,
            display_text
        );
    }
    stdout.flush()?;

    // Calculate how many lines the text spans (counting newlines + wrapping)
    let mut total_lines = 1;
    let mut current_line_len = prompt_len;

    for c in state.text.chars() {
        if c == '\n' {
            total_lines += 1;
            current_line_len = 0;
        } else {
            current_line_len += 1;
            if term_width > 0 && current_line_len > term_width {
                total_lines += 1;
                current_line_len = 1;
            }
        }
    }
    state.prev_wrapped_lines = total_lines;

    // Render suggestions below if active
    let mut lines_rendered = 0;
    if state.showing_suggestions && !state.suggestions.is_empty() {
        // Move to next line for suggestions (use \r\n in raw mode)
        print!("\r\n");
        lines_rendered += 1;

        for (i, suggestion) in state.suggestions.iter().enumerate() {
            let is_selected = i as i32 == state.selected;
            let prefix = if is_selected { "▸" } else { " " };

            if is_selected {
                if suggestion.is_dir {
                    print!(
                        "  {}{} {}{}\r\n",
                        ansi::CYAN,
                        prefix,
                        suggestion.display,
                        ansi::RESET
                    );
                } else {
                    print!(
                        "  {}{} {}{}\r\n",
                        ansi::WHITE,
                        prefix,
                        suggestion.display,
                        ansi::RESET
                    );
                }
            } else {
                print!(
                    "  {}{} {}{}\r\n",
                    ansi::DIM,
                    prefix,
                    suggestion.display,
                    ansi::RESET
                );
            }
            lines_rendered += 1;
        }

        // Print hint
        print!(
            "  {}[↑↓ navigate, Enter select, Esc cancel]{}\r\n",
            ansi::DIM,
            ansi::RESET
        );
        lines_rendered += 1;
    }

    // Position cursor correctly within input (handling newlines)
    // Calculate which line and column the cursor is on
    let mut cursor_line = 0;
    let mut cursor_col = prompt_len;

    for (i, c) in state.text.chars().enumerate() {
        if i >= state.cursor {
            break;
        }
        if c == '\n' {
            cursor_line += 1;
            cursor_col = 0;
        } else {
            cursor_col += 1;
            if term_width > 0 && cursor_col >= term_width {
                cursor_line += 1;
                cursor_col = 0;
            }
        }
    }

    // Move cursor from end of text to correct position
    let lines_after_cursor = total_lines.saturating_sub(cursor_line + 1) + lines_rendered;
    if lines_after_cursor > 0 {
        execute!(stdout, cursor::MoveUp(lines_after_cursor as u16))?;
    }
    execute!(stdout, cursor::MoveToColumn(cursor_col as u16))?;

    stdout.flush()?;
    Ok(lines_rendered)
}

/// Clear rendered suggestion lines
fn clear_suggestions(num_lines: usize, stdout: &mut io::Stdout) -> io::Result<()> {
    if num_lines > 0 {
        // Save position, clear lines below, restore
        for _ in 0..num_lines {
            execute!(stdout, cursor::MoveDown(1), Clear(ClearType::CurrentLine))?;
        }
        execute!(stdout, MoveUp(num_lines as u16))?;
    }
    Ok(())
}

/// Read user input with Claude Code-style @ file picker
/// If `plan_mode` is true, shows the plan mode indicator below the prompt
pub fn read_input_with_file_picker(
    prompt: &str,
    project_path: &std::path::Path,
    plan_mode: bool,
) -> InputResult {
    let mut stdout = io::stdout();

    // Always ensure cursor is visible at start of input (may have been hidden by progress indicator)
    print!("{}", ansi::SHOW_CURSOR);
    let _ = stdout.flush();

    // Enable raw mode
    if terminal::enable_raw_mode().is_err() {
        return read_simple_input(prompt);
    }

    // Enable bracketed paste mode to detect paste vs keypress
    let _ = execute!(stdout, EnableBracketedPaste);

    // Print prompt with mode indicator inline (no separate line)
    if plan_mode {
        print!(
            "{}★{} {}{}{} ",
            ansi::ORANGE,
            ansi::RESET,
            ansi::SUCCESS,
            prompt,
            ansi::RESET
        );
    } else {
        print!("{}{}{} ", ansi::SUCCESS, prompt, ansi::RESET);
    }
    let _ = stdout.flush();

    // Create state after printing prompt so start_row is correct
    let mut state = InputState::new(project_path.to_path_buf(), plan_mode);

    let result = loop {
        match event::read() {
            // Handle paste events - insert all pasted text at once
            Ok(Event::Paste(pasted_text)) => {
                // Normalize line endings: \r\n -> \n, lone \r -> \n
                let normalized = pasted_text.replace("\r\n", "\n").replace('\r', "\n");
                for c in normalized.chars() {
                    state.insert_char(c);
                }
                // Render after paste completes
                state.rendered_lines = render(&mut state, prompt, &mut stdout).unwrap_or(0);
            }
            Ok(Event::Key(key_event)) => {
                match key_event.code {
                    KeyCode::Enter => {
                        // Shift+Enter or Alt+Enter inserts newline instead of submitting
                        if key_event.modifiers.contains(KeyModifiers::SHIFT)
                            || key_event.modifiers.contains(KeyModifiers::ALT)
                        {
                            state.insert_char('\n');
                        } else if state.showing_suggestions && state.selected >= 0 {
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
                    KeyCode::BackTab => {
                        // Shift+Tab toggles planning mode
                        print!("\r\n");
                        let _ = stdout.flush();
                        break InputResult::TogglePlanMode;
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
                        // Ctrl+C always exits (consistent with standard CLI behavior)
                        print!("\r\n");
                        let _ = stdout.flush();
                        break InputResult::Cancel;
                    }
                    KeyCode::Char('d') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        print!("\r\n");
                        let _ = stdout.flush();
                        break InputResult::Exit;
                    }
                    KeyCode::Up => {
                        if state.showing_suggestions {
                            state.select_up();
                        } else {
                            state.cursor_up();
                        }
                    }
                    KeyCode::Down => {
                        if state.showing_suggestions {
                            state.select_down();
                        } else {
                            state.cursor_down();
                        }
                    }
                    KeyCode::Left => {
                        state.cursor_left();
                        // Close suggestions if cursor moves before @
                        if let Some(start) = state.completion_start
                            && state.cursor <= start
                        {
                            state.close_suggestions();
                        }
                    }
                    KeyCode::Right => {
                        state.cursor_right();
                    }
                    KeyCode::Home | KeyCode::Char('a')
                        if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        state.cursor_home();
                        state.close_suggestions();
                    }
                    KeyCode::End | KeyCode::Char('e')
                        if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        state.cursor_end();
                    }
                    // Ctrl+U - Clear entire input
                    KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.clear_all();
                    }
                    // Ctrl+K - Delete to beginning of current line (works on all platforms)
                    KeyCode::Char('k') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.delete_to_line_start();
                    }
                    // Ctrl+Shift+Backspace - Delete to beginning of current line (cross-platform)
                    KeyCode::Backspace
                        if key_event.modifiers.contains(KeyModifiers::CONTROL)
                            && key_event.modifiers.contains(KeyModifiers::SHIFT) =>
                    {
                        state.delete_to_line_start();
                    }
                    // Cmd+Backspace (Mac) - Delete to beginning of current line (if terminal passes it)
                    KeyCode::Backspace if key_event.modifiers.contains(KeyModifiers::SUPER) => {
                        state.delete_to_line_start();
                    }
                    // Ctrl+W or Alt+Backspace - Delete word left
                    KeyCode::Char('w') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.delete_word_left();
                    }
                    KeyCode::Backspace if key_event.modifiers.contains(KeyModifiers::ALT) => {
                        state.delete_word_left();
                    }
                    KeyCode::Backspace if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.delete_word_left();
                    }
                    // Ctrl+J - Insert newline (multi-line input)
                    KeyCode::Char('j') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.insert_char('\n');
                    }
                    KeyCode::Backspace => {
                        state.backspace();
                    }
                    KeyCode::Char('\n') => {
                        // Handle newline char that might come through during paste
                        state.insert_char('\n');
                    }
                    KeyCode::Char(c) => {
                        state.insert_char(c);
                    }
                    _ => {}
                }

                // Only render if no more events are pending (batches rapid input like paste)
                // This prevents thousands of renders during paste operations
                let should_render =
                    !event::poll(std::time::Duration::from_millis(0)).unwrap_or(false);
                if should_render {
                    state.rendered_lines = render(&mut state, prompt, &mut stdout).unwrap_or(0);
                }
            }
            Ok(Event::Resize(_, _)) => {
                // Redraw on resize
                state.rendered_lines = render(&mut state, prompt, &mut stdout).unwrap_or(0);
            }
            Err(_) => {
                break InputResult::Cancel;
            }
            _ => {}
        }
    };

    // Disable bracketed paste mode
    let _ = execute!(stdout, DisableBracketedPaste);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn new_state() -> InputState {
        InputState::new(PathBuf::from("/tmp"), false)
    }

    #[test]
    fn test_insert_char_basic() {
        let mut state = new_state();
        state.insert_char('h');
        state.insert_char('i');
        assert_eq!(state.text, "hi");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn test_insert_char_utf8() {
        let mut state = new_state();
        state.insert_char('日');
        state.insert_char('本');
        assert_eq!(state.text, "日本");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn test_insert_char_skips_cr() {
        let mut state = new_state();
        state.insert_char('a');
        state.insert_char('\r');
        state.insert_char('b');
        assert_eq!(state.text, "ab");
    }

    #[test]
    fn test_backspace_basic() {
        let mut state = new_state();
        state.insert_char('h');
        state.insert_char('e');
        state.insert_char('l');
        state.backspace();
        assert_eq!(state.text, "he");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn test_backspace_utf8() {
        let mut state = new_state();
        state.insert_char('日');
        state.insert_char('本');
        state.backspace();
        assert_eq!(state.text, "日");
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn test_backspace_at_start() {
        let mut state = new_state();
        state.backspace(); // Should not panic
        assert_eq!(state.text, "");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_cursor_movement() {
        let mut state = new_state();
        state.insert_char('h');
        state.insert_char('e');
        state.insert_char('l');
        state.insert_char('l');
        state.insert_char('o');
        assert_eq!(state.cursor, 5);

        state.cursor_left();
        assert_eq!(state.cursor, 4);

        state.cursor_home();
        assert_eq!(state.cursor, 0);

        state.cursor_right();
        assert_eq!(state.cursor, 1);

        state.cursor_end();
        assert_eq!(state.cursor, 5);
    }

    #[test]
    fn test_cursor_bounds() {
        let mut state = new_state();
        state.insert_char('a');

        state.cursor_left();
        state.cursor_left(); // Should not go below 0
        assert_eq!(state.cursor, 0);

        state.cursor_right();
        state.cursor_right(); // Should not go beyond text length
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn test_char_to_byte_pos_ascii() {
        let mut state = new_state();
        state.text = "hello".to_string();
        assert_eq!(state.char_to_byte_pos(0), 0);
        assert_eq!(state.char_to_byte_pos(2), 2);
        assert_eq!(state.char_to_byte_pos(5), 5);
    }

    #[test]
    fn test_char_to_byte_pos_utf8() {
        let mut state = new_state();
        state.text = "日本語".to_string(); // Each char is 3 bytes
        assert_eq!(state.char_to_byte_pos(0), 0);
        assert_eq!(state.char_to_byte_pos(1), 3);
        assert_eq!(state.char_to_byte_pos(2), 6);
        assert_eq!(state.char_to_byte_pos(3), 9);
    }

    #[test]
    fn test_clear_all() {
        let mut state = new_state();
        state.insert_char('h');
        state.insert_char('e');
        state.insert_char('l');
        state.clear_all();
        assert_eq!(state.text, "");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_delete_word_left() {
        let mut state = new_state();
        for c in "hello world".chars() {
            state.insert_char(c);
        }
        state.delete_word_left();
        assert_eq!(state.text, "hello ");
        assert_eq!(state.cursor, 6);
    }

    #[test]
    fn test_multiline_cursor_navigation() {
        let mut state = new_state();
        // "ab\ncd"
        for c in "ab".chars() {
            state.insert_char(c);
        }
        state.insert_char('\n');
        for c in "cd".chars() {
            state.insert_char(c);
        }
        assert_eq!(state.cursor, 5); // at end

        state.cursor_up();
        assert_eq!(state.cursor, 2); // end of first line "ab"

        state.cursor_down();
        assert_eq!(state.cursor, 5); // back to end
    }

    #[test]
    fn test_get_filter_at_symbol() {
        let mut state = new_state();
        state.text = "@src".to_string();
        state.cursor = 4;
        state.completion_start = Some(0);
        assert_eq!(state.get_filter(), Some("src".to_string()));
    }

    #[test]
    fn test_get_filter_no_completion() {
        let mut state = new_state();
        state.text = "hello".to_string();
        state.cursor = 5;
        assert_eq!(state.get_filter(), None);
    }
}
