//! List deployment capabilities tool for the agent
//!
//! Wraps the existing `get_provider_deployment_statuses` function to allow
//! the agent to discover available deployment options for a project.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::{PlatformApiClient, PlatformApiError};
use crate::wizard::get_provider_deployment_statuses;

/// Arguments for the list deployment capabilities tool
#[derive(Debug, Deserialize)]
pub struct ListDeploymentCapabilitiesArgs {
    /// The project UUID to check capabilities for
    pub project_id: String,
}

/// Error type for list deployment capabilities operations
#[derive(Debug, thiserror::Error)]
#[error("List deployment capabilities error: {0}")]
pub struct ListDeploymentCapabilitiesError(String);

/// Tool to list available deployment capabilities for a project
///
/// Returns information about connected providers, available clusters,
/// registries, and Cloud Run availability.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListDeploymentCapabilitiesTool;

impl ListDeploymentCapabilitiesTool {
    /// Create a new ListDeploymentCapabilitiesTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ListDeploymentCapabilitiesTool {
    const NAME: &'static str = "list_deployment_capabilities";

    type Error = ListDeploymentCapabilitiesError;
    type Args = ListDeploymentCapabilitiesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"List available deployment capabilities for a project.

Returns information about which cloud providers are connected and what deployment
targets are available (clusters, registries, Cloud Run).

**Parameters:**
- project_id: The UUID of the project to check

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`
- User must have access to the project

**What it returns:**
- providers: Array of provider status objects with:
  - provider: Provider name (Gcp, Hetzner, Aws, Azure)
  - is_connected: Whether the provider has cloud credentials
  - cloud_runner_available: Whether Cloud Run/serverless is available
  - clusters: Array of available Kubernetes clusters
  - registries: Array of available container registries
  - summary: Human-readable status

**Use Cases:**
- Before creating a deployment, check what options are available
- Verify a provider is connected before attempting deployment
- Find cluster and registry IDs for deployment configuration"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "The UUID of the project"
                    }
                },
                "required": ["project_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate project_id
        if args.project_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "list_deployment_capabilities",
                ErrorCategory::ValidationFailed,
                "project_id cannot be empty",
                Some(vec![
                    "Use list_projects to find valid project IDs",
                    "Use current_context to get the currently selected project",
                ]),
            ));
        }

        // Create the API client
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_api_error("list_deployment_capabilities", e));
            }
        };

        // Get provider deployment statuses
        match get_provider_deployment_statuses(&client, &args.project_id).await {
            Ok(statuses) => {
                // Count connected providers
                let connected_count = statuses.iter().filter(|s| s.is_connected).count();
                let total_clusters: usize = statuses.iter().map(|s| s.clusters.len()).sum();
                let total_registries: usize = statuses.iter().map(|s| s.registries.len()).sum();

                // Build provider data
                let provider_data: Vec<serde_json::Value> = statuses
                    .iter()
                    .map(|s| {
                        let clusters: Vec<serde_json::Value> = s
                            .clusters
                            .iter()
                            .map(|c| {
                                json!({
                                    "id": c.id,
                                    "name": c.name,
                                    "region": c.region,
                                    "is_healthy": c.is_healthy,
                                })
                            })
                            .collect();

                        let registries: Vec<serde_json::Value> = s
                            .registries
                            .iter()
                            .map(|r| {
                                json!({
                                    "id": r.id,
                                    "name": r.name,
                                    "region": r.region,
                                    "is_ready": r.is_ready,
                                })
                            })
                            .collect();

                        json!({
                            "provider": format!("{:?}", s.provider),
                            "is_connected": s.is_connected,
                            "cloud_runner_available": s.cloud_runner_available,
                            "clusters": clusters,
                            "registries": registries,
                            "summary": s.summary,
                        })
                    })
                    .collect();

                // Build summary
                let summary = if connected_count == 0 {
                    "No providers connected. Connect a cloud provider in platform settings first.".to_string()
                } else {
                    let mut parts = vec![format!("{} provider{} connected", connected_count, if connected_count == 1 { "" } else { "s" })];
                    if total_clusters > 0 {
                        parts.push(format!("{} cluster{}", total_clusters, if total_clusters == 1 { "" } else { "s" }));
                    }
                    if total_registries > 0 {
                        parts.push(format!("{} registr{}", total_registries, if total_registries == 1 { "y" } else { "ies" }));
                    }
                    parts.join(", ")
                };

                let result = json!({
                    "success": true,
                    "project_id": args.project_id,
                    "providers": provider_data,
                    "summary": summary,
                    "connected_providers_count": connected_count,
                    "total_clusters": total_clusters,
                    "total_registries": total_registries,
                    "next_steps": if connected_count > 0 {
                        vec![
                            "Use analyze_project to discover Dockerfiles in the project",
                            "Use create_deployment_config to create a deployment configuration",
                            "For Cloud Run deployments, no cluster is needed"
                        ]
                    } else {
                        vec![
                            "Use open_provider_settings to connect a cloud provider",
                            "After connecting, run this tool again to see available options"
                        ]
                    }
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| ListDeploymentCapabilitiesError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_api_error("list_deployment_capabilities", e)),
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
                "Use list_projects to find valid project IDs",
            ]),
        ),
        PlatformApiError::PermissionDenied(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::PermissionDenied,
            &format!("Permission denied: {}", msg),
            Some(vec![
                "The user does not have access to this project",
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
            Some(vec!["Check the error message for details"]),
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
        assert_eq!(ListDeploymentCapabilitiesTool::NAME, "list_deployment_capabilities");
    }

    #[test]
    fn test_tool_creation() {
        let tool = ListDeploymentCapabilitiesTool::new();
        assert!(format!("{:?}", tool).contains("ListDeploymentCapabilitiesTool"));
    }
}
