//! Interactive chat session with /model and /provider commands
//!
//! Provides a rich REPL experience similar to Claude Code with:
//! - `/model` - Select from available models based on configured API keys
//! - `/provider` - Switch provider (prompts for API key if not set)
//! - `/cost` - Show token usage and estimated cost
//! - `/help` - Show available commands
//! - `/clear` - Clear conversation history
//! - `/exit` or `/quit` - Exit the session

use crate::agent::commands::{TokenUsage, SLASH_COMMANDS};
use crate::agent::{AgentError, AgentResult, ProviderType};
use crate::agent::ui::ansi;
use crate::config::{load_agent_config, save_agent_config};
use colored::Colorize;
use std::io::{self, Write};
use std::path::Path;

const ROBOT: &str = "🤖";

/// Available models per provider
pub fn get_available_models(provider: ProviderType) -> Vec<(&'static str, &'static str)> {
    match provider {
        ProviderType::OpenAI => vec![
            ("gpt-5.2", "GPT-5.2 - Latest reasoning model (Dec 2025)"),
            ("gpt-5.2-mini", "GPT-5.2 Mini - Fast and affordable"),
            ("gpt-4o", "GPT-4o - Multimodal workhorse"),
            ("o1-preview", "o1-preview - Advanced reasoning"),
        ],
        ProviderType::Anthropic => vec![
            ("claude-sonnet-4-20250514", "Claude 4 Sonnet - Latest (May 2025)"),
            ("claude-3-5-sonnet-latest", "Claude 3.5 Sonnet - Previous gen"),
            ("claude-3-opus-latest", "Claude 3 Opus - Most capable"),
            ("claude-3-haiku-latest", "Claude 3 Haiku - Fast and cheap"),
        ],
    }
}

/// Chat session state
pub struct ChatSession {
    pub provider: ProviderType,
    pub model: String,
    pub project_path: std::path::PathBuf,
    pub history: Vec<(String, String)>, // (role, content)
    pub token_usage: TokenUsage,
}

impl ChatSession {
    pub fn new(project_path: &Path, provider: ProviderType, model: Option<String>) -> Self {
        let default_model = match provider {
            ProviderType::OpenAI => "gpt-5.2".to_string(),
            ProviderType::Anthropic => "claude-sonnet-4-20250514".to_string(),
        };
        
        Self {
            provider,
            model: model.unwrap_or(default_model),
            project_path: project_path.to_path_buf(),
            history: Vec::new(),
            token_usage: TokenUsage::new(),
        }
    }

    /// Check if API key is configured for a provider (env var OR config file)
    pub fn has_api_key(provider: ProviderType) -> bool {
        // Check environment variable first
        let env_key = match provider {
            ProviderType::OpenAI => std::env::var("OPENAI_API_KEY").ok(),
            ProviderType::Anthropic => std::env::var("ANTHROPIC_API_KEY").ok(),
        };
        
        if env_key.is_some() {
            return true;
        }
        
        // Check config file
        let agent_config = load_agent_config();
        match provider {
            ProviderType::OpenAI => agent_config.openai_api_key.is_some(),
            ProviderType::Anthropic => agent_config.anthropic_api_key.is_some(),
        }
    }
    
    /// Load API key from config if not in env, and set it in env for use
    pub fn load_api_key_to_env(provider: ProviderType) {
        let env_var = match provider {
            ProviderType::OpenAI => "OPENAI_API_KEY",
            ProviderType::Anthropic => "ANTHROPIC_API_KEY",
        };
        
        // If already in env, do nothing
        if std::env::var(env_var).is_ok() {
            return;
        }
        
        // Load from config and set in env
        let agent_config = load_agent_config();
        let key = match provider {
            ProviderType::OpenAI => agent_config.openai_api_key,
            ProviderType::Anthropic => agent_config.anthropic_api_key,
        };
        
        if let Some(key) = key {
            // SAFETY: Single-threaded CLI context during initialization
            unsafe {
                std::env::set_var(env_var, &key);
            }
        }
    }

    /// Get configured providers (those with API keys)
    pub fn get_configured_providers() -> Vec<ProviderType> {
        let mut providers = Vec::new();
        if Self::has_api_key(ProviderType::OpenAI) {
            providers.push(ProviderType::OpenAI);
        }
        if Self::has_api_key(ProviderType::Anthropic) {
            providers.push(ProviderType::Anthropic);
        }
        providers
    }

