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
    get_default_machine_type, get_default_region, get_machine_types_for_provider,
    get_regions_for_provider, CloudRegion, MachineType,
    // Dynamic Hetzner availability functions for agent use
    get_hetzner_regions_dynamic, get_hetzner_server_types_dynamic,
    check_hetzner_availability, get_recommended_server_type,
    find_best_region, find_cheapest_available,
    DynamicCloudRegion, DynamicMachineType, HetznerFetchResult,
};
pub use cluster_selection::{select_cluster, ClusterSelectionResult};
pub use config_form::{
    collect_config, collect_env_vars, discover_env_files, parse_env_file, ConfigFormResult,
    EnvFileEntry,
};
pub use dockerfile_selection::{select_dockerfile, DockerfileSelectionResult};
pub use environment_creation::{create_environment_wizard, EnvironmentCreationResult};
pub use environment_selection::{select_environment, EnvironmentSelectionResult};
pub use infrastructure_selection::{
    select_infrastructure, select_infrastructure_sync, InfrastructureSelectionResult,
};
pub use orchestrator::{run_wizard, WizardResult};
pub use provider_selection::{
    get_provider_deployment_statuses, select_provider, ProviderSelectionResult,
};
pub use registry_provisioning::{provision_registry, RegistryProvisioningResult};
pub use registry_selection::{select_registry, RegistrySelectionResult};
pub use repository_selection::{select_repository, RepositorySelectionResult};
pub use recommendations::{
    recommend_deployment, DeploymentRecommendation, MachineOption, ProviderOption,
    RecommendationAlternatives, RecommendationInput, RegionOption,
};
pub use render::{count_badge, display_step_header, status_indicator, wizard_render_config};
pub use service_endpoints::{
    collect_network_endpoint_env_vars, collect_service_endpoint_env_vars,
    extract_network_endpoints, filter_endpoints_for_provider, get_available_endpoints,
    match_env_vars_to_services, AvailableServiceEndpoint, EndpointSuggestion, MatchConfidence,
    NetworkEndpointInfo,
};
pub use target_selection::{select_target, TargetSelectionResult};
