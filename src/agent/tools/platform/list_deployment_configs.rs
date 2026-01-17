//! List deployment configs tool for the agent
//!
//! Allows the agent to list deployment configurations for a project.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::{PlatformApiClient, PlatformApiError};

/// Arguments for the list deployment configs tool
#[derive(Debug, Deserialize)]
pub struct ListDeploymentConfigsArgs {
    /// The project ID to list deployment configs for
    pub project_id: String,
}

/// Error type for list deployment configs operations
#[derive(Debug, thiserror::Error)]
#[error("List deployment configs error: {0}")]
pub struct ListDeploymentConfigsError(String);

/// Tool to list deployment configurations for a project
///
/// Returns all deployment configs with service names, branches, target types,
/// and auto-deploy settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListDeploymentConfigsTool;

impl ListDeploymentConfigsTool {
    /// Create a new ListDeploymentConfigsTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ListDeploymentConfigsTool {
    const NAME: &'static str = "list_deployment_configs";

    type Error = ListDeploymentConfigsError;
    type Args = ListDeploymentConfigsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"List deployment configurations for a project.

Returns all deployment configs associated with the project, including:
- Service name and branch
- Target type (kubernetes or cloud_runner)
- Auto-deploy status
- Port configuration

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`
- A project must be selected (use select_project first)

**Use Cases:**
- View available deployment configurations before triggering a deployment
- Check auto-deploy settings for services
- Find the config_id needed to trigger a deployment"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "The UUID of the project to list deployment configs for"
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
                "list_deployment_configs",
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
                return Ok(format_api_error("list_deployment_configs", e));
            }
        };

        // Fetch deployment configs
        match client.list_deployment_configs(&args.project_id).await {
            Ok(configs) => {
                if configs.is_empty() {
                    return Ok(json!({
                        "success": true,
                        "configs": [],
                        "count": 0,
                        "message": "No deployment configs found for this project. You may need to create a deployment configuration first."
                    })
                    .to_string());
                }

                let config_list: Vec<serde_json::Value> = configs
                    .iter()
                    .map(|config| {
                        json!({
                            "id": config.id,
                            "service_name": config.service_name,
                            "repository": config.repository_full_name,
                            "branch": config.branch,
                            "target_type": config.target_type,
                            "port": config.port,
                            "auto_deploy_enabled": config.auto_deploy_enabled,
                            "deployment_strategy": config.deployment_strategy,
                            "environment_id": config.environment_id,
                            "created_at": config.created_at.to_rfc3339()
                        })
                    })
                    .collect();

                let result = json!({
                    "success": true,
                    "configs": config_list,
                    "count": configs.len(),
                    "message": format!("Found {} deployment configuration(s)", configs.len())
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| ListDeploymentConfigsError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_api_error("list_deployment_configs", e)),
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
        assert_eq!(ListDeploymentConfigsTool::NAME, "list_deployment_configs");
    }

    #[test]
    fn test_tool_creation() {
        let tool = ListDeploymentConfigsTool::new();
        assert!(format!("{:?}", tool).contains("ListDeploymentConfigsTool"));
    }
}
