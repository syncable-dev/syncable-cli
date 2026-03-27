//! Slash command handlers for the chat session.
//!
//! This module contains all the `/command` handlers:
//! - `/model` - Interactive model selection
//! - `/provider` - Switch provider with API key prompt if needed
//! - `/reset` - Reset provider credentials
//! - `/profile` - Manage global profiles
//! - `/plans` - Show incomplete plans
//! - `/resume` - Browse and select a session to resume
//! - `/sessions` - List available sessions

use super::ChatSession;
use super::plan_mode::find_incomplete_plans;
use super::providers::{get_available_models, prompt_api_key};
use crate::agent::{AgentResult, ProviderType};
use crate::config::{load_agent_config, save_agent_config};
use colored::Colorize;
use std::io::{self, Write};

/// Handle /model command - interactive model selection
pub fn handle_model_command(session: &mut ChatSession) -> AgentResult<()> {
    let models = get_available_models(session.provider);

    println!(
        "\n{}",
        format!("Available models for {}:", session.provider)
            .cyan()
            .bold()
    );
    println!();

    for (i, (id, desc)) in models.iter().enumerate() {
        let marker = if *id == session.model { "-> " } else { "  " };
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
        println!("{}", format!("Keeping model: {}", session.model).dimmed());
        return Ok(());
    }

    if let Ok(num) = input.parse::<usize>() {
        if num >= 1 && num <= models.len() {
            let (id, desc) = models[num - 1];
            session.model = id.to_string();

            // Save model choice to config for persistence
            let mut agent_config = load_agent_config();
            agent_config.default_model = Some(id.to_string());
            if let Err(e) = save_agent_config(&agent_config) {
                eprintln!(
                    "{}",
                    format!("Warning: Could not save config: {}", e).yellow()
                );
            }

            println!("{}", format!("Switched to {} - {}", id, desc).green());
        } else {
            println!("{}", "Invalid selection".red());
        }
    } else {
        // Allow direct model name input
        session.model = input.to_string();

        // Save model choice to config for persistence
        let mut agent_config = load_agent_config();
        agent_config.default_model = Some(input.to_string());
        if let Err(e) = save_agent_config(&agent_config) {
            eprintln!(
                "{}",
                format!("Warning: Could not save config: {}", e).yellow()
            );
        }

        println!("{}", format!("Set model to: {}", input).green());
    }

    Ok(())
}

/// Handle /provider command - switch provider with API key prompt if needed
pub fn handle_provider_command(session: &mut ChatSession) -> AgentResult<()> {
    let providers = [
        ProviderType::OpenAI,
        ProviderType::Anthropic,
        ProviderType::Bedrock,
    ];

    println!("\n{}", "Available providers:".cyan().bold());
    println!();

    for (i, provider) in providers.iter().enumerate() {
        let marker = if *provider == session.provider {
            "-> "
        } else {
            "  "
        };
        let has_key = if ChatSession::has_api_key(*provider) {
            "API key configured".green()
        } else {
            "No API key".yellow()
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
            if !ChatSession::has_api_key(new_provider) {
                prompt_api_key(new_provider)?;
            }

            // Load API key/credentials from config to environment
            // This is essential for Bedrock bearer token auth!
            ChatSession::load_api_key_to_env(new_provider);

            session.provider = new_provider;

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
            session.model = default_model.clone();

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
                format!("Switched to {} with model {}", new_provider, default_model).green()
            );
        } else {
            println!("{}", "Invalid selection".red());
        }
    }

    Ok(())
}

