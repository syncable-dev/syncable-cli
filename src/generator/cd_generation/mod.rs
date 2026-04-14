//! CD Pipeline Generation Module
//!
//! Generates CD (Continuous Deployment) pipeline skeletons from project
//! analysis. Mirrors the CI generation architecture: context collection →
//! schema assembly → template rendering → file writing.
//!
//! ## Submodules
//!
//! - `context`        — `CdContext` struct and context collector (CD-02)
//! - `schema`         — Platform-agnostic `CdPipeline` data model (CD-17)
//! - `token_resolver` — Two-pass placeholder token engine for CD (CD-15 adapted)
//! - `manifest`       — `cd-manifest.toml` writer (CD-22)

pub mod context;
pub mod manifest;
pub mod schema;
pub mod token_resolver;
