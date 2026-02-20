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
    /// Optional project ID to check actual deployment status (for public_url)
    pub project_id: Option<String>,
    /// Optional service name to find the specific deployment
    pub service_name: Option<String>,
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
            description: r#"Get the status of a deployment task and optionally check the actual service status.

Returns the current status of a deployment, including progress percentage,
current step, overall status, and optionally the public URL if the service is ready.

**CRITICAL - DO NOT POLL IN A LOOP:**
After checking status, you MUST inform the user and WAIT for them to ask again.
DO NOT call this tool repeatedly in succession. Deployments take 1-3 minutes.
The response includes an "action" field - follow it:
- "STOP_POLLING": Deployment is done (success or failure). Tell the user.
- "INFORM_USER_AND_WAIT": Tell user the current status and wait for them to ask for updates.

**IMPORTANT for Cloud Runner:**
The task may show "completed" when infrastructure is provisioned, but the actual
service build and deployment takes longer. Pass project_id and service_name to
also check if the service has a public URL (meaning it's actually ready).

**Status Values:**
- Task status: "processing", "completed", "failed"
- Overall status: "generating", "building", "deploying", "healthy", "failed"
- Service ready: Only when public_url is available

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`
- A deployment must have been triggered (use trigger_deployment first)

**Use Cases:**
- Check deployment status ONCE after triggering, then inform user
- Let user ask for updates when they want them
- Get error details if deployment failed"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "The deployment task ID (from trigger_deployment response)"
                    },
                    "project_id": {
                        "type": "string",
                        "description": "Optional: Project ID to check actual service status and public URL"
                    },
                    "service_name": {
                        "type": "string",
                        "description": "Optional: Service name to find the specific deployment"
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

        // Get the deployment status (Backstage task)
        match client.get_deployment_status(&args.task_id).await {
            Ok(status) => {
                let task_complete = status.status == "completed";
                let is_failed = status.status == "failed" || status.overall_status == "failed";
                let is_healthy = status.overall_status == "healthy";

                // Also check actual deployment if project_id and service_name provided
                // This is crucial for Cloud Runner where task completes but service takes longer
                let (service_status, public_url, service_ready) =
                    if let (Some(project_id), Some(service_name)) =
                        (&args.project_id, &args.service_name)
                    {
                        match client.list_deployments(project_id, Some(10)).await {
                            Ok(paginated) => {
                                // Find the deployment for this service
                                let deployment = paginated
                                    .data
                                    .iter()
                                    .find(|d| d.service_name.eq_ignore_ascii_case(service_name));

                                match deployment {
                                    Some(d) => (
                                        Some(d.status.clone()),
                                        d.public_url.clone(),
                                        d.public_url.is_some() && d.status == "running",
                                    ),
                                    None => (None, None, false),
                                }
                            }
                            Err(_) => (None, None, false),
                        }
                    } else {
                        (None, None, false)
                    };

                // True completion = task done AND (service has URL or no service check requested)
                let truly_ready = if args.project_id.is_some() {
                    service_ready
                } else {
                    is_healthy
                };

                let mut result = json!({
                    "success": true,
                    "task_id": args.task_id,
                    "task_status": status.status,
                    "task_progress": status.progress,
                    "current_step": status.current_step,
                    "overall_status": status.overall_status,
                    "overall_message": status.overall_message,
                    "task_complete": task_complete,
                    "is_failed": is_failed,
                    "service_ready": truly_ready
                });

                // Add service-specific info if we checked
                if let Some(svc_status) = service_status {
                    result["service_status"] = json!(svc_status);
                }
                if let Some(url) = &public_url {
                    result["public_url"] = json!(url);
                }

                // Add error details if failed
                if let Some(error) = &status.error {
                    result["error"] = json!(error);
                }

                // Add next steps based on actual status
                // IMPORTANT: Guide agent to STOP polling and inform user
                if is_failed {
                    result["next_steps"] = json!([
                        "STOP - Deployment failed. Inform the user of the error.",
                        "Review the error message for details",
                        "Check the deployment configuration",
                        "Verify the code builds successfully locally"
                    ]);
                    result["action"] = json!("STOP_POLLING");
                } else if truly_ready && public_url.is_some() {
                    result["next_steps"] = json!([
                        format!(
                            "STOP - Service is live at: {}",
                            public_url.as_ref().unwrap()
                        ),
                        "Deployment completed successfully!",
                        "Inform the user their service is ready"
                    ]);
                    result["action"] = json!("STOP_POLLING");
                } else if task_complete && !truly_ready {
                    result["next_steps"] = json!([
                        "STOP POLLING - Inform the user that deployment is in progress",
                        "Infrastructure is ready, Cloud Runner is building the container",
                        "Tell the user to wait 1-2 minutes, then they can ask you to check status again",
                        "DO NOT call get_deployment_status again automatically - wait for user to ask"
                    ]);
                    result["action"] = json!("INFORM_USER_AND_WAIT");
                    result["estimated_wait"] = json!("1-2 minutes");
                    result["note"] = json!(
                        "Task shows 100% but container is still being built/deployed. This is normal. DO NOT poll repeatedly - inform the user and wait for them to ask for status."
                    );
                } else if !task_complete {
                    result["next_steps"] = json!([
                        format!(
                            "STOP POLLING - Deployment is {} ({}% complete)",
                            status.overall_status, status.progress
                        ),
                        "Inform the user of current progress",
                        "Tell them to wait and ask again in 30 seconds if they want an update",
                        "DO NOT call get_deployment_status again automatically"
                    ]);
                    result["action"] = json!("INFORM_USER_AND_WAIT");
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
        assert_eq!(GetDeploymentStatusTool::NAME, "get_deployment_status");
    }

    #[test]
    fn test_tool_creation() {
        let tool = GetDeploymentStatusTool::new();
        assert!(format!("{:?}", tool).contains("GetDeploymentStatusTool"));
    }
}
