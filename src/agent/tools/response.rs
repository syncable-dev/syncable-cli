//! Response formatting utilities for agent tools
//!
//! This module provides consistent response formatting for all agent tools.
//! It works alongside the error utilities in `error.rs` to provide a complete
//! response infrastructure.
//!
//! ## Pattern
//!
//! Tools should use these utilities for successful responses:
//! 1. Use `format_success` for simple successful operations
//! 2. Use `format_success_with_metadata` when including truncation/compression info
//! 3. Use `format_file_content` for file read operations
//! 4. Use `format_list` for directory listings and other lists
//!
//! ## Example
//!
//! ```ignore
//! use crate::agent::tools::response::{format_success, format_file_content, ResponseMetadata};
//!
//! // Simple success response
//! let response = format_success("read_file", json!({"content": "file contents"}));
//!
//! // File content response with metadata
//! let response = format_file_content(
//!     "src/main.rs",
//!     &file_content,
//!     100,  // total lines
//!     100,  // returned lines
//!     false, // not truncated
//! );
//! ```

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::truncation::TruncationLimits;

/// Metadata about a tool response
///
/// This provides additional context about the response, such as whether
/// the output was truncated or the original size of the data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Whether the output was truncated to fit size limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    /// Original size of the data before truncation (in bytes or count)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_size: Option<usize>,
    /// Final size after truncation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_size: Option<usize>,
    /// Number of items (for lists/arrays)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_count: Option<usize>,
    /// Total items before truncation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_items: Option<usize>,
    /// Whether data was compressed/stored for retrieval
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed: Option<bool>,
    /// Reference ID for retrieving full data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_ref: Option<String>,
}

impl ResponseMetadata {
    /// Create metadata for truncated output
    pub fn truncated(original_size: usize, final_size: usize) -> Self {
        Self {
            truncated: Some(true),
            original_size: Some(original_size),
            final_size: Some(final_size),
            ..Default::default()
        }
    }

    /// Create metadata for a list with item counts
    pub fn for_list(item_count: usize, total_items: usize) -> Self {
        Self {
            item_count: Some(item_count),
            total_items: Some(total_items),
            truncated: Some(item_count < total_items),
            ..Default::default()
        }
    }

    /// Create metadata for compressed output with retrieval reference
    pub fn compressed(retrieval_ref: String, original_size: usize) -> Self {
        Self {
            compressed: Some(true),
            retrieval_ref: Some(retrieval_ref),
            original_size: Some(original_size),
            ..Default::default()
        }
    }

    /// Check if this metadata indicates any modification (truncation/compression)
    pub fn is_modified(&self) -> bool {
        self.truncated.unwrap_or(false) || self.compressed.unwrap_or(false)
    }
}

/// Standard tool response structure
///
/// This provides a consistent response format for all tools while remaining
/// backward compatible with existing tool outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// The response data (tool-specific)
    #[serde(flatten)]
    pub data: Value,
    /// Optional metadata about the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
}

impl ToolResponse {
    /// Create a successful response with data
    pub fn success(data: Value) -> Self {
        Self {
            success: true,
            data,
            metadata: None,
        }
    }

    /// Create a successful response with metadata
    pub fn success_with_metadata(data: Value, metadata: ResponseMetadata) -> Self {
        Self {
            success: true,
            data,
            metadata: Some(metadata),
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| {
            r#"{"success": false, "error": "Failed to serialize response"}"#.to_string()
        })
    }
}

/// Format a simple success response
///
/// Use this for operations that don't need metadata about truncation/compression.
///
/// # Arguments
///
/// * `tool_name` - Name of the tool (for debugging/logging)
/// * `data` - The response data to serialize
///
/// # Returns
///
/// JSON string of the response
pub fn format_success<T: Serialize>(tool_name: &str, data: &T) -> String {
    let value = serde_json::to_value(data).unwrap_or_else(|e| {
        json!({
            "error": true,
            "tool": tool_name,
            "message": format!("Failed to serialize response: {}", e)
        })
    });

    let response = ToolResponse::success(value);
    response.to_json()
}

/// Format a success response with metadata
///
/// Use this when you need to include information about truncation, compression,
/// or item counts.
///
/// # Arguments
///
/// * `tool_name` - Name of the tool
/// * `data` - The response data
/// * `metadata` - Response metadata
pub fn format_success_with_metadata<T: Serialize>(
    tool_name: &str,
    data: &T,
    metadata: ResponseMetadata,
) -> String {
    let value = serde_json::to_value(data).unwrap_or_else(|e| {
        json!({
            "error": true,
            "tool": tool_name,
            "message": format!("Failed to serialize response: {}", e)
        })
    });

    let response = ToolResponse::success_with_metadata(value, metadata);
    response.to_json()
}

