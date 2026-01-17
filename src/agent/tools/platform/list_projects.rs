//! List projects tool for the agent
//!
//! Allows the agent to list all projects within an organization.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::{PlatformApiClient, PlatformApiError};

/// Arguments for the list projects tool
#[derive(Debug, Deserialize)]
pub struct ListProjectsArgs {
    /// The organization ID to list projects for
    pub organization_id: String,
}

/// Error type for list projects operations
#[derive(Debug, thiserror::Error)]
#[error("List projects error: {0}")]
pub struct ListProjectsError(String);

/// Tool to list all projects within an organization
///
/// This tool queries the Syncable Platform API to retrieve all projects
/// in the specified organization that the user has access to.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListProjectsTool;

impl ListProjectsTool {
    /// Create a new ListProjectsTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ListProjectsTool {
    const NAME: &'static str = "list_projects";

    type Error = ListProjectsError;
    type Args = ListProjectsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"List all projects within an organization.

Returns a list of projects with their IDs, names, and descriptions.
Use this after getting organization IDs from list_organizations.

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`
- User must have access to the specified organization

**Use Cases:**
- Finding project IDs to select a project context
- Discovering available projects in an organization
- Getting project details before selection"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "organization_id": {
                        "type": "string",
                        "description": "The UUID of the organization to list projects for"
                    }
                },
                "required": ["organization_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate organization_id is not empty
        if args.organization_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "list_projects",
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
                return Ok(format_api_error("list_projects", e));
            }
        };

        // Fetch projects for the organization
        match client.list_projects(&args.organization_id).await {
            Ok(projects) => {
                if projects.is_empty() {
                    return Ok(json!({
                        "success": true,
                        "organization_id": args.organization_id,
                        "projects": [],
                        "count": 0,
                        "message": "No projects found in this organization. You may need to create a project."
                    })
                    .to_string());
                }

                let project_list: Vec<serde_json::Value> = projects
                    .iter()
                    .map(|proj| {
                        json!({
                            "id": proj.id,
                            "name": proj.name,
                            "description": proj.description,
                            "organization_id": proj.organization_id,
                            "created_at": proj.created_at.to_rfc3339()
                        })
                    })
                    .collect();

                let result = json!({
                    "success": true,
                    "organization_id": args.organization_id,
                    "projects": project_list,
                    "count": projects.len()
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| ListProjectsError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_api_error("list_projects", e)),
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
            &format!("Organization not found: {}", msg),
            Some(vec![
                "The organization ID may be incorrect",
                "Use list_organizations to find valid organization IDs",
            ]),
        ),
        PlatformApiError::PermissionDenied(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::PermissionDenied,
            &format!("Permission denied: {}", msg),
            Some(vec![
                "The user does not have access to this organization",
                "Contact the organization admin for access",
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
        assert_eq!(ListProjectsTool::NAME, "list_projects");
    }

    #[test]
    fn test_tool_creation() {
        let tool = ListProjectsTool::new();
        assert!(format!("{:?}", tool).contains("ListProjectsTool"));
    }
}
