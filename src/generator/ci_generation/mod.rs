//! CI/CD Pipeline Generation Module
//!
//! Generates CI and CD pipeline skeletons from project analysis.
//! Follows the same analyze → generate → write pattern as the existing
//! Dockerfile and Compose generators.
//!
//! ## Submodules
//!
//! - `context`          — `CiContext` struct and context collector (CI-02)
//! - `runtime_resolver` — Runtime version resolver (CI-03)
//! - `cache`            — Dependency cache strategy (CI-04)
//! - `schema`           — Platform-agnostic `CiPipeline` data model (CI-14)
//! - `templates`        — Per-platform YAML assemblers (CI-11, CI-12, CI-13)
//! - `token_resolver`   — Two-pass placeholder token engine (CI-15)
//! - `triggers`         — Trigger configuration resolver (CI-18)
pub mod build_step;
pub mod cache;
pub mod ci_config;
pub mod context;
pub mod coverage_step;
pub mod dry_run;
pub mod matrix;
pub mod monorepo;
pub mod notify_step;
pub mod secrets_doc;
pub mod docker_step;
pub mod image_scan_step;
pub mod lint_step;
pub mod runtime_resolver;
pub mod secret_scan_step;
pub mod schema;
pub mod templates;
pub mod test_step;
pub mod token_resolver;
pub mod triggers;
pub mod writer;

#[cfg(test)]
pub mod test_helpers;
