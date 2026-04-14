//! CD Pipeline Generation Module
//!
//! Generates CD (Continuous Deployment) pipeline skeletons from project
//! analysis. Mirrors the CI generation architecture: context collection →
//! schema assembly → template rendering → file writing.
//!
//! ## Submodules
//!
//! - `context`         — `CdContext` struct and context collector (CD-02)
//! - `schema`          — Platform-agnostic `CdPipeline` data model (CD-17)
//! - `token_resolver`  — Two-pass placeholder token engine for CD (CD-15 adapted)
//! - `manifest`        — `cd-manifest.toml` writer (CD-22)
//! - `registry`        — Container registry login steps + image tag strategy (CD-03)
//! - `auth_azure`      — Azure OIDC authentication step (CD-04)
//! - `auth_gcp`        — GCP Workload Identity Federation auth step (CD-05)
//! - `auth_hetzner`    — Hetzner SSH / kubeconfig auth step (CD-06)
//! - `deploy_azure`    — Azure deploy steps: App Service, AKS, Container Apps (CD-07)
//! - `deploy_gcp`      — GCP deploy steps: Cloud Run, GKE (CD-08)
//! - `deploy_hetzner`  — Hetzner deploy steps: VPS, HetznerK8s, Coolify (CD-09)
//! - `migration`       — Database migration step generator (CD-10)
//! - `health_check`    — Post-deploy health check step (CD-11)
//! - `templates`       — Full workflow YAML builders: Azure, GCP, Hetzner (CD-18/19/20)
//! - `writer`          — CD file writer with conflict detection

pub mod auth_azure;
pub mod auth_gcp;
pub mod auth_hetzner;
pub mod context;
pub mod deploy_azure;
pub mod deploy_gcp;
pub mod deploy_hetzner;
pub mod health_check;
pub mod manifest;
pub mod migration;
pub mod pipeline;
pub mod registry;
pub mod schema;
pub mod templates;
pub mod token_resolver;
pub mod writer;