/// Format file content response
///
/// Creates a consistent response format for file read operations.
/// This is backward compatible with the existing ReadFileTool output format.
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `content` - File content (already truncated if needed)
/// * `total_lines` - Total lines in the original file
/// * `returned_lines` - Number of lines actually returned
/// * `truncated` - Whether the content was truncated
pub fn format_file_content(
    path: &str,
    content: &str,
    total_lines: usize,
    returned_lines: usize,
    truncated: bool,
) -> String {
    let data = json!({
        "file": path,
        "total_lines": total_lines,
        "lines_returned": returned_lines,
        "truncated": truncated,
        "content": content
    });

    serde_json::to_string_pretty(&data).unwrap_or_else(|_| {
        format!(
            r#"{{"file": "{}", "error": "Failed to serialize content"}}"#,
            path
        )
    })
}

/// Format file content response for line range
///
/// Creates a response for a specific line range read.
pub fn format_file_content_range(
    path: &str,
    content: &str,
    start_line: usize,
    end_line: usize,
    total_lines: usize,
) -> String {
    let data = json!({
        "file": path,
        "lines": format!("{}-{}", start_line, end_line),
        "total_lines": total_lines,
        "content": content
    });

    serde_json::to_string_pretty(&data).unwrap_or_else(|_| {
        format!(
            r#"{{"file": "{}", "error": "Failed to serialize content"}}"#,
            path
        )
    })
}

/// Format a list/directory response
///
/// Creates a consistent response format for list operations (directories, search results, etc.).
/// This is backward compatible with the existing ListDirectoryTool output format.
///
/// # Arguments
///
/// * `path` - The path that was listed (for directories) or query context
/// * `entries` - The list of items
/// * `total_count` - Total number of items (before truncation)
/// * `truncated` - Whether the list was truncated
pub fn format_list(path: &str, entries: &[Value], total_count: usize, truncated: bool) -> String {
    let data = if truncated {
        let limits = TruncationLimits::default();
        json!({
            "path": path,
            "entries": entries,
            "entries_returned": entries.len(),
            "total_count": total_count,
            "truncated": true,
            "note": format!(
                "Showing first {} of {} entries. Use a more specific path to see others.",
                entries.len().min(limits.max_dir_entries),
                total_count
            )
        })
    } else {
        json!({
            "path": path,
            "entries": entries,
            "total_count": total_count
        })
    };

    serde_json::to_string_pretty(&data).unwrap_or_else(|_| {
        format!(
            r#"{{"path": "{}", "error": "Failed to serialize entries"}}"#,
            path
        )
    })
}

