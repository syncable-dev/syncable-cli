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
//! - `templates`        — Per-platform YAML assemblers (CI-11, CI-12, CI-13)//! - `triggers`         — Trigger configuration resolver (CI-18)
pub mod cache;
pub mod context;
pub mod runtime_resolver;
pub mod schema;
pub mod templates;
pub mod triggers;

#[cfg(test)]
pub mod test_helpers;
