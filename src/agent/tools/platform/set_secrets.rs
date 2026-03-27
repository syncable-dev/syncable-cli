//! Set deployment secrets tool for the agent
//!
//! Allows the agent to set environment variables and secrets on a deployment config.
//! SECURITY: Secret values are NEVER returned in tool responses. Only key names are confirmed.
//! For secrets (is_secret=true), values are collected via terminal prompt — the LLM never sees them.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;

use crate::agent::tools::ExecutionContext;
use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::types::DeploymentSecretInput;
use crate::platform::api::{PlatformApiClient, PlatformApiError};

/// Result of prompting the user for a secret value in the terminal.
pub(super) enum SecretPromptResult {
    /// User entered a non-empty value
    Value(String),
    /// User skipped this secret (Esc or empty input)
    Skipped,
    /// User cancelled all secret entry (Ctrl+C)
    Cancelled,
}

/// Prompt the user for a secret value using masked terminal input.
///
/// The value is collected directly from the terminal and never enters the LLM context.
pub(super) fn prompt_secret_value(key_name: &str) -> SecretPromptResult {
    use colored::Colorize;
    use inquire::{InquireError, Password, PasswordDisplayMode};

    println!();
    println!(
        "  {} Enter value for {} {}",
        "\u{1f512}".dimmed(),
        key_name.cyan(),
        "(hidden \u{2014} not visible to AI agent)".dimmed()
    );

    match Password::new(key_name)
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_help_message("Esc to skip, Ctrl+C to cancel all")
        .without_confirmation()
        .prompt()
    {
        Ok(v) if v.trim().is_empty() => SecretPromptResult::Skipped,
        Ok(v) => {
            println!("  {} {} set", "\u{2713}".green(), key_name.cyan());
            SecretPromptResult::Value(v)
        }
        Err(InquireError::OperationCanceled) => SecretPromptResult::Skipped,
        Err(InquireError::OperationInterrupted) => SecretPromptResult::Cancelled,
        Err(_) => SecretPromptResult::Cancelled,
    }
}

/// A single secret argument from the agent
#[derive(Debug, Deserialize)]
pub struct SecretArg {
    /// Environment variable name
    pub key: String,
    /// Environment variable value.
    /// OMIT for secrets (is_secret=true) — user will be prompted in terminal.
    /// Provide for non-secrets (NODE_ENV, PORT, etc.)
    pub value: Option<String>,
    /// Whether this is a secret (masked in responses). Default: true for safety
    #[serde(default = "default_true")]
    pub is_secret: bool,
}

pub(super) fn default_true() -> bool {
    true
}

/// Arguments for the set deployment secrets tool
#[derive(Debug, Deserialize)]
pub struct SetDeploymentSecretsArgs {
    /// Deployment config ID to set secrets on
    pub config_id: String,
    /// Environment variables to set
    pub secrets: Vec<SecretArg>,
}

/// Error type for set deployment secrets operations
#[derive(Debug, thiserror::Error)]
#[error("Set deployment secrets error: {0}")]
pub struct SetDeploymentSecretsError(String);

/// Tool to set environment variables and secrets on a deployment configuration.
///
/// SECURITY: Secret values are sent securely to the backend and stored encrypted.
/// Values are NEVER included in tool responses - only key names are confirmed.
/// For secrets, values are collected via terminal prompt — the LLM never sees them.
#[derive(Debug, Clone)]
pub struct SetDeploymentSecretsTool {
    execution_context: ExecutionContext,
}

impl SetDeploymentSecretsTool {
    /// Create a new SetDeploymentSecretsTool (defaults to InteractiveCli)
    pub fn new() -> Self {
        Self {
            execution_context: ExecutionContext::InteractiveCli,
        }
    }

    /// Create with explicit execution context
    pub fn with_context(ctx: ExecutionContext) -> Self {
        Self {
            execution_context: ctx,
        }
    }
}

impl Default for SetDeploymentSecretsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for SetDeploymentSecretsTool {
    const NAME: &'static str = "set_deployment_secrets";

