//! Interactive chat session with /model and /provider commands
//!
//! Provides a rich REPL experience similar to Claude Code with:
//! - `/model` - Select from available models based on configured API keys
//! - `/provider` - Switch provider (prompts for API key if not set)
//! - `/cost` - Show token usage and estimated cost
//! - `/help` - Show available commands
//! - `/clear` - Clear conversation history
//! - `/exit` or `/quit` - Exit the session

// Submodules
mod commands;
mod plan_mode;
mod providers;
mod ui;

// Re-exports for backward compatibility
pub use plan_mode::{IncompletePlan, PlanMode, find_incomplete_plans};
pub use providers::{get_available_models, get_configured_providers, prompt_api_key};

use crate::agent::commands::TokenUsage;
use crate::agent::{AgentResult, ProviderType};
use crate::platform::PlatformSession;
use colored::Colorize;
use std::io;
use std::path::Path;

/// Chat session state
pub struct ChatSession {
    pub provider: ProviderType,
    pub model: String,
    pub project_path: std::path::PathBuf,
    pub history: Vec<(String, String)>, // (role, content)
    pub token_usage: TokenUsage,
    /// Current planning mode state
    pub plan_mode: PlanMode,
    /// Session loaded via /resume command, to be processed by main loop
    pub pending_resume: Option<crate::agent::persistence::ConversationRecord>,
    /// Platform session state (selected project/org context)
    pub platform_session: PlatformSession,
}

impl ChatSession {
    pub fn new(project_path: &Path, provider: ProviderType, model: Option<String>) -> Self {
        let default_model = match provider {
            ProviderType::OpenAI => "gpt-5.2".to_string(),
            ProviderType::Anthropic => "claude-sonnet-4-5-20250929".to_string(),
            ProviderType::Bedrock => "global.anthropic.claude-sonnet-4-20250514-v1:0".to_string(),
        };

        // Load platform session from disk (returns default if not exists)
        let platform_session = PlatformSession::load().unwrap_or_default();

        Self {
            provider,
            model: model.unwrap_or(default_model),
            project_path: project_path.to_path_buf(),
            history: Vec::new(),
            token_usage: TokenUsage::new(),
            plan_mode: PlanMode::default(),
            pending_resume: None,
            platform_session,
        }
    }

    /// Update the platform session and save to disk
    pub fn update_platform_session(&mut self, session: PlatformSession) {
        self.platform_session = session;
        if let Err(e) = self.platform_session.save() {
            eprintln!(
                "{}",
                format!("Warning: Failed to save platform session: {}", e).yellow()
            );
        }
    }

    /// Toggle planning mode and return the new mode
    pub fn toggle_plan_mode(&mut self) -> PlanMode {
        self.plan_mode = self.plan_mode.toggle();
        self.plan_mode
    }

    /// Check if currently in planning mode
    pub fn is_planning(&self) -> bool {
        self.plan_mode.is_planning()
    }

    /// Check if API key is configured for a provider (env var OR config file)
    pub fn has_api_key(provider: ProviderType) -> bool {
        providers::has_api_key(provider)
    }

    /// Load API key from config if not in env, and set it in env for use
    pub fn load_api_key_to_env(provider: ProviderType) {
        providers::load_api_key_to_env(provider)
    }

    /// Prompt user to enter API key for a provider
    pub fn prompt_api_key(provider: ProviderType) -> AgentResult<String> {
        providers::prompt_api_key(provider)
    }

    /// Handle /model command - interactive model selection
    pub fn handle_model_command(&mut self) -> AgentResult<()> {
        commands::handle_model_command(self)
    }

    /// Handle /provider command - switch provider with API key prompt if needed
    pub fn handle_provider_command(&mut self) -> AgentResult<()> {
        commands::handle_provider_command(self)
    }

    /// Handle /reset command - reset provider credentials
    pub fn handle_reset_command(&mut self) -> AgentResult<()> {
        commands::handle_reset_command(self)
    }

