//! Provision registry tool for the agent
//!
//! Allows the agent to provision a new container registry for storing images.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::types::{CreateRegistryRequest, RegistryTaskState};
use crate::platform::api::{PlatformApiClient, PlatformApiError};

/// Maximum time to wait for registry provisioning (5 minutes)
const PROVISIONING_TIMEOUT_SECS: u64 = 300;
/// Polling interval between status checks
const POLL_INTERVAL_SECS: u64 = 3;

/// Arguments for the provision registry tool
#[derive(Debug, Deserialize)]
pub struct ProvisionRegistryArgs {
    /// The project UUID
    pub project_id: String,
    /// Cluster ID to associate registry with
    pub cluster_id: String,
    /// Cluster name for display
    pub cluster_name: String,
    /// Cloud provider: "gcp" or "hetzner"
    pub provider: String,
    /// Region for the registry
    pub region: String,
    /// Name for the registry (auto-generated if not provided)
    pub registry_name: Option<String>,
    /// GCP project ID (required for GCP provider)
    pub gcp_project_id: Option<String>,
}

/// Error type for provision registry operations
#[derive(Debug, thiserror::Error)]
#[error("Provision registry error: {0}")]
pub struct ProvisionRegistryError(String);

/// Tool to provision a new container registry
///
/// Creates a container registry for storing Docker images used in deployments.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvisionRegistryTool;

impl ProvisionRegistryTool {
    /// Create a new ProvisionRegistryTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ProvisionRegistryTool {
    const NAME: &'static str = "provision_registry";

