//! Check provider connection tool for the agent
//!
//! Checks if a cloud provider is connected to a project.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::{CloudProvider, PlatformApiClient, PlatformApiError};

/// Arguments for the check provider connection tool
#[derive(Debug, Deserialize)]
pub struct CheckProviderConnectionArgs {
    /// The project ID to check
    pub project_id: String,
    /// The cloud provider to check (gcp, aws, azure, hetzner)
    pub provider: String,
}

/// Error type for check provider connection operations
#[derive(Debug, thiserror::Error)]
#[error("Check provider connection error: {0}")]
pub struct CheckProviderConnectionError(String);

/// Tool to check if a cloud provider is connected to a project
///
/// SECURITY NOTE: This tool only returns connection STATUS (connected/not connected).
/// It NEVER returns actual credentials, tokens, or API keys. The agent should never
/// have access to sensitive authentication material.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckProviderConnectionTool;

impl CheckProviderConnectionTool {
    /// Create a new CheckProviderConnectionTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for CheckProviderConnectionTool {
    const NAME: &'static str = "check_provider_connection";

    type Error = CheckProviderConnectionError;
    type Args = CheckProviderConnectionArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Check if a cloud provider is connected to a project.

Returns connection status (connected or not connected) for the specified provider.
This tool NEVER returns actual credentials - only connection status.

**Supported Providers:**
- gcp (Google Cloud Platform)
- aws (Amazon Web Services)
- azure (Microsoft Azure)
- hetzner (Hetzner Cloud)

**Use Cases:**
- Verify a provider was connected after user completes setup in browser
- Check prerequisites before deployment operations
- Determine which providers are available for a project

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`
- A project must be selected (use select_project first)"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "The UUID of the project to check"
                    },
                    "provider": {
                        "type": "string",
                        "enum": ["gcp", "aws", "azure", "hetzner"],
                        "description": "The cloud provider to check: gcp, aws, azure, or hetzner"
                    }
                },
                "required": ["project_id", "provider"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate project_id
        if args.project_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "check_provider_connection",
                ErrorCategory::ValidationFailed,
                "project_id cannot be empty",
                Some(vec![
                    "Use list_projects to find valid project IDs",
                    "Use select_project to set the current project context",
                ]),
            ));
        }

        // Parse and validate provider
        let provider: CloudProvider = match args.provider.parse() {
            Ok(p) => p,
            Err(_) => {
                return Ok(format_error_for_llm(
                    "check_provider_connection",
                    ErrorCategory::ValidationFailed,
                    &format!("Invalid provider: '{}'. Must be one of: gcp, aws, azure, hetzner", args.provider),
                    Some(vec![
                        "Use 'gcp' for Google Cloud Platform",
                        "Use 'aws' for Amazon Web Services",
                        "Use 'azure' for Microsoft Azure",
                        "Use 'hetzner' for Hetzner Cloud",
                    ]),
                ));
            }
        };

        // Create the API client
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_api_error("check_provider_connection", e));
            }
        };

        // Check the connection status
        match client.check_provider_connection(&provider, &args.project_id).await {
            Ok(Some(status)) => {
                // Provider is connected
                let result = json!({
                    "connected": true,
                    "provider": provider.as_str(),
                    "provider_name": provider.display_name(),
                    "project_id": args.project_id,
                    "credential_id": status.id,
                    "message": format!("{} is connected to this project", provider.display_name())
                    // NOTE: We intentionally do NOT include any credential values here
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| CheckProviderConnectionError(format!("Failed to serialize: {}", e)))
            }
            Ok(None) => {
                // Provider is NOT connected
                let result = json!({
                    "connected": false,
                    "provider": provider.as_str(),
                    "provider_name": provider.display_name(),
                    "project_id": args.project_id,
                    "message": format!("{} is NOT connected to this project", provider.display_name()),
                    "next_steps": [
                        "Use open_provider_settings to open the settings page",
                        "Have the user connect their account in the browser",
                        "Call check_provider_connection again to verify"
                    ]
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| CheckProviderConnectionError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_api_error("check_provider_connection", e)),
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
        assert_eq!(CheckProviderConnectionTool::NAME, "check_provider_connection");
    }

    #[test]
    fn test_tool_creation() {
        let tool = CheckProviderConnectionTool::new();
        assert!(format!("{:?}", tool).contains("CheckProviderConnectionTool"));
    }

    #[test]
    fn test_provider_parsing() {
        assert!("gcp".parse::<CloudProvider>().is_ok());
        assert!("aws".parse::<CloudProvider>().is_ok());
        assert!("azure".parse::<CloudProvider>().is_ok());
        assert!("hetzner".parse::<CloudProvider>().is_ok());
        assert!("invalid".parse::<CloudProvider>().is_err());
    }
}
