//! Select project tool for the agent
//!
//! Allows the agent to select a project as the current context for platform operations.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::{PlatformApiClient, PlatformApiError};
use crate::platform::PlatformSession;

/// Arguments for the select project tool
#[derive(Debug, Deserialize)]
pub struct SelectProjectArgs {
    /// The project ID to select
    pub project_id: String,
    /// The organization ID the project belongs to
    pub organization_id: String,
}

/// Error type for select project operations
#[derive(Debug, thiserror::Error)]
#[error("Select project error: {0}")]
pub struct SelectProjectError(String);

/// Tool to select a project as the current context
///
/// This tool sets the current project context for platform operations.
/// The selection is persisted to `~/.syncable/platform-session.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SelectProjectTool;

impl SelectProjectTool {
    /// Create a new SelectProjectTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for SelectProjectTool {
    const NAME: &'static str = "select_project";

    type Error = SelectProjectError;
    type Args = SelectProjectArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Select a project as the current context for platform operations.

This persists the selection so future operations will use this project context.
The selection is stored in ~/.syncable/platform-session.json.

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`
- The project_id and organization_id must be valid

**Use Cases:**
- Setting up context before creating tasks or deployments
- Switching between projects
- Establishing project context for platform-aware operations

**Workflow:**
1. Use list_organizations to find the organization
2. Use list_projects to find the project within the organization
3. Call select_project with both IDs"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "The UUID of the project to select"
                    },
                    "organization_id": {
                        "type": "string",
                        "description": "The UUID of the organization the project belongs to"
                    }
                },
                "required": ["project_id", "organization_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate inputs
        if args.project_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "select_project",
                ErrorCategory::ValidationFailed,
                "project_id cannot be empty",
                Some(vec![
                    "Use list_projects to find valid project IDs",
                    "Pass the project ID as a UUID string",
                ]),
            ));
        }

        if args.organization_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "select_project",
                ErrorCategory::ValidationFailed,
                "organization_id cannot be empty",
                Some(vec![
                    "Use list_organizations to find valid organization IDs",
                    "Pass the organization ID as a UUID string",
                ]),
            ));
        }

        // Create the API client
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_api_error("select_project", e));
            }
        };

        // Verify project exists and user has access
        let project = match client.get_project(&args.project_id).await {
            Ok(p) => p,
            Err(e) => {
                return Ok(format_api_error("select_project", e));
            }
        };

        // Verify organization exists and user has access
        let organization = match client.get_organization(&args.organization_id).await {
            Ok(o) => o,
            Err(e) => {
                return Ok(format_api_error("select_project", e));
            }
        };

        // Verify the project belongs to the specified organization
        if project.organization_id != args.organization_id {
            return Ok(format_error_for_llm(
                "select_project",
                ErrorCategory::ValidationFailed,
                "Project does not belong to the specified organization",
                Some(vec![
                    &format!(
                        "Project '{}' belongs to organization '{}'",
                        project.name, project.organization_id
                    ),
                    "Use the correct organization_id for this project",
                ]),
            ));
        }

        // Create and save the session
        let session = PlatformSession::with_project(
            project.id.clone(),
            project.name.clone(),
            organization.id.clone(),
            organization.name.clone(),
        );

        if let Err(e) = session.save() {
            return Ok(format_error_for_llm(
                "select_project",
                ErrorCategory::InternalError,
                &format!("Failed to save session: {}", e),
                Some(vec![
                    "The session could not be persisted to disk",
                    "Check permissions on ~/.syncable/ directory",
                ]),
            ));
        }

        // Return success response
        let result = json!({
            "success": true,
            "message": format!("Selected project '{}' in organization '{}'", project.name, organization.name),
            "context": {
                "project_id": project.id,
                "project_name": project.name,
                "organization_id": organization.id,
                "organization_name": organization.name
            },
            "session_path": PlatformSession::session_path().display().to_string()
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| SelectProjectError(format!("Failed to serialize: {}", e)))
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
                "The project or organization ID may be incorrect",
                "Use list_organizations and list_projects to find valid IDs",
            ]),
        ),
        PlatformApiError::PermissionDenied(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::PermissionDenied,
            &format!("Permission denied: {}", msg),
            Some(vec![
                "The user does not have access to this resource",
                "Contact the organization or project admin for access",
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
        assert_eq!(SelectProjectTool::NAME, "select_project");
    }

    #[test]
    fn test_tool_creation() {
        let tool = SelectProjectTool::new();
        assert!(format!("{:?}", tool).contains("SelectProjectTool"));
    }
}
