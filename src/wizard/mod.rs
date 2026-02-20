//! Interactive deployment wizard for configuring new services
//!
//! Provides a step-by-step TUI wizard for deploying services to the Syncable platform.

mod cloud_provider_data;
mod cluster_selection;
mod config_form;
mod dockerfile_selection;
mod environment_creation;
mod environment_selection;
mod infrastructure_selection;
mod orchestrator;
mod provider_selection;
pub mod recommendations;
mod registry_provisioning;
mod registry_selection;
mod render;
mod repository_selection;
mod service_endpoints;
mod target_selection;

pub use cloud_provider_data::{
    CloudRegion,
    DynamicCloudRegion,
    DynamicMachineType,
    HetznerFetchResult,
    MachineType,
    check_hetzner_availability,
    find_best_region,
    find_cheapest_available,
    get_default_machine_type,
    get_default_region,
    // Dynamic Hetzner availability functions for agent use
    get_hetzner_regions_dynamic,
    get_hetzner_server_types_dynamic,
    get_machine_types_for_provider,
    get_recommended_server_type,
    get_regions_for_provider,
};
pub use cluster_selection::{ClusterSelectionResult, select_cluster};
pub use config_form::{
    ConfigFormResult, EnvFileEntry, collect_config, collect_env_vars, discover_env_files,
    parse_env_file,
};
pub use dockerfile_selection::{DockerfileSelectionResult, select_dockerfile};
pub use environment_creation::{EnvironmentCreationResult, create_environment_wizard};
pub use environment_selection::{EnvironmentSelectionResult, select_environment};
pub use infrastructure_selection::{
    InfrastructureSelectionResult, select_infrastructure, select_infrastructure_sync,
};
pub use orchestrator::{WizardResult, run_wizard};
pub use provider_selection::{
    ProviderSelectionResult, get_provider_deployment_statuses, select_provider,
};
pub use recommendations::{
    DeploymentRecommendation, MachineOption, ProviderOption, RecommendationAlternatives,
    RecommendationInput, RegionOption, recommend_deployment,
};
pub use registry_provisioning::{RegistryProvisioningResult, provision_registry};
pub use registry_selection::{RegistrySelectionResult, select_registry};
pub use render::{count_badge, display_step_header, status_indicator, wizard_render_config};
pub use repository_selection::{RepositorySelectionResult, select_repository};
pub use service_endpoints::{
    AvailableServiceEndpoint, EndpointSuggestion, MatchConfidence, NetworkEndpointInfo,
    collect_network_endpoint_env_vars, collect_service_endpoint_env_vars,
    extract_network_endpoints, filter_endpoints_for_provider, get_available_endpoints,
    match_env_vars_to_services,
};
pub use target_selection::{TargetSelectionResult, select_target};
