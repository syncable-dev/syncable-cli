pub mod agent;
pub mod analyzer;
pub mod bedrock; // Inlined rig-bedrock with extended thinking fixes
pub mod cli;
pub mod common;
pub mod config;
pub mod error;
pub mod generator;
pub mod handlers;
pub mod telemetry; // Add telemetry module

// Re-export commonly used types and functions
pub use analyzer::{ProjectAnalysis, analyze_project};
use cli::Commands;
pub use error::{IaCGeneratorError, Result};
pub use generator::{generate_compose, generate_dockerfile, generate_terraform};
pub use handlers::*;
pub use telemetry::{TelemetryClient, TelemetryConfig, UserId}; // Re-export telemetry types

/// The current version of the CLI tool
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn run_command(command: Commands) -> Result<()> {
    match command {
        Commands::Analyze {
            path,
            json,
            detailed,
            display,
            only,
            color_scheme,
        } => {
            match handlers::handle_analyze(path, json, detailed, display, only, color_scheme) {
                Ok(_output) => Ok(()), // The output was already printed by display_analysis_with_return
                Err(e) => Err(e),
            }
        }
        Commands::Generate {
            path,
            output,
            dockerfile,
            compose,
            terraform,
            all,
            dry_run,
            force,
        } => handlers::handle_generate(
            path, output, dockerfile, compose, terraform, all, dry_run, force,
        ),
        Commands::Validate { path, types, fix } => handlers::handle_validate(path, types, fix),
        Commands::Support {
            languages,
            frameworks,
            detailed,
        } => handlers::handle_support(languages, frameworks, detailed),
        Commands::Dependencies {
            path,
            licenses,
            vulnerabilities,
            prod_only,
            dev_only,
            format,
        } => handlers::handle_dependencies(
            path,
            licenses,
            vulnerabilities,
            prod_only,
            dev_only,
            format,
        )
        .await
        .map(|_| ()),
        Commands::Vulnerabilities {
            path,
            severity,
            format,
            output,
        } => handlers::handle_vulnerabilities(path, severity, format, output).await,
        Commands::Security {
            path,
            mode,
            include_low,
            no_secrets,
            no_code_patterns,
            no_infrastructure,
            no_compliance,
            frameworks,
            format,
            output,
            fail_on_findings,
        } => {
            handlers::handle_security(
                path,
                mode,
                include_low,
                no_secrets,
                no_code_patterns,
                no_infrastructure,
                no_compliance,
                frameworks,
                format,
                output,
                fail_on_findings,
            )
            .map(|_| ()) // Map Result<String> to Result<()>
        }
        Commands::Tools { command } => handlers::handle_tools(command).await,
        Commands::Chat {
            path,
            provider,
            model,
            query,
            resume,
            list_sessions: _, // Handled in main.rs
        } => {
            use agent::ProviderType;
            use cli::ChatProvider;
            use config::load_agent_config;

            let project_path = path.canonicalize().unwrap_or(path);

            // Handle --resume flag
            if let Some(ref resume_arg) = resume {
                use agent::persistence::{SessionSelector, format_relative_time};

                let selector = SessionSelector::new(&project_path);
                if let Some(session_info) = selector.resolve_session(resume_arg) {
                    let time = format_relative_time(session_info.last_updated);
                    println!(
                        "\nResuming session: {} ({}, {} messages)",
                        session_info.display_name, time, session_info.message_count
                    );
                    println!("Session ID: {}\n", session_info.id);

                    // Load the session
                    match selector.load_conversation(&session_info) {
                        Ok(record) => {
                            // Display previous messages as context
                            println!("--- Previous conversation ---");
                            for msg in record.messages.iter().take(5) {
                                let role = match msg.role {
                                    agent::persistence::MessageRole::User => "You",
                                    agent::persistence::MessageRole::Assistant => "AI",
                                    agent::persistence::MessageRole::System => "System",
                                };
                                let preview = if msg.content.len() > 100 {
                                    format!("{}...", &msg.content[..100])
                                } else {
                                    msg.content.clone()
                                };
                                println!("  {}: {}", role, preview);
                            }
                            if record.messages.len() > 5 {
                                println!("  ... and {} more messages", record.messages.len() - 5);
                            }
                            println!("--- End of history ---\n");
                            // TODO: Load history into conversation context
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to load session history: {}", e);
                        }
                    }
                } else {
                    eprintln!(
                        "Session '{}' not found. Use --list-sessions to see available sessions.",
                        resume_arg
                    );
                    return Ok(());
                }
            }

            // Load saved config for Auto mode
            let agent_config = load_agent_config();

            // Determine provider - use saved default if Auto
            let (provider_type, effective_model) = match provider {
                ChatProvider::Openai => (ProviderType::OpenAI, model),
                ChatProvider::Anthropic => (ProviderType::Anthropic, model),
                ChatProvider::Bedrock => (ProviderType::Bedrock, model),
                ChatProvider::Ollama => {
                    eprintln!("Ollama support coming soon. Using OpenAI as fallback.");
                    (ProviderType::OpenAI, model)
                }
                ChatProvider::Auto => {
                    // Load from saved config
                    let saved_provider = match agent_config.default_provider.as_str() {
                        "openai" => ProviderType::OpenAI,
                        "anthropic" => ProviderType::Anthropic,
                        "bedrock" => ProviderType::Bedrock,
                        _ => ProviderType::OpenAI, // Fallback
                    };
                    // Use saved model if no explicit model provided
                    let saved_model = if model.is_some() {
                        model
                    } else {
                        agent_config.default_model.clone()
                    };
                    (saved_provider, saved_model)
                }
            };

            // Load API key/credentials from config to environment
            // This is essential for Bedrock bearer token auth!
            agent::session::ChatSession::load_api_key_to_env(provider_type);

            if let Some(q) = query {
                let response =
                    agent::run_query(&project_path, &q, provider_type, effective_model).await?;
                println!("{}", response);
                Ok(())
            } else {
                agent::run_interactive(&project_path, provider_type, effective_model).await?;
                Ok(())
            }
        }
    }
}
