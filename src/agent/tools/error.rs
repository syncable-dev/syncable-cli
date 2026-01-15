//! Common error utilities for agent tools
//!
//! This module provides shared error handling infrastructure without replacing
//! individual tool error types. Each tool keeps its own error type (e.g., ReadFileError,
//! ShellError) but uses these utilities for consistent formatting.
//!
//! ## Pattern
//!
//! Tools should:
//! 1. Keep their own error type deriving `thiserror::Error`
//! 2. Use `ToolErrorContext` trait to add context when propagating errors
//! 3. Use `format_error_for_llm` when returning error JSON to the agent
//!
//! ## Example
//!
//! ```ignore
//! use crate::agent::tools::error::{ToolErrorContext, ErrorCategory, format_error_for_llm};
//!
//! fn read_config(&self, path: &Path) -> Result<String, ReadFileError> {
//!     fs::read_to_string(path)
//!         .with_tool_context("read_file", "reading configuration file")
//!         .map_err(|e| ReadFileError(e))
//! }
//!
//! // In tool call, for JSON error responses:
//! let error_json = format_error_for_llm(
//!     "read_file",
//!     ErrorCategory::FileNotFound,
//!     "File not found: config.yaml",
//!     Some(vec!["Check if the file exists", "Verify the path is correct"]),
//! );
//! ```

use serde::Serialize;
use serde_json::json;
use std::fmt;

/// Common error categories for tool errors
///
/// These categories help the LLM understand what kind of error occurred
/// and how to potentially recover from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// File or path not found
    FileNotFound,
    /// Permission denied for operation
    PermissionDenied,
    /// Path is outside allowed directory
    PathOutsideBoundary,
    /// Input validation failed
    ValidationFailed,
    /// Serialization/deserialization error
    SerializationError,
    /// External command or tool failed
    ExternalCommandFailed,
    /// Command was rejected (not allowed)
    CommandRejected,
    /// Operation timed out
    Timeout,
    /// Network or connection error
    NetworkError,
    /// Resource not available
    ResourceUnavailable,
    /// Internal tool error
    InternalError,
    /// User cancelled the operation
    UserCancelled,
}

impl ErrorCategory {
    /// Returns a human-readable description of the category
    pub fn description(&self) -> &'static str {
        match self {
            Self::FileNotFound => "The requested file or path was not found",
            Self::PermissionDenied => "Permission was denied for this operation",
            Self::PathOutsideBoundary => "The path is outside the allowed project directory",
            Self::ValidationFailed => "Input validation failed",
            Self::SerializationError => "Failed to serialize or deserialize data",
            Self::ExternalCommandFailed => "An external command or tool failed",
            Self::CommandRejected => "The command was rejected (not in allowed list)",
            Self::Timeout => "The operation timed out",
            Self::NetworkError => "A network or connection error occurred",
            Self::ResourceUnavailable => "The requested resource is not available",
            Self::InternalError => "An internal error occurred",
            Self::UserCancelled => "The operation was cancelled by the user",
        }
    }

    /// Returns whether this error is potentially recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::FileNotFound
                | Self::ValidationFailed
                | Self::Timeout
                | Self::NetworkError
                | Self::ResourceUnavailable
                | Self::UserCancelled
        )
    }

    /// Returns the error code string for this category
    pub fn code(&self) -> &'static str {
        match self {
            Self::FileNotFound => "FILE_NOT_FOUND",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::PathOutsideBoundary => "PATH_OUTSIDE_BOUNDARY",
            Self::ValidationFailed => "VALIDATION_FAILED",
            Self::SerializationError => "SERIALIZATION_ERROR",
            Self::ExternalCommandFailed => "EXTERNAL_COMMAND_FAILED",
            Self::CommandRejected => "COMMAND_REJECTED",
            Self::Timeout => "TIMEOUT",
            Self::NetworkError => "NETWORK_ERROR",
            Self::ResourceUnavailable => "RESOURCE_UNAVAILABLE",
            Self::InternalError => "INTERNAL_ERROR",
            Self::UserCancelled => "USER_CANCELLED",
        }
    }
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Format an error for LLM consumption
///
/// Returns a JSON string with structured error information that helps
/// the LLM understand what went wrong and how to potentially fix it.
///
/// # Arguments
///
/// * `tool_name` - Name of the tool that produced the error
/// * `category` - The error category
/// * `message` - Human-readable error message
/// * `suggestions` - Optional list of suggestions for recovery
///
/// # Example
///
/// ```ignore
/// let error_json = format_error_for_llm(
///     "read_file",
///     ErrorCategory::FileNotFound,
///     "File not found: /path/to/file.txt",
///     Some(vec!["Check if the file exists", "Use list_directory to explore"]),
/// );
/// ```
pub fn format_error_for_llm(
    tool_name: &str,
    category: ErrorCategory,
    message: &str,
    suggestions: Option<Vec<&str>>,
) -> String {
    let mut error_obj = json!({
        "error": true,
        "tool": tool_name,
        "category": category,
        "code": category.code(),
        "message": message,
        "recoverable": category.is_recoverable(),
    });

    if let Some(suggs) = suggestions {
        if !suggs.is_empty() {
            error_obj["suggestions"] = json!(suggs);
        }
    }

    serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| {
        format!(
            r#"{{"error": true, "tool": "{}", "message": "{}"}}"#,
            tool_name, message
        )
    })
}

