//! CI/CD Pipeline Generation Module
//!
//! Generates CI and CD pipeline skeletons from project analysis.
//! Follows the same analyze → generate → write pattern as the existing
//! Dockerfile and Compose generators.
//!
//! ## Submodules
//!
//! - `context`  — `CiContext` struct and context collector (CI-02)
//! - `schema`   — Platform-agnostic `CiPipeline` data model (CI-14)
//! - `templates`— Per-platform YAML assemblers (CI-11, CI-12, CI-13)

pub mod context;
pub mod schema;
pub mod templates;
