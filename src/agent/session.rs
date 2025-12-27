//! Interactive chat session with /model and /provider commands
//!
//! Provides a rich REPL experience similar to Claude Code with:
//! - `/model` - Select from available models based on configured API keys
//! - `/provider` - Switch provider (prompts for API key if not set)
//! - `/cost` - Show token usage and estimated cost
//! - `/help` - Show available commands
//! - `/clear` - Clear conversation history
//! - `/exit` or `/quit` - Exit the session

use crate::agent::commands::{SLASH_COMMANDS, TokenUsage};
use crate::agent::ui::ansi;
use crate::agent::{AgentError, AgentResult, ProviderType};
use crate::config::{load_agent_config, save_agent_config};
use colored::Colorize;
use std::io::{self, Write};
use std::path::Path;

const ROBOT: &str = "🤖";

/// Information about an incomplete plan
#[derive(Debug, Clone)]
pub struct IncompletePlan {
    pub path: String,
    pub filename: String,
    pub done: usize,
    pub pending: usize,
    pub total: usize,
}

/// Find incomplete plans in the plans/ directory
pub fn find_incomplete_plans(project_path: &std::path::Path) -> Vec<IncompletePlan> {
    use regex::Regex;

    let plans_dir = project_path.join("plans");
    if !plans_dir.exists() {
        return Vec::new();
    }

    let task_regex = Regex::new(r"^\s*-\s*\[([ x~!])\]").unwrap();
    let mut incomplete = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&plans_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false)
                && let Ok(content) = std::fs::read_to_string(&path)
            {
                let mut done = 0;
                let mut pending = 0;
                let mut in_progress = 0;

                for line in content.lines() {
                    if let Some(caps) = task_regex.captures(line) {
                        match caps.get(1).map(|m| m.as_str()) {
                            Some("x") => done += 1,
                            Some(" ") => pending += 1,
                            Some("~") => in_progress += 1,
                            Some("!") => done += 1, // Failed counts as "attempted"
                            _ => {}
                        }
                    }
                }

                let total = done + pending + in_progress;
                if total > 0 && (pending > 0 || in_progress > 0) {
                    let rel_path = path
                        .strip_prefix(project_path)
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| path.display().to_string());

                    incomplete.push(IncompletePlan {
                        path: rel_path,
                        filename: path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        done,
                        pending: pending + in_progress,
                        total,
                    });
                }
            }
        }
    }

    // Sort by most recently modified (newest first)
    incomplete.sort_by(|a, b| b.filename.cmp(&a.filename));
    incomplete
}

/// Planning mode state - toggles between standard and plan mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlanMode {
    /// Standard mode - all tools available, normal operation
    #[default]
    Standard,
    /// Planning mode - read-only exploration, no file modifications
    Planning,
}

impl PlanMode {
    /// Toggle between Standard and Planning mode
    pub fn toggle(&self) -> Self {
        match self {
            PlanMode::Standard => PlanMode::Planning,
            PlanMode::Planning => PlanMode::Standard,
        }
    }

    /// Check if in planning mode
    pub fn is_planning(&self) -> bool {
        matches!(self, PlanMode::Planning)
    }

    /// Get display name for the mode
    pub fn display_name(&self) -> &'static str {
        match self {
            PlanMode::Standard => "standard mode",
            PlanMode::Planning => "plan mode",
        }
    }
}

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
            (
                "claude-opus-4-5-20251101",
                "Claude Opus 4.5 - Most capable (Nov 2025)",
            ),
            (
                "claude-sonnet-4-5-20250929",
                "Claude Sonnet 4.5 - Balanced (Sep 2025)",
            ),
            (
                "claude-haiku-4-5-20251001",
                "Claude Haiku 4.5 - Fast (Oct 2025)",
            ),
            ("claude-sonnet-4-20250514", "Claude Sonnet 4 - Previous gen"),
        ],
        // Bedrock models - use cross-region inference profile format (global. prefix)
        ProviderType::Bedrock => vec![
            (
                "global.anthropic.claude-opus-4-5-20251101-v1:0",
                "Claude Opus 4.5 - Most capable (Nov 2025)",
            ),
            (
                "global.anthropic.claude-sonnet-4-5-20250929-v1:0",
                "Claude Sonnet 4.5 - Balanced (Sep 2025)",
            ),
            (
                "global.anthropic.claude-haiku-4-5-20251001-v1:0",
                "Claude Haiku 4.5 - Fast (Oct 2025)",
            ),
            (
                "global.anthropic.claude-sonnet-4-20250514-v1:0",
                "Claude Sonnet 4 - Previous gen",
            ),
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
    /// Current planning mode state
    pub plan_mode: PlanMode,
}

