//! List deployments tool for the agent
//!
//! Allows the agent to list recent deployments for a project.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::{PlatformApiClient, PlatformApiError};

/// Arguments for the list deployments tool
#[derive(Debug, Deserialize)]
pub struct ListDeploymentsArgs {
    /// The project ID to list deployments for
    pub project_id: String,
    /// Optional limit on number of deployments to return (default 10)
    pub limit: Option<i32>,
}

/// Error type for list deployments operations
#[derive(Debug, thiserror::Error)]
#[error("List deployments error: {0}")]
pub struct ListDeploymentsError(String);

/// Tool to list recent deployments for a project
///
/// Returns a paginated list of deployments with status, commit info, and public URLs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListDeploymentsTool;

impl ListDeploymentsTool {
    /// Create a new ListDeploymentsTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ListDeploymentsTool {
    const NAME: &'static str = "list_deployments";

    type Error = ListDeploymentsError;
    type Args = ListDeploymentsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"List recent deployments for a project.

Returns a list of deployments with their status, commit SHA, public URLs,
and creation timestamps.

**Parameters:**
- project_id: The project UUID
- limit: Optional number of deployments to return (default 10)

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`

**Use Cases:**
- View deployment history for a project
- Find the public URL of a deployed service
- Check the status of recent deployments
- Get task IDs for checking deployment status"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "The UUID of the project to list deployments for"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Optional: number of deployments to return (default 10)"
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
                "list_deployments",
                ErrorCategory::ValidationFailed,
                "project_id cannot be empty",
                Some(vec![
                    "Use list_projects to find valid project IDs",
                    "Use select_project to set the current project context",
                ]),
            ));
        }

        // Create the API client
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_api_error("list_deployments", e));
            }
        };

        // Fetch deployments
        match client.list_deployments(&args.project_id, args.limit).await {
            Ok(paginated) => {
                if paginated.data.is_empty() {
                    return Ok(json!({
                        "success": true,
                        "deployments": [],
                        "count": 0,
                        "has_more": false,
                        "message": "No deployments found for this project. Use trigger_deployment to start a deployment."
                    })
                    .to_string());
                }

                let deployment_list: Vec<serde_json::Value> = paginated
                    .data
                    .iter()
                    .map(|deployment| {
                        json!({
                            "id": deployment.id,
                            "service_name": deployment.service_name,
                            "repository": deployment.repository_full_name,
                            "status": deployment.status,
                            "task_id": deployment.backstage_task_id,
                            "commit_sha": deployment.commit_sha,
                            "public_url": deployment.public_url,
                            "created_at": deployment.created_at.to_rfc3339()
                        })
                    })
                    .collect();

                let result = json!({
                    "success": true,
                    "deployments": deployment_list,
                    "count": paginated.data.len(),
                    "has_more": paginated.pagination.has_more,
                    "next_cursor": paginated.pagination.next_cursor,
                    "message": format!("Found {} deployment(s)", paginated.data.len())
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| ListDeploymentsError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_api_error("list_deployments", e)),
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
        assert_eq!(ListDeploymentsTool::NAME, "list_deployments");
    }

    #[test]
    fn test_tool_creation() {
        let tool = ListDeploymentsTool::new();
        assert!(format!("{:?}", tool).contains("ListDeploymentsTool"));
    }
}
