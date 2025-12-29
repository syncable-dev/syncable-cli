//! Helmlint-RS: Native Rust Helm Chart Linter
//!
//! A Rust implementation of a comprehensive Helm chart linter, inspired by
//! and partially derived from the helmtest project.
//!
//! # Attribution
//!
//! This module is a derivative work inspired by [helmtest](https://github.com/stackrox/helmtest),
//! originally written in Go by StackRox (Red Hat).
//!
//! **Original Project:** <https://github.com/stackrox/helmtest>
//! **Original License:** Apache-2.0
//! **Original Copyright:** Copyright (c) StackRox, Inc.
//!
//! This Rust translation maintains compatibility with the Apache-2.0 license.
//! See THIRD_PARTY_NOTICES.md and LICENSE files for full details.
//!
//! # Features
//!
//! - Chart.yaml validation (structure, versions, dependencies)
//! - values.yaml validation (types, schema, unused values)
//! - Template syntax analysis (unclosed blocks, undefined variables)
//! - Security checks (privileged containers, host access)
//! - Best practice validation (resource limits, probes, deprecated APIs)
//! - Inline pragma support for ignoring rules
//!
//! # Example
//!
//! ```rust,ignore
//! use syncable_cli::analyzer::helmlint::{lint_chart, HelmlintConfig, LintResult};
//! use std::path::Path;
//!
//! let config = HelmlintConfig::default();
//! let result = lint_chart(Path::new("./my-chart"), &config);
//!
//! for failure in result.failures {
//!     println!("{}: {} - {}", failure.file, failure.code, failure.message);
//! }
//! ```
//!
//! # Rules
//!
//! | Category | Code Range | Description |
//! |----------|------------|-------------|
//! | Structure | HL1xxx | Chart.yaml and file structure |
//! | Values | HL2xxx | values.yaml validation |
//! | Templates | HL3xxx | Go template syntax |
//! | Security | HL4xxx | Container security |
//! | Best Practices | HL5xxx | K8s best practices |

pub mod config;
pub mod formatter;
pub mod k8s;
pub mod lint;
pub mod parser;
pub mod pragma;
pub mod rules;
pub mod types;

// Re-export main types and functions
pub use config::HelmlintConfig;
pub use formatter::{OutputFormat, format_result, format_result_to_string};
pub use lint::{LintResult, lint_chart, lint_chart_file};
pub use types::{CheckFailure, RuleCode, Severity};
