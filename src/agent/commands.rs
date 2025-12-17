//! Slash command definitions and interactive command picker
//!
//! Provides Gemini CLI-style "/" command system with:
//! - Interactive command picker when typing "/"
//! - Arrow key navigation
//! - Auto-complete on Enter
//! - Token usage tracking via /cost

use crate::agent::ui::colors::ansi;
use crossterm::{
    cursor::{self, MoveUp, MoveToColumn},
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};

/// A slash command definition
#[derive(Clone)]
pub struct SlashCommand {
    /// Command name (without the /)
    pub name: &'static str,
    /// Short alias (e.g., "m" for "model")
    pub alias: Option<&'static str>,
    /// Description shown in picker
    pub description: &'static str,
    /// Whether this command auto-executes on selection (vs. inserting text)
    pub auto_execute: bool,
}

/// All available slash commands
pub const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "model",
        alias: Some("m"),
        description: "Select a different AI model",
        auto_execute: true,
    },
    SlashCommand {
        name: "provider",
        alias: Some("p"),
        description: "Switch provider (OpenAI/Anthropic)",
        auto_execute: true,
    },
    SlashCommand {
        name: "cost",
        alias: None,
        description: "Show token usage and estimated cost",
        auto_execute: true,
    },
    SlashCommand {
        name: "clear",
        alias: Some("c"),
        description: "Clear conversation history",
        auto_execute: true,
    },
    SlashCommand {
        name: "help",
        alias: Some("h"),
        description: "Show available commands",
        auto_execute: true,
    },
    SlashCommand {
        name: "exit",
        alias: Some("q"),
        description: "Exit the chat",
        auto_execute: true,
    },
];

/// Token usage statistics for /cost command
#[derive(Debug, Default, Clone)]
pub struct TokenUsage {
    /// Total prompt/input tokens
    pub prompt_tokens: u64,
    /// Total completion/output tokens  
    pub completion_tokens: u64,
    /// Number of requests made
    pub request_count: u64,
    /// Session start time
    pub session_start: Option<std::time::Instant>,
}

impl TokenUsage {
    pub fn new() -> Self {
        Self {
            session_start: Some(std::time::Instant::now()),
            ..Default::default()
        }
    }

    /// Add tokens from a request
    pub fn add_request(&mut self, prompt: u64, completion: u64) {
        self.prompt_tokens += prompt;
        self.completion_tokens += completion;
        self.request_count += 1;
    }

    /// Estimate token count from text (rough approximation: ~4 chars per token)
    pub fn estimate_tokens(text: &str) -> u64 {
        (text.len() as f64 / 4.0).ceil() as u64
    }

    /// Get total tokens
    pub fn total_tokens(&self) -> u64 {
        self.prompt_tokens + self.completion_tokens
    }

    /// Get session duration
    pub fn session_duration(&self) -> std::time::Duration {
        self.session_start
            .map(|start| start.elapsed())
            .unwrap_or_default()
    }

    /// Estimate cost based on model (rough estimates in USD)
    /// Returns (input_cost, output_cost, total_cost)
    pub fn estimate_cost(&self, model: &str) -> (f64, f64, f64) {
        // Pricing per 1M tokens (as of Dec 2025, approximate)
        let (input_per_m, output_per_m) = match model {
            m if m.starts_with("gpt-5.2-mini") => (0.15, 0.60),
            m if m.starts_with("gpt-5") => (2.50, 10.00),
            m if m.starts_with("gpt-4o") => (2.50, 10.00),
            m if m.starts_with("o1") => (15.00, 60.00),
            m if m.contains("sonnet") => (3.00, 15.00),
            m if m.contains("opus") => (15.00, 75.00),
            m if m.contains("haiku") => (0.25, 1.25),
            _ => (2.50, 10.00), // Default to GPT-4o pricing
        };

        let input_cost = (self.prompt_tokens as f64 / 1_000_000.0) * input_per_m;
        let output_cost = (self.completion_tokens as f64 / 1_000_000.0) * output_per_m;
        
        (input_cost, output_cost, input_cost + output_cost)
    }

