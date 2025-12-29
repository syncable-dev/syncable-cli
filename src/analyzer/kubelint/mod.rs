//! KubeLint-RS: Native Rust Kubernetes Linter
//!
//! A Rust translation of the kube-linter project.
//!
//! # Attribution
//!
//! This module is a derivative work based on [kube-linter](https://github.com/stackrox/kube-linter),
//! originally written in Go by StackRox (Red Hat).
//!
//! **Original Project:** <https://github.com/stackrox/kube-linter>
//! **Original License:** Apache-2.0
//! **Original Copyright:** Copyright (c) StackRox, Inc.
//!
//! This Rust translation maintains compatibility with the Apache-2.0 license.
//! See THIRD_PARTY_NOTICES.md and LICENSE files for full details.
//!
//! # Features
//!
//! - Kubernetes YAML file validation
//! - Helm chart linting (with template rendering)
//! - Kustomize directory support
//! - 63 built-in security and best practice checks
//! - Annotation-based rule ignoring
//! - Multiple output formats (JSON, SARIF, plain text)
//!
//! # Example
//!
//! ```rust,ignore
//! use syncable_cli::analyzer::kubelint::{lint, KubelintConfig, LintResult};
//! use std::path::Path;
//!
//! let config = KubelintConfig::default();
//! let result = lint(Path::new("./k8s/deployment.yaml"), &config);
//!
//! for failure in result.failures {
//!     println!("{}: {} - {}", failure.file_path.display(), failure.code, failure.message);
//! }
//! ```
//!
//! # Checks
//!
//! KubeLint includes 63 built-in checks covering:
//!
//! ## Security Checks
//! - Privileged containers
//! - Privilege escalation
//! - Run as non-root
//! - Read-only root filesystem
//! - Linux capabilities
//! - Host namespace access (network, PID, IPC)
//! - Host path mounts
//!
//! ## Best Practice Checks
//! - Image tag policies (no :latest)
//! - Liveness/readiness probes
//! - Resource requirements (CPU/memory)
//! - Minimum replicas
//! - Anti-affinity rules
//! - Rolling update strategy
//!
//! ## RBAC Checks
//! - Cluster admin bindings
//! - Wildcard rules
//! - Access to sensitive resources
//!
//! ## Validation Checks
//! - Dangling services/ingresses
//! - Selector mismatches
//! - Invalid target ports

pub mod checks;
pub mod config;
pub mod context;
pub mod extract;
pub mod formatter;
pub mod lint;
pub mod objectkinds;
pub mod parser;
pub mod pragma;
pub mod templates;
pub mod types;

// Re-export main types and functions
pub use config::KubelintConfig;
pub use formatter::{OutputFormat, format_result, format_result_to_string};
pub use lint::{LintResult, LintSummary, lint, lint_content, lint_file};
pub use types::{CheckFailure, Diagnostic, RuleCode, Severity};
