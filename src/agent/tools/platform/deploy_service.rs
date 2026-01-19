//! Deploy service tool for the agent
//!
//! A compound tool that enables conversational deployment with intelligent recommendations.
//! Analyzes the project, provides recommendations with reasoning, and executes deployment.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::str::FromStr;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::analyzer::{AnalysisConfig, TechnologyCategory, analyze_project_with_config};
use crate::platform::api::types::{
    CloudProvider, CreateDeploymentConfigRequest, build_cloud_runner_config,
};
use crate::platform::api::{PlatformApiClient, PlatformApiError, TriggerDeploymentRequest};
use crate::platform::PlatformSession;
use crate::wizard::{
    RecommendationInput, recommend_deployment, get_provider_deployment_statuses,
};

/// Arguments for the deploy service tool
#[derive(Debug, Deserialize)]
pub struct DeployServiceArgs {
    /// Optional: specific subdirectory/service to deploy (for monorepos)
    pub path: Option<String>,
    /// Optional: override recommended provider (gcp, hetzner)
    pub provider: Option<String>,
    /// Optional: override machine type selection
    pub machine_type: Option<String>,
    /// Optional: override region selection
    pub region: Option<String>,
    /// Optional: override detected port
    pub port: Option<u16>,
    /// If true (default), show recommendation but don't deploy yet
    /// If false with settings, deploy immediately
    #[serde(default = "default_preview")]
    pub preview_only: bool,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployServiceTool {
    project_path: PathBuf,
}

impl DeployServiceTool {
    /// Create a new DeployServiceTool
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
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
- provider: Override recommendation (gcp, hetzner)
- machine_type: Override machine selection (e.g., cx22, e2-small)
- region: Override region selection (e.g., nbg1, us-central1)
- port: Override detected port
- preview_only: If true (default), show recommendation only

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
1. Tool returns analysis + recommendation + confirmation prompt
2. User: "yes, deploy it" or "use GCP instead"
3. Call tool again with confirmed settings and preview_only=false

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
                        "enum": ["gcp", "hetzner"],
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
                    "preview_only": {
                        "type": "boolean",
                        "description": "If true (default), show recommendation only. If false, deploy."
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
                    "Connect GCP or Hetzner in platform settings",
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
                        "To see more options: check the alternatives section above".to_string(),
                    ]
                )
            };

            // Production warning
            let production_warning = if is_production {
                Some("⚠️  WARNING: This will deploy to PRODUCTION environment. Please confirm you intend to deploy to production.")
            } else {
                None
            };

            let response = json!({
                "status": "recommendation",
                "deployment_mode": deployment_mode,
                "mode_explanation": mode_explanation,
                "environment": {
                    "id": resolved_env_id,
                    "name": resolved_env_name,
                    "is_production": is_production,
                },
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
                },
                "recommendation": {
                    "provider": recommendation.provider.as_str(),
                    "provider_reasoning": recommendation.provider_reasoning,
                    "target": recommendation.target.as_str(),
                    "target_reasoning": recommendation.target_reasoning,
                    "machine_type": recommendation.machine_type,
                    "machine_reasoning": recommendation.machine_reasoning,
                    "region": recommendation.region,
                    "region_reasoning": recommendation.region_reasoning,
                    "port": recommendation.port,
                    "health_check_path": recommendation.health_check_path,
                    "confidence": recommendation.confidence,
                },
                "alternatives": {
                    "providers": recommendation.alternatives.providers.iter().map(|p| json!({
                        "provider": p.provider.as_str(),
                        "available": p.available,
                        "reason_if_unavailable": p.reason_if_unavailable,
                    })).collect::<Vec<_>>(),
                    "machine_types": recommendation.alternatives.machine_types.iter().map(|m| json!({
                        "machine_type": m.machine_type,
                        "vcpu": m.vcpu,
                        "memory_gb": m.memory_gb,
                        "description": m.description,
                    })).collect::<Vec<_>>(),
                    "regions": recommendation.alternatives.regions.iter().map(|r| json!({
                        "region": r.region,
                        "display_name": r.display_name,
                    })).collect::<Vec<_>>(),
                },
                "service_name": service_name,
                "next_steps": next_steps,
                "confirmation_prompt": if existing_config.is_some() {
                    format!(
                        "REDEPLOY '{}' to {} environment?{}",
                        service_name,
                        resolved_env_name,
                        if is_production { " ⚠️  (PRODUCTION)" } else { "" }
                    )
                } else {
                    format!(
                        "Deploy NEW service '{}' to {} ({}) with {} in {} on {} environment?{}",
                        service_name,
                        recommendation.provider.display_name(),
                        recommendation.target.display_name(),
                        recommendation.machine_type,
                        recommendation.region,
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

        let final_machine = args.machine_type
            .clone()
            .unwrap_or(recommendation.machine_type.clone());

        let final_region = args.region
            .clone()
            .unwrap_or(recommendation.region.clone());

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

        let repo = match repositories.repositories.first() {
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
        let (dockerfile_path, build_context) = analysis.docker_analysis
            .as_ref()
            .and_then(|d| d.dockerfiles.first())
            .map(|df| {
                // Get the dockerfile path relative to project root
                let df_path = df.path.strip_prefix(&analysis_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| df.path.to_string_lossy().to_string());

                // Build context is the parent directory of the Dockerfile
                let context = df.path.parent()
                    .and_then(|p| p.strip_prefix(&analysis_path).ok())
                    .map(|p| {
                        let s = p.to_string_lossy().to_string();
                        if s.is_empty() { ".".to_string() } else { s }
                    })
                    .unwrap_or_else(|| ".".to_string());

                (df_path, context)
            })
            .unwrap_or_else(|| ("Dockerfile".to_string(), ".".to_string()));

        let cloud_runner_config = build_cloud_runner_config(
            &final_provider,
            &final_region,
            &final_machine,
            true, // is_public
            recommendation.health_check_path.as_deref(),
        );

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
            is_public: Some(true),
            cloud_runner_config: Some(cloud_runner_config),
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
            preview_only: true,
        };

        let result = tool.call(args).await.unwrap();
        assert!(result.contains("error") || result.contains("not found") || result.contains("Path not found"));
    }
}