    type Error = SetDeploymentSecretsError;
    type Args = SetDeploymentSecretsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Set environment variables and secrets on a deployment configuration.

Secret values are sent securely to the backend and stored encrypted.
Values are NEVER returned in tool responses - only key names are confirmed.

The is_secret flag (default: true) controls:
- true: Value masked as "********" in UI and API responses, passed via secure terraform -var flags
- false: Value visible in UI, stored in GitOps ConfigMap

For secrets (is_secret=true): OMIT the "value" field. The user will be
prompted securely in the terminal. The value goes directly to the backend.
NEVER ask the user to type secret values in chat.

For non-secrets (is_secret=false): Include the "value" field directly.

Common secrets: DATABASE_URL, API_KEY, JWT_SECRET, REDIS_URL, etc.
Common non-secrets: NODE_ENV, PORT, LOG_LEVEL, APP_NAME, etc.

**Parameters:**
- config_id: The deployment config ID (get from deploy_service or list_deployment_configs)
- secrets: Array of {key, value?, is_secret} objects

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`
- A deployment config must exist (create one with deploy_service first)

**Example:**
Set DATABASE_URL as a secret (value omitted — prompted in terminal) and NODE_ENV as a plain env var:
```json
{
  "config_id": "config-123",
  "secrets": [
    {"key": "DATABASE_URL", "is_secret": true},
    {"key": "NODE_ENV", "value": "production", "is_secret": false}
  ]
}
```

**IMPORTANT - After setting secrets:**
- Trigger a new deployment for the secrets to take effect
- Use trigger_deployment or deploy_service with preview_only=false"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "config_id": {
                        "type": "string",
                        "description": "The deployment config ID to set secrets on"
                    },
                    "secrets": {
                        "type": "array",
                        "description": "Environment variables to set. For secrets, omit value \u{2014} user is prompted in terminal.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "key": {
                                    "type": "string",
                                    "description": "Environment variable name (e.g., DATABASE_URL)"
                                },
                                "value": {
                                    "type": "string",
                                    "description": "Environment variable value. Omit for secrets \u{2014} user will be prompted securely in terminal."
                                },
                                "is_secret": {
                                    "type": "boolean",
                                    "description": "Whether this is a secret (default: true). Secrets are masked in UI and API responses.",
                                    "default": true
                                }
                            },
                            "required": ["key"]
                        }
                    }
                },
                "required": ["config_id", "secrets"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate config_id
        if args.config_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "set_deployment_secrets",
                ErrorCategory::ValidationFailed,
                "config_id cannot be empty",
                Some(vec![
                    "Use list_deployment_configs to find valid config IDs",
                    "Or deploy a service first with deploy_service",
                ]),
            ));
        }

        // Validate secrets list
        if args.secrets.is_empty() {
            return Ok(format_error_for_llm(
                "set_deployment_secrets",
                ErrorCategory::ValidationFailed,
                "secrets array cannot be empty",
                Some(vec!["Provide at least one secret with key and value"]),
            ));
        }

        // Validate key format
        for secret in &args.secrets {
            if secret.key.trim().is_empty() {
                return Ok(format_error_for_llm(
                    "set_deployment_secrets",
                    ErrorCategory::ValidationFailed,
                    "Secret key cannot be empty",
                    Some(vec!["Each secret must have a non-empty key name"]),
                ));
            }
        }

        // Resolve values — prompt for missing secret values in CLI mode
        let mut resolved_secrets: Vec<DeploymentSecretInput> = Vec::new();
        for secret in &args.secrets {
            let value = match &secret.value {
                Some(v) => v.clone(),
                None if self.execution_context.has_terminal() => {
                    match prompt_secret_value(&secret.key) {
                        SecretPromptResult::Value(v) => v,
                        SecretPromptResult::Skipped => continue,
                        SecretPromptResult::Cancelled => {
                            return Ok(format_error_for_llm(
                                "set_deployment_secrets",
                                ErrorCategory::ValidationFailed,
                                "Secret entry cancelled by user",
                                Some(vec![
                                    "The user cancelled secret input. Try again when ready.",
                                ]),
                            ));
                        }
                    }
                }
                None => {
                    return Ok(format_error_for_llm(
                        "set_deployment_secrets",
                        ErrorCategory::ValidationFailed,
                        &format!(
                            "Value required for secret '{}' in server mode (no terminal available)",
                            secret.key
                        ),
                        Some(vec![
                            "In server mode, all secrets must include a value",
                            "The frontend should collect secret values via its own password UI",
                        ]),
                    ));
                }
            };
            resolved_secrets.push(DeploymentSecretInput {
                key: secret.key.clone(),
                value,
                is_secret: secret.is_secret,
            });
        }

        if resolved_secrets.is_empty() {
            return Ok(format_error_for_llm(
                "set_deployment_secrets",
                ErrorCategory::ValidationFailed,
                "All secrets were skipped",
                Some(vec!["Provide at least one secret value when prompted"]),
            ));
        }

        // Create the API client
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_api_error("set_deployment_secrets", e));
            }
        };

        // Call the API
        match client
            .update_deployment_config_secrets(&args.config_id, &resolved_secrets)
            .await
        {
            Ok(()) => {
                let secret_count = resolved_secrets.iter().filter(|s| s.is_secret).count();
                let plain_count = resolved_secrets.len() - secret_count;

                // SECURITY: Response contains ONLY keys, never values
                let secrets_set: Vec<serde_json::Value> = resolved_secrets
                    .iter()
                    .map(|s| {
                        json!({
                            "key": s.key,
                            "is_secret": s.is_secret,
                        })
                    })
                    .collect();

                let result = json!({
                    "success": true,
                    "config_id": args.config_id,
                    "secrets_set": secrets_set,
                    "message": format!(
                        "Set {} environment variable(s) ({} secret, {} plain)",
                        resolved_secrets.len(),
                        secret_count,
                        plain_count
                    ),
                    "next_steps": [
                        "Trigger a new deployment for the secrets to take effect",
                        format!("Use trigger_deployment with config_id '{}'", args.config_id),
                    ],
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| SetDeploymentSecretsError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_api_error("set_deployment_secrets", e)),
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
            &format!("Deployment config not found: {}", msg),
            Some(vec![
                "The config_id may be incorrect",
                "Use list_deployment_configs to find valid config IDs",
            ]),
        ),
        PlatformApiError::PermissionDenied(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::PermissionDenied,
            &format!("Permission denied: {}", msg),
            Some(vec!["Contact the project admin for access"]),
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
            Some(vec!["Check network connectivity"]),
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
            Some(vec!["Try again later"]),
        ),
        PlatformApiError::ConnectionFailed => format_error_for_llm(
            tool_name,
            ErrorCategory::NetworkError,
            "Could not connect to Syncable API",
            Some(vec!["Check your internet connection"]),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(SetDeploymentSecretsTool::NAME, "set_deployment_secrets");
    }

    #[test]
    fn test_tool_creation() {
        let tool = SetDeploymentSecretsTool::new();
        assert!(format!("{:?}", tool).contains("SetDeploymentSecretsTool"));
    }

    #[test]
    fn test_tool_with_context() {
        let tool = SetDeploymentSecretsTool::with_context(ExecutionContext::HeadlessServer);
        assert!(format!("{:?}", tool).contains("SetDeploymentSecretsTool"));
    }

    #[test]
    fn test_default_is_secret_true() {
        assert!(default_true());
    }
}
