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
mod plan_mode;
mod providers;

// Re-exports for backward compatibility
pub use plan_mode::{find_incomplete_plans, IncompletePlan, PlanMode};
pub use providers::{get_available_models, get_configured_providers, prompt_api_key};

use crate::agent::commands::{SLASH_COMMANDS, TokenUsage};
use crate::agent::ui::ansi;
use crate::agent::{AgentResult, ProviderType};
use crate::config::{load_agent_config, save_agent_config};
use colored::Colorize;
use std::io::{self, Write};
use std::path::Path;

const ROBOT: &str = "🤖";

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
}

impl ChatSession {
    pub fn new(project_path: &Path, provider: ProviderType, model: Option<String>) -> Self {
        let default_model = match provider {
            ProviderType::OpenAI => "gpt-5.2".to_string(),
            ProviderType::Anthropic => "claude-sonnet-4-5-20250929".to_string(),
            ProviderType::Bedrock => "global.anthropic.claude-sonnet-4-20250514-v1:0".to_string(),
        };

        Self {
            provider,
            model: model.unwrap_or(default_model),
            project_path: project_path.to_path_buf(),
            history: Vec::new(),
            token_usage: TokenUsage::new(),
            plan_mode: PlanMode::default(),
            pending_resume: None,
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
        let models = get_available_models(self.provider);

        println!(
            "\n{}",
            format!("📋 Available models for {}:", self.provider)
                .cyan()
                .bold()
        );
        println!();

        for (i, (id, desc)) in models.iter().enumerate() {
            let marker = if *id == self.model { "→ " } else { "  " };
            let num = format!("[{}]", i + 1);
            println!(
                "  {} {} {} - {}",
                marker,
                num.dimmed(),
                id.white().bold(),
                desc.dimmed()
            );
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

                // Save model choice to config for persistence
                let mut agent_config = load_agent_config();
                agent_config.default_model = Some(id.to_string());
                if let Err(e) = save_agent_config(&agent_config) {
                    eprintln!(
                        "{}",
                        format!("Warning: Could not save config: {}", e).yellow()
                    );
                }

                println!("{}", format!("✓ Switched to {} - {}", id, desc).green());
            } else {
                println!("{}", "Invalid selection".red());
            }
        } else {
            // Allow direct model name input
            self.model = input.to_string();

            // Save model choice to config for persistence
            let mut agent_config = load_agent_config();
            agent_config.default_model = Some(input.to_string());
            if let Err(e) = save_agent_config(&agent_config) {
                eprintln!(
                    "{}",
                    format!("Warning: Could not save config: {}", e).yellow()
                );
            }

            println!("{}", format!("✓ Set model to: {}", input).green());
        }

        Ok(())
    }

