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
        Commands::Chat { path, provider, model, query } => {
            use agent::ProviderType;
            use cli::ChatProvider;

            let project_path = path.canonicalize().unwrap_or(path);
            let provider_type = match provider {
                ChatProvider::Openai => ProviderType::OpenAI,
                ChatProvider::Anthropic => ProviderType::Anthropic,
                ChatProvider::Ollama => {
                    eprintln!("Ollama support coming soon. Using OpenAI as fallback.");
                    ProviderType::OpenAI
                }
            };

            if let Some(q) = query {
                let response = agent::run_query(&project_path, &q, provider_type, model).await?;
                println!("{}", response);
                Ok(())
            } else {
                agent::run_interactive(&project_path, provider_type, model).await?;
                Ok(())
            }
        }
    }
}