/// Handle /reset command - reset provider credentials
pub fn handle_reset_command(session: &mut ChatSession) -> AgentResult<()> {
    let providers = [
        ProviderType::OpenAI,
        ProviderType::Anthropic,
        ProviderType::Bedrock,
    ];

    println!("\n{}", "Reset Provider Credentials".cyan().bold());
    println!();

    for (i, provider) in providers.iter().enumerate() {
        let status = if ChatSession::has_api_key(*provider) {
            "configured".green()
        } else {
            "not configured".dimmed()
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
            println!("{}", "OpenAI credentials cleared".green());
        }
        "2" => {
            agent_config.anthropic_api_key = None;
            unsafe {
                std::env::remove_var("ANTHROPIC_API_KEY");
            }
            println!("{}", "Anthropic credentials cleared".green());
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
            println!("{}", "Bedrock credentials cleared".green());
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
            println!("{}", "All provider credentials cleared".green());
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
        "1" => session.provider == ProviderType::OpenAI,
        "2" => session.provider == ProviderType::Anthropic,
        "3" => session.provider == ProviderType::Bedrock,
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
pub fn handle_profile_command(session: &mut ChatSession) -> AgentResult<()> {
    use crate::config::types::{AnthropicProfile, OpenAIProfile, Profile};

    let mut agent_config = load_agent_config();

    println!("\n{}", "Profile Management".cyan().bold());
    println!();

    // Show current profiles
    list_profiles(&agent_config);

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

            println!("{}", format!("Profile '{}' created", name).green());
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
                    session.provider = p;
                }
            }

            if let Err(e) = save_agent_config(&agent_config) {
                eprintln!(
                    "{}",
                    format!("Warning: Could not save config: {}", e).yellow()
                );
            }

            println!("{}", format!("Switched to profile '{}'", name).green());
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
                        format!("OpenAI configured for profile '{}'", profile_name).green()
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
                        format!("Anthropic configured for profile '{}'", profile_name).green()
                    );
                }
                "3" => {
                    // Configure Bedrock - use the wizard
                    println!("{}", "Running Bedrock setup...".dimmed());
                    let selected_model = super::providers::run_bedrock_setup_wizard()?;

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
                        format!("Bedrock configured for profile '{}'", profile_name).green()
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

                println!("{}", format!("Deleted profile '{}'", name).green());
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
pub fn handle_plans_command(session: &ChatSession) -> AgentResult<()> {
    let incomplete = find_incomplete_plans(&session.project_path);

    if incomplete.is_empty() {
        println!("\n{}", "No incomplete plans found.".dimmed());
        println!(
            "{}",
            "Create a plan using plan mode (Shift+Tab) and the plan_create tool.".dimmed()
        );
        return Ok(());
    }

    println!("\n{}", "Incomplete Plans".cyan().bold());
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
pub fn handle_resume_command(session: &mut ChatSession) -> AgentResult<bool> {
    use crate::agent::persistence::{SessionSelector, browse_sessions, format_relative_time};

    let selector = SessionSelector::new(&session.project_path);
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
    if let Some(selected) = browse_sessions(&session.project_path) {
        // User selected a session - load it
        let time = format_relative_time(selected.last_updated);

        match selector.load_conversation(&selected) {
            Ok(record) => {
                println!(
                    "\n{} Resuming: {} ({}, {} messages)",
                    "ok".green(),
                    selected.display_name.white().bold(),
                    time.dimmed(),
                    record.messages.len()
                );

                // Store for main loop to process
                session.pending_resume = Some(record);
                return Ok(true);
            }
            Err(e) => {
                eprintln!("{} Failed to load session: {}", "error".red(), e);
            }
        }
    }

    Ok(false)
}

/// Handle /sessions command - list available sessions
pub fn handle_list_sessions_command(session: &ChatSession) {
    use crate::agent::persistence::{SessionSelector, format_relative_time};

    let selector = SessionSelector::new(&session.project_path);
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
        format!("Sessions ({})", sessions.len()).cyan().bold()
    );
    println!();

    for s in &sessions {
        let time = format_relative_time(s.last_updated);
        println!(
            "  {} {} {}",
            format!("[{}]", s.index).cyan(),
            s.display_name.white(),
            format!("({})", time).dimmed()
        );
        println!(
            "      {} messages - ID: {}",
            s.message_count.to_string().dimmed(),
            s.id[..8].to_string().dimmed()
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

/// List all profiles (helper function)
fn list_profiles(config: &crate::config::types::AgentConfig) {
    let active = config.active_profile.as_deref();

    if config.profiles.is_empty() {
        println!("{}", "  No profiles configured yet.".dimmed());
        println!();
        return;
    }

    println!("{}", "Profiles:".cyan());
    for (name, profile) in &config.profiles {
        let marker = if Some(name.as_str()) == active {
            "-> "
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