/// Format a list response with custom metadata
///
/// More flexible version of format_list that allows custom metadata fields.
pub fn format_list_with_metadata(
    entries: &[Value],
    metadata: ResponseMetadata,
    extra_fields: &[(&str, Value)],
) -> String {
    let mut data = json!({
        "entries": entries,
    });

    // Add extra fields
    if let Some(obj) = data.as_object_mut() {
        for (key, value) in extra_fields {
            obj.insert((*key).to_string(), value.clone());
        }

        // Add metadata fields directly (flattened)
        if let Some(truncated) = metadata.truncated {
            obj.insert("truncated".to_string(), json!(truncated));
        }
        if let Some(total) = metadata.total_items {
            obj.insert("total_count".to_string(), json!(total));
        }
        if let Some(count) = metadata.item_count {
            obj.insert("entries_returned".to_string(), json!(count));
        }
    }

    serde_json::to_string_pretty(&data)
        .unwrap_or_else(|_| r#"{"error": "Failed to serialize list response"}"#.to_string())
}

/// Format a write operation response
///
/// Creates a consistent response for file/resource write operations.
pub fn format_write_success(
    path: &str,
    action: &str,
    lines_written: usize,
    bytes_written: usize,
) -> String {
    let data = json!({
        "success": true,
        "action": action,
        "path": path,
        "lines_written": lines_written,
        "bytes_written": bytes_written
    });

    serde_json::to_string_pretty(&data).unwrap_or_else(|_| {
        format!(
            r#"{{"success": true, "action": "{}", "path": "{}"}}"#,
            action, path
        )
    })
}

/// Format a cancelled operation response
///
/// Creates a response indicating the operation was cancelled by the user.
pub fn format_cancelled(path: &str, reason: &str, feedback: Option<&str>) -> String {
    let mut data = json!({
        "cancelled": true,
        "STOP": "User has rejected this operation. Do NOT create this file or any alternative files.",
        "reason": reason,
        "original_path": path,
        "action_required": "Stop creating files. Ask the user what they want instead."
    });

    if let Some(fb) = feedback {
        data["user_feedback"] = json!(fb);
        data["STOP"] =
            json!("Do NOT create this file or any similar files. Wait for user instruction.");
        data["action_required"] = json!(
            "Read the user_feedback and respond accordingly. Do NOT try to create alternative files."
        );
    }

    serde_json::to_string_pretty(&data)
        .unwrap_or_else(|_| format!(r#"{{"cancelled": true, "reason": "{}"}}"#, reason))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_metadata_truncated() {
        let meta = ResponseMetadata::truncated(1000, 500);
        assert_eq!(meta.truncated, Some(true));
        assert_eq!(meta.original_size, Some(1000));
        assert_eq!(meta.final_size, Some(500));
        assert!(meta.is_modified());
    }

    #[test]
    fn test_response_metadata_for_list() {
        let meta = ResponseMetadata::for_list(10, 100);
        assert_eq!(meta.item_count, Some(10));
        assert_eq!(meta.total_items, Some(100));
        assert_eq!(meta.truncated, Some(true));
    }

    #[test]
    fn test_response_metadata_compressed() {
        let meta = ResponseMetadata::compressed("ref-123".to_string(), 50000);
        assert_eq!(meta.compressed, Some(true));
        assert_eq!(meta.retrieval_ref, Some("ref-123".to_string()));
        assert!(meta.is_modified());
    }

    #[test]
    fn test_format_file_content() {
        let response = format_file_content("test.rs", "fn main() {}", 10, 10, false);
        let parsed: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["file"], "test.rs");
        assert_eq!(parsed["total_lines"], 10);
        assert_eq!(parsed["lines_returned"], 10);
        assert_eq!(parsed["truncated"], false);
        assert_eq!(parsed["content"], "fn main() {}");
    }

    #[test]
    fn test_format_file_content_truncated() {
        let response = format_file_content("large.rs", "content...", 5000, 2000, true);
        let parsed: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["truncated"], true);
        assert_eq!(parsed["total_lines"], 5000);
        assert_eq!(parsed["lines_returned"], 2000);
    }

    #[test]
    fn test_format_list() {
        let entries = vec![
            json!({"name": "file1.rs", "type": "file"}),
            json!({"name": "file2.rs", "type": "file"}),
        ];

        let response = format_list("src/", &entries, 2, false);
        let parsed: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["path"], "src/");
        assert_eq!(parsed["total_count"], 2);
        assert!(parsed["entries"].is_array());
        // No truncated field when not truncated
        assert!(parsed.get("truncated").is_none());
    }

    #[test]
    fn test_format_list_truncated() {
        let entries: Vec<Value> = (0..10)
            .map(|i| json!({"name": format!("file{}.rs", i)}))
            .collect();

        let response = format_list("src/", &entries, 100, true);
        let parsed: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["truncated"], true);
        assert_eq!(parsed["total_count"], 100);
        assert_eq!(parsed["entries_returned"], 10);
        assert!(parsed["note"].as_str().unwrap().contains("100 entries"));
    }

    #[test]
    fn test_format_write_success() {
        let response = format_write_success("Dockerfile", "Created", 25, 500);
        let parsed: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["action"], "Created");
        assert_eq!(parsed["path"], "Dockerfile");
        assert_eq!(parsed["lines_written"], 25);
        assert_eq!(parsed["bytes_written"], 500);
    }

    #[test]
    fn test_format_cancelled() {
        let response = format_cancelled("test.txt", "User cancelled the operation", None);
        let parsed: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["cancelled"], true);
        assert!(parsed["STOP"].as_str().unwrap().contains("rejected"));
    }

    #[test]
    fn test_format_cancelled_with_feedback() {
        let response = format_cancelled(
            "test.txt",
            "User requested changes",
            Some("Please add comments"),
        );
        let parsed: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["cancelled"], true);
        assert_eq!(parsed["user_feedback"], "Please add comments");
        assert!(
            parsed["action_required"]
                .as_str()
                .unwrap()
                .contains("user_feedback")
        );
    }

    #[test]
    fn test_tool_response_success() {
        let data = json!({"message": "Operation completed"});
        let response = ToolResponse::success(data);

        assert!(response.success);
        assert!(response.metadata.is_none());

        let json = response.to_json();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["message"], "Operation completed");
    }

    #[test]
    fn test_tool_response_with_metadata() {
        let data = json!({"items": [1, 2, 3]});
        let metadata = ResponseMetadata::for_list(3, 100);
        let response = ToolResponse::success_with_metadata(data, metadata);

        assert!(response.success);
        assert!(response.metadata.is_some());

        let json = response.to_json();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["metadata"]["truncated"], true);
    }
}
