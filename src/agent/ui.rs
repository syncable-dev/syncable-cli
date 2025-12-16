//! Beautiful terminal UI for the agent
//!
//! Provides colorful output, markdown rendering, and tool call animations.

use console::{style, Emoji, Term};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

// Emojis for different states
pub static ROBOT: Emoji<'_, '_> = Emoji("🤖 ", "");
pub static THINKING: Emoji<'_, '_> = Emoji("💭 ", "");
pub static TOOL: Emoji<'_, '_> = Emoji("🔧 ", "");
pub static SUCCESS: Emoji<'_, '_> = Emoji("✅ ", "[OK] ");
pub static ERROR: Emoji<'_, '_> = Emoji("❌ ", "[ERR] ");
pub static SEARCH: Emoji<'_, '_> = Emoji("🔍 ", "");
pub static SECURITY: Emoji<'_, '_> = Emoji("🛡️ ", "");
pub static FILE: Emoji<'_, '_> = Emoji("📄 ", "");
pub static FOLDER: Emoji<'_, '_> = Emoji("📁 ", "");
pub static SPARKLES: Emoji<'_, '_> = Emoji("✨ ", "");
pub static ARROW: Emoji<'_, '_> = Emoji("➜ ", "> ");

/// Print the SYNCABLE ASCII art logo with gradient colors
pub fn print_logo() {
    // Colors matching the logo gradient: purple → orange → pink
    // Using ANSI 256 colors for better gradient
    
    // Purple shades for S, y
    let purple = "\x1b[38;5;141m";  // Light purple
    // Orange shades for n, c  
    let orange = "\x1b[38;5;216m";  // Peach/orange
    // Pink shades for a, b, l, e
    let pink = "\x1b[38;5;212m";    // Hot pink
    let magenta = "\x1b[38;5;207m"; // Magenta
    let reset = "\x1b[0m";

    println!();
    println!(
        "{}  ███████╗{}{} ██╗   ██╗{}{}███╗   ██╗{}{} ██████╗{}{}  █████╗ {}{}██████╗ {}{}██╗     {}{}███████╗{}",
        purple, reset, purple, reset, orange, reset, orange, reset, pink, reset, pink, reset, magenta, reset, magenta, reset
    );
    println!(
        "{}  ██╔════╝{}{} ╚██╗ ██╔╝{}{}████╗  ██║{}{} ██╔════╝{}{} ██╔══██╗{}{}██╔══██╗{}{}██║     {}{}██╔════╝{}",
        purple, reset, purple, reset, orange, reset, orange, reset, pink, reset, pink, reset, magenta, reset, magenta, reset
    );
    println!(
        "{}  ███████╗{}{}  ╚████╔╝ {}{}██╔██╗ ██║{}{} ██║     {}{} ███████║{}{}██████╔╝{}{}██║     {}{}█████╗  {}",
        purple, reset, purple, reset, orange, reset, orange, reset, pink, reset, pink, reset, magenta, reset, magenta, reset
    );
    println!(
        "{}  ╚════██║{}{}   ╚██╔╝  {}{}██║╚██╗██║{}{} ██║     {}{} ██╔══██║{}{}██╔══██╗{}{}██║     {}{}██╔══╝  {}",
        purple, reset, purple, reset, orange, reset, orange, reset, pink, reset, pink, reset, magenta, reset, magenta, reset
    );
    println!(
        "{}  ███████║{}{}    ██║   {}{}██║ ╚████║{}{} ╚██████╗{}{} ██║  ██║{}{}██████╔╝{}{}███████╗{}{}███████╗{}",
        purple, reset, purple, reset, orange, reset, orange, reset, pink, reset, pink, reset, magenta, reset, magenta, reset
    );
    println!(
        "{}  ╚══════╝{}{}    ╚═╝   {}{}╚═╝  ╚═══╝{}{}  ╚═════╝{}{} ╚═╝  ╚═╝{}{}╚═════╝ {}{}╚══════╝{}{}╚══════╝{}",
        purple, reset, purple, reset, orange, reset, orange, reset, pink, reset, pink, reset, magenta, reset, magenta, reset
    );
    println!();
}