    /// Print cost report
    pub fn print_report(&self, model: &str) {
        let duration = self.session_duration();
        let (input_cost, output_cost, total_cost) = self.estimate_cost(model);

        println!();
        println!("  {}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", ansi::PURPLE, ansi::RESET);
        println!("  {}💰 Session Cost & Usage{}", ansi::PURPLE, ansi::RESET);
        println!("  {}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", ansi::PURPLE, ansi::RESET);
        println!();
        println!("  {}Model:{} {}", ansi::DIM, ansi::RESET, model);
        println!("  {}Duration:{} {:02}:{:02}:{:02}", 
            ansi::DIM, ansi::RESET,
            duration.as_secs() / 3600,
            (duration.as_secs() % 3600) / 60,
            duration.as_secs() % 60
        );
        println!("  {}Requests:{} {}", ansi::DIM, ansi::RESET, self.request_count);
        println!();
        println!("  {}Tokens:{}", ansi::CYAN, ansi::RESET);
        println!("    Input:  {:>10} tokens", self.prompt_tokens);
        println!("    Output: {:>10} tokens", self.completion_tokens);
        println!("    {}Total:  {:>10} tokens{}", ansi::BOLD, self.total_tokens(), ansi::RESET);
        println!();
        println!("  {}Estimated Cost:{}", ansi::SUCCESS, ansi::RESET);
        println!("    Input:  ${:.4}", input_cost);
        println!("    Output: ${:.4}", output_cost);
        println!("    {}Total:  ${:.4}{}", ansi::BOLD, total_cost, ansi::RESET);
        println!();
        println!("  {}(Estimates based on public API pricing){}", ansi::DIM, ansi::RESET);
        println!();
    }
}

/// Interactive command picker state
pub struct CommandPicker {
    /// Current filter text (after the /)
    pub filter: String,
    /// Currently selected index
    pub selected_index: usize,
    /// Filtered commands
    pub filtered_commands: Vec<&'static SlashCommand>,
}

impl CommandPicker {
    pub fn new() -> Self {
        Self {
            filter: String::new(),
            selected_index: 0,
            filtered_commands: SLASH_COMMANDS.iter().collect(),
        }
    }

    /// Update filter and refresh filtered commands
    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.to_lowercase();
        self.filtered_commands = SLASH_COMMANDS
            .iter()
            .filter(|cmd| {
                cmd.name.starts_with(&self.filter) ||
                cmd.alias.map(|a| a.starts_with(&self.filter)).unwrap_or(false)
            })
            .collect();
        
        // Reset selection if out of bounds
        if self.selected_index >= self.filtered_commands.len() {
            self.selected_index = 0;
        }
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if !self.filtered_commands.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down  
    pub fn move_down(&mut self) {
        if !self.filtered_commands.is_empty() && self.selected_index < self.filtered_commands.len() - 1 {
            self.selected_index += 1;
        }
    }

    /// Get currently selected command
    pub fn selected_command(&self) -> Option<&'static SlashCommand> {
        self.filtered_commands.get(self.selected_index).copied()
    }

    /// Render the picker suggestions below current line
    pub fn render_suggestions(&self) -> usize {
        let mut stdout = io::stdout();
        
        if self.filtered_commands.is_empty() {
            println!("\n  {}No matching commands{}", ansi::DIM, ansi::RESET);
            let _ = stdout.flush();
            return 1;
        }

        for (i, cmd) in self.filtered_commands.iter().enumerate() {
            let is_selected = i == self.selected_index;
            
            if is_selected {
                // Selected item - highlighted with arrow
                println!("  {}▸ /{:<15}{} {}{}{}", 
                    ansi::PURPLE, cmd.name, ansi::RESET,
                    ansi::PURPLE, cmd.description, ansi::RESET);
            } else {
                // Normal item - dimmed
                println!("  {}  /{:<15} {}{}", 
                    ansi::DIM, cmd.name, cmd.description, ansi::RESET);
            }
        }
        
        let _ = stdout.flush();
        self.filtered_commands.len()
    }

    /// Clear n lines above cursor
    pub fn clear_lines(&self, num_lines: usize) {
        let mut stdout = io::stdout();
        for _ in 0..num_lines {
            let _ = execute!(stdout, MoveUp(1), Clear(ClearType::CurrentLine));
        }
        let _ = stdout.flush();
    }
}

