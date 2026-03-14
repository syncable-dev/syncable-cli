//! CI/CD Template Builders
//!
//! Each submodule assembles a final YAML file for a specific platform
//! by rendering a `CiPipeline` schema into the target format.
//!
//! - `github_actions`  — `.github/workflows/ci.yml`   (CI-11)
//! - `azure_pipelines` — `azure-pipelines.yml`         (CI-12)
//! - `cloud_build`     — `cloudbuild.yaml`             (CI-13)

pub mod azure_pipelines;
pub mod cloud_build;
pub mod github_actions;
