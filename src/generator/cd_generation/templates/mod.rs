//! CD Template Builders
//!
//! Each submodule assembles a final GitHub Actions workflow YAML file for
//! a specific cloud platform by stitching together the step snippets
//! produced by the `auth_*`, `registry`, `deploy_*`, `migration`, and
//! `health_check` modules.
//!
//! - `azure`   — `.github/workflows/deploy-azure.yml`   (CD-18)
//! - `gcp`     — `.github/workflows/deploy-gcp.yml`     (CD-19)
//! - `hetzner` — `.github/workflows/deploy-hetzner.yml` (CD-20)

pub mod azure;
pub mod gcp;
pub mod hetzner;
