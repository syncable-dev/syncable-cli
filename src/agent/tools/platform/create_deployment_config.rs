//! Create deployment config tool for the agent
//!
//! Allows the agent to create a new deployment configuration for a service.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::types::CreateDeploymentConfigRequest;
use crate::platform::api::{PlatformApiClient, PlatformApiError};

/// Arguments for the create deployment config tool
#[derive(Debug, Deserialize)]
pub struct CreateDeploymentConfigArgs {
    /// The project UUID
    pub project_id: String,
    /// Service name for the deployment
    pub service_name: String,
    /// Repository ID from GitHub integration
    pub repository_id: i64,
    /// Full repository name (e.g., "owner/repo")
    pub repository_full_name: String,
    /// Port the service listens on
    pub port: i32,
    /// Git branch to deploy from
    pub branch: String,
    /// Target type: "kubernetes" or "cloud_runner"
    pub target_type: String,
    /// Cloud provider: "gcp" or "hetzner"
    pub provider: String,
    /// Environment ID for deployment
    pub environment_id: String,
    /// Path to Dockerfile relative to repo root
    pub dockerfile_path: Option<String>,
    /// Build context path relative to repo root
    pub build_context: Option<String>,
    /// Cluster ID (required for kubernetes target)
    pub cluster_id: Option<String>,
    /// Registry ID (optional - will provision new if not provided)
    pub registry_id: Option<String>,
    /// Enable auto-deploy on push (defaults to true)
    #[serde(default = "default_auto_deploy")]
    pub auto_deploy_enabled: bool,
}

fn default_auto_deploy() -> bool {
    true
}

/// Error type for create deployment config operations
#[derive(Debug, thiserror::Error)]
#[error("Create deployment config error: {0}")]
pub struct CreateDeploymentConfigError(String);

/// Tool to create a new deployment configuration
///
/// Creates a deployment config that defines how to build and deploy a service.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateDeploymentConfigTool;

impl CreateDeploymentConfigTool {
    /// Create a new CreateDeploymentConfigTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for CreateDeploymentConfigTool {
    const NAME: &'static str = "create_deployment_config";