/// Terminal UI handler for the agent
pub struct AgentUI {
    #[allow(dead_code)]
    term: Term,
    spinner: Option<ProgressBar>,
}

impl AgentUI {
    pub fn new() -> Self {
        Self {
            term: Term::stderr(),
            spinner: None,
        }
    }

    /// Pause the current spinner temporarily
    pub fn pause_spinner(&mut self) {
        if let Some(ref spinner) = self.spinner {
            spinner.finish_and_clear();
        }
        self.spinner = None;
    }

    /// Print the welcome banner
    pub fn print_welcome(&self, provider: &str, model: &str) {
        // Print the gradient ASCII logo
        print_logo();

        // Print agent info
        println!(
            "  {} {} powered by {}: {}",
            ROBOT,
            style("Syncable Agent").white().bold(),
            style(provider).cyan(),
            style(model).cyan()
        );
        println!(
            "  {}",
            style("Your AI-powered code analysis assistant").dim()
        );
        println!();
        println!(
            "  {} Type your questions. Use {} to exit.\n",
            style("→").cyan(),
            style("exit").yellow().bold()
        );
    }

    /// Print the prompt
    pub fn print_prompt(&self) {
        print!(
            "\n{} {} ",
            style("you").green().bold(),
            style("›").green()
        );
        use std::io::Write;
        std::io::stdout().flush().ok();
    }