    /// Prompt user to enter API key for a provider
    pub fn prompt_api_key(provider: ProviderType) -> AgentResult<String> {
        let env_var = match provider {
            ProviderType::OpenAI => "OPENAI_API_KEY",
            ProviderType::Anthropic => "ANTHROPIC_API_KEY",
        };
        
        println!("\n{}", format!("🔑 No API key found for {}", provider).yellow());
        println!("Please enter your {} API key:", provider);
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut key = String::new();
        io::stdin().read_line(&mut key).map_err(|e| AgentError::ToolError(e.to_string()))?;
        let key = key.trim().to_string();
        
        if key.is_empty() {
            return Err(AgentError::MissingApiKey(env_var.to_string()));
        }
        
        // Set for current session
        // SAFETY: We're in a single-threaded CLI context during initialization
        unsafe {
            std::env::set_var(env_var, &key);
        }
        
        // Save to config file for persistence
        let mut agent_config = load_agent_config();
        match provider {
            ProviderType::OpenAI => agent_config.openai_api_key = Some(key.clone()),
            ProviderType::Anthropic => agent_config.anthropic_api_key = Some(key.clone()),
        }
        
        if let Err(e) = save_agent_config(&agent_config) {
            eprintln!("{}", format!("Warning: Could not save config: {}", e).yellow());
        } else {
            println!("{}", "✓ API key saved to ~/.syncable.toml".green());
        }
        
        Ok(key)
    }

    /// Handle /model command - interactive model selection
    pub fn handle_model_command(&mut self) -> AgentResult<()> {
        let models = get_available_models(self.provider);
        
        println!("\n{}", format!("📋 Available models for {}:", self.provider).cyan().bold());
        println!();
        
        for (i, (id, desc)) in models.iter().enumerate() {
            let marker = if *id == self.model { "→ " } else { "  " };
            let num = format!("[{}]", i + 1);
            println!("  {} {} {} - {}", marker, num.dimmed(), id.white().bold(), desc.dimmed());
        }
        
        println!();
        println!("Enter number to select, or press Enter to keep current:");
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim();
        
        if input.is_empty() {
            println!("{}", format!("Keeping model: {}", self.model).dimmed());
            return Ok(());
        }
        
        if let Ok(num) = input.parse::<usize>() {
            if num >= 1 && num <= models.len() {
                let (id, desc) = models[num - 1];
                self.model = id.to_string();
                println!("{}", format!("✓ Switched to {} - {}", id, desc).green());
            } else {
                println!("{}", "Invalid selection".red());
            }
        } else {
            // Allow direct model name input
            self.model = input.to_string();
            println!("{}", format!("✓ Set model to: {}", input).green());
        }
        
        Ok(())
    }

    /// Handle /provider command - switch provider with API key prompt if needed
    pub fn handle_provider_command(&mut self) -> AgentResult<()> {
        let providers = [ProviderType::OpenAI, ProviderType::Anthropic];
        
        println!("\n{}", "🔄 Available providers:".cyan().bold());
        println!();
        
        for (i, provider) in providers.iter().enumerate() {
            let marker = if *provider == self.provider { "→ " } else { "  " };
            let has_key = if Self::has_api_key(*provider) {
                "✓ API key configured".green()
            } else {
                "⚠ No API key".yellow()
            };
            let num = format!("[{}]", i + 1);
            println!("  {} {} {} - {}", marker, num.dimmed(), provider.to_string().white().bold(), has_key);
        }
        
        println!();
        println!("Enter number to select:");
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim();
        
        if let Ok(num) = input.parse::<usize>() {
            if num >= 1 && num <= providers.len() {
                let new_provider = providers[num - 1];
                
                // Check if API key exists, prompt if not
                if !Self::has_api_key(new_provider) {
                    Self::prompt_api_key(new_provider)?;
                }
                
                self.provider = new_provider;
                
                // Set default model for new provider
                let default_model = match new_provider {
                    ProviderType::OpenAI => "gpt-5.2",
                    ProviderType::Anthropic => "claude-sonnet-4-20250514",
                };
                self.model = default_model.to_string();
                
                println!("{}", format!("✓ Switched to {} with model {}", new_provider, default_model).green());
            } else {
                println!("{}", "Invalid selection".red());
            }
        }
        
        Ok(())
    }

