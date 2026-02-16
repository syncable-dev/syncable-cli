//! Deploy service tool for the agent
//!
//! A compound tool that enables conversational deployment with intelligent recommendations.
//! Analyzes the project, provides recommendations with reasoning, and executes deployment.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::str::FromStr;

use crate::agent::tools::ExecutionContext;
use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::analyzer::{AnalysisConfig, TechnologyCategory, analyze_project_with_config};
use crate::platform::api::types::{
    CloudProvider, CloudRunnerConfigInput, CreateDeploymentConfigRequest, DeploymentSecretInput,
    ProjectRepository, build_cloud_runner_config_v2,
};

use super::set_secrets::{SecretPromptResult, default_true, prompt_secret_value};
use crate::platform::api::{PlatformApiClient, PlatformApiError, TriggerDeploymentRequest};
use crate::platform::PlatformSession;
use crate::wizard::{
    RecommendationInput, recommend_deployment, get_provider_deployment_statuses,
    get_hetzner_regions_dynamic, get_hetzner_server_types_dynamic, HetznerFetchResult,
    DynamicCloudRegion, DynamicMachineType, discover_env_files, parse_env_file,
    get_available_endpoints, filter_endpoints_for_provider, match_env_vars_to_services,
    extract_network_endpoints,
};
use std::process::Command;

/// Cached Hetzner availability data for smart recommendations
struct HetznerAvailabilityData {
    regions: Vec<DynamicCloudRegion>,
    server_types: Vec<DynamicMachineType>,
}

/// A single secret/env var key input for the deploy tool
#[derive(Debug, Deserialize)]
pub struct SecretKeyInput {
    /// Environment variable name
    pub key: String,
    /// Value to set. OMIT for secrets — user will be prompted in terminal.
    pub value: Option<String>,
    /// Whether this is a secret (default: true for safety)
    #[serde(default = "default_true")]
    pub is_secret: bool,
}

/// Arguments for the deploy service tool
#[derive(Debug, Deserialize)]
pub struct DeployServiceArgs {
    /// Optional: specific subdirectory/service to deploy (for monorepos)
    pub path: Option<String>,
    /// Optional: override recommended provider (gcp, hetzner, azure)
    pub provider: Option<String>,
    /// Optional: override machine type selection
    pub machine_type: Option<String>,
    /// Optional: override region selection
    pub region: Option<String>,
    /// Optional: override detected port
    pub port: Option<u16>,
    /// Whether to make the service publicly accessible (default: false for safety)
    /// Internal services can only be accessed within the cluster/network
    #[serde(default)]
    pub is_public: bool,
    /// Optional: CPU allocation (for GCP Cloud Run / Azure ACA)
    pub cpu: Option<String>,
    /// Optional: Memory allocation (for GCP Cloud Run / Azure ACA)
    pub memory: Option<String>,
    /// Optional: min instances/replicas
    pub min_instances: Option<i32>,
    /// Optional: max instances/replicas
    pub max_instances: Option<i32>,
    /// If true (default), show recommendation but don't deploy yet
    /// If false with settings, deploy immediately
    #[serde(default = "default_preview")]
    pub preview_only: bool,
    /// Optional: environment variable keys to set during deployment.
    /// For secrets (is_secret=true), values are collected via terminal prompt.
    /// For non-secrets, include the value directly.
    pub secret_keys: Option<Vec<SecretKeyInput>>,
}

fn default_preview() -> bool {
    true
}

/// Error type for deploy service operations
#[derive(Debug, thiserror::Error)]
#[error("Deploy service error: {0}")]
pub struct DeployServiceError(String);

/// Tool to analyze a project and deploy it with intelligent recommendations
///
/// Provides an end-to-end deployment experience:
/// 1. Analyzes the project (language, framework, ports, health endpoints)
/// 2. Checks available deployment capabilities
/// 3. Generates smart recommendations with reasoning
/// 4. Shows a preview for user confirmation
/// 5. Creates deployment config and triggers deployment
#[derive(Debug, Clone)]
pub struct DeployServiceTool {
    project_path: PathBuf,
    execution_context: ExecutionContext,
}

impl DeployServiceTool {
    /// Create a new DeployServiceTool (defaults to InteractiveCli)
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            execution_context: ExecutionContext::InteractiveCli,
        }
    }

    /// Create with explicit execution context
    pub fn with_context(project_path: PathBuf, ctx: ExecutionContext) -> Self {
        Self {
            project_path,
            execution_context: ctx,
        }
    }
}

impl Tool for DeployServiceTool {
    const NAME: &'static str = "deploy_service";

