//! Get deployment status tool for the agent
//!
//! Allows the agent to check the status of a deployment task.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::{PlatformApiClient, PlatformApiError};

/// Arguments for the get deployment status tool
#[derive(Debug, Deserialize)]
pub struct GetDeploymentStatusArgs {
    /// The task ID to check status for
    pub task_id: String,
}

/// Error type for get deployment status operations
#[derive(Debug, thiserror::Error)]
#[error("Get deployment status error: {0}")]
pub struct GetDeploymentStatusError(String);

/// Tool to get deployment task status
///
/// Returns the current status of a deployment including progress percentage,
/// current step, and overall status.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetDeploymentStatusTool;

impl GetDeploymentStatusTool {
    /// Create a new GetDeploymentStatusTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GetDeploymentStatusTool {
    const NAME: &'static str = "get_deployment_status";

    type Error = GetDeploymentStatusError;
    type Args = GetDeploymentStatusArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Get the status of a deployment task.

Returns the current status of a deployment, including progress percentage,
current step, and overall status.

**Status Values:**
- Task status: "processing", "completed", "failed"
- Overall status: "generating", "building", "deploying", "healthy", "failed"

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`
- A deployment must have been triggered (use trigger_deployment first)

**Use Cases:**
- Monitor deployment progress after triggering
- Check if a deployment has completed
- Get error details if deployment failed"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "The deployment task ID (from trigger_deployment response)"
                    }
                },
                "required": ["task_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate task_id
        if args.task_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "get_deployment_status",
                ErrorCategory::ValidationFailed,
                "task_id cannot be empty",
                Some(vec![
                    "Use trigger_deployment to start a deployment and get a task_id",
                    "Use list_deployments to find previous deployment task IDs",
                ]),
            ));
        }

        // Create the API client
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_api_error("get_deployment_status", e));
            }
        };

        // Get the deployment status
        match client.get_deployment_status(&args.task_id).await {
            Ok(status) => {
                let is_complete = status.status == "completed";
                let is_failed = status.status == "failed" || status.overall_status == "failed";
                let is_healthy = status.overall_status == "healthy";

                let mut result = json!({
                    "success": true,
                    "task_id": args.task_id,
                    "status": status.status,
                    "progress": status.progress,
                    "current_step": status.current_step,
                    "overall_status": status.overall_status,
                    "overall_message": status.overall_message,
                    "is_complete": is_complete,
                    "is_failed": is_failed,
                    "is_healthy": is_healthy
                });

                // Add error details if failed
                if let Some(error) = &status.error {
                    result["error"] = json!(error);
                }

                // Add next steps based on status
                if is_failed {
                    result["next_steps"] = json!([
                        "Review the error message for details",
                        "Check the deployment configuration",
                        "Verify the code builds successfully locally",
                        "Try triggering a new deployment after fixing the issue"
                    ]);
                } else if is_healthy {
                    result["next_steps"] = json!([
                        "Deployment completed successfully",
                        "Use list_deployments to see the deployed service details",
                        "Check the public_url to access the deployed service"
                    ]);
                } else if !is_complete {
                    result["next_steps"] = json!([
                        format!("Deployment is {} ({}% complete)", status.overall_status, status.progress),
                        "Call get_deployment_status again to check progress"
                    ]);
                }

                serde_json::to_string_pretty(&result)
                    .map_err(|e| GetDeploymentStatusError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_api_error("get_deployment_status", e)),
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
            &format!("Deployment task not found: {}", msg),
            Some(vec![
                "The task_id may be incorrect or expired",
                "Use trigger_deployment to start a new deployment",
            ]),
        ),
        PlatformApiError::PermissionDenied(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::PermissionDenied,
            &format!("Permission denied: {}", msg),
            Some(vec![
                "The user does not have access to this deployment",
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(GetDeploymentStatusTool::NAME, "get_deployment_status");
    }

    #[test]
    fn test_tool_creation() {
        let tool = GetDeploymentStatusTool::new();
        assert!(format!("{:?}", tool).contains("GetDeploymentStatusTool"));
    }
}