    /// Handle /provider command - switch provider with API key prompt if needed
    pub fn handle_provider_command(&mut self) -> AgentResult<()> {
        let providers = [
            ProviderType::OpenAI,
            ProviderType::Anthropic,
            ProviderType::Bedrock,
        ];

        println!("\n{}", "🔄 Available providers:".cyan().bold());
        println!();

        for (i, provider) in providers.iter().enumerate() {
            let marker = if *provider == self.provider {
                "→ "
            } else {
                "  "
            };
            let has_key = if Self::has_api_key(*provider) {
                "✓ API key configured".green()
            } else {
                "⚠ No API key".yellow()
            };
            let num = format!("[{}]", i + 1);
            println!(
                "  {} {} {} - {}",
                marker,
                num.dimmed(),
                provider.to_string().white().bold(),
                has_key
            );
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
                    prompt_api_key(new_provider)?;
                }

                // Load API key/credentials from config to environment
                // This is essential for Bedrock bearer token auth!
                Self::load_api_key_to_env(new_provider);

                self.provider = new_provider;

                // Set default model for new provider (check saved config for Bedrock)
                let default_model = match new_provider {
                    ProviderType::OpenAI => "gpt-5.2".to_string(),
                    ProviderType::Anthropic => "claude-sonnet-4-5-20250929".to_string(),
                    ProviderType::Bedrock => {
                        // Use saved model preference if available
                        let agent_config = load_agent_config();
                        agent_config
                            .bedrock
                            .and_then(|b| b.default_model)
                            .unwrap_or_else(|| {
                                "global.anthropic.claude-sonnet-4-5-20250929-v1:0".to_string()
                            })
                    }
                };
                self.model = default_model.clone();

                // Save provider choice to config for persistence
                let mut agent_config = load_agent_config();
                agent_config.default_provider = new_provider.to_string();
                agent_config.default_model = Some(default_model.clone());
                if let Err(e) = save_agent_config(&agent_config) {
                    eprintln!(
                        "{}",
                        format!("Warning: Could not save config: {}", e).yellow()
                    );
                }

                println!(
                    "{}",
                    format!(
                        "✓ Switched to {} with model {}",
                        new_provider, default_model
                    )
                    .green()
                );
            } else {
                println!("{}", "Invalid selection".red());
            }
        }

        Ok(())
    }

    /// Handle /reset command - reset provider credentials
    pub fn handle_reset_command(&mut self) -> AgentResult<()> {
        let providers = [
            ProviderType::OpenAI,
            ProviderType::Anthropic,
            ProviderType::Bedrock,
        ];

        println!("\n{}", "🔄 Reset Provider Credentials".cyan().bold());
        println!();

        for (i, provider) in providers.iter().enumerate() {
            let status = if Self::has_api_key(*provider) {
                "✓ configured".green()
            } else {
                "○ not configured".dimmed()
            };
            let num = format!("[{}]", i + 1);
            println!(
                "  {} {} - {}",
                num.dimmed(),
                provider.to_string().white().bold(),
                status
            );
        }
        println!("  {} All providers", "[4]".dimmed());
        println!();
        println!("Select provider to reset (or press Enter to cancel):");
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim();

        if input.is_empty() {
            println!("{}", "Cancelled".dimmed());
            return Ok(());
        }

        let mut agent_config = load_agent_config();

        match input {
            "1" => {
                agent_config.openai_api_key = None;
                // SAFETY: Single-threaded CLI context during command handling
                unsafe {
                    std::env::remove_var("OPENAI_API_KEY");
                }
                println!("{}", "✓ OpenAI credentials cleared".green());
            }
            "2" => {
                agent_config.anthropic_api_key = None;
                unsafe {
                    std::env::remove_var("ANTHROPIC_API_KEY");
                }
                println!("{}", "✓ Anthropic credentials cleared".green());
            }
            "3" => {
                agent_config.bedrock = None;
                agent_config.bedrock_configured = Some(false);
                // SAFETY: Single-threaded CLI context during command handling
                unsafe {
                    std::env::remove_var("AWS_PROFILE");
                    std::env::remove_var("AWS_ACCESS_KEY_ID");
                    std::env::remove_var("AWS_SECRET_ACCESS_KEY");
                    std::env::remove_var("AWS_REGION");
                }
                println!("{}", "✓ Bedrock credentials cleared".green());
            }
            "4" => {
                agent_config.openai_api_key = None;
                agent_config.anthropic_api_key = None;
                agent_config.bedrock = None;
                agent_config.bedrock_configured = Some(false);
                // SAFETY: Single-threaded CLI context during command handling
                unsafe {
                    std::env::remove_var("OPENAI_API_KEY");
                    std::env::remove_var("ANTHROPIC_API_KEY");
                    std::env::remove_var("AWS_PROFILE");
                    std::env::remove_var("AWS_ACCESS_KEY_ID");
                    std::env::remove_var("AWS_SECRET_ACCESS_KEY");
                    std::env::remove_var("AWS_REGION");
                }
                println!("{}", "✓ All provider credentials cleared".green());
            }
            _ => {
                println!("{}", "Invalid selection".red());
                return Ok(());
            }
        }

        // Save updated config
        if let Err(e) = save_agent_config(&agent_config) {
            eprintln!(
                "{}",
                format!("Warning: Could not save config: {}", e).yellow()
            );
        } else {
            println!("{}", "Configuration saved to ~/.syncable.toml".dimmed());
        }

        // Prompt to reconfigure if current provider was reset
        let current_cleared = match input {
            "1" => self.provider == ProviderType::OpenAI,
            "2" => self.provider == ProviderType::Anthropic,
            "3" => self.provider == ProviderType::Bedrock,
            "4" => true,
            _ => false,
        };

        if current_cleared {
            println!();
            println!("{}", "Current provider credentials were cleared.".yellow());
            println!(
                "Use {} to reconfigure or {} to switch providers.",
                "/provider".cyan(),
                "/p".cyan()
            );
        }

        Ok(())
    }

    /// Handle /profile command - manage global profiles
    pub fn handle_profile_command(&mut self) -> AgentResult<()> {
        use crate::config::types::{AnthropicProfile, OpenAIProfile, Profile};

        let mut agent_config = load_agent_config();

        println!("\n{}", "👤 Profile Management".cyan().bold());
        println!();

        // Show current profiles
        self.list_profiles(&agent_config);

        println!("  {} Create new profile", "[1]".cyan());
        println!("  {} Switch active profile", "[2]".cyan());
        println!("  {} Configure provider in profile", "[3]".cyan());
        println!("  {} Delete a profile", "[4]".cyan());
        println!();
        println!("Select action (or press Enter to cancel):");
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim();

        if input.is_empty() {
            println!("{}", "Cancelled".dimmed());
            return Ok(());
        }

        match input {
            "1" => {
                // Create new profile
                println!("\n{}", "Create Profile".white().bold());
                print!("Profile name (e.g., work, personal): ");
                io::stdout().flush().unwrap();
                let mut name = String::new();
                io::stdin().read_line(&mut name).ok();
                let name = name.trim().to_string();

                if name.is_empty() {
                    println!("{}", "Profile name cannot be empty".red());
                    return Ok(());
                }

                if agent_config.profiles.contains_key(&name) {
                    println!("{}", format!("Profile '{}' already exists", name).yellow());
                    return Ok(());
                }

                print!("Description (optional): ");
                io::stdout().flush().unwrap();
                let mut desc = String::new();
                io::stdin().read_line(&mut desc).ok();
                let desc = desc.trim();

                let profile = Profile {
                    description: if desc.is_empty() {
                        None
                    } else {
                        Some(desc.to_string())
                    },
                    default_provider: None,
                    default_model: None,
                    openai: None,
                    anthropic: None,
                    bedrock: None,
                };

                agent_config.profiles.insert(name.clone(), profile);

                // Set as active if it's the first profile
                if agent_config.active_profile.is_none() {
                    agent_config.active_profile = Some(name.clone());
                }

                if let Err(e) = save_agent_config(&agent_config) {
                    eprintln!(
                        "{}",
                        format!("Warning: Could not save config: {}", e).yellow()
                    );
                }

                println!("{}", format!("✓ Profile '{}' created", name).green());
                println!(
                    "{}",
                    "Use option [3] to configure providers for this profile".dimmed()
                );
            }
            "2" => {
                // Switch active profile
                if agent_config.profiles.is_empty() {
                    println!(
                        "{}",
                        "No profiles configured. Create one first with option [1].".yellow()
                    );
                    return Ok(());
                }

                print!("Enter profile name to activate: ");
                io::stdout().flush().unwrap();
                let mut name = String::new();
                io::stdin().read_line(&mut name).ok();
                let name = name.trim().to_string();

                if name.is_empty() {
                    println!("{}", "Cancelled".dimmed());
                    return Ok(());
                }

                if !agent_config.profiles.contains_key(&name) {
                    println!("{}", format!("Profile '{}' not found", name).red());
                    return Ok(());
                }

                agent_config.active_profile = Some(name.clone());

                // Load credentials from the new profile
                if let Some(profile) = agent_config.profiles.get(&name) {
                    // Clear old env vars and load new ones
                    if let Some(openai) = &profile.openai {
                        unsafe {
                            std::env::set_var("OPENAI_API_KEY", &openai.api_key);
                        }
                    }
                    if let Some(anthropic) = &profile.anthropic {
                        unsafe {
                            std::env::set_var("ANTHROPIC_API_KEY", &anthropic.api_key);
                        }
                    }
                    if let Some(bedrock) = &profile.bedrock {
                        if let Some(region) = &bedrock.region {
                            unsafe {
                                std::env::set_var("AWS_REGION", region);
                            }
                        }
                        if let Some(aws_profile) = &bedrock.profile {
                            unsafe {
                                std::env::set_var("AWS_PROFILE", aws_profile);
                            }
                        } else if let (Some(key_id), Some(secret)) =
                            (&bedrock.access_key_id, &bedrock.secret_access_key)
                        {
                            unsafe {
                                std::env::set_var("AWS_ACCESS_KEY_ID", key_id);
                                std::env::set_var("AWS_SECRET_ACCESS_KEY", secret);
                            }
                        }
                    }

                    // Update current provider if profile has a default
                    if let Some(default_provider) = &profile.default_provider
                        && let Ok(p) = default_provider.parse()
                    {
                        self.provider = p;
                    }
                }

                if let Err(e) = save_agent_config(&agent_config) {
                    eprintln!(
                        "{}",
                        format!("Warning: Could not save config: {}", e).yellow()
                    );
                }

                println!("{}", format!("✓ Switched to profile '{}'", name).green());
            }
            "3" => {
                // Configure provider in profile
                let profile_name = if let Some(name) = &agent_config.active_profile {
                    name.clone()
                } else if agent_config.profiles.is_empty() {
                    println!(
                        "{}",
                        "No profiles configured. Create one first with option [1].".yellow()
                    );
                    return Ok(());
                } else {
                    print!("Enter profile name to configure: ");
                    io::stdout().flush().unwrap();
                    let mut name = String::new();
                    io::stdin().read_line(&mut name).ok();
                    name.trim().to_string()
                };

                if profile_name.is_empty() {
                    println!("{}", "Cancelled".dimmed());
                    return Ok(());
                }

                if !agent_config.profiles.contains_key(&profile_name) {
                    println!("{}", format!("Profile '{}' not found", profile_name).red());
                    return Ok(());
                }

                println!(
                    "\n{}",
                    format!("Configure provider for '{}':", profile_name)
                        .white()
                        .bold()
                );
                println!("  {} OpenAI", "[1]".cyan());
                println!("  {} Anthropic", "[2]".cyan());
                println!("  {} AWS Bedrock", "[3]".cyan());
                print!("> ");
                io::stdout().flush().unwrap();

                let mut provider_choice = String::new();
                io::stdin().read_line(&mut provider_choice).ok();

                match provider_choice.trim() {
                    "1" => {
                        // Configure OpenAI
                        print!("OpenAI API Key: ");
                        io::stdout().flush().unwrap();
                        let mut api_key = String::new();
                        io::stdin().read_line(&mut api_key).ok();
                        let api_key = api_key.trim().to_string();

                        if api_key.is_empty() {
                            println!("{}", "API key cannot be empty".red());
                            return Ok(());
                        }

                        if let Some(profile) = agent_config.profiles.get_mut(&profile_name) {
                            profile.openai = Some(OpenAIProfile {
                                api_key,
                                description: None,
                                default_model: None,
                            });
                        }
                        println!(
                            "{}",
                            format!("✓ OpenAI configured for profile '{}'", profile_name).green()
                        );
                    }
                    "2" => {
                        // Configure Anthropic
                        print!("Anthropic API Key: ");
                        io::stdout().flush().unwrap();
                        let mut api_key = String::new();
                        io::stdin().read_line(&mut api_key).ok();
                        let api_key = api_key.trim().to_string();

                        if api_key.is_empty() {
                            println!("{}", "API key cannot be empty".red());
                            return Ok(());
                        }

                        if let Some(profile) = agent_config.profiles.get_mut(&profile_name) {
                            profile.anthropic = Some(AnthropicProfile {
                                api_key,
                                description: None,
                                default_model: None,
                            });
                        }
                        println!(
                            "{}",
                            format!("✓ Anthropic configured for profile '{}'", profile_name)
                                .green()
                        );
                    }
                    "3" => {
                        // Configure Bedrock - use the wizard
                        println!("{}", "Running Bedrock setup...".dimmed());
                        let selected_model = providers::run_bedrock_setup_wizard()?;

                        // Get the saved bedrock config and copy it to the profile
                        let fresh_config = load_agent_config();
                        if let Some(bedrock) = fresh_config.bedrock.clone()
                            && let Some(profile) = agent_config.profiles.get_mut(&profile_name)
                        {
                            profile.bedrock = Some(bedrock);
                            profile.default_model = Some(selected_model);
                        }
                        println!(
                            "{}",
                            format!("✓ Bedrock configured for profile '{}'", profile_name).green()
                        );
                    }
                    _ => {
                        println!("{}", "Invalid selection".red());
                        return Ok(());
                    }
                }

                if let Err(e) = save_agent_config(&agent_config) {
                    eprintln!(
                        "{}",
                        format!("Warning: Could not save config: {}", e).yellow()
                    );
                }
            }
            "4" => {
                // Delete profile
                if agent_config.profiles.is_empty() {
                    println!("{}", "No profiles to delete.".yellow());
                    return Ok(());
                }

                print!("Enter profile name to delete: ");
                io::stdout().flush().unwrap();
                let mut name = String::new();
                io::stdin().read_line(&mut name).ok();
                let name = name.trim().to_string();

                if name.is_empty() {
                    println!("{}", "Cancelled".dimmed());
                    return Ok(());
                }

                if agent_config.profiles.remove(&name).is_some() {
                    // If this was the active profile, clear it
                    if agent_config.active_profile.as_deref() == Some(name.as_str()) {
                        agent_config.active_profile = None;
                    }

                    if let Err(e) = save_agent_config(&agent_config) {
                        eprintln!(
                            "{}",
                            format!("Warning: Could not save config: {}", e).yellow()
                        );
                    }

                    println!("{}", format!("✓ Deleted profile '{}'", name).green());
                } else {
                    println!("{}", format!("Profile '{}' not found", name).red());
                }
            }
            _ => {
                println!("{}", "Invalid selection".red());
            }
        }

        Ok(())
    }

    /// Handle /plans command - show incomplete plans and offer to continue
    pub fn handle_plans_command(&self) -> AgentResult<()> {
        let incomplete = find_incomplete_plans(&self.project_path);

        if incomplete.is_empty() {
            println!("\n{}", "No incomplete plans found.".dimmed());
            println!(
                "{}",
                "Create a plan using plan mode (Shift+Tab) and the plan_create tool.".dimmed()
            );
            return Ok(());
        }

        println!("\n{}", "📋 Incomplete Plans".cyan().bold());
        println!();

        for (i, plan) in incomplete.iter().enumerate() {
            let progress = format!("{}/{}", plan.done, plan.total);
            let percent = if plan.total > 0 {
                (plan.done as f64 / plan.total as f64 * 100.0) as usize
            } else {
                0
            };

            println!(
                "  {} {} {} ({} - {}%)",
                format!("[{}]", i + 1).cyan(),
                plan.filename.white().bold(),
                format!("({} pending)", plan.pending).yellow(),
                progress.dimmed(),
                percent
            );
            println!("      {}", plan.path.dimmed());
        }

        println!();
        println!("{}", "To continue a plan, say:".dimmed());
        println!("  {}", "\"continue the plan at plans/FILENAME.md\"".cyan());
        println!(
            "  {}",
            "or just \"continue\" to resume the most recent one".cyan()
        );
        println!();

        Ok(())
    }

    /// Handle /resume command - browse and select a session to resume
    /// Returns true if a session was loaded and should be displayed
    pub fn handle_resume_command(&mut self) -> AgentResult<bool> {
        use crate::agent::persistence::{SessionSelector, browse_sessions, format_relative_time};

        let selector = SessionSelector::new(&self.project_path);
        let sessions = selector.list_sessions();

        if sessions.is_empty() {
            println!(
                "\n{}",
                "No previous sessions found for this project.".yellow()
            );
            println!(
                "{}",
                "Sessions are automatically saved during conversations.".dimmed()
            );
            return Ok(false);
        }

        // Show the interactive browser
        if let Some(selected) = browse_sessions(&self.project_path) {
            // User selected a session - load it
            let time = format_relative_time(selected.last_updated);

            match selector.load_conversation(&selected) {
                Ok(record) => {
                    println!(
                        "\n{} Resuming: {} ({}, {} messages)",
                        "✓".green(),
                        selected.display_name.white().bold(),
                        time.dimmed(),
                        record.messages.len()
                    );

                    // Store for main loop to process
                    self.pending_resume = Some(record);
                    return Ok(true);
                }
                Err(e) => {
                    eprintln!("{} Failed to load session: {}", "✗".red(), e);
                }
            }
        }

        Ok(false)
    }

    /// Handle /sessions command - list available sessions
    pub fn handle_list_sessions_command(&self) {
        use crate::agent::persistence::{SessionSelector, format_relative_time};

        let selector = SessionSelector::new(&self.project_path);
        let sessions = selector.list_sessions();

        if sessions.is_empty() {
            println!(
                "\n{}",
                "No previous sessions found for this project.".yellow()
            );
            return;
        }

        println!(
            "\n{}",
            format!("📋 Sessions ({})", sessions.len()).cyan().bold()
        );
        println!();

        for session in &sessions {
            let time = format_relative_time(session.last_updated);
            println!(
                "  {} {} {}",
                format!("[{}]", session.index).cyan(),
                session.display_name.white(),
                format!("({})", time).dimmed()
            );
            println!(
                "      {} messages · ID: {}",
                session.message_count.to_string().dimmed(),
                session.id[..8].to_string().dimmed()
            );
        }

        println!();
        println!("{}", "To resume a session:".dimmed());
        println!(
            "  {} or {}",
            "/resume".cyan(),
            "sync-ctl chat --resume <NUMBER|ID>".cyan()
        );
        println!();
    }

    /// List all profiles
    fn list_profiles(&self, config: &crate::config::types::AgentConfig) {
        let active = config.active_profile.as_deref();

        if config.profiles.is_empty() {
            println!("{}", "  No profiles configured yet.".dimmed());
            println!();
            return;
        }

        println!("{}", "📋 Profiles:".cyan());
        for (name, profile) in &config.profiles {
            let marker = if Some(name.as_str()) == active {
                "→ "
            } else {
                "  "
            };
            let desc = profile.description.as_deref().unwrap_or("");
            let desc_fmt = if desc.is_empty() {
                String::new()
            } else {
                format!(" - {}", desc)
            };

            // Show which providers are configured
            let mut providers = Vec::new();
            if profile.openai.is_some() {
                providers.push("OpenAI");
            }
            if profile.anthropic.is_some() {
                providers.push("Anthropic");
            }
            if profile.bedrock.is_some() {
                providers.push("Bedrock");
            }

            let providers_str = if providers.is_empty() {
                "(no providers configured)".to_string()
            } else {
                format!("[{}]", providers.join(", "))
            };

            println!(
                "  {} {}{} {}",
                marker,
                name.white().bold(),
                desc_fmt.dimmed(),
                providers_str.dimmed()
            );
        }
        println!();
    }

    /// Handle /help command
    pub fn print_help() {
        println!();
        println!(
            "  {}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}",
            ansi::PURPLE,
            ansi::RESET
        );
        println!("  {}📖 Available Commands{}", ansi::PURPLE, ansi::RESET);
        println!(
            "  {}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}",
            ansi::PURPLE,
            ansi::RESET
        );
        println!();

        for cmd in SLASH_COMMANDS.iter() {
            let alias = cmd.alias.map(|a| format!(" ({})", a)).unwrap_or_default();
            println!(
                "  {}/{:<12}{}{} - {}{}{}",
                ansi::CYAN,
                cmd.name,
                alias,
                ansi::RESET,
                ansi::DIM,
                cmd.description,
                ansi::RESET
            );
        }

        println!();
        println!(
            "  {}Tip: Type / to see interactive command picker!{}",
            ansi::DIM,
            ansi::RESET
        );
        println!();
    }

    /// Print session banner with colorful SYNCABLE ASCII art
    pub fn print_logo() {
        // Colors matching the logo gradient: purple → orange → pink
        // Using ANSI 256 colors for better gradient

        // Purple shades for S, y
        let purple = "\x1b[38;5;141m"; // Light purple
        // Orange shades for n, c
        let orange = "\x1b[38;5;216m"; // Peach/orange
        // Pink shades for a, b, l, e
        let pink = "\x1b[38;5;212m"; // Hot pink
        let magenta = "\x1b[38;5;207m"; // Magenta
        let reset = "\x1b[0m";

        println!();
        println!(
            "{}  ███████╗{}{} ██╗   ██╗{}{}███╗   ██╗{}{} ██████╗{}{}  █████╗ {}{}██████╗ {}{}██╗     {}{}███████╗{}",
            purple,
            reset,
            purple,
            reset,
            orange,
            reset,
            orange,
            reset,
            pink,
            reset,
            pink,
            reset,
            magenta,
            reset,
            magenta,
            reset
        );
        println!(
            "{}  ██╔════╝{}{} ╚██╗ ██╔╝{}{}████╗  ██║{}{} ██╔════╝{}{} ██╔══██╗{}{}██╔══██╗{}{}██║     {}{}██╔════╝{}",
            purple,
            reset,
            purple,
            reset,
            orange,
            reset,
            orange,
            reset,
            pink,
            reset,
            pink,
            reset,
            magenta,
            reset,
            magenta,
            reset
        );
        println!(
            "{}  ███████╗{}{}  ╚████╔╝ {}{}██╔██╗ ██║{}{} ██║     {}{} ███████║{}{}██████╔╝{}{}██║     {}{}█████╗  {}",
            purple,
            reset,
            purple,
            reset,
            orange,
            reset,
            orange,
            reset,
            pink,
            reset,
            pink,
            reset,
            magenta,
            reset,
            magenta,
            reset
        );
        println!(
            "{}  ╚════██║{}{}   ╚██╔╝  {}{}██║╚██╗██║{}{} ██║     {}{} ██╔══██║{}{}██╔══██╗{}{}██║     {}{}██╔══╝  {}",
            purple,
            reset,
            purple,
            reset,
            orange,
            reset,
            orange,
            reset,
            pink,
            reset,
            pink,
            reset,
            magenta,
            reset,
            magenta,
            reset
        );
        println!(
            "{}  ███████║{}{}    ██║   {}{}██║ ╚████║{}{} ╚██████╗{}{} ██║  ██║{}{}██████╔╝{}{}███████╗{}{}███████╗{}",
            purple,
            reset,
            purple,
            reset,
            orange,
            reset,
            orange,
            reset,
            pink,
            reset,
            pink,
            reset,
            magenta,
            reset,
            magenta,
            reset
        );
        println!(
            "{}  ╚══════╝{}{}    ╚═╝   {}{}╚═╝  ╚═══╝{}{}  ╚═════╝{}{} ╚═╝  ╚═╝{}{}╚═════╝ {}{}╚══════╝{}{}╚══════╝{}",
            purple,
            reset,
            purple,
            reset,
            orange,
            reset,
            orange,
            reset,
            pink,
            reset,
            pink,
            reset,
            magenta,
            reset,
            magenta,
            reset
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
            "Want to deploy? Deploy instantly from Syncable Platform → https://syncable.dev"
                .dimmed()
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
        println!("  {}", "Your AI-powered code analysis assistant".dimmed());

        // Check for incomplete plans and show a hint
        let incomplete_plans = find_incomplete_plans(&self.project_path);
        if !incomplete_plans.is_empty() {
            println!();
            if incomplete_plans.len() == 1 {
                let plan = &incomplete_plans[0];
                println!(
                    "  {} {} ({}/{} done)",
                    "📋 Incomplete plan:".yellow(),
                    plan.filename.white(),
                    plan.done,
                    plan.total
                );
                println!(
                    "     {} \"{}\" {}",
                    "→".cyan(),
                    "continue".cyan().bold(),
                    "to resume".dimmed()
                );
            } else {
                println!(
                    "  {} {} incomplete plans found. Use {} to see them.",
                    "📋".yellow(),
                    incomplete_plans.len(),
                    "/plans".cyan()
                );
            }
        }

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

        Ok(read_input_with_file_picker(
            ">",
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