    type Error = DeployServiceError;
    type Args = DeployServiceArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Analyze a project and deploy it with intelligent recommendations.

This tool provides an end-to-end deployment experience:
1. Analyzes the project to detect language, framework, ports, and health endpoints
2. Checks available deployment capabilities (providers, clusters, registries)
3. Generates smart recommendations with reasoning
4. Shows a preview for user confirmation
5. Creates deployment config and triggers deployment

**Default behavior (preview_only=true):**
Returns analysis and recommendations. User should confirm before actual deployment.

**Direct deployment (preview_only=false):**
Uses provided overrides or recommendation defaults to deploy immediately.

**Parameters:**
- path: Optional subdirectory for monorepo services
- provider: Override recommendation (gcp, hetzner, azure)
- machine_type: Override machine selection (e.g., cx22, e2-small)
- region: Override region selection (e.g., nbg1, us-central1)
- port: Override detected port
- is_public: Whether service should be publicly accessible (default: false)
- preview_only: If true (default), show recommendation only

**IMPORTANT - Public vs Internal:**
- is_public=false (default): Service is internal-only, not accessible from internet
- is_public=true: Service gets a public URL, accessible from anywhere
- ALWAYS show this in the preview and ask user before deploying public services

**What it analyzes:**
- Programming language and framework
- Port configuration from source code, package.json, Dockerfiles
- Health check endpoints (/health, /healthz, etc.)
- Existing infrastructure (K8s manifests, Helm charts)

**Recommendation reasoning includes:**
- Why a specific provider was chosen
- Why a machine type fits the workload (based on memory requirements)
- Where the port was detected from
- Confidence level in the recommendation

**Example flow:**
User: "deploy this service"
1. Call with preview_only=true → Shows recommendation
2. User: "yes, deploy it" → Call with preview_only=false to deploy
3. User: "make it public" → Call with preview_only=true AND is_public=true to show NEW preview
4. User: "yes" → NOW call with preview_only=false to deploy

**CRITICAL - Human in the loop:**
- NEVER deploy (preview_only=false) immediately after user requests a CHANGE
- If user says "make it public", "use GCP", "change region", etc. → show NEW preview first
- Only deploy after user explicitly confirms the final settings with "yes", "deploy", "confirm"
- A change request is NOT a deployment confirmation

**Multiple cloud providers:**
- The response includes connected_providers listing ALL connected providers (e.g. Hetzner AND Azure)
- ALWAYS mention all connected providers to the user, not just the recommended one
- The user can override the provider with the provider parameter
- If deploying related services, consider whether they should be on the same provider for private networking

**Deployed service endpoints:**
- The response includes deployed_service_endpoints showing services already running in the project
- Services may have public URLs (reachable from anywhere) or private IPs (only reachable from the same cloud provider network)
- endpoint_suggestions maps detected env vars to deployed services (e.g. SENTIMENT_SERVICE_URL -> sentiment-analysis)
- Private endpoints are pre-filtered to only show services on the same provider network
- ALWAYS mention available endpoints when deploying services that have env vars matching deployed services

**Private networks (project_networks):**
- The response includes project_networks showing provisioned VPCs/networks for the target provider
- Each network includes connection_details with key/value pairs (VPC_ID, SUBNET_ID, DEFAULT_DOMAIN, etc.)
- If networks have useful connection details (e.g., a default domain, VPC connector), mention them to the user
- Ask the user if they want to inject any network details as environment variables
- Network details are NOT secrets — they are infrastructure identifiers
- Private networks enable service-to-service communication on the same provider

**Environment variables (secret_keys) and .env files:**
- The preview response includes parsed_env_files: discovered .env files with their parsed keys/values
- If .env files are found, ALWAYS ask the user: "I found a .env file with N variables. Should I inject these into the deployment?"
- For non-secret vars from .env files, pass them as secret_keys with is_secret=false and include the value
- For secret vars (API keys, tokens, passwords), pass them as secret_keys with is_secret=true and omit the value — the user is prompted securely in the terminal
- Secret values from .env files are NEVER included in parsed_env_files or this conversation
- If no .env files found but detected_env_vars exist, mention those and ask user how to provide them
- NEVER ask the user to type secret values in chat

**Prerequisites:**
- User must be authenticated (sync-ctl auth login)
- A project must be selected (use select_project first)
- Provider must be connected (check with list_deployment_capabilities)"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Subdirectory to deploy (for monorepos)"
                    },
                    "provider": {
                        "type": "string",
                        "enum": ["gcp", "hetzner", "azure"],
                        "description": "Override: cloud provider"
                    },
                    "machine_type": {
                        "type": "string",
                        "description": "Override: machine type (e.g., cx22, e2-small)"
                    },
                    "region": {
                        "type": "string",
                        "description": "Override: deployment region"
                    },
                    "port": {
                        "type": "integer",
                        "description": "Override: port to expose"
                    },
                    "is_public": {
                        "type": "boolean",
                        "description": "Whether service should be publicly accessible. Default: false (internal only). Set to true for public URL."
                    },
                    "preview_only": {
                        "type": "boolean",
                        "description": "If true (default), show recommendation only. If false, deploy."
                    },
                    "secret_keys": {
                        "type": "array",
                        "description": "Env vars to include in deployment. For secrets, omit value \u{2014} user is prompted in terminal.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "key": {
                                    "type": "string",
                                    "description": "Environment variable name"
                                },
                                "value": {
                                    "type": "string",
                                    "description": "Omit for secrets \u{2014} user will be prompted securely in terminal."
                                },
                                "is_secret": {
                                    "type": "boolean",
                                    "description": "Whether this is a secret (default: true). Secrets are prompted in terminal.",
                                    "default": true
                                }
                            },
                            "required": ["key"]
                        }
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 1. Determine analysis path
        let analysis_path = if let Some(ref subpath) = args.path {
            self.project_path.join(subpath)
        } else {
            self.project_path.clone()
        };

        // Validate path exists
        if !analysis_path.exists() {
            return Ok(format_error_for_llm(
                "deploy_service",
                ErrorCategory::FileNotFound,
                &format!("Path not found: {}", analysis_path.display()),
                Some(vec!["Check if the path exists", "Use list_directory to explore"]),
            ));
        }

        // 2. Run project analysis
        let config = AnalysisConfig {
            deep_analysis: true,
            ..Default::default()
        };

        let analysis = match analyze_project_with_config(&analysis_path, &config) {
            Ok(a) => a,
            Err(e) => {
                return Ok(format_error_for_llm(
                    "deploy_service",
                    ErrorCategory::InternalError,
                    &format!("Analysis failed: {}", e),
                    Some(vec!["Check if the directory contains a valid project"]),
                ));
            }
        };

        // 3. Get API client and context
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(_) => {
                return Ok(format_error_for_llm(
                    "deploy_service",
                    ErrorCategory::PermissionDenied,
                    "Not authenticated",
                    Some(vec!["Run: sync-ctl auth login"]),
                ));
            }
        };

        // Load platform session for context
        let session = match PlatformSession::load() {
            Ok(s) => s,
            Err(_) => {
                return Ok(format_error_for_llm(
                    "deploy_service",
                    ErrorCategory::InternalError,
                    "Failed to load platform session",
                    Some(vec!["Try selecting a project with select_project"]),
                ));
            }
        };

        if !session.is_project_selected() {
            return Ok(format_error_for_llm(
                "deploy_service",
                ErrorCategory::ValidationFailed,
                "No project selected",
                Some(vec!["Use select_project to choose a project first"]),
            ));
        }

        let project_id = session.project_id.clone().unwrap_or_default();
        let environment_id = session.environment_id.clone();

        // 4. Check for existing deployment configs (duplicate detection)
        let existing_configs = match client.list_deployment_configs(&project_id).await {
            Ok(configs) => configs,
            Err(e) => {
                // Non-fatal - continue without duplicate detection
                tracing::warn!("Failed to fetch existing configs: {}", e);
                Vec::new()
            }
        };

        // Get service name early to check for duplicates
        let service_name = get_service_name(&analysis_path);

        // Find existing config with same service name
        let existing_config = existing_configs
            .iter()
            .find(|c| c.service_name.eq_ignore_ascii_case(&service_name));

        // 5. Get environment info for display
        let environments = match client.list_environments(&project_id).await {
            Ok(envs) => envs,
            Err(_) => Vec::new(),
        };

        // Resolve environment name for display
        let (resolved_env_id, resolved_env_name, is_production) = if let Some(ref env_id) = environment_id {
            let env = environments.iter().find(|e| e.id == *env_id);
            let name = env.map(|e| e.name.clone()).unwrap_or_else(|| "Unknown".to_string());
            let is_prod = name.to_lowercase().contains("prod");
            (env_id.clone(), name, is_prod)
        } else if let Some(existing) = &existing_config {
            // Use the environment from existing config
            let env = environments.iter().find(|e| e.id == existing.environment_id);
            let name = env.map(|e| e.name.clone()).unwrap_or_else(|| "Unknown".to_string());
            let is_prod = name.to_lowercase().contains("prod");
            (existing.environment_id.clone(), name, is_prod)
        } else if let Some(first_env) = environments.first() {
            let is_prod = first_env.name.to_lowercase().contains("prod");
            (first_env.id.clone(), first_env.name.clone(), is_prod)
        } else {
            ("".to_string(), "No environment".to_string(), false)
        };

        // 6. Get available providers
        let capabilities = match get_provider_deployment_statuses(&client, &project_id).await {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_error_for_llm(
                    "deploy_service",
                    ErrorCategory::NetworkError,
                    &format!("Failed to get deployment capabilities: {}", e),
                    None,
                ));
            }
        };

        // Check if any provider is available
        let available_providers: Vec<CloudProvider> = capabilities
            .iter()
            .filter(|s| s.provider.is_available() && s.is_connected)
            .map(|s| s.provider.clone())
            .collect();

        if available_providers.is_empty() {
            return Ok(format_error_for_llm(
                "deploy_service",
                ErrorCategory::ResourceUnavailable,
                "No cloud providers connected",
                Some(vec![
                    "Connect a cloud provider (GCP, Hetzner, or Azure) in platform settings",
                    "Use open_provider_settings to configure a provider",
                ]),
            ));
        }

        // 5. Check for existing K8s clusters
        let has_existing_k8s = capabilities.iter().any(|s| !s.clusters.is_empty());

        // 6. Generate recommendation
        let recommendation_input = RecommendationInput {
            analysis: analysis.clone(),
            available_providers: available_providers.clone(),
            has_existing_k8s,
            user_region_hint: args.region.clone(),
        };

        let recommendation = recommend_deployment(recommendation_input);

        // 6.5. For Hetzner deployments, fetch real-time availability and update recommendations
        // We require real-time data - no static fallback allowed
        let final_provider_for_check = args.provider
            .as_ref()
            .and_then(|p| CloudProvider::from_str(p).ok())
            .unwrap_or(recommendation.provider.clone());

        // Store Hetzner availability data for smart recommendations
        let mut hetzner_availability: Option<HetznerAvailabilityData> = None;

        if final_provider_for_check == CloudProvider::Hetzner {
            // Fetch real-time Hetzner regions and server types
            let regions = match get_hetzner_regions_dynamic(&client, &project_id).await {
                HetznerFetchResult::Success(r) if !r.is_empty() => r,
                HetznerFetchResult::Success(_) => {
                    return Ok(format_error_for_llm(
                        "deploy_service",
                        ErrorCategory::ResourceUnavailable,
                        "No Hetzner regions available",
                        Some(vec![
                            "Check your Hetzner account status",
                            "Use list_hetzner_availability to see current availability",
                        ]),
                    ));
                }
                HetznerFetchResult::NoCredentials => {
                    return Ok(format_error_for_llm(
                        "deploy_service",
                        ErrorCategory::PermissionDenied,
                        "Cannot recommend Hetzner deployment: Hetzner credentials not configured",
                        Some(vec![
                            "Add your Hetzner API token in project settings",
                            "Use open_provider_settings to configure Hetzner",
                            "Or specify a different provider (e.g., provider='gcp')",
                        ]),
                    ));
                }
                HetznerFetchResult::ApiError(err) => {
                    return Ok(format_error_for_llm(
                        "deploy_service",
                        ErrorCategory::NetworkError,
                        &format!("Cannot recommend Hetzner deployment: Failed to fetch availability - {}", err),
                        Some(vec![
                            "Use list_hetzner_availability to check current status",
                            "Or specify a different provider (e.g., provider='gcp')",
                        ]),
                    ));
                }
            };

            // Fetch server types with optional location filter
            let server_types = match get_hetzner_server_types_dynamic(&client, &project_id, args.region.as_deref()).await {
                HetznerFetchResult::Success(s) if !s.is_empty() => s,
                HetznerFetchResult::Success(_) => {
                    return Ok(format_error_for_llm(
                        "deploy_service",
                        ErrorCategory::ResourceUnavailable,
                        "No Hetzner server types available",
                        Some(vec![
                            "Check your Hetzner account status",
                            "Use list_hetzner_availability to see current availability",
                        ]),
                    ));
                }
                HetznerFetchResult::NoCredentials => {
                    return Ok(format_error_for_llm(
                        "deploy_service",
                        ErrorCategory::PermissionDenied,
                        "Cannot recommend Hetzner deployment: Hetzner credentials not configured",
                        Some(vec![
                            "Add your Hetzner API token in project settings",
                            "Use open_provider_settings to configure Hetzner",
                        ]),
                    ));
                }
                HetznerFetchResult::ApiError(err) => {
                    return Ok(format_error_for_llm(
                        "deploy_service",
                        ErrorCategory::NetworkError,
                        &format!("Cannot recommend Hetzner deployment: Failed to fetch server types - {}", err),
                        Some(vec!["Use list_hetzner_availability to check current status"]),
                    ));
                }
            };

            // Store for later use in recommendations
            hetzner_availability = Some(HetznerAvailabilityData { regions, server_types });
        }

        // 7. Extract analysis summary
        let primary_language = analysis.languages.first()
            .map(|l| l.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let primary_framework = analysis.technologies.iter()
            .find(|t| matches!(t.category, TechnologyCategory::BackendFramework | TechnologyCategory::MetaFramework))
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "None detected".to_string());

        let has_dockerfile = analysis.docker_analysis
            .as_ref()
            .map(|d| !d.dockerfiles.is_empty())
            .unwrap_or(false);

        let has_k8s = analysis.infrastructure
            .as_ref()
            .map(|i| i.has_kubernetes)
            .unwrap_or(false);

        // 10. If preview_only, return recommendation
        if args.preview_only {
            // Build the deployment mode info
            let (deployment_mode, mode_explanation, next_steps) = if let Some(existing) = &existing_config {
                (
                    "REDEPLOY",
                    format!(
                        "Service '{}' already has a deployment config (ID: {}). Deploying will trigger a REDEPLOY of the existing service.",
                        existing.service_name, existing.id
                    ),
                    vec![
                        "To redeploy with current config: call deploy_service with preview_only=false".to_string(),
                        "This will trigger a new deployment of the existing service".to_string(),
                        "The existing configuration will be used".to_string(),
                    ]
                )
            } else {
                (
                    "NEW_DEPLOYMENT",
                    format!(
                        "No existing deployment config found for '{}'. This will create a NEW deployment configuration.",
                        service_name
                    ),
                    vec![
                        "To deploy with these settings: call deploy_service with preview_only=false".to_string(),
                        "To customize: specify provider, machine_type, region, or port parameters".to_string(),
                        "Check parsed_env_files — if .env files were found, ask user whether to inject them as secret_keys".to_string(),
                        "To see more options: check the hetzner_availability section for current pricing".to_string(),
                    ]
                )
            };

            // Production warning
            let production_warning = if is_production {
                Some("⚠️  WARNING: This will deploy to PRODUCTION environment. Please confirm you intend to deploy to production.")
            } else {
                None
            };

            // For Hetzner, use real-time availability to select best options
            let (final_machine_type, final_region, machine_reasoning, region_reasoning, price_monthly) =
                if let Some(ref hetzner) = hetzner_availability {
                    // SMART SELECTION: Find the best region + machine combination
                    // Strategy: Find cheapest machine with 4GB+ that's actually available somewhere

                    // First, find all server types that are actually available (non-empty available_in)
                    let available_types: Vec<_> = hetzner.server_types.iter()
                        .filter(|st| !st.available_in.is_empty())
                        .collect();

                    // If user specified a region, check if anything is available there
                    let user_region = args.region.as_deref();

                    // Find best machine: cheapest with 4GB+ that's available
                    let best_machine_with_region = if let Some(region) = user_region {
                        // User specified region - find best machine for that region
                        available_types.iter()
                            .filter(|st| st.memory_gb >= 4.0 && st.available_in.contains(&region.to_string()))
                            .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
                            .map(|st| (*st, region.to_string()))
                            .or_else(|| {
                                // No 4GB+ available in that region, try any machine
                                available_types.iter()
                                    .filter(|st| st.available_in.contains(&region.to_string()))
                                    .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
                                    .map(|st| (*st, region.to_string()))
                            })
                    } else {
                        // No region specified - find globally cheapest 4GB+ machine and use its best region
                        available_types.iter()
                            .filter(|st| st.memory_gb >= 4.0)
                            .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
                            .map(|st| {
                                // Pick the first available region for this machine
                                let region = st.available_in.first()
                                    .cloned()
                                    .unwrap_or_else(|| "nbg1".to_string());
                                (*st, region)
                            })
                            .or_else(|| {
                                // No 4GB+ available anywhere, find any cheapest machine
                                available_types.iter()
                                    .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
                                    .map(|st| {
                                        let region = st.available_in.first()
                                            .cloned()
                                            .unwrap_or_else(|| "nbg1".to_string());
                                        (*st, region)
                                    })
                            })
                    };

                    if let Some((machine, region_id)) = best_machine_with_region {
                        let region_name = hetzner.regions.iter()
                            .find(|r| r.id == region_id)
                            .map(|r| format!("{}, {}", r.name, r.location))
                            .unwrap_or_else(|| region_id.clone());

                        let available_count = hetzner.regions.iter()
                            .find(|r| r.id == region_id)
                            .map(|r| r.available_server_types.len())
                            .unwrap_or(0);

                        (
                            args.machine_type.clone().unwrap_or_else(|| machine.id.clone()),
                            region_id.clone(),
                            format!(
                                "Selected {} ({} vCPU, {:.0} GB RAM) - cheapest AVAILABLE option at €{:.2}/mo",
                                machine.id, machine.cores, machine.memory_gb, machine.price_monthly
                            ),
                            format!("Selected {} ({}) - {} server types available", region_id, region_name, available_count),
                            Some(machine.price_monthly),
                        )
                    } else {
                        // No server types available anywhere - this shouldn't happen if we passed validation
                        (
                            args.machine_type.clone().unwrap_or_else(|| recommendation.machine_type.clone()),
                            args.region.clone().unwrap_or_else(|| recommendation.region.clone()),
                            "WARNING: No server types currently available - using fallback".to_string(),
                            "Using fallback region".to_string(),
                            None,
                        )
                    }
                } else {
                    // Non-Hetzner provider - use static recommendation
                    (
                        args.machine_type.clone().unwrap_or_else(|| recommendation.machine_type.clone()),
                        args.region.clone().unwrap_or_else(|| recommendation.region.clone()),
                        recommendation.machine_reasoning.clone(),
                        recommendation.region_reasoning.clone(),
                        None,
                    )
                };

            // Build availability info for response
            let hetzner_availability_info = hetzner_availability.as_ref().map(|h| {
                json!({
                    "regions": h.regions.iter().map(|r| json!({
                        "id": r.id,
                        "name": r.name,
                        "country": r.location,
                        "available_server_types_count": r.available_server_types.len(),
                    })).collect::<Vec<_>>(),
                    "server_types": h.server_types.iter().take(10).map(|st| json!({
                        "id": st.id,
                        "cores": st.cores,
                        "memory_gb": st.memory_gb,
                        "price_monthly_eur": st.price_monthly,
                        "available_in": st.available_in,
                    })).collect::<Vec<_>>(),
                    "cheapest_4gb": h.server_types.iter()
                        .filter(|st| st.memory_gb >= 4.0)
                        .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
                        .map(|st| json!({
                            "id": st.id,
                            "specs": format!("{} vCPU, {:.0} GB RAM", st.cores, st.memory_gb),
                            "price_monthly_eur": st.price_monthly,
                        })),
                })
            });

            // Discover .env files and parse their contents for agent surfacing
            let discovered_env_files_raw = discover_env_files(&analysis_path);
            let discovered_env_file_paths: Vec<String> = discovered_env_files_raw
                .iter()
                .map(|p| p.display().to_string())
                .collect();

            // Parse each .env file so the LLM can present keys to the user
            let parsed_env_files: Vec<serde_json::Value> = discovered_env_files_raw
                .iter()
                .filter_map(|rel_path| {
                    let abs_path = analysis_path.join(rel_path);
                    match parse_env_file(&abs_path) {
                        Ok(entries) if !entries.is_empty() => Some(json!({
                            "file": rel_path.display().to_string(),
                            "variable_count": entries.len(),
                            "variables": entries.iter().map(|e| json!({
                                "key": e.key,
                                "is_secret": e.is_secret,
                                // Only include values for non-secret vars — secrets are
                                // never exposed to the LLM conversation.
                                "value": if e.is_secret { None } else { Some(&e.value) },
                            })).collect::<Vec<_>>(),
                        })),
                        Ok(_) => None, // empty file
                        Err(e) => {
                            tracing::debug!("Could not parse env file {:?}: {}", rel_path, e);
                            None
                        }
                    }
                })
                .collect();

            // Fetch deployed services and compute endpoint suggestions
            let deployed_endpoints = match client.list_deployments(&project_id, Some(50)).await {
                Ok(paginated) => get_available_endpoints(&paginated.data),
                Err(e) => {
                    tracing::debug!("Could not fetch deployments for endpoint matching: {}", e);
                    Vec::new()
                }
            };
            let deployed_endpoints: Vec<_> = deployed_endpoints
                .into_iter()
                .filter(|ep| ep.service_name != service_name)
                .collect();
            // Only show private endpoints from the same cloud provider — private
            // IPs are not reachable across different provider networks.
            let deployed_endpoints = filter_endpoints_for_provider(
                deployed_endpoints,
                final_provider_for_check.as_str(),
            );

            let detected_env_var_names: Vec<String> = analysis
                .environment_variables
                .iter()
                .map(|e| e.name.clone())
                .collect();

            let endpoint_suggestions =
                match_env_vars_to_services(&detected_env_var_names, &deployed_endpoints);

            // Fetch project networks for the target provider
            let project_networks = match client.list_project_networks(&project_id).await {
                Ok(nets) => nets,
                Err(e) => {
                    tracing::debug!("Could not fetch project networks: {}", e);
                    Vec::new()
                }
            };

            let network_endpoints = extract_network_endpoints(
                &project_networks,
                final_provider_for_check.as_str(),
                Some(&resolved_env_id),
            );

            let response = json!({
                "status": "recommendation",
                "deployment_mode": deployment_mode,
                "mode_explanation": mode_explanation,
                "environment": {
                    "id": resolved_env_id,
                    "name": resolved_env_name,
                    "is_production": is_production,
                },
                "connected_providers": capabilities.iter()
                    .filter(|s| s.provider.is_available() && s.is_connected)
                    .map(|s| json!({
                        "provider": s.provider.as_str(),
                        "display_name": s.provider.display_name(),
                        "cloud_runner_available": s.cloud_runner_available,
                        "clusters": s.clusters.len(),
                        "registries": s.registries.len(),
                        "summary": s.summary,
                    }))
                    .collect::<Vec<_>>(),
                "production_warning": production_warning,
                "existing_config": existing_config.map(|c| json!({
                    "id": c.id,
                    "service_name": c.service_name,
                    "environment_id": c.environment_id,
                    "branch": c.branch,
                    "port": c.port,
                    "auto_deploy_enabled": c.auto_deploy_enabled,
                    "created_at": c.created_at.to_rfc3339(),
                })),
                "analysis": {
                    "path": analysis_path.display().to_string(),
                    "language": primary_language,
                    "framework": primary_framework,
                    "detected_port": recommendation.port,
                    "port_source": recommendation.port_source,
                    "health_endpoint": recommendation.health_check_path,
                    "has_dockerfile": has_dockerfile,
                    "has_kubernetes": has_k8s,
                    "detected_env_vars": analysis.environment_variables.iter().map(|e| json!({
                        "name": e.name,
                        "required": e.required,
                        "has_default": e.default_value.is_some(),
                        "description": e.description,
                    })).collect::<Vec<_>>(),
                },
                "recommendation": {
                    "provider": recommendation.provider.as_str(),
                    "provider_reasoning": recommendation.provider_reasoning,
                    "target": recommendation.target.as_str(),
                    "target_reasoning": recommendation.target_reasoning,
                    "machine_type": final_machine_type,
                    "machine_reasoning": machine_reasoning,
                    "region": final_region,
                    "region_reasoning": region_reasoning,
                    "price_monthly_eur": price_monthly,
                    "port": recommendation.port,
                    "health_check_path": recommendation.health_check_path,
                    "is_public": args.is_public,
                    "is_public_note": if args.is_public {
                        "Service will be PUBLICLY accessible from the internet"
                    } else {
                        "Service will be INTERNAL only (not accessible from internet)"
                    },
                    "confidence": recommendation.confidence,
                    "availability_source": if hetzner_availability.is_some() { "real-time" } else { "static" },
                },
                "hetzner_availability": hetzner_availability_info,
                "alternatives": {
                    "providers": recommendation.alternatives.providers.iter().map(|p| json!({
                        "provider": p.provider.as_str(),
                        "available": p.available,
                        "reason_if_unavailable": p.reason_if_unavailable,
                    })).collect::<Vec<_>>(),
                    "machine_types": if hetzner_availability.is_some() {
                        // Use real-time data for Hetzner
                        hetzner_availability.as_ref().unwrap().server_types.iter().take(6).map(|st| json!({
                            "machine_type": st.id,
                            "vcpu": st.cores,
                            "memory_gb": st.memory_gb,
                            "price_monthly_eur": st.price_monthly,
                            "available_in": st.available_in,
                        })).collect::<Vec<_>>()
                    } else {
                        recommendation.alternatives.machine_types.iter().map(|m| json!({
                            "machine_type": m.machine_type,
                            "vcpu": m.vcpu,
                            "memory_gb": m.memory_gb,
                            "description": m.description,
                        })).collect::<Vec<_>>()
                    },
                    "regions": if hetzner_availability.is_some() {
                        // Use real-time data for Hetzner
                        hetzner_availability.as_ref().unwrap().regions.iter().map(|r| json!({
                            "region": r.id,
                            "display_name": format!("{}, {}", r.name, r.location),
                            "available_server_types_count": r.available_server_types.len(),
                        })).collect::<Vec<_>>()
                    } else {
                        recommendation.alternatives.regions.iter().map(|r| json!({
                            "region": r.region,
                            "display_name": r.display_name,
                        })).collect::<Vec<_>>()
                    },
                },
                "service_name": service_name,
                "discovered_env_files": discovered_env_file_paths,
                "parsed_env_files": parsed_env_files,
                "deployed_service_endpoints": deployed_endpoints.iter().map(|ep| json!({
                    "service_name": ep.service_name,
                    "url": ep.url,
                    "is_private": ep.is_private,
                    "status": ep.status,
                })).collect::<Vec<_>>(),
                "endpoint_suggestions": endpoint_suggestions.iter().map(|s| json!({
                    "env_var": s.env_var_name,
                    "service_name": s.service.service_name,
                    "url": s.service.url,
                    "is_private": s.service.is_private,
                    "confidence": format!("{:?}", s.confidence),
                    "reason": s.reason,
                })).collect::<Vec<_>>(),
                "project_networks": network_endpoints.iter().map(|ne| json!({
                    "network_id": ne.network_id,
                    "cloud_provider": ne.cloud_provider,
                    "region": ne.region,
                    "status": ne.status,
                    "environment_id": ne.environment_id,
                    "connection_details": ne.connection_details.iter().map(|(k, v)| json!({
                        "key": k,
                        "value": v,
                        "suggested_env_var": k,
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
                "next_steps": next_steps,
                "confirmation_prompt": if existing_config.is_some() {
                    format!(
                        "REDEPLOY '{}' to {} environment?{}",
                        service_name,
                        resolved_env_name,
                        if is_production { " ⚠️  (PRODUCTION)" } else { "" }
                    )
                } else {
                    let price_info = price_monthly.map(|p| format!(" (€{:.2}/mo)", p)).unwrap_or_default();
                    format!(
                        "Deploy NEW service '{}' to {} ({}) with {}{} in {} on {} environment?{}",
                        service_name,
                        recommendation.provider.display_name(),
                        recommendation.target.display_name(),
                        final_machine_type,
                        price_info,
                        final_region,
                        resolved_env_name,
                        if is_production { " ⚠️  (PRODUCTION)" } else { "" }
                    )
                },
            });

            return serde_json::to_string_pretty(&response)
                .map_err(|e| DeployServiceError(format!("Failed to serialize: {}", e)));
        }

        // 11. Execute deployment - EITHER redeploy existing OR create new

        // If existing config found, trigger redeploy instead of creating new config
        if let Some(existing) = &existing_config {
            let trigger_request = TriggerDeploymentRequest {
                project_id: project_id.clone(),
                config_id: existing.id.clone(),
                commit_sha: None,
            };

            return match client.trigger_deployment(&trigger_request).await {
                Ok(response) => {
                    let result = json!({
                        "status": "redeployed",
                        "deployment_mode": "REDEPLOY",
                        "config_id": existing.id,
                        "task_id": response.backstage_task_id,
                        "service_name": service_name,
                        "environment": {
                            "id": resolved_env_id,
                            "name": resolved_env_name,
                            "is_production": is_production,
                        },
                        "message": format!(
                            "Redeploy triggered for existing service '{}' on {} environment. Task ID: {}",
                            service_name, resolved_env_name, response.backstage_task_id
                        ),
                        "next_steps": [
                            format!("Monitor progress: use get_deployment_status with task_id '{}'", response.backstage_task_id),
                            "View logs after deployment: use get_service_logs",
                        ],
                    });

                    serde_json::to_string_pretty(&result)
                        .map_err(|e| DeployServiceError(format!("Failed to serialize: {}", e)))
                }
                Err(e) => Ok(format_api_error("deploy_service", e)),
            };
        }

        // NEW DEPLOYMENT PATH - no existing config found
        let final_provider = args.provider
            .as_ref()
            .and_then(|p| CloudProvider::from_str(p).ok())
            .unwrap_or(recommendation.provider.clone());

        // For Hetzner, use real-time availability data to select best options
        let (final_machine, final_region) = if let Some(ref hetzner) = hetzner_availability {
            // SMART SELECTION: Same logic as preview

            // Find all server types that are actually available (non-empty available_in)
            let available_types: Vec<_> = hetzner.server_types.iter()
                .filter(|st| !st.available_in.is_empty())
                .collect();

            let user_region = args.region.as_deref();

            // Find best machine: cheapest with 4GB+ that's available
            let best_machine_with_region = if let Some(region) = user_region {
                // User specified region - find best machine for that region
                available_types.iter()
                    .filter(|st| st.memory_gb >= 4.0 && st.available_in.contains(&region.to_string()))
                    .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
                    .map(|st| (st.id.clone(), region.to_string()))
                    .or_else(|| {
                        available_types.iter()
                            .filter(|st| st.available_in.contains(&region.to_string()))
                            .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
                            .map(|st| (st.id.clone(), region.to_string()))
                    })
            } else {
                // No region specified - find globally cheapest 4GB+ machine
                available_types.iter()
                    .filter(|st| st.memory_gb >= 4.0)
                    .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
                    .map(|st| {
                        let region = st.available_in.first()
                            .cloned()
                            .unwrap_or_else(|| "nbg1".to_string());
                        (st.id.clone(), region)
                    })
                    .or_else(|| {
                        available_types.iter()
                            .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
                            .map(|st| {
                                let region = st.available_in.first()
                                    .cloned()
                                    .unwrap_or_else(|| "nbg1".to_string());
                                (st.id.clone(), region)
                            })
                    })
            };

            if let Some((machine, region)) = best_machine_with_region {
                (
                    args.machine_type.clone().unwrap_or(machine),
                    args.region.clone().unwrap_or(region),
                )
            } else {
                // Fallback to static defaults
                (
                    args.machine_type.clone().unwrap_or_else(|| recommendation.machine_type.clone()),
                    args.region.clone().unwrap_or_else(|| recommendation.region.clone()),
                )
            }
        } else {
            // Non-Hetzner or no availability data - use static defaults
            let machine = args.machine_type.clone()
                .unwrap_or_else(|| recommendation.machine_type.clone());
            let region = args.region.clone()
                .unwrap_or_else(|| recommendation.region.clone());
            (machine, region)
        };

        let final_port = args.port
            .unwrap_or(recommendation.port);

        // Get repository info
        let repositories = match client.list_project_repositories(&project_id).await {
            Ok(repos) => repos,
            Err(e) => {
                return Ok(format_error_for_llm(
                    "deploy_service",
                    ErrorCategory::NetworkError,
                    &format!("Failed to get repositories: {}", e),
                    Some(vec!["Ensure a repository is connected to the project"]),
                ));
            }
        };

        // Smart repository selection: match local git remote or find non-gitops repo
        let repo = match find_matching_repository(&repositories.repositories, &self.project_path) {
            Some(r) => r,
            None => {
                return Ok(format_error_for_llm(
                    "deploy_service",
                    ErrorCategory::ResourceUnavailable,
                    "No repository connected to project",
                    Some(vec![
                        "Connect a GitHub repository to the project first",
                        "Use the platform UI to connect a repository",
                    ]),
                ));
            }
        };

        tracing::info!(
            "Deploy service: Using repository {} (id: {}), default_branch: {:?}",
            repo.repository_full_name,
            repo.repository_id,
            repo.default_branch
        );

        // Use resolved environment ID from earlier
        if resolved_env_id.is_empty() {
            return Ok(format_error_for_llm(
                "deploy_service",
                ErrorCategory::ResourceUnavailable,
                "No environment found for project",
                Some(vec!["Create an environment in the platform first"]),
            ));
        }

        // Build deployment config request
        // Derive dockerfile path and build context from DockerfileInfo
        //
        // IMPORTANT: Paths must be relative to the REPO ROOT for Cloud Runner.
        // Cloud Runner clones the GitHub repo and builds from there.
        //
        // Example: User analyzes path="services/contact-intelligence" which has a Dockerfile.
        // The GitHub repo structure is:
        //   repo-root/
        //     services/
        //       contact-intelligence/
        //         Dockerfile
        //
        // Cloud Runner needs:
        //   dockerfile: "services/contact-intelligence/Dockerfile"
        //   context: "services/contact-intelligence"
        //
        // NOT:
        //   dockerfile: "Dockerfile", context: "."  (would look at repo root)
        let (dockerfile_path, build_context) = analysis.docker_analysis
            .as_ref()
            .and_then(|d| d.dockerfiles.first())
            .map(|df| {
                // Get dockerfile filename (e.g., "Dockerfile" or "Dockerfile.prod")
                let dockerfile_name = df.path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Dockerfile".to_string());

                // Derive dockerfile's directory relative to analysis_path
                let analysis_relative_dir = df.path.parent()
                    .and_then(|p| p.strip_prefix(&analysis_path).ok())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Build paths relative to REPO ROOT by prepending args.path (the subdirectory)
                // This ensures Cloud Runner finds the Dockerfile in the cloned repo
                let subpath = args.path.as_deref().unwrap_or("");

                if subpath.is_empty() {
                    // Analyzing repo root - use paths as-is
                    if analysis_relative_dir.is_empty() {
                        (dockerfile_name, ".".to_string())
                    } else {
                        (format!("{}/{}", analysis_relative_dir, dockerfile_name), analysis_relative_dir)
                    }
                } else {
                    // Analyzing a subdirectory - prepend subpath to make repo-root-relative
                    if analysis_relative_dir.is_empty() {
                        // Dockerfile at root of analyzed subdir
                        // e.g., subpath="services/contact-intelligence" -> dockerfile="services/contact-intelligence/Dockerfile"
                        (format!("{}/{}", subpath, dockerfile_name), subpath.to_string())
                    } else {
                        // Dockerfile in nested dir within analyzed subdir
                        // e.g., subpath="services", analysis_relative_dir="contact-intelligence"
                        let full_context = format!("{}/{}", subpath, analysis_relative_dir);
                        (format!("{}/{}", full_context, dockerfile_name), full_context)
                    }
                }
            })
            .unwrap_or_else(|| {
                // No dockerfile found - use subpath as context if provided, else root
                let subpath = args.path.as_deref().unwrap_or("");
                if subpath.is_empty() {
                    ("Dockerfile".to_string(), ".".to_string())
                } else {
                    (format!("{}/Dockerfile", subpath), subpath.to_string())
                }
            });

        tracing::debug!(
            "Deploy service docker config: dockerfile_path={}, build_context={}, subpath={:?}",
            dockerfile_path,
            build_context,
            args.path
        );

        // Fetch provider_account_id from credentials for GCP/Azure
        let mut gcp_project_id = None;
        let mut subscription_id = None;
        if matches!(final_provider, CloudProvider::Gcp | CloudProvider::Azure) {
            if let Ok(Some(cred)) = client.check_provider_connection(&final_provider, &project_id).await {
                match final_provider {
                    CloudProvider::Gcp => gcp_project_id = cred.provider_account_id,
                    CloudProvider::Azure => subscription_id = cred.provider_account_id,
                    _ => {}
                }
            }
        }

        // Determine CPU/memory from args or recommendation
        let final_cpu = args.cpu.clone()
            .or_else(|| recommendation.cpu.clone());
        let final_memory = args.memory.clone()
            .or_else(|| recommendation.memory.clone());

        let config_input = CloudRunnerConfigInput {
            provider: Some(final_provider.clone()),
            region: Some(final_region.clone()),
            server_type: if final_provider == CloudProvider::Hetzner { Some(final_machine.clone()) } else { None },
            gcp_project_id,
            cpu: final_cpu.clone(),
            memory: final_memory.clone(),
            min_instances: args.min_instances,
            max_instances: args.max_instances,
            allow_unauthenticated: Some(args.is_public),
            subscription_id,
            is_public: Some(args.is_public),
            health_check_path: recommendation.health_check_path.clone(),
            ..Default::default()
        };
        let cloud_runner_config = build_cloud_runner_config_v2(&config_input);

        // Resolve secrets if provided
        let secrets = if let Some(ref keys) = args.secret_keys {
            let mut resolved = Vec::new();
            for sk in keys {
                let value = match &sk.value {
                    Some(v) => v.clone(),
                    None if self.execution_context.has_terminal() => {
                        match prompt_secret_value(&sk.key) {
                            SecretPromptResult::Value(v) => v,
                            SecretPromptResult::Skipped => continue,
                            SecretPromptResult::Cancelled => {
                                return Ok(format_error_for_llm(
                                    "deploy_service",
                                    ErrorCategory::ValidationFailed,
                                    "Secret entry cancelled by user",
                                    Some(vec!["The user cancelled secret input. Try again when ready."]),
                                ));
                            }
                        }
                    }
                    None => continue, // server mode, skip secrets without values
                };
                resolved.push(DeploymentSecretInput {
                    key: sk.key.clone(),
                    value,
                    is_secret: sk.is_secret,
                });
            }
            if resolved.is_empty() { None } else { Some(resolved) }
        } else {
            None
        };

        // SECURITY: Pre-compute response info (keys only, no values) before moving secrets
        let secrets_set_info = secrets.as_ref().map(|s| {
            s.iter()
                .map(|si| json!({"key": si.key, "is_secret": si.is_secret}))
                .collect::<Vec<_>>()
        });

        let config_request = CreateDeploymentConfigRequest {
            project_id: project_id.clone(),
            service_name: service_name.clone(),
            repository_id: repo.repository_id,
            repository_full_name: repo.repository_full_name.clone(),
            dockerfile_path: Some(dockerfile_path.clone()),
            dockerfile: Some(dockerfile_path.clone()),
            build_context: Some(build_context.clone()),
            context: Some(build_context.clone()),
            port: final_port as i32,
            branch: repo.default_branch.clone().unwrap_or_else(|| "main".to_string()),
            target_type: recommendation.target.as_str().to_string(),
            cloud_provider: final_provider.as_str().to_string(),
            environment_id: resolved_env_id.clone(),
            cluster_id: None, // Cloud Runner doesn't need cluster
            registry_id: None, // Auto-provision
            auto_deploy_enabled: true,
            is_public: Some(args.is_public),
            cloud_runner_config: Some(cloud_runner_config),
            secrets,
        };

        // Create config
        let config = match client.create_deployment_config(&config_request).await {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_api_error("deploy_service", e));
            }
        };

        // Trigger deployment
        let trigger_request = TriggerDeploymentRequest {
            project_id: project_id.clone(),
            config_id: config.id.clone(),
            commit_sha: None,
        };

        match client.trigger_deployment(&trigger_request).await {
            Ok(response) => {
                let result = json!({
                    "status": "deployed",
                    "deployment_mode": "NEW_DEPLOYMENT",
                    "config_id": config.id,
                    "task_id": response.backstage_task_id,
                    "service_name": service_name,
                    "environment": {
                        "id": resolved_env_id,
                        "name": resolved_env_name,
                        "is_production": is_production,
                    },
                    "provider": final_provider.as_str(),
                    "machine_type": final_machine,
                    "region": final_region,
                    "port": final_port,
                    "docker_config": {
                        "dockerfile_path": dockerfile_path,
                        "build_context": build_context,
                    },
                    "secrets_set": secrets_set_info,
                    "message": format!(
                        "NEW deployment started for '{}' on {} environment. Task ID: {}",
                        service_name, resolved_env_name, response.backstage_task_id
                    ),
                    "next_steps": [
                        format!("Monitor progress: use get_deployment_status with task_id '{}'", response.backstage_task_id),
                        "View logs after deployment: use get_service_logs",
                    ],
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| DeployServiceError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_api_error("deploy_service", e)),
        }
    }
}

/// Extract service name from path
fn get_service_name(path: &PathBuf) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_lowercase().replace(['_', ' '], "-"))
        .unwrap_or_else(|| "service".to_string())
}

/// Detect the git remote URL from a directory
fn detect_git_remote(project_path: &PathBuf) -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(project_path)
        .output()
        .ok()?;

    if output.status.success() {
        let url = String::from_utf8(output.stdout).ok()?;
        Some(url.trim().to_string())
    } else {
        None
    }
}

/// Parse repository full name from git remote URL
/// Handles both SSH (git@github.com:owner/repo.git) and HTTPS (https://github.com/owner/repo.git)
fn parse_repo_from_url(url: &str) -> Option<String> {
    let url = url.trim();

    // SSH format: git@github.com:owner/repo.git
    if url.starts_with("git@") {
        let parts: Vec<&str> = url.split(':').collect();
        if parts.len() == 2 {
            let path = parts[1].trim_end_matches(".git");
            return Some(path.to_string());
        }
    }

    // HTTPS format: https://github.com/owner/repo.git
    if url.starts_with("https://") || url.starts_with("http://") {
        if let Some(path) = url.split('/').skip(3).collect::<Vec<_>>().join("/").strip_suffix(".git") {
            return Some(path.to_string());
        }
        // Without .git suffix
        let path: String = url.split('/').skip(3).collect::<Vec<_>>().join("/");
        if !path.is_empty() {
            return Some(path);
        }
    }

    None
}

/// Find repository matching local git remote, or fall back to non-gitops repo
fn find_matching_repository<'a>(
    repositories: &'a [ProjectRepository],
    project_path: &PathBuf,
) -> Option<&'a ProjectRepository> {
    // First, try to detect from local git remote
    if let Some(detected_name) = detect_git_remote(project_path).and_then(|url| parse_repo_from_url(&url)) {
        tracing::debug!("Detected local git remote: {}", detected_name);

        if let Some(repo) = repositories.iter().find(|r| {
            r.repository_full_name.eq_ignore_ascii_case(&detected_name)
        }) {
            tracing::debug!("Matched detected repo: {}", repo.repository_full_name);
            return Some(repo);
        }
    }

    // Fall back: find first non-GitOps repository
    // GitOps repos are typically infrastructure/config repos, not application repos
    if let Some(repo) = repositories.iter().find(|r| {
        r.is_primary_git_ops != Some(true) &&
        !r.repository_full_name.to_lowercase().contains("infrastructure") &&
        !r.repository_full_name.to_lowercase().contains("gitops")
    }) {
        tracing::debug!("Using non-gitops repo: {}", repo.repository_full_name);
        return Some(repo);
    }

    // Last resort: first repo
    repositories.first()
}