    type Error = ProvisionRegistryError;
    type Args = ProvisionRegistryArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Provision a new container registry for storing Docker images.

A container registry is required for deployments. This tool starts provisioning
and polls until completion (may take 1-3 minutes).

**Parameters:**
- project_id: The project UUID
- cluster_id: Cluster ID to associate the registry with
- cluster_name: Cluster name for display purposes
- provider: "gcp" or "hetzner"
- region: Region for the registry (e.g., "us-central1", "nbg1")
- registry_name: Name for the registry (optional - defaults to "main")
- gcp_project_id: Required for GCP provider

**Prerequisites:**
- User must be authenticated
- Provider must be connected
- Cluster must exist (use list_deployment_capabilities to find clusters)

**Async Behavior:**
- Provisioning takes 1-3 minutes
- This tool polls until complete or failed
- Returns registry details on success

**Returns:**
- registry_id: The created registry ID
- registry_name, region, provider
- registry_url: URL for pushing images
- status: "completed" or error details"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "The UUID of the project"
                    },
                    "cluster_id": {
                        "type": "string",
                        "description": "Cluster ID to associate registry with"
                    },
                    "cluster_name": {
                        "type": "string",
                        "description": "Cluster name for display"
                    },
                    "provider": {
                        "type": "string",
                        "enum": ["gcp", "hetzner"],
                        "description": "Cloud provider"
                    },
                    "region": {
                        "type": "string",
                        "description": "Region for the registry"
                    },
                    "registry_name": {
                        "type": "string",
                        "description": "Name for the registry (defaults to 'main')"
                    },
                    "gcp_project_id": {
                        "type": "string",
                        "description": "GCP project ID (required for GCP)"
                    }
                },
                "required": ["project_id", "cluster_id", "cluster_name", "provider", "region"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate required fields
        if args.project_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "provision_registry",
                ErrorCategory::ValidationFailed,
                "project_id cannot be empty",
                Some(vec!["Use list_projects to find valid project IDs"]),
            ));
        }

        if args.cluster_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "provision_registry",
                ErrorCategory::ValidationFailed,
                "cluster_id cannot be empty",
                Some(vec!["Use list_deployment_capabilities to find available clusters"]),
            ));
        }

        // Validate provider
        let valid_providers = ["gcp", "hetzner"];
        if !valid_providers.contains(&args.provider.as_str()) {
            return Ok(format_error_for_llm(
                "provision_registry",
                ErrorCategory::ValidationFailed,
                &format!(
                    "Invalid provider '{}'. Must be 'gcp' or 'hetzner'",
                    args.provider
                ),
                Some(vec![
                    "Use list_deployment_capabilities to see connected providers",
                ]),
            ));
        }

        // GCP requires gcp_project_id
        if args.provider == "gcp" && args.gcp_project_id.is_none() {
            return Ok(format_error_for_llm(
                "provision_registry",
                ErrorCategory::ValidationFailed,
                "gcp_project_id is required for GCP provider",
                Some(vec![
                    "The GCP project ID can be found in the GCP Console",
                    "This is different from the Syncable project_id",
                ]),
            ));
        }

        // Create the API client
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_api_error("provision_registry", e));
            }
        };

        // Generate registry name if not provided
        let registry_name = args
            .registry_name
            .as_deref()
            .map(sanitize_registry_name)
            .unwrap_or_else(|| "main".to_string());

        // Build the request
        let request = CreateRegistryRequest {
            project_id: args.project_id.clone(),
            cluster_id: args.cluster_id.clone(),
            cluster_name: args.cluster_name.clone(),
            registry_name: registry_name.clone(),
            cloud_provider: args.provider.clone(),
            region: args.region.clone(),
            gcp_project_id: args.gcp_project_id.clone(),
        };

        // Start provisioning
        let response = match client.create_registry(&args.project_id, &request).await {
            Ok(r) => r,
            Err(e) => {
                return Ok(format_api_error("provision_registry", e));
            }
        };

        let task_id = response.task_id;

        // Poll for completion with timeout
        let start = std::time::Instant::now();
        loop {
            if start.elapsed().as_secs() > PROVISIONING_TIMEOUT_SECS {
                return Ok(format_error_for_llm(
                    "provision_registry",
                    ErrorCategory::Timeout,
                    &format!(
                        "Registry provisioning timed out after {} seconds. Task ID: {}",
                        PROVISIONING_TIMEOUT_SECS, task_id
                    ),
                    Some(vec![
                        "The provisioning may still complete in the background",
                        "Use the platform UI to check the registry status",
                        &format!("Task ID for reference: {}", task_id),
                    ]),
                ));
            }

            sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;

            let status = match client.get_registry_task_status(&task_id).await {
                Ok(s) => s,
                Err(e) => {
                    return Ok(format_error_for_llm(
                        "provision_registry",
                        ErrorCategory::NetworkError,
                        &format!("Failed to get task status: {}", e),
                        Some(vec![
                            "The provisioning may still be running",
                            &format!("Task ID: {}", task_id),
                        ]),
                    ));
                }
            };

            match status.status {
                RegistryTaskState::Completed => {
                    let registry_url = status.output.registry_url.clone();
                    let final_registry_name = status
                        .output
                        .registry_name
                        .clone()
                        .unwrap_or_else(|| registry_name.clone());

                    // The task_id serves as the registry identifier for now
                    // The actual registry ID may be returned in the output after provisioning completes
                    let result = json!({
                        "success": true,
                        "task_id": task_id,
                        "registry_name": final_registry_name,
                        "region": args.region,
                        "provider": args.provider,
                        "registry_url": registry_url,
                        "status": "completed",
                        "message": format!(
                            "Registry '{}' provisioned successfully",
                            final_registry_name
                        ),
                        "next_steps": [
                            "The registry is now ready for use",
                            "Use list_deployment_capabilities to get the full registry details",
                            "Docker images will be pushed to this registry during deployments"
                        ]
                    });

                    return serde_json::to_string_pretty(&result)
                        .map_err(|e| ProvisionRegistryError(format!("Failed to serialize: {}", e)));
                }
                RegistryTaskState::Failed => {
                    let error_msg = status
                        .error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Unknown error".to_string());

                    return Ok(format_error_for_llm(
                        "provision_registry",
                        ErrorCategory::ExternalCommandFailed,
                        &format!("Registry provisioning failed: {}", error_msg),
                        Some(vec![
                            "Check provider connectivity",
                            "Verify cluster and region are valid",
                            "The provider may have resource limits",
                        ]),
                    ));
                }
                RegistryTaskState::Cancelled => {
                    return Ok(format_error_for_llm(
                        "provision_registry",
                        ErrorCategory::UserCancelled,
                        "Registry provisioning was cancelled",
                        Some(vec!["The task was cancelled externally"]),
                    ));
                }
                RegistryTaskState::Processing | RegistryTaskState::Unknown => {
                    // Continue polling
                }
            }
        }
    }
}

/// Sanitize registry name (lowercase, alphanumeric, hyphens)
fn sanitize_registry_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Format a PlatformApiError for LLM consumption
fn format_api_error(tool_name: &str, error: PlatformApiError) -> String {
    match error {
        PlatformApiError::Unauthorized => format_error_for_llm(
            tool_name,
            ErrorCategory::PermissionDenied,
            "Not authenticated - please run `sync-ctl auth login` first",
            Some(vec!["Run: sync-ctl auth login"]),
        ),
        PlatformApiError::NotFound(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::ResourceUnavailable,
            &format!("Resource not found: {}", msg),
            Some(vec![
                "The project or cluster ID may be incorrect",
                "Use list_deployment_capabilities to find valid IDs",
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
            "Rate limit exceeded",
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
        assert_eq!(ProvisionRegistryTool::NAME, "provision_registry");
    }

    #[test]
    fn test_tool_creation() {
        let tool = ProvisionRegistryTool::new();
        assert!(format!("{:?}", tool).contains("ProvisionRegistryTool"));
    }

    #[test]
    fn test_sanitize_registry_name() {
        assert_eq!(sanitize_registry_name("My Registry"), "my-registry");
        assert_eq!(sanitize_registry_name("test_name"), "test-name");
        assert_eq!(sanitize_registry_name("--test--"), "test");
        assert_eq!(sanitize_registry_name("MAIN"), "main");
    }
}
