//! Interactive deployment wizard for configuring new services
//!
//! Provides a step-by-step TUI wizard for deploying services to the Syncable platform.

mod cluster_selection;
mod config_form;
mod environment_creation;
mod orchestrator;
mod provider_selection;
mod registry_provisioning;
mod registry_selection;
mod render;
mod target_selection;

pub use cluster_selection::{select_cluster, ClusterSelectionResult};
pub use config_form::{collect_config, ConfigFormResult};
pub use environment_creation::{create_environment_wizard, EnvironmentCreationResult};
pub use orchestrator::{run_wizard, WizardResult};
pub use provider_selection::{
    get_provider_deployment_statuses, select_provider, ProviderSelectionResult,
};
pub use registry_provisioning::{provision_registry, RegistryProvisioningResult};
pub use registry_selection::{select_registry, RegistrySelectionResult};
pub use render::{count_badge, display_step_header, status_indicator, wizard_render_config};
pub use target_selection::{select_target, TargetSelectionResult};
