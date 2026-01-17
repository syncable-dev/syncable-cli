//! Interactive deployment wizard for configuring new services
//!
//! Provides a step-by-step TUI wizard for deploying services to the Syncable platform.

mod cluster_selection;
mod config_form;
mod orchestrator;
mod provider_selection;
mod registry_selection;
mod render;
mod target_selection;

pub use cluster_selection::{select_cluster, ClusterSelectionResult};
pub use config_form::{collect_config, ConfigFormResult};
pub use orchestrator::{run_wizard, WizardResult};
pub use provider_selection::{
    get_provider_deployment_statuses, select_provider, ProviderSelectionResult,
};
pub use registry_selection::{select_registry, RegistrySelectionResult};
pub use render::{count_badge, display_step_header, status_indicator, wizard_render_config};
pub use target_selection::{select_target, TargetSelectionResult};
