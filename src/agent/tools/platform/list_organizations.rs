//! List organizations tool for the agent
//!
//! Allows the agent to list all organizations the authenticated user belongs to.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::{PlatformApiClient, PlatformApiError};

/// Arguments for the list organizations tool (none required)
#[derive(Debug, Deserialize)]
pub struct ListOrganizationsArgs {}

/// Error type for list organizations operations
#[derive(Debug, thiserror::Error)]
#[error("List organizations error: {0}")]
pub struct ListOrganizationsError(String);

/// Tool to list all organizations the authenticated user belongs to
///
/// This tool queries the Syncable Platform API to retrieve all organizations
/// that the currently authenticated user is a member of.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListOrganizationsTool;

impl ListOrganizationsTool {
    /// Create a new ListOrganizationsTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ListOrganizationsTool {
    const NAME: &'static str = "list_organizations";

    type Error = ListOrganizationsError;
    type Args = ListOrganizationsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"List all organizations the authenticated user belongs to.

Returns a list of organizations with their IDs, names, and slugs.
Use this to discover available organizations before listing projects.

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`

**Use Cases:**
- Finding the organization ID to list projects
- Discovering which organizations the user has access to
- Getting organization details for project selection"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Create the API client
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_api_error("list_organizations", e));
            }
        };

        // Fetch organizations
        match client.list_organizations().await {
            Ok(orgs) => {
                if orgs.is_empty() {
                    return Ok(json!({
                        "success": true,
                        "organizations": [],
                        "count": 0,
                        "message": "No organizations found. You may need to create or join an organization."
                    })
                    .to_string());
                }

                let org_list: Vec<serde_json::Value> = orgs
                    .iter()
                    .map(|org| {
                        json!({
                            "id": org.id,
                            "name": org.name,
                            "slug": org.slug,
                            "created_at": org.created_at.to_rfc3339()
                        })
                    })
                    .collect();

                let result = json!({
                    "success": true,
                    "organizations": org_list,
                    "count": orgs.len()
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| ListOrganizationsError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_api_error("list_organizations", e)),
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
            Some(vec!["The requested resource does not exist"]),
        ),
        PlatformApiError::PermissionDenied(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::PermissionDenied,
            &format!("Permission denied: {}", msg),
            Some(vec!["The user does not have access to this resource"]),
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
        assert_eq!(ListOrganizationsTool::NAME, "list_organizations");
    }

    #[test]
    fn test_tool_creation() {
        let tool = ListOrganizationsTool::new();
        assert!(format!("{:?}", tool).contains("ListOrganizationsTool"));
    }
}
