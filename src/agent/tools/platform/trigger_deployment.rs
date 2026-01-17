//! Trigger deployment tool for the agent
//!
//! Allows the agent to trigger a deployment using a deployment config.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::{PlatformApiClient, PlatformApiError, TriggerDeploymentRequest};

/// Arguments for the trigger deployment tool
#[derive(Debug, Deserialize)]
pub struct TriggerDeploymentArgs {
    /// The project ID for the deployment
    pub project_id: String,
    /// The deployment config ID to use
    pub config_id: String,
    /// Optional specific commit SHA to deploy
    pub commit_sha: Option<String>,
}

/// Error type for trigger deployment operations
#[derive(Debug, thiserror::Error)]
#[error("Trigger deployment error: {0}")]
pub struct TriggerDeploymentError(String);

/// Tool to trigger a deployment using a deployment config
///
/// Starts a new deployment for the specified configuration. Returns a task ID
/// that can be used to monitor deployment progress.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TriggerDeploymentTool;

impl TriggerDeploymentTool {
    /// Create a new TriggerDeploymentTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for TriggerDeploymentTool {
    const NAME: &'static str = "trigger_deployment";

    type Error = TriggerDeploymentError;
    type Args = TriggerDeploymentArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Trigger a deployment using a deployment configuration.

Starts a new deployment for the specified config. Returns a task ID that can be
used to monitor deployment progress with `get_deployment_status`.

**Parameters:**
- project_id: The project UUID
- config_id: The deployment config ID (get from list_deployment_configs)
- commit_sha: Optional specific commit to deploy (defaults to latest on branch)

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`
- A deployment config must exist for the project

**Use Cases:**
- Deploy the latest code from a branch
- Deploy a specific commit version
- Trigger a manual deployment for a service

**Returns:**
- task_id: Use this to check deployment progress with get_deployment_status
- status: Initial deployment status
- message: Human-readable status message"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "The UUID of the project"
                    },
                    "config_id": {
                        "type": "string",
                        "description": "The deployment config ID (from list_deployment_configs)"
                    },
                    "commit_sha": {
                        "type": "string",
                        "description": "Optional: specific commit SHA to deploy (defaults to latest)"
                    }
                },
                "required": ["project_id", "config_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate project_id
        if args.project_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "trigger_deployment",
                ErrorCategory::ValidationFailed,
                "project_id cannot be empty",
                Some(vec![
                    "Use list_projects to find valid project IDs",
                    "Use select_project to set the current project context",
                ]),
            ));
        }

        // Validate config_id
        if args.config_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "trigger_deployment",
                ErrorCategory::ValidationFailed,
                "config_id cannot be empty",
                Some(vec![
                    "Use list_deployment_configs to find available deployment configs",
                ]),
            ));
        }

        // Create the API client
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_api_error("trigger_deployment", e));
            }
        };

        // Build the request
        let request = TriggerDeploymentRequest {
            project_id: args.project_id.clone(),
            config_id: args.config_id.clone(),
            commit_sha: args.commit_sha.clone(),
        };

        // Trigger the deployment
        match client.trigger_deployment(&request).await {
            Ok(response) => {
                let result = json!({
                    "success": true,
                    "task_id": response.backstage_task_id,
                    "config_id": response.config_id,
                    "status": response.status,
                    "message": response.message,
                    "next_steps": [
                        format!("Use get_deployment_status with task_id '{}' to monitor progress", response.backstage_task_id),
                        "Deployment typically takes 2-5 minutes to complete"
                    ]
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| TriggerDeploymentError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_api_error("trigger_deployment", e)),
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
                "The project ID or config ID may be incorrect",
                "Use list_deployment_configs to find valid config IDs",
            ]),
        ),
        PlatformApiError::PermissionDenied(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::PermissionDenied,
            &format!("Permission denied: {}", msg),
            Some(vec![
                "The user does not have permission to trigger deployments",
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
        assert_eq!(TriggerDeploymentTool::NAME, "trigger_deployment");
    }

    #[test]
    fn test_tool_creation() {
        let tool = TriggerDeploymentTool::new();
        assert!(format!("{:?}", tool).contains("TriggerDeploymentTool"));
    }
}