impl ChatSession {
    pub fn new(project_path: &Path, provider: ProviderType, model: Option<String>) -> Self {
        let default_model = match provider {
            ProviderType::OpenAI => "gpt-5.2".to_string(),
            ProviderType::Anthropic => "claude-sonnet-4-5-20250929".to_string(),
            ProviderType::Bedrock => "global.anthropic.claude-sonnet-4-5-20250929-v1:0".to_string(),
        };

        Self {
            provider,
            model: model.unwrap_or(default_model),
            project_path: project_path.to_path_buf(),
            history: Vec::new(),
            token_usage: TokenUsage::new(),
            plan_mode: PlanMode::default(),
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
        // Check environment variable first
        let env_key = match provider {
            ProviderType::OpenAI => std::env::var("OPENAI_API_KEY").ok(),
            ProviderType::Anthropic => std::env::var("ANTHROPIC_API_KEY").ok(),
            ProviderType::Bedrock => {
                // Check for AWS credentials from env vars
                if std::env::var("AWS_ACCESS_KEY_ID").is_ok()
                    && std::env::var("AWS_SECRET_ACCESS_KEY").is_ok()
                {
                    return true;
                }
                if std::env::var("AWS_PROFILE").is_ok() {
                    return true;
                }
                None
            }
        };

        if env_key.is_some() {
            return true;
        }

        // Check config file - first try active global profile
        let agent_config = load_agent_config();

        // Check active global profile first
        if let Some(profile_name) = &agent_config.active_profile
            && let Some(profile) = agent_config.profiles.get(profile_name)
        {
            match provider {
                ProviderType::OpenAI => {
                    if profile
                        .openai
                        .as_ref()
                        .map(|o| !o.api_key.is_empty())
                        .unwrap_or(false)
                    {
                        return true;
                    }
                }
                ProviderType::Anthropic => {
                    if profile
                        .anthropic
                        .as_ref()
                        .map(|a| !a.api_key.is_empty())
                        .unwrap_or(false)
                    {
                        return true;
                    }
                }
                ProviderType::Bedrock => {
                    if let Some(bedrock) = &profile.bedrock
                        && (bedrock.profile.is_some()
                            || (bedrock.access_key_id.is_some()
                                && bedrock.secret_access_key.is_some()))
                    {
                        return true;
                    }
                }
            }
        }

        // Check any profile that has this provider configured
        for profile in agent_config.profiles.values() {
            match provider {
                ProviderType::OpenAI => {
                    if profile
                        .openai
                        .as_ref()
                        .map(|o| !o.api_key.is_empty())
                        .unwrap_or(false)
                    {
                        return true;
                    }
                }
                ProviderType::Anthropic => {
                    if profile
                        .anthropic
                        .as_ref()
                        .map(|a| !a.api_key.is_empty())
                        .unwrap_or(false)
                    {
                        return true;
                    }
                }
                ProviderType::Bedrock => {
                    if let Some(bedrock) = &profile.bedrock
                        && (bedrock.profile.is_some()
                            || (bedrock.access_key_id.is_some()
                                && bedrock.secret_access_key.is_some()))
                    {
                        return true;
                    }
                }
            }
        }

        // Fall back to legacy config
        match provider {
            ProviderType::OpenAI => agent_config.openai_api_key.is_some(),
            ProviderType::Anthropic => agent_config.anthropic_api_key.is_some(),
            ProviderType::Bedrock => {
                if let Some(bedrock) = &agent_config.bedrock {
                    bedrock.profile.is_some()
                        || (bedrock.access_key_id.is_some() && bedrock.secret_access_key.is_some())
                } else {
                    agent_config.bedrock_configured.unwrap_or(false)
                }
            }
        }
    }

    /// Load API key from config if not in env, and set it in env for use
    pub fn load_api_key_to_env(provider: ProviderType) {
        let agent_config = load_agent_config();

        // Try to get credentials from active global profile first
        let active_profile = agent_config
            .active_profile
            .as_ref()
            .and_then(|name| agent_config.profiles.get(name));

        match provider {
            ProviderType::OpenAI => {
                if std::env::var("OPENAI_API_KEY").is_ok() {
                    return;
                }
                // Check active global profile
                if let Some(key) = active_profile
                    .and_then(|p| p.openai.as_ref())
                    .map(|o| o.api_key.clone())
                    .filter(|k| !k.is_empty())
                {
                    unsafe {
                        std::env::set_var("OPENAI_API_KEY", &key);
                    }
                    return;
                }
                // Fall back to legacy key
                if let Some(key) = &agent_config.openai_api_key {
                    unsafe {
                        std::env::set_var("OPENAI_API_KEY", key);
                    }
                }
            }
            ProviderType::Anthropic => {
                if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                    return;
                }
                // Check active global profile
                if let Some(key) = active_profile
                    .and_then(|p| p.anthropic.as_ref())
                    .map(|a| a.api_key.clone())
                    .filter(|k| !k.is_empty())
                {
                    unsafe {
                        std::env::set_var("ANTHROPIC_API_KEY", &key);
                    }
                    return;
                }
                // Fall back to legacy key
                if let Some(key) = &agent_config.anthropic_api_key {
                    unsafe {
                        std::env::set_var("ANTHROPIC_API_KEY", key);
                    }
                }
            }
            ProviderType::Bedrock => {
                // Check active global profile first
                let bedrock_config = active_profile
                    .and_then(|p| p.bedrock.as_ref())
                    .or(agent_config.bedrock.as_ref());

                if let Some(bedrock) = bedrock_config {
                    // Load region
                    if std::env::var("AWS_REGION").is_err()
                        && let Some(region) = &bedrock.region
                    {
                        unsafe {
                            std::env::set_var("AWS_REGION", region);
                        }
                    }
                    // Load profile OR access keys (profile takes precedence)
                    if let Some(profile) = &bedrock.profile
                        && std::env::var("AWS_PROFILE").is_err()
                    {
                        unsafe {
                            std::env::set_var("AWS_PROFILE", profile);
                        }
                    } else if let (Some(key_id), Some(secret)) =
                        (&bedrock.access_key_id, &bedrock.secret_access_key)
                    {
                        if std::env::var("AWS_ACCESS_KEY_ID").is_err() {
                            unsafe {
                                std::env::set_var("AWS_ACCESS_KEY_ID", key_id);
                            }
                        }
                        if std::env::var("AWS_SECRET_ACCESS_KEY").is_err() {
                            unsafe {
                                std::env::set_var("AWS_SECRET_ACCESS_KEY", secret);
                            }
                        }
                    }
                }
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

    /// Interactive wizard to set up AWS Bedrock credentials
    fn run_bedrock_setup_wizard() -> AgentResult<String> {
        use crate::config::types::BedrockConfig as BedrockConfigType;

        println!();
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan()
        );
        println!("{}", "  🔧 AWS Bedrock Setup Wizard".cyan().bold());
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan()
        );
        println!();
        println!("AWS Bedrock provides access to Claude models via AWS.");
        println!("You'll need an AWS account with Bedrock access enabled.");
        println!();

        // Step 1: Choose authentication method
        println!("{}", "Step 1: Choose authentication method".white().bold());
        println!();
        println!(
            "  {} Use AWS Profile (from ~/.aws/credentials)",
            "[1]".cyan()
        );
        println!(
            "      {}",
            "Best for: AWS CLI users, SSO, multiple accounts".dimmed()
        );
        println!();
        println!("  {} Enter Access Keys directly", "[2]".cyan());
        println!(
            "      {}",
            "Best for: Quick setup, CI/CD environments".dimmed()
        );
        println!();
        println!("  {} Use existing environment variables", "[3]".cyan());
        println!(
            "      {}",
            "Best for: Already configured AWS_* env vars".dimmed()
        );
        println!();
        print!("Enter choice [1-3]: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin()
            .read_line(&mut choice)
            .map_err(|e| AgentError::ToolError(e.to_string()))?;
        let choice = choice.trim();

        let mut bedrock_config = BedrockConfigType::default();

        match choice {
            "1" => {
                // AWS Profile
                println!();
                println!("{}", "Step 2: Enter AWS Profile".white().bold());
                println!("{}", "Press Enter for 'default' profile".dimmed());
                print!("Profile name: ");
                io::stdout().flush().unwrap();

                let mut profile = String::new();
                io::stdin()
                    .read_line(&mut profile)
                    .map_err(|e| AgentError::ToolError(e.to_string()))?;
                let profile = profile.trim();
                let profile = if profile.is_empty() {
                    "default"
                } else {
                    profile
                };

                bedrock_config.profile = Some(profile.to_string());

                // Set in env for current session
                unsafe {
                    std::env::set_var("AWS_PROFILE", profile);
                }
                println!("{}", format!("✓ Using profile: {}", profile).green());
            }
            "2" => {
                // Access Keys
                println!();
                println!("{}", "Step 2: Enter AWS Access Keys".white().bold());
                println!(
                    "{}",
                    "Get these from AWS Console → IAM → Security credentials".dimmed()
                );
                println!();

                print!("AWS Access Key ID: ");
                io::stdout().flush().unwrap();
                let mut access_key = String::new();
                io::stdin()
                    .read_line(&mut access_key)
                    .map_err(|e| AgentError::ToolError(e.to_string()))?;
                let access_key = access_key.trim().to_string();

                if access_key.is_empty() {
                    return Err(AgentError::MissingApiKey("AWS_ACCESS_KEY_ID".to_string()));
                }

                print!("AWS Secret Access Key: ");
                io::stdout().flush().unwrap();
                let mut secret_key = String::new();
                io::stdin()
                    .read_line(&mut secret_key)
                    .map_err(|e| AgentError::ToolError(e.to_string()))?;
                let secret_key = secret_key.trim().to_string();

                if secret_key.is_empty() {
                    return Err(AgentError::MissingApiKey(
                        "AWS_SECRET_ACCESS_KEY".to_string(),
                    ));
                }

                bedrock_config.access_key_id = Some(access_key.clone());
                bedrock_config.secret_access_key = Some(secret_key.clone());

                // Set in env for current session
                unsafe {
                    std::env::set_var("AWS_ACCESS_KEY_ID", &access_key);
                    std::env::set_var("AWS_SECRET_ACCESS_KEY", &secret_key);
                }
                println!("{}", "✓ Access keys configured".green());
            }
            "3" => {
                // Use existing env vars
                if std::env::var("AWS_ACCESS_KEY_ID").is_err()
                    && std::env::var("AWS_PROFILE").is_err()
                {
                    println!("{}", "⚠ No AWS credentials found in environment!".yellow());
                    println!("Set AWS_ACCESS_KEY_ID + AWS_SECRET_ACCESS_KEY or AWS_PROFILE");
                    return Err(AgentError::MissingApiKey("AWS credentials".to_string()));
                }
                println!("{}", "✓ Using existing environment variables".green());
            }
            _ => {
                println!("{}", "Invalid choice, using environment variables".yellow());
            }
        }

        // Step 2: Region selection
        if bedrock_config.region.is_none() {
            println!();
            println!("{}", "Step 2: Select AWS Region".white().bold());
            println!(
                "{}",
                "Bedrock is available in select regions. Common choices:".dimmed()
            );
            println!();
            println!(
                "  {} us-east-1     (N. Virginia) - Most models",
                "[1]".cyan()
            );
            println!("  {} us-west-2     (Oregon)", "[2]".cyan());
            println!("  {} eu-west-1     (Ireland)", "[3]".cyan());
            println!("  {} ap-northeast-1 (Tokyo)", "[4]".cyan());
            println!();
            print!("Enter choice [1-4] or region name: ");
            io::stdout().flush().unwrap();

            let mut region_choice = String::new();
            io::stdin()
                .read_line(&mut region_choice)
                .map_err(|e| AgentError::ToolError(e.to_string()))?;
            let region = match region_choice.trim() {
                "1" | "" => "us-east-1",
                "2" => "us-west-2",
                "3" => "eu-west-1",
                "4" => "ap-northeast-1",
                other => other,
            };

            bedrock_config.region = Some(region.to_string());
            unsafe {
                std::env::set_var("AWS_REGION", region);
            }
            println!("{}", format!("✓ Region: {}", region).green());
        }

        // Step 3: Model selection
        println!();
        println!("{}", "Step 3: Select Default Model".white().bold());
        println!();
        let models = get_available_models(ProviderType::Bedrock);
        for (i, (id, desc)) in models.iter().enumerate() {
            let marker = if i == 0 { "→ " } else { "  " };
            println!("  {} {} {}", marker, format!("[{}]", i + 1).cyan(), desc);
            println!("      {}", id.dimmed());
        }
        println!();
        print!("Enter choice [1-{}] (default: 1): ", models.len());
        io::stdout().flush().unwrap();

        let mut model_choice = String::new();
        io::stdin()
            .read_line(&mut model_choice)
            .map_err(|e| AgentError::ToolError(e.to_string()))?;
        let model_idx: usize = model_choice.trim().parse().unwrap_or(1);
        let model_idx = model_idx.saturating_sub(1).min(models.len() - 1);
        let selected_model = models[model_idx].0.to_string();

        bedrock_config.default_model = Some(selected_model.clone());
        println!(
            "{}",
            format!(
                "✓ Default model: {}",
                models[model_idx]
                    .1
                    .split(" - ")
                    .next()
                    .unwrap_or(&selected_model)
            )
            .green()
        );

        // Save configuration
        let mut agent_config = load_agent_config();
        agent_config.bedrock = Some(bedrock_config);
        agent_config.bedrock_configured = Some(true);

        if let Err(e) = save_agent_config(&agent_config) {
            eprintln!(
                "{}",
                format!("Warning: Could not save config: {}", e).yellow()
            );
        } else {
            println!();
            println!("{}", "✓ Configuration saved to ~/.syncable.toml".green());
        }

        println!();
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan()
        );
        println!("{}", "  ✅ AWS Bedrock setup complete!".green().bold());
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan()
        );
        println!();

        Ok(selected_model)
    }