/// Show interactive command picker and return selected command
/// This is called when user types "/" - shows suggestions immediately
/// Returns None if cancelled, Some(command_name) if selected
pub fn show_command_picker(initial_filter: &str) -> Option<String> {
    let mut picker = CommandPicker::new();
    picker.set_filter(initial_filter);
    
    // Enable raw mode for real-time key handling
    if terminal::enable_raw_mode().is_err() {
        // Fallback to simple mode if raw mode fails
        return show_simple_picker(&picker);
    }
    
    let mut stdout = io::stdout();
    let mut input_buffer = format!("/{}", initial_filter);
    let mut last_rendered_lines = 0;
    
    // Initial render
    println!(); // Move to new line for suggestions
    last_rendered_lines = picker.render_suggestions();
    
    // Move back up to input line and position cursor
    let _ = execute!(stdout, MoveUp(last_rendered_lines as u16 + 1), MoveToColumn(0));
    print!("{}You: {}{}", ansi::SUCCESS, ansi::RESET, input_buffer);
    let _ = stdout.flush();
    
    // Move down to after suggestions
    let _ = execute!(stdout, cursor::MoveDown(last_rendered_lines as u16 + 1));
    
    let result = loop {
        // Wait for key event
        if let Ok(Event::Key(key_event)) = event::read() {
            match key_event.code {
                KeyCode::Esc => {
                    // Cancel
                    break None;
                }
                KeyCode::Enter => {
                    // Select current
                    if let Some(cmd) = picker.selected_command() {
                        break Some(cmd.name.to_string());
                    }
                    break None;
                }
                KeyCode::Up => {
                    picker.move_up();
                }
                KeyCode::Down => {
                    picker.move_down();
                }
                KeyCode::Backspace => {
                    if input_buffer.len() > 1 {
                        input_buffer.pop();
                        let filter = input_buffer.trim_start_matches('/');
                        picker.set_filter(filter);
                    } else {
                        // Backspace on just "/" - cancel
                        break None;
                    }
                }
                KeyCode::Char(c) => {
                    // Add character to filter
                    input_buffer.push(c);
                    let filter = input_buffer.trim_start_matches('/');
                    picker.set_filter(filter);
                    
                    // If there's an exact match and user typed enough, auto-select
                    if picker.filtered_commands.len() == 1 {
                        // Perfect match - could auto-complete
                    }
                }
                KeyCode::Tab => {
                    // Tab to auto-complete current selection
                    if let Some(cmd) = picker.selected_command() {
                        break Some(cmd.name.to_string());
                    }
                }
                _ => {}
            }
            
            // Clear old suggestions and re-render
            picker.clear_lines(last_rendered_lines);
            
            // Re-render input line
            let _ = execute!(stdout, Clear(ClearType::CurrentLine), MoveToColumn(0));
            print!("{}You: {}{}", ansi::SUCCESS, ansi::RESET, input_buffer);
            let _ = stdout.flush();
            
            // Render suggestions below
            println!();
            last_rendered_lines = picker.render_suggestions();
            
            // Move back to input line position
            let _ = execute!(stdout, MoveUp(last_rendered_lines as u16 + 1));
            let _ = execute!(stdout, MoveToColumn((5 + input_buffer.len()) as u16));
            let _ = stdout.flush();
            
            // Move down to after suggestions for next iteration
            let _ = execute!(stdout, cursor::MoveDown(last_rendered_lines as u16 + 1));
        }
    };
    
    // Disable raw mode
    let _ = terminal::disable_raw_mode();
    
    // Clean up display
    picker.clear_lines(last_rendered_lines);
    let _ = execute!(stdout, Clear(ClearType::CurrentLine), MoveToColumn(0));
    let _ = stdout.flush();
    
    result
}

/// Fallback simple picker when raw mode is not available
fn show_simple_picker(picker: &CommandPicker) -> Option<String> {
    println!();
    println!("  {}📋 Available Commands:{}", ansi::CYAN, ansi::RESET);
    println!();
    
    for (i, cmd) in picker.filtered_commands.iter().enumerate() {
        print!("  {} {}/{:<12}", format!("[{}]", i + 1), ansi::PURPLE, cmd.name);
        if let Some(alias) = cmd.alias {
            print!(" ({})", alias);
        }
        println!("{} - {}{}{}", ansi::RESET, ansi::DIM, cmd.description, ansi::RESET);
    }
    
    println!();
    print!("  Select (1-{}) or press Enter to cancel: ", picker.filtered_commands.len());
    let _ = io::stdout().flush();
    
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let input = input.trim();
        if let Ok(num) = input.parse::<usize>() {
            if num >= 1 && num <= picker.filtered_commands.len() {
                return Some(picker.filtered_commands[num - 1].name.to_string());
            }
        }
    }
    
    None
}

/// Check if a command matches a query (name or alias)
pub fn match_command(query: &str) -> Option<&'static SlashCommand> {
    let query = query.trim_start_matches('/').to_lowercase();
    
    SLASH_COMMANDS.iter().find(|cmd| {
        cmd.name == query || cmd.alias.map(|a| a == query).unwrap_or(false)
    })
}
