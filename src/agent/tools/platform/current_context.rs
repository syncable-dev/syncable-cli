//! Current context tool for the agent
//!
//! Allows the agent to query the currently selected project context.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::PlatformSession;

/// Arguments for the current context tool (none required)
#[derive(Debug, Deserialize)]
pub struct CurrentContextArgs {}

/// Error type for current context operations
#[derive(Debug, thiserror::Error)]
#[error("Current context error: {0}")]
pub struct CurrentContextError(String);

/// Tool to get the currently selected project context
///
/// This tool reads the platform session from `~/.syncable/platform-session.json`
/// and returns information about the selected project and organization.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CurrentContextTool;

impl CurrentContextTool {
    /// Create a new CurrentContextTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for CurrentContextTool {
    const NAME: &'static str = "current_context";

    type Error = CurrentContextError;
    type Args = CurrentContextArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Get the currently selected project context.

Returns information about the currently selected project and organization,
or indicates if no project is selected.

**Use Cases:**
- Checking which project is currently active before operations
- Verifying context after selection
- Determining if context setup is needed

**No Prerequisites:**
- This tool can be called at any time
- Returns helpful message if no project is selected"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Load the platform session
        let session = match PlatformSession::load() {
            Ok(s) => s,
            Err(e) => {
                return Ok(format_error_for_llm(
                    "current_context",
                    ErrorCategory::InternalError,
                    &format!("Failed to load platform session: {}", e),
                    Some(vec![
                        "The session file may be corrupted",
                        "Try selecting a project with select_project",
                    ]),
                ));
            }
        };

        // Check if a project is selected
        if !session.is_project_selected() {
            let result = json!({
                "success": true,
                "has_context": false,
                "message": "No project currently selected",
                "suggestion": "Use list_organizations and list_projects to find a project, then select_project to set context"
            });

            return serde_json::to_string_pretty(&result)
                .map_err(|e| CurrentContextError(format!("Failed to serialize: {}", e)));
        }

        // Return the current context
        let result = json!({
            "success": true,
            "has_context": true,
            "context": {
                "project_id": session.project_id,
                "project_name": session.project_name,
                "organization_id": session.org_id,
                "organization_name": session.org_name,
                "display": session.display_context(),
                "last_updated": session.last_updated.map(|dt| dt.to_rfc3339())
            }
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| CurrentContextError(format!("Failed to serialize: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(CurrentContextTool::NAME, "current_context");
    }

    #[test]
    fn test_tool_creation() {
        let tool = CurrentContextTool::new();
        assert!(format!("{:?}", tool).contains("CurrentContextTool"));
    }
}