    /// Prompt user to enter API key for a provider
    pub fn prompt_api_key(provider: ProviderType) -> AgentResult<String> {
        // Bedrock uses AWS credential chain - run setup wizard
        if matches!(provider, ProviderType::Bedrock) {
            return Self::run_bedrock_setup_wizard();
        }

        let env_var = match provider {
            ProviderType::OpenAI => "OPENAI_API_KEY",
            ProviderType::Anthropic => "ANTHROPIC_API_KEY",
            ProviderType::Bedrock => unreachable!(), // Handled above
        };

        println!(
            "\n{}",
            format!("🔑 No API key found for {}", provider).yellow()
        );
        println!("Please enter your {} API key:", provider);
        print!("> ");
        io::stdout().flush().unwrap();

        let mut key = String::new();
        io::stdin()
            .read_line(&mut key)
            .map_err(|e| AgentError::ToolError(e.to_string()))?;
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
            ProviderType::Bedrock => unreachable!(), // Handled above
        }

        if let Err(e) = save_agent_config(&agent_config) {
            eprintln!(
                "{}",
                format!("Warning: Could not save config: {}", e).yellow()
            );
        } else {
            println!("{}", "✓ API key saved to ~/.syncable.toml".green());
        }

        Ok(key)
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
                    Self::prompt_api_key(new_provider)?;
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
                        let selected_model = Self::run_bedrock_setup_wizard()?;

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
            "You:",
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