    /// Start a thinking spinner
    pub fn start_thinking(&mut self) {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&[
                    "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏",
                ])
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.set_message(format!("{} Thinking...", THINKING));
        spinner.enable_steady_tick(Duration::from_millis(80));
        self.spinner = Some(spinner);
    }

    /// Update spinner with tool call info
    pub fn show_tool_call(&mut self, tool_name: &str) {
        let emoji = match tool_name {
            "analyze_project" => SEARCH,
            "security_scan" => SECURITY,
            "check_vulnerabilities" => SECURITY,
            "read_file" => FILE,
            "list_directory" => FOLDER,
            _ => TOOL,
        };

        let action = match tool_name {
            "analyze_project" => "Analyzing project structure...",
            "security_scan" => "Scanning for security issues...",
            "check_vulnerabilities" => "Checking dependencies for vulnerabilities...",
            "read_file" => "Reading file contents...",
            "list_directory" => "Listing directory...",
            _ => "Running tool...",
        };

        if let Some(ref spinner) = self.spinner {
            spinner.set_message(format!("{} {}", emoji, style(action).cyan()));
        }
    }

    /// Stop the spinner
    pub fn stop_thinking(&mut self) {
        if let Some(spinner) = self.spinner.take() {
            spinner.finish_and_clear();
        }
    }

    /// Print the assistant header for streaming response
    pub fn print_assistant_header(&self) {
        println!();
        println!(
            "{} {} ",
            style("assistant").magenta().bold(),
            style("›").magenta()
        );
    }

    /// Start a streaming indicator
    pub fn start_streaming(&mut self) {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂"])
                .template("  {spinner:.magenta} {msg}")
                .unwrap(),
        );
        spinner.set_message(style("Generating response...").dim().to_string());
        spinner.enable_steady_tick(Duration::from_millis(80));
        self.spinner = Some(spinner);
    }

    /// Update streaming progress
    pub fn update_streaming(&mut self, char_count: usize) {
        if let Some(ref spinner) = self.spinner {
            spinner.set_message(
                style(format!("Generating... ({} chars)", char_count)).dim().to_string()
            );
        }
    }

    /// Stop streaming and print the response
    pub fn finish_streaming_and_render(&mut self, response: &str) {
        if let Some(spinner) = self.spinner.take() {
            spinner.finish_and_clear();
        }
        println!();
        self.render_markdown(response);
        println!();
    }

    /// Print streaming text chunk (no newline) - real-time output
    pub fn print_stream_chunk(&self, text: &str) {
        print!("{}", text);
        use std::io::Write;
        std::io::stdout().flush().ok();
    }

    /// Print tool call notification during streaming
    pub fn print_tool_call_notification(&self, tool_name: &str) {
        let emoji = match tool_name {
            "analyze_project" => SEARCH,
            "security_scan" => SECURITY,
            "check_vulnerabilities" => SECURITY,
            "read_file" => FILE,
            "list_directory" => FOLDER,
            _ => TOOL,
        };

        let action = match tool_name {
            "analyze_project" => "Analyzing project structure",
            "security_scan" => "Scanning for security issues",
            "check_vulnerabilities" => "Checking dependencies for vulnerabilities",
            "read_file" => "Reading file contents",
            "list_directory" => "Listing directory",
            _ => tool_name,
        };

        println!();
        println!(
            "  {} {} {}",
            style("┌─").dim(),
            emoji,
            style(format!("Calling: {}", action)).cyan().bold()
        );
    }

    /// Print tool call completion
    pub fn print_tool_call_complete(&self, tool_name: &str) {
        let emoji = match tool_name {
            "analyze_project" => SEARCH,
            "security_scan" => SECURITY,
            "check_vulnerabilities" => SECURITY,
            "read_file" => FILE,
            "list_directory" => FOLDER,
            _ => TOOL,
        };
        
        println!(
            "  {} {} {}",
            style("└─").dim(),
            emoji,
            style(format!("{} completed", tool_name)).green()
        );
        println!();
    }

    /// End the streaming response
    pub fn end_stream(&self) {
        println!();
        println!();
    }

    /// Print the assistant's response with markdown rendering
    pub fn print_response(&self, response: &str) {
        println!();
        println!(
            "{} {} ",
            style("assistant").magenta().bold(),
            style("›").magenta()
        );
        println!();

        // Render markdown
        self.render_markdown(response);

        println!();
    }

    /// Render markdown content beautifully
    fn render_markdown(&self, content: &str) {
        use termimad::MadSkin;
        use termimad::crossterm::style::Color;

        let mut skin = MadSkin::default();

        // Customize colors using crossterm colors
        skin.set_headers_fg(Color::Cyan);
        skin.bold.set_fg(Color::White);
        skin.italic.set_fg(Color::Magenta);
        skin.inline_code.set_bg(Color::DarkGrey);
        skin.inline_code.set_fg(Color::Yellow);
        skin.code_block.set_bg(Color::DarkGrey);
        skin.code_block.set_fg(Color::Green);

        // Print markdown to terminal
        skin.print_text(content);
    }

    /// Print an error message
    pub fn print_error(&self, message: &str) {
        println!(
            "\n  {} {}",
            ERROR,
            style(message).red()
        );
    }

    /// Print a success message
    pub fn print_success(&self, message: &str) {
        println!(
            "\n  {} {}",
            SUCCESS,
            style(message).green()
        );
    }

    /// Print tool execution result summary
    pub fn print_tool_result(&self, tool_name: &str, success: bool) {
        let emoji = if success { SUCCESS } else { ERROR };
        let status = if success {
            style("completed").green()
        } else {
            style("failed").red()
        };
        
        println!(
            "  {} {} {}",
            style("│").dim(),
            emoji,
            style(format!("{} {}", tool_name, status)).dim()
        );
    }
}

impl Default for AgentUI {
    fn default() -> Self {
        Self::new()
    }
}

/// Format tool calls for display
pub fn format_tool_summary(tools_called: &[&str]) -> String {
    if tools_called.is_empty() {
        return String::new();
    }

    let mut summary = String::from("\n  ");
    summary.push_str(&style("Tools used: ").dim().to_string());
    
    for (i, tool) in tools_called.iter().enumerate() {
        if i > 0 {
            summary.push_str(", ");
        }
        summary.push_str(&style(*tool).cyan().to_string());
    }
    
    summary
}

/// Create a simple progress bar for long operations
pub fn create_progress_bar(len: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  {spinner:.cyan} [{bar:40.cyan/dim}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("━━╸"),
    );
    pb.set_message(message.to_string());
    pb
}
