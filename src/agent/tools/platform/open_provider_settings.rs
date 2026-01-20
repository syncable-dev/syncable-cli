//! Open provider settings tool for the agent
//!
//! Opens the cloud providers settings page in the user's browser.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};

/// Arguments for the open provider settings tool
#[derive(Debug, Deserialize)]
pub struct OpenProviderSettingsArgs {
    /// The project ID to open settings for
    pub project_id: String,
}

/// Error type for open provider settings operations
#[derive(Debug, thiserror::Error)]
#[error("Open provider settings error: {0}")]
pub struct OpenProviderSettingsError(String);

/// Tool to open the cloud providers settings page in the browser
///
/// This tool opens the Syncable platform's cloud providers settings page
/// where users can connect their GCP, AWS, Azure, or Hetzner accounts.
///
/// SECURITY NOTE: The actual credential connection happens entirely in the
/// browser through the platform's secure OAuth flow. The CLI agent NEVER
/// handles or sees the actual credentials.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenProviderSettingsTool;

impl OpenProviderSettingsTool {
    /// Create a new OpenProviderSettingsTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for OpenProviderSettingsTool {
    const NAME: &'static str = "open_provider_settings";

    type Error = OpenProviderSettingsError;
    type Args = OpenProviderSettingsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Open the cloud providers settings page in the user's browser.

This opens the Syncable platform's settings page where users can connect their
cloud provider accounts (GCP, AWS, Azure, Hetzner).

**Important:**
- The actual credential connection happens in the browser, NOT through the CLI
- After calling this tool, ask the user to confirm when they've completed the setup
- Use check_provider_connection to verify the connection was successful

**Workflow:**
1. Call open_provider_settings with the project_id
2. Ask user: "Please connect your [provider] account in the browser. Let me know when done."
3. Call check_provider_connection to verify the connection

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`
- User must have a valid project_id (from select_project or list_projects)"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "The UUID of the project to configure cloud providers for"
                    }
                },
                "required": ["project_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate input
        if args.project_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "open_provider_settings",
                ErrorCategory::ValidationFailed,
                "project_id cannot be empty",
                Some(vec![
                    "Use list_projects to find valid project IDs",
                    "Use select_project to set the current project context",
                ]),
            ));
        }

        // Build the settings URL
        let url = format!(
            "https://syncable.dev/projects/{}/settings?tab=cloud-providers",
            args.project_id
        );

        // Open the URL in the default browser
        match open::that(&url) {
            Ok(()) => {
                let result = json!({
                    "success": true,
                    "message": "Opened cloud providers settings in your browser",
                    "url": url,
                    "next_steps": [
                        "Connect your cloud provider account in the browser",
                        "Once done, tell me which provider you connected",
                        "I'll verify the connection with check_provider_connection"
                    ]
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| OpenProviderSettingsError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_error_for_llm(
                "open_provider_settings",
                ErrorCategory::ExternalCommandFailed,
                &format!("Failed to open browser: {}", e),
                Some(vec![
                    &format!("You can manually open: {}", url),
                    "Check if a default browser is configured",
                ]),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(OpenProviderSettingsTool::NAME, "open_provider_settings");
    }

    #[test]
    fn test_tool_creation() {
        let tool = OpenProviderSettingsTool::new();
        assert!(format!("{:?}", tool).contains("OpenProviderSettingsTool"));
    }

    #[test]
    fn test_settings_url_format() {
        let project_id = "proj-12345-uuid";
        let expected_url = format!(
            "https://syncable.dev/projects/{}/settings?tab=cloud-providers",
            project_id
        );
        assert!(expected_url.contains(project_id));
        assert!(expected_url.contains("cloud-providers"));
    }
}