    /// Handle /profile command - manage global profiles
    pub fn handle_profile_command(&mut self) -> AgentResult<()> {
        commands::handle_profile_command(self)
    }

    /// Handle /plans command - show incomplete plans and offer to continue
    pub fn handle_plans_command(&self) -> AgentResult<()> {
        commands::handle_plans_command(self)
    }

    /// Handle /resume command - browse and select a session to resume
    /// Returns true if a session was loaded and should be displayed
    pub fn handle_resume_command(&mut self) -> AgentResult<bool> {
        commands::handle_resume_command(self)
    }

    /// Handle /sessions command - list available sessions
    pub fn handle_list_sessions_command(&self) {
        commands::handle_list_sessions_command(self)
    }

    /// Handle /help command - delegates to ui module
    pub fn print_help() {
        ui::print_help()
    }

    /// Print session banner with colorful SYNCABLE ASCII art - delegates to ui module
    pub fn print_logo() {
        ui::print_logo()
    }

    /// Print the welcome banner - delegates to ui module
    pub fn print_banner(&self) {
        ui::print_banner(self)
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
            "/reset" | "/r" => {
                self.handle_reset_command()?;
            }
            "/profile" => {
                self.handle_profile_command()?;
            }
            "/plans" => {
                self.handle_plans_command()?;
            }
            "/resume" | "/s" => {
                // Resume loads session into self.pending_resume
                // Main loop in mod.rs will detect and process it
                let _ = self.handle_resume_command()?;
            }
            "/sessions" | "/ls" => {
                self.handle_list_sessions_command();
            }
            _ => {
                if cmd.starts_with('/') {
                    // Unknown command - interactive picker already handled in read_input
                    println!(
                        "{}",
                        format!(
                            "Unknown command: {}. Type /help for available commands.",
                            cmd
                        )
                        .yellow()
                    );
                }
            }
        }

        Ok(true)
    }

    /// Check if input is a command
    pub fn is_command(input: &str) -> bool {
        input.trim().starts_with('/')
    }

    /// Strip @ prefix from file/folder references for AI consumption
    /// Keeps the path but removes the leading @ that was used for autocomplete
    /// e.g., "check @src/main.rs for issues" -> "check src/main.rs for issues"
    fn strip_file_references(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '@' {
                // Check if this @ is at start or after whitespace (valid file reference trigger)
                let is_valid_trigger = i == 0 || chars[i - 1].is_whitespace();

                if is_valid_trigger {
                    // Check if there's a path after @ (not just @ followed by space/end)
                    let has_path = i + 1 < chars.len() && !chars[i + 1].is_whitespace();

                    if has_path {
                        // Skip the @ but keep the path
                        i += 1;
                        continue;
                    }
                }
            }
            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// Read user input with prompt - with interactive file picker support
    /// Uses custom terminal handling for @ file references and / commands
    /// Returns InputResult which the main loop should handle
    pub fn read_input(&self) -> io::Result<crate::agent::ui::input::InputResult> {
        use crate::agent::ui::input::read_input_with_file_picker;

        // Build prompt with platform context if project is selected
        let prompt = if self.platform_session.is_project_selected() {
            format!(
                "{} >",
                self.platform_session.display_context()
            )
        } else {
            ">".to_string()
        };

        Ok(read_input_with_file_picker(
            &prompt,
            &self.project_path,
            self.plan_mode.is_planning(),
        ))
    }

    /// Process a submitted input text - strips @ references and handles suggestion format
    pub fn process_submitted_text(text: &str) -> String {
        let trimmed = text.trim();
        // Handle case where full suggestion was submitted (e.g., "/model        Description")
        // Extract just the command if it looks like a suggestion format
        if trimmed.starts_with('/') && trimmed.contains("  ") {
            // This looks like a suggestion format, extract just the command
            if let Some(cmd) = trimmed.split_whitespace().next() {
                return cmd.to_string();
            }
        }
        // Strip @ prefix from file references before sending to AI
        // The @ is for UI autocomplete, but the AI should see just the path
        Self::strip_file_references(trimmed)
    }
}
