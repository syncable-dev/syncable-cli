pub mod agent;
pub mod analyzer;
pub mod cli;
pub mod common;
pub mod config;
pub mod error;
pub mod generator;
pub mod handlers;
pub mod telemetry;  // Add telemetry module

// Re-export commonly used types and functions
pub use analyzer::{ProjectAnalysis, analyze_project};
use cli::Commands;
pub use error::{IaCGeneratorError, Result};
pub use generator::{generate_compose, generate_dockerfile, generate_terraform};
pub use handlers::*;
pub use telemetry::{TelemetryClient, TelemetryConfig, UserId};  // Re-export telemetry types

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
        Commands::Chat { path, provider, model, query, setup } => {
            use agent::{run_interactive, run_query, ProviderType};
            use agent::config::{ensure_credentials, run_setup_wizard};
            use cli::ChatProvider;
            
            // If setup flag is passed, run the wizard
            if setup {
                run_setup_wizard()
                    .map(|_| ())
                    .map_err(|e| error::IaCGeneratorError::Config(
                        error::ConfigError::ParsingFailed(e.to_string()),
                    ))?;
                return Ok(());
            }
            
            let project_path = path.canonicalize().unwrap_or(path);
            
            // Convert CLI provider to agent provider type
            let cli_provider = provider.map(|p| match p {
                ChatProvider::Openai => ProviderType::OpenAI,
                ChatProvider::Anthropic => ProviderType::Anthropic,
                ChatProvider::Ollama => ProviderType::OpenAI, // Fallback
            });
            
            // Ensure credentials are available (prompts if needed)
            let (agent_provider, default_model) = ensure_credentials(cli_provider)
                .map_err(|e| error::IaCGeneratorError::Config(
                    error::ConfigError::ParsingFailed(e.to_string()),
                ))?;
            
            // Use provided model, or default from config
            let model = model.or(default_model);
            
            if let Some(q) = query {
                run_query(&project_path, &q, agent_provider, model)
                    .await
                    .map(|response| {
                        println!("{}", response);
                    })
                    .map_err(|e| error::IaCGeneratorError::Config(
                        error::ConfigError::ParsingFailed(e.to_string()),
                    ))
            } else {
                run_interactive(&project_path, agent_provider, model)
                    .await
                    .map_err(|e| error::IaCGeneratorError::Config(
                        error::ConfigError::ParsingFailed(e.to_string()),
                    ))
            }
        }
    }
}