/// Format an error with additional context fields
///
/// Similar to `format_error_for_llm` but allows adding arbitrary context.
///
/// # Arguments
///
/// * `tool_name` - Name of the tool that produced the error
/// * `category` - The error category
/// * `message` - Human-readable error message
/// * `context` - Additional context as key-value pairs
pub fn format_error_with_context(
    tool_name: &str,
    category: ErrorCategory,
    message: &str,
    context: &[(&str, serde_json::Value)],
) -> String {
    let mut error_obj = json!({
        "error": true,
        "tool": tool_name,
        "category": category,
        "code": category.code(),
        "message": message,
        "recoverable": category.is_recoverable(),
    });

    // Add context fields
    if let Some(obj) = error_obj.as_object_mut() {
        for (key, value) in context {
            obj.insert((*key).to_string(), value.clone());
        }
    }

    serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| {
        format!(
            r#"{{"error": true, "tool": "{}", "message": "{}"}}"#,
            tool_name, message
        )
    })
}

/// Extension trait for adding tool context to errors
///
/// This trait provides a convenient way to add context when propagating errors
/// through the ? operator.
pub trait ToolErrorContext<T, E> {
    /// Add tool context to an error
    ///
    /// # Arguments
    ///
    /// * `tool_name` - Name of the tool
    /// * `operation` - Description of the operation being performed
    fn with_tool_context(self, tool_name: &str, operation: &str) -> Result<T, String>;
}

impl<T, E: fmt::Display> ToolErrorContext<T, E> for Result<T, E> {
    fn with_tool_context(self, tool_name: &str, operation: &str) -> Result<T, String> {
        self.map_err(|e| format!("[{}] {} failed: {}", tool_name, operation, e))
    }
}

/// Helper to detect error category from common error patterns
///
/// Analyzes an error message to suggest an appropriate category.
/// This is a heuristic and may not always be accurate.
pub fn detect_error_category(error_msg: &str) -> ErrorCategory {
    let lower = error_msg.to_lowercase();

    if lower.contains("not found")
        || lower.contains("no such file")
        || lower.contains("does not exist")
    {
        ErrorCategory::FileNotFound
    } else if lower.contains("permission denied") || lower.contains("access denied") {
        ErrorCategory::PermissionDenied
    } else if lower.contains("outside") && (lower.contains("project") || lower.contains("boundary"))
    {
        ErrorCategory::PathOutsideBoundary
    } else if lower.contains("timeout") || lower.contains("timed out") {
        ErrorCategory::Timeout
    } else if lower.contains("connection")
        || lower.contains("network")
        || lower.contains("unreachable")
    {
        ErrorCategory::NetworkError
    } else if lower.contains("serialize")
        || lower.contains("deserialize")
        || lower.contains("json")
        || lower.contains("parse")
    {
        ErrorCategory::SerializationError
    } else if lower.contains("not allowed") || lower.contains("rejected") {
        ErrorCategory::CommandRejected
    } else if lower.contains("cancelled") || lower.contains("canceled") {
        ErrorCategory::UserCancelled
    } else if lower.contains("validation") || lower.contains("invalid") {
        ErrorCategory::ValidationFailed
    } else {
        ErrorCategory::InternalError
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category_codes() {
        assert_eq!(ErrorCategory::FileNotFound.code(), "FILE_NOT_FOUND");
        assert_eq!(ErrorCategory::PermissionDenied.code(), "PERMISSION_DENIED");
        assert_eq!(ErrorCategory::CommandRejected.code(), "COMMAND_REJECTED");
    }

    #[test]
    fn test_error_category_recoverable() {
        assert!(ErrorCategory::FileNotFound.is_recoverable());
        assert!(ErrorCategory::Timeout.is_recoverable());
        assert!(!ErrorCategory::PermissionDenied.is_recoverable());
        assert!(!ErrorCategory::InternalError.is_recoverable());
    }

    #[test]
    fn test_format_error_for_llm() {
        let json_str = format_error_for_llm(
            "read_file",
            ErrorCategory::FileNotFound,
            "File not found: test.txt",
            Some(vec!["Check path", "Use list_directory"]),
        );

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["error"], true);
        assert_eq!(parsed["tool"], "read_file");
        assert_eq!(parsed["code"], "FILE_NOT_FOUND");
        assert_eq!(parsed["recoverable"], true);
        assert!(parsed["suggestions"].is_array());
    }

    #[test]
    fn test_format_error_with_context() {
        let json_str = format_error_with_context(
            "shell",
            ErrorCategory::CommandRejected,
            "Command not allowed",
            &[
                ("blocked_command", json!("rm -rf /")),
                ("allowed_commands", json!(["ls", "cat"])),
            ],
        );

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["error"], true);
        assert_eq!(parsed["blocked_command"], "rm -rf /");
        assert!(parsed["allowed_commands"].is_array());
    }

    #[test]
    fn test_detect_error_category() {
        assert_eq!(
            detect_error_category("File not found: config.yaml"),
            ErrorCategory::FileNotFound
        );
        assert_eq!(
            detect_error_category("Permission denied"),
            ErrorCategory::PermissionDenied
        );
        assert_eq!(
            detect_error_category("Path is outside project boundary"),
            ErrorCategory::PathOutsideBoundary
        );
        assert_eq!(
            detect_error_category("Connection timeout"),
            ErrorCategory::Timeout
        );
        assert_eq!(
            detect_error_category("JSON parse error"),
            ErrorCategory::SerializationError
        );
        assert_eq!(
            detect_error_category("Command not allowed"),
            ErrorCategory::CommandRejected
        );
    }

    #[test]
    fn test_tool_error_context() {
        let result: Result<(), std::io::Error> =
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "file missing"));

        let with_context = result.with_tool_context("read_file", "reading config");
        assert!(with_context.is_err());

        let err_msg = with_context.unwrap_err();
        assert!(err_msg.contains("[read_file]"));
        assert!(err_msg.contains("reading config failed"));
    }
}