    type Error = CreateDeploymentConfigError;
    type Args = CreateDeploymentConfigArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Create a new deployment configuration for a service.

A deployment config defines how to build and deploy a service, including:
- Source repository and branch
- Dockerfile location and build context
- Target (Cloud Runner or Kubernetes)
- Port configuration
- Auto-deploy settings

**Required Parameters:**
- project_id: The project UUID
- service_name: Name for the service (lowercase, hyphens allowed)
- repository_id: GitHub repository ID (from platform GitHub integration)
- repository_full_name: Full repo name like "owner/repo"
- port: Port the service listens on
- branch: Git branch to deploy from (e.g., "main")
- target_type: "kubernetes" or "cloud_runner"
- provider: "gcp" or "hetzner"
- environment_id: Environment to deploy to

**Optional Parameters:**
- dockerfile_path: Path to Dockerfile (default: "Dockerfile")
- build_context: Build context path (default: ".")
- cluster_id: Required for kubernetes target
- registry_id: Container registry ID (provisions new if not provided)
- auto_deploy_enabled: Enable auto-deploy on push (default: true)

**Prerequisites:**
- User must be authenticated
- GitHub repository must be connected to the project
- Provider must be connected (check with check_provider_connection)
- For kubernetes: cluster must exist (check with list_deployment_capabilities)

**Returns:**
- config_id: The created deployment config ID
- service_name, branch, target_type, provider
- next_steps: How to trigger a deployment"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "The UUID of the project"
                    },
                    "service_name": {
                        "type": "string",
                        "description": "Name for the service (lowercase, hyphens allowed)"
                    },
                    "repository_id": {
                        "type": "integer",
                        "description": "GitHub repository ID from platform integration"
                    },
                    "repository_full_name": {
                        "type": "string",
                        "description": "Full repository name (e.g., 'owner/repo')"
                    },
                    "port": {
                        "type": "integer",
                        "description": "Port the service listens on"
                    },
                    "branch": {
                        "type": "string",
                        "description": "Git branch to deploy from"
                    },
                    "target_type": {
                        "type": "string",
                        "enum": ["kubernetes", "cloud_runner"],
                        "description": "Deployment target type"
                    },
                    "provider": {
                        "type": "string",
                        "enum": ["gcp", "hetzner"],
                        "description": "Cloud provider"
                    },
                    "environment_id": {
                        "type": "string",
                        "description": "Environment ID for deployment"
                    },
                    "dockerfile_path": {
                        "type": "string",
                        "description": "Path to Dockerfile relative to repo root"
                    },
                    "build_context": {
                        "type": "string",
                        "description": "Build context path relative to repo root"
                    },
                    "cluster_id": {
                        "type": "string",
                        "description": "Cluster ID (required for kubernetes target)"
                    },
                    "registry_id": {
                        "type": "string",
                        "description": "Registry ID (optional - provisions new if not provided)"
                    },
                    "auto_deploy_enabled": {
                        "type": "boolean",
                        "description": "Enable auto-deploy on push (default: true)"
                    }
                },
                "required": [
                    "project_id", "service_name", "repository_id", "repository_full_name",
                    "port", "branch", "target_type", "provider", "environment_id"
                ]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate required fields
        if args.project_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "create_deployment_config",
                ErrorCategory::ValidationFailed,
                "project_id cannot be empty",
                Some(vec![
                    "Use list_projects to find valid project IDs",
                    "Use current_context to get the selected project",
                ]),
            ));
        }

        if args.service_name.trim().is_empty() {
            return Ok(format_error_for_llm(
                "create_deployment_config",
                ErrorCategory::ValidationFailed,
                "service_name cannot be empty",
                Some(vec![
                    "Use analyze_project to discover suggested service names",
                    "Service name should be lowercase with hyphens",
                ]),
            ));
        }

        // Validate target_type
        let valid_targets = ["kubernetes", "cloud_runner"];
        if !valid_targets.contains(&args.target_type.as_str()) {
            return Ok(format_error_for_llm(
                "create_deployment_config",
                ErrorCategory::ValidationFailed,
                &format!(
                    "Invalid target_type '{}'. Must be 'kubernetes' or 'cloud_runner'",
                    args.target_type
                ),
                Some(vec![
                    "Use 'cloud_runner' for GCP Cloud Run or Hetzner containers",
                    "Use 'kubernetes' for deploying to a K8s cluster",
                ]),
            ));
        }

        // Validate provider
        let valid_providers = ["gcp", "hetzner"];
        if !valid_providers.contains(&args.provider.as_str()) {
            return Ok(format_error_for_llm(
                "create_deployment_config",
                ErrorCategory::ValidationFailed,
                &format!(
                    "Invalid provider '{}'. Must be 'gcp' or 'hetzner'",
                    args.provider
                ),
                Some(vec![
                    "Use list_deployment_capabilities to see connected providers",
                    "Connect a provider in platform settings first",
                ]),
            ));
        }

        // Kubernetes target requires cluster_id
        if args.target_type == "kubernetes" && args.cluster_id.is_none() {
            return Ok(format_error_for_llm(
                "create_deployment_config",
                ErrorCategory::ValidationFailed,
                "cluster_id is required for kubernetes target",
                Some(vec![
                    "Use list_deployment_capabilities to find available clusters",
                    "Or use 'cloud_runner' target which doesn't require a cluster",
                ]),
            ));
        }

        // Create the API client
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_api_error("create_deployment_config", e));
            }
        };

        // Build the request
        let request = CreateDeploymentConfigRequest {
            service_name: args.service_name.clone(),
            repository_id: args.repository_id,
            repository_full_name: args.repository_full_name.clone(),
            dockerfile_path: args.dockerfile_path.clone(),
            build_context: args.build_context.clone(),
            port: args.port,
            branch: args.branch.clone(),
            target_type: args.target_type.clone(),
            provider: args.provider.clone(),
            environment_id: args.environment_id.clone(),
            cluster_id: args.cluster_id.clone(),
            registry_id: args.registry_id.clone(),
            auto_deploy_enabled: args.auto_deploy_enabled,
            deployment_strategy: None,
        };

        // Create the deployment config
        match client.create_deployment_config(&args.project_id, &request).await {
            Ok(config) => {
                let result = json!({
                    "success": true,
                    "config_id": config.id,
                    "service_name": config.service_name,
                    "branch": config.branch,
                    "target_type": args.target_type,
                    "provider": args.provider,
                    "auto_deploy_enabled": args.auto_deploy_enabled,
                    "message": format!(
                        "Deployment config created for service '{}' on {} ({})",
                        config.service_name, args.target_type, args.provider
                    ),
                    "next_steps": [
                        format!("Use trigger_deployment with config_id '{}' to deploy", config.id),
                        "Use get_deployment_status to monitor deployment progress",
                        if args.auto_deploy_enabled {
                            "Auto-deploy is enabled - pushing to the branch will trigger deployments"
                        } else {
                            "Auto-deploy is disabled - deployments must be triggered manually"
                        }
                    ]
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| CreateDeploymentConfigError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_api_error("create_deployment_config", e)),
        }
    }
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
                "The repository may not be connected to the project",
                "Use list_projects to find valid project IDs",
            ]),
        ),
        PlatformApiError::PermissionDenied(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::PermissionDenied,
            &format!("Permission denied: {}", msg),
            Some(vec![
                "The user does not have permission to create deployment configs",
                "Contact the project admin for access",
            ]),
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
            Some(vec![
                "Check network connectivity",
                "The Syncable API may be temporarily unavailable",
            ]),
        ),
        PlatformApiError::ParseError(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::InternalError,
            &format!("Failed to parse API response: {}", msg),
            Some(vec!["This may be a temporary API issue"]),
        ),
        PlatformApiError::ApiError { status, message } => format_error_for_llm(
            tool_name,
            ErrorCategory::ExternalCommandFailed,
            &format!("API error ({}): {}", status, message),
            Some(vec![
                "Check the error message for details",
                "The repository may not be properly connected",
            ]),
        ),
        PlatformApiError::ServerError { status, message } => format_error_for_llm(
            tool_name,
            ErrorCategory::ExternalCommandFailed,
            &format!("Server error ({}): {}", status, message),
            Some(vec![
                "The Syncable API is experiencing issues",
                "Try again later",
            ]),
        ),
        PlatformApiError::ConnectionFailed => format_error_for_llm(
            tool_name,
            ErrorCategory::NetworkError,
            "Could not connect to Syncable API",
            Some(vec![
                "Check your internet connection",
                "The Syncable API may be temporarily unavailable",
            ]),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(CreateDeploymentConfigTool::NAME, "create_deployment_config");
    }

    #[test]
    fn test_tool_creation() {
        let tool = CreateDeploymentConfigTool::new();
        assert!(format!("{:?}", tool).contains("CreateDeploymentConfigTool"));
    }

    #[test]
    fn test_default_auto_deploy() {
        assert!(default_auto_deploy());
    }
}