    /// Handle /help command
    pub fn print_help() {
        println!();
        println!("  {}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", ansi::PURPLE, ansi::RESET);
        println!("  {}📖 Available Commands{}", ansi::PURPLE, ansi::RESET);
        println!("  {}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", ansi::PURPLE, ansi::RESET);
        println!();
        
        for cmd in SLASH_COMMANDS.iter() {
            let alias = cmd.alias.map(|a| format!(" ({})", a)).unwrap_or_default();
            println!("  {}/{:<12}{}{} - {}{}{}", 
                ansi::CYAN, cmd.name, alias, ansi::RESET,
                ansi::DIM, cmd.description, ansi::RESET
            );
        }
        
        println!();
        println!("  {}Tip: Type / to see interactive command picker!{}", ansi::DIM, ansi::RESET);
        println!();
    }


    /// Print session banner with colorful SYNCABLE ASCII art
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

    /// Print the welcome banner
    pub fn print_banner(&self) {
        // Print the gradient ASCII logo
        Self::print_logo();

        // Platform promo
        println!(
            "  {} {}",
            "🚀".dimmed(),
            "Want to deploy? Deploy instantly from Syncable Platform → https://syncable.dev".dimmed()
        );
        println!();

        // Print agent info
        println!(
            "  {} {} powered by {}: {}",
            ROBOT,
            "Syncable Agent".white().bold(),
            self.provider.to_string().cyan(),
            self.model.cyan()
        );
        println!(
            "  {}",
            "Your AI-powered code analysis assistant".dimmed()
        );
        println!();
        println!(
            "  {} Type your questions. Use {} to exit.\n",
            "→".cyan(),
            "exit".yellow().bold()
        );
    }


    /// Process a command (returns true if should continue, false if should exit)
    pub fn process_command(&mut self, input: &str) -> AgentResult<bool> {
        let cmd = input.trim().to_lowercase();
        
        // Handle bare "/" - now handled interactively in read_input
        // Just show help if they somehow got here
        if cmd == "/" {
            Self::print_help();
            return Ok(true);
        }
        
        match cmd.as_str() {
            "/exit" | "/quit" | "/q" => {
                println!("\n{}", "👋 Goodbye!".green());
                return Ok(false);
            }
            "/help" | "/h" | "/?" => {
                Self::print_help();
            }
            "/model" | "/m" => {
                self.handle_model_command()?;
            }
            "/provider" | "/p" => {
                self.handle_provider_command()?;
            }
            "/cost" => {
                self.token_usage.print_report(&self.model);
            }
            "/clear" | "/c" => {
                self.history.clear();
                println!("{}", "✓ Conversation history cleared".green());
            }
            _ => {
                if cmd.starts_with('/') {
                    // Unknown command - interactive picker already handled in read_input
                    println!("{}", format!("Unknown command: {}. Type /help for available commands.", cmd).yellow());
                }
            }
        }
        
        Ok(true)
    }

    /// Check if input is a command
    pub fn is_command(input: &str) -> bool {
        input.trim().starts_with('/')
    }

    /// Read user input with prompt - with interactive file picker support
    /// Uses custom terminal handling for @ file references and / commands
    pub fn read_input(&self) -> io::Result<String> {
        use crate::agent::ui::input::{read_input_with_file_picker, InputResult};

        match read_input_with_file_picker("You:", &self.project_path) {
            InputResult::Submit(text) => {
                let trimmed = text.trim();
                // Handle case where full suggestion was submitted (e.g., "/model        Description")
                // Extract just the command if it looks like a suggestion format
                if trimmed.starts_with('/') && trimmed.contains("  ") {
                    // This looks like a suggestion format, extract just the command
                    if let Some(cmd) = trimmed.split_whitespace().next() {
                        return Ok(cmd.to_string());
                    }
                }
                Ok(trimmed.to_string())
            }
            InputResult::Cancel => Ok("exit".to_string()),  // Ctrl+C exits
            InputResult::Exit => Ok("exit".to_string()),
        }
    }
}