/// Format a PlatformApiError for LLM consumption
fn format_api_error(tool_name: &str, error: PlatformApiError) -> String {
    match error {
        PlatformApiError::Unauthorized => format_error_for_llm(
            tool_name,
            ErrorCategory::PermissionDenied,
            "Not authenticated - please run `sync-ctl auth login` first",
            Some(vec![
                "The user needs to authenticate with the Syncable platform",
                "Run: sync-ctl auth login",
            ]),
        ),
        PlatformApiError::NotFound(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::ResourceUnavailable,
            &format!("Resource not found: {}", msg),
            Some(vec![
                "The project ID may be incorrect",
                "Use list_projects to find valid project IDs",
            ]),
        ),
        PlatformApiError::PermissionDenied(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::PermissionDenied,
            &format!("Permission denied: {}", msg),
            Some(vec!["Contact the project admin for access"]),
        ),
        PlatformApiError::RateLimited => format_error_for_llm(
            tool_name,
            ErrorCategory::ResourceUnavailable,
            "Rate limit exceeded - please try again later",
            Some(vec!["Wait a moment before retrying"]),
        ),
        PlatformApiError::HttpError(e) => format_error_for_llm(
            tool_name,
            ErrorCategory::NetworkError,
            &format!("Network error: {}", e),
            Some(vec!["Check network connectivity"]),
        ),
        PlatformApiError::ParseError(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::InternalError,
            &format!("Failed to parse API response: {}", msg),
            None,
        ),
        PlatformApiError::ApiError { status, message } => format_error_for_llm(
            tool_name,
            ErrorCategory::ExternalCommandFailed,
            &format!("API error ({}): {}", status, message),
            Some(vec!["Check the error message for details"]),
        ),
        PlatformApiError::ServerError { status, message } => format_error_for_llm(
            tool_name,
            ErrorCategory::ExternalCommandFailed,
            &format!("Server error ({}): {}", status, message),
            Some(vec!["Try again later"]),
        ),
        PlatformApiError::ConnectionFailed => format_error_for_llm(
            tool_name,
            ErrorCategory::NetworkError,
            "Could not connect to Syncable API",
            Some(vec!["Check your internet connection"]),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(DeployServiceTool::NAME, "deploy_service");
    }

    #[test]
    fn test_default_preview_only() {
        assert!(default_preview());
    }

    #[test]
    fn test_get_service_name() {
        assert_eq!(
            get_service_name(&PathBuf::from("/path/to/my_service")),
            "my-service"
        );
        assert_eq!(
            get_service_name(&PathBuf::from("/path/to/MyApp")),
            "myapp"
        );
        assert_eq!(
            get_service_name(&PathBuf::from("/path/to/api-service")),
            "api-service"
        );
    }

    #[test]
    fn test_tool_creation() {
        let tool = DeployServiceTool::new(PathBuf::from("/test"));
        assert!(format!("{:?}", tool).contains("DeployServiceTool"));
    }

    #[tokio::test]
    async fn test_nonexistent_path_returns_error() {
        let tool = DeployServiceTool::new(PathBuf::from("/nonexistent/path/that/does/not/exist"));
        let args = DeployServiceArgs {
            path: Some("nope".to_string()),
            provider: None,
            machine_type: None,
            region: None,
            port: None,
            is_public: false,
            cpu: None,
            memory: None,
            min_instances: None,
            max_instances: None,
            preview_only: true,
            secret_keys: None,
        };

        let result = tool.call(args).await.unwrap();
        assert!(result.contains("error") || result.contains("not found") || result.contains("Path not found"));
    }
}
