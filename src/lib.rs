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

// Re-export commonly used types and functions
pub use analyzer::{analyze_project, ProjectAnalysis};
pub use error::{IaCGeneratorError, Result};
pub use generator::{generate_dockerfile, generate_compose, generate_terraform};

/// The current version of the CLI tool
pub const VERSION: &str = env!("CARGO_PKG_VERSION"); 