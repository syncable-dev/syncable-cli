//! # Syncable IaC CLI
//!
//! A Rust-based command-line application that analyzes code repositories and automatically
//! generates Infrastructure as Code configurations including Dockerfiles, Docker Compose
//! files, and Terraform configurations.
//!
//! ## Features
//!
//! - **Language Detection**: Automatically detects programming languages and their versions
//! - **Framework Analysis**: Identifies frameworks and libraries used in the project
//! - **Smart Generation**: Creates optimized IaC configurations based on project analysis
//! - **Multiple Formats**: Supports Docker, Docker Compose, and Terraform generation
//! - **Security-First**: Generates secure configurations following best practices
//!
//! ## Example
//!
//! ```rust,no_run
//! use syncable_cli::{analyze_project, generate_dockerfile};
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let project_path = Path::new("./my-project");
//! let analysis = analyze_project(project_path)?;
//! let dockerfile = generate_dockerfile(&analysis)?;
//! println!("{}", dockerfile);
//! # Ok(())
//! # }
//! ```

pub mod analyzer;
pub mod cli;
pub mod common;
pub mod config;
pub mod error;
pub mod generator;
pub mod handlers;

// Re-export commonly used types and functions
pub use analyzer::{analyze_project, ProjectAnalysis};
pub use error::{IaCGeneratorError, Result};
pub use generator::{generate_dockerfile, generate_compose, generate_terraform};
pub use handlers::*;
use cli::Commands;

/// The current version of the CLI tool
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn run_command(command: Commands) -> Result<()> {
    match command {
        Commands::Analyze { path, json, detailed, display, only } => {
            match handlers::handle_analyze(path, json, detailed, display, only) {
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
            force
        } => {
            handlers::handle_generate(path, output, dockerfile, compose, terraform, all, dry_run, force)
        }
        Commands::Validate { path, types, fix } => {
            handlers::handle_validate(path, types, fix)
        }
        Commands::Support { languages, frameworks, detailed } => {
            handlers::handle_support(languages, frameworks, detailed)
        }
        Commands::Dependencies { path, licenses, vulnerabilities, prod_only, dev_only, format } => {
            handlers::handle_dependencies(path, licenses, vulnerabilities, prod_only, dev_only, format).await.map(|_| ())
        }
        Commands::Vulnerabilities { path, severity, format, output } => {
            handlers::handle_vulnerabilities(path, severity, format, output).await
        }
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
            fail_on_findings
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
                fail_on_findings
            ).map(|_| ()) // Map Result<String> to Result<()>
        }
        Commands::Tools { command } => {
            handlers::handle_tools(command).await
        }
    }
} 