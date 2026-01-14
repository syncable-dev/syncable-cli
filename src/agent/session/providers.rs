//! Provider-related logic for API key management, model selection, and credential handling.
//!
//! This module contains:
//! - `get_available_models` - Returns available models per provider
//! - `has_api_key` - Checks if API key is configured for a provider
//! - `load_api_key_to_env` - Loads API key from config and sets in environment
//! - `get_configured_providers` - Returns list of providers with valid credentials
//! - `prompt_api_key` - Prompts user for API key interactively

use crate::agent::{AgentError, AgentResult, ProviderType};
use crate::config::{load_agent_config, save_agent_config};
use colored::Colorize;
use std::io::{self, Write};

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
    if has_api_key(ProviderType::OpenAI) {
        providers.push(ProviderType::OpenAI);
    }
    if has_api_key(ProviderType::Anthropic) {
        providers.push(ProviderType::Anthropic);
    }
    providers
}

/// Interactive wizard to set up AWS Bedrock credentials
pub(crate) fn run_bedrock_setup_wizard() -> AgentResult<String> {
    use crate::config::types::BedrockConfig as BedrockConfigType;

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan()
    );
    println!("{}", "  AWS Bedrock Setup Wizard".cyan().bold());
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
            println!("{}", format!("Using profile: {}", profile).green());
        }
        "2" => {
            // Access Keys
            println!();
            println!("{}", "Step 2: Enter AWS Access Keys".white().bold());
            println!(
                "{}",
                "Get these from AWS Console -> IAM -> Security credentials".dimmed()
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
            println!("{}", "Access keys configured".green());
        }
        "3" => {
            // Use existing env vars
            if std::env::var("AWS_ACCESS_KEY_ID").is_err()
                && std::env::var("AWS_PROFILE").is_err()
            {
                println!("{}", "No AWS credentials found in environment!".yellow());
                println!("Set AWS_ACCESS_KEY_ID + AWS_SECRET_ACCESS_KEY or AWS_PROFILE");
                return Err(AgentError::MissingApiKey("AWS credentials".to_string()));
            }
            println!("{}", "Using existing environment variables".green());
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
        println!("{}", format!("Region: {}", region).green());
    }

    // Step 3: Model selection
    println!();
    println!("{}", "Step 3: Select Default Model".white().bold());
    println!();
    let models = get_available_models(ProviderType::Bedrock);
    for (i, (id, desc)) in models.iter().enumerate() {
        let marker = if i == 0 { "-> " } else { "  " };
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
            "Default model: {}",
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
        println!("{}", "Configuration saved to ~/.syncable.toml".green());
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan()
    );
    println!("{}", "  AWS Bedrock setup complete!".green().bold());
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
        return run_bedrock_setup_wizard();
    }

    let env_var = match provider {
        ProviderType::OpenAI => "OPENAI_API_KEY",
        ProviderType::Anthropic => "ANTHROPIC_API_KEY",
        ProviderType::Bedrock => unreachable!(), // Handled above
    };

    println!(
        "\n{}",
        format!("No API key found for {}", provider).yellow()
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
        println!("{}", "API key saved to ~/.syncable.toml".green());
    }

    Ok(key)
}
