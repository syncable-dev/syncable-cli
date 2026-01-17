//! Interactive deployment wizard for configuring new services
//!
//! Provides a step-by-step TUI wizard for deploying services to the Syncable platform.

mod provider_selection;
mod render;

pub use provider_selection::{
    get_provider_deployment_statuses, select_provider, ProviderSelectionResult,
};
pub use render::{count_badge, display_step_header, status_indicator, wizard_render_config};
