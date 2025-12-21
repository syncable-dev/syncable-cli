//! Hadolint-RS: Native Rust Dockerfile Linter
//!
//! A Rust translation of the Hadolint Dockerfile linter.
//!
//! # Attribution
//!
//! This module is a derivative work based on [Hadolint](https://github.com/hadolint/hadolint),
//! originally written in Haskell by Lukas Martinelli and contributors.
//!
//! **Original Project:** <https://github.com/hadolint/hadolint>
//! **Original License:** GPL-3.0
//! **Original Copyright:** Copyright (c) 2016-2024 Lukas Martinelli and contributors
//!
//! This Rust translation is licensed under GPL-3.0 as required by the original license.
//! See THIRD_PARTY_NOTICES.md and LICENSE files for full details.
//!
//! # Features
//!
//! - Dockerfile parsing into an AST
//! - Configurable linting rules (DL3xxx, DL4xxx)
//! - ShellCheck-inspired RUN instruction analysis
//! - Inline pragma support for ignoring rules
//!
//! # Example
//!
//! ```rust,ignore
//! use syncable_cli::analyzer::hadolint::{lint, HadolintConfig, LintResult};
//!
//! let dockerfile = r#"
//! FROM ubuntu:latest
//! RUN apt-get update && apt-get install -y nginx
//! "#;
//!
//! let config = HadolintConfig::default();
//! let result = lint(dockerfile, &config);
//!
//! for failure in result.failures {
//!     println!("{}: {} - {}", failure.line, failure.code, failure.message);
//! }
//! ```

pub mod config;
pub mod formatter;
pub mod lint;
pub mod parser;
pub mod pragma;
pub mod rules;
pub mod shell;
pub mod types;

// Re-export main types and functions
pub use config::HadolintConfig;
pub use formatter::{format_result, format_result_to_string, Formatter, OutputFormat};
pub use lint::{lint, lint_file, LintResult};
pub use types::{CheckFailure, RuleCode, Severity};
