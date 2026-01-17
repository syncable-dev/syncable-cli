//! Get service logs tool for the agent
//!
//! Allows the agent to fetch container logs for deployed services.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::api::{PlatformApiClient, PlatformApiError};

/// Arguments for the get service logs tool
#[derive(Debug, Deserialize)]
pub struct GetServiceLogsArgs {
    /// Service ID (from list_deployments output)
    pub service_id: String,
    /// Start time filter (ISO timestamp, optional)
    pub start: Option<String>,
    /// End time filter (ISO timestamp, optional)
    pub end: Option<String>,
    /// Maximum number of log lines to return (default: 100)
    pub limit: Option<i32>,
}

/// Error type for get service logs operations
#[derive(Debug, thiserror::Error)]
#[error("Get service logs error: {0}")]
pub struct GetServiceLogsError(String);

/// Tool to get container logs for a deployed service
///
/// Returns recent log entries with timestamps and container metadata.
/// Supports time filtering and line limits for efficient log retrieval.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetServiceLogsTool;

impl GetServiceLogsTool {
    /// Create a new GetServiceLogsTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GetServiceLogsTool {
    const NAME: &'static str = "get_service_logs";

    type Error = GetServiceLogsError;
    type Args = GetServiceLogsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Get container logs for a deployed service.

Returns recent log entries from the service's containers with timestamps
and metadata. Useful for debugging and monitoring deployed services.

**Parameters:**
- service_id: The deployment/service ID (from list_deployments output)
- start: Optional ISO timestamp to filter logs from (e.g., "2024-01-01T00:00:00Z")
- end: Optional ISO timestamp to filter logs until
- limit: Optional max number of log lines (default: 100)

**Prerequisites:**
- User must be authenticated via `sync-ctl auth login`
- Service must be deployed (use list_deployments to find service IDs)

**Use Cases:**
- Debug application errors by viewing recent logs
- Monitor service behavior after deployment
- Investigate issues by filtering logs to a specific time range
- View startup logs to verify configuration"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "service_id": {
                        "type": "string",
                        "description": "The deployment/service ID (from list_deployments output)"
                    },
                    "start": {
                        "type": "string",
                        "description": "Optional: ISO timestamp to filter logs from (e.g., \"2024-01-01T00:00:00Z\")"
                    },
                    "end": {
                        "type": "string",
                        "description": "Optional: ISO timestamp to filter logs until"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Optional: max number of log lines to return (default 100)"
                    }
                },
                "required": ["service_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate service_id
        if args.service_id.trim().is_empty() {
            return Ok(format_error_for_llm(
                "get_service_logs",
                ErrorCategory::ValidationFailed,
                "service_id cannot be empty",
                Some(vec![
                    "Use list_deployments to find valid service IDs",
                    "The service_id is the 'id' field from deployment entries",
                ]),
            ));
        }

        // Create the API client
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(e) => {
                return Ok(format_api_error("get_service_logs", e));
            }
        };

        // Fetch logs
        let start_ref = args.start.as_deref();
        let end_ref = args.end.as_deref();

        match client
            .get_service_logs(&args.service_id, start_ref, end_ref, args.limit)
            .await
        {
            Ok(response) => {
                if response.data.is_empty() {
                    return Ok(json!({
                        "success": true,
                        "logs": [],
                        "count": 0,
                        "stats": {
                            "entries_returned": 0,
                            "query_time_ms": response.stats.query_time_ms
                        },
                        "message": "No logs found for this service. The service may not have produced any logs yet, or the time filter may be too restrictive."
                    })
                    .to_string());
                }

                // Format log entries for readability
                let log_entries: Vec<serde_json::Value> = response
                    .data
                    .iter()
                    .map(|entry| {
                        json!({
                            "timestamp": entry.timestamp,
                            "message": entry.message,
                            "labels": entry.labels
                        })
                    })
                    .collect();

                let result = json!({
                    "success": true,
                    "logs": log_entries,
                    "count": response.data.len(),
                    "stats": {
                        "entries_returned": response.stats.entries_returned,
                        "query_time_ms": response.stats.query_time_ms
                    },
                    "message": format!("Retrieved {} log entries", response.data.len())
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| GetServiceLogsError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_api_error("get_service_logs", e)),
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
            &format!("Service not found: {}", msg),
            Some(vec![
                "The service_id may be incorrect or the service no longer exists",
                "Use list_deployments to find valid service IDs",
            ]),
        ),
        PlatformApiError::PermissionDenied(msg) => format_error_for_llm(
            tool_name,
            ErrorCategory::PermissionDenied,
            &format!("Permission denied: {}", msg),
            Some(vec![
                "The user does not have access to view logs for this service",
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
        assert_eq!(GetServiceLogsTool::NAME, "get_service_logs");
    }

    #[test]
    fn test_tool_creation() {
        let tool = GetServiceLogsTool::new();
        assert!(format!("{:?}", tool).contains("GetServiceLogsTool"));
    }
}
