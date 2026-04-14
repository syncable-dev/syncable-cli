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
//! - `registry`       — Container registry login steps + image tag strategy (CD-03)
//! - `auth_azure`     — Azure OIDC authentication step (CD-04)
//! - `auth_gcp`       — GCP Workload Identity Federation auth step (CD-05)
//! - `auth_hetzner`   — Hetzner SSH / kubeconfig auth step (CD-06)

pub mod auth_azure;
pub mod auth_gcp;
pub mod auth_hetzner;
pub mod context;
pub mod manifest;
pub mod registry;
pub mod schema;
pub mod token_resolver;
