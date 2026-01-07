//! Retrieve Output Tool - RAG retrieval for compressed tool outputs
//!
//! Allows the agent to retrieve full details from previously compressed outputs.
//! This is the retrieval part of the RAG pattern.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::output_store;

/// Arguments for the retrieve_output tool
#[derive(Debug, Deserialize)]
pub struct RetrieveOutputArgs {
    /// Reference ID from a compressed tool output (e.g., "kubelint_abc123")
    pub ref_id: String,
    /// Optional query to filter results
    /// Examples: "severity:critical", "file:deployment.yaml", "code:DL3008", "container:nginx"
    pub query: Option<String>,
}

/// Error type for retrieve tool
#[derive(Debug, thiserror::Error)]
#[error("Retrieve error: {0}")]
pub struct RetrieveError(String);

/// Tool to retrieve detailed data from compressed tool outputs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetrieveOutputTool;

impl RetrieveOutputTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for RetrieveOutputTool {
    const NAME: &'static str = "retrieve_output";

    type Error = RetrieveError;
    type Args = RetrieveOutputArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Retrieve detailed data from a previous tool output that was compressed.

Use this tool when:
- You received a compressed summary with a 'full_data_ref' field
- You need full details about specific issues mentioned in a summary
- You want to filter issues by severity, file, code, or container

The ref_id comes from the 'full_data_ref' field in compressed outputs from tools like kubelint, k8s_optimize, or analyze_project.

Query examples:
- "severity:critical" - Get all critical issues
- "severity:high" - Get all high severity issues
- "file:deployment.yaml" - Get issues in a specific file
- "code:DL3008" - Get all issues with a specific code
- "container:nginx" - Get issues for a specific container
- No query - Get all stored data"#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ref_id": {
                        "type": "string",
                        "description": "Reference ID from the compressed output's 'full_data_ref' field (e.g., 'kubelint_abc123')"
                    },
                    "query": {
                        "type": "string",
                        "description": "Optional filter query. Format: 'field:value'. Fields: severity, file, code, container. Examples: 'severity:critical', 'file:Dockerfile', 'code:DL3008'"
                    }
                },
                "required": ["ref_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Try to retrieve filtered data
        let result = output_store::retrieve_filtered(&args.ref_id, args.query.as_deref());

        match result {
            Some(data) => {
                let json_str = serde_json::to_string_pretty(&data)
                    .map_err(|e| RetrieveError(format!("Failed to serialize: {}", e)))?;

                // Check if result is too large and warn
                if json_str.len() > 50_000 {
                    Ok(format!(
                        "{}\n\n[NOTE: Large result ({} bytes). Consider using a more specific query to filter results.]",
                        json_str,
                        json_str.len()
                    ))
                } else {
                    Ok(json_str)
                }
            }
            None => {
                // Check if the ref_id exists at all
                let outputs = output_store::list_outputs();
                let available: Vec<&str> =
                    outputs.iter().map(|o| o.ref_id.as_str()).take(5).collect();

                if available.is_empty() {
                    Err(RetrieveError(format!(
                        "Output '{}' not found. No stored outputs available. Outputs are stored temporarily and may have expired.",
                        args.ref_id
                    )))
                } else {
                    Err(RetrieveError(format!(
                        "Output '{}' not found. Available outputs: {:?}. Note: Outputs expire after 1 hour.",
                        args.ref_id, available
                    )))
                }
            }
        }
    }
}

/// Tool to list all available stored outputs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListOutputsTool;

impl ListOutputsTool {
    pub fn new() -> Self {
        Self
    }
}

/// Arguments for list_outputs tool (none required)
#[derive(Debug, Deserialize)]
pub struct ListOutputsArgs {}

impl Tool for ListOutputsTool {
    const NAME: &'static str = "list_stored_outputs";

    type Error = RetrieveError;
    type Args = ListOutputsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List all stored tool outputs that can be retrieved. Shows ref_id, tool name, timestamp, and size for each stored output.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let outputs = output_store::list_outputs();

        if outputs.is_empty() {
            return Ok("No stored outputs available. Outputs are created when tools like kubelint, k8s_optimize, or analyze_project produce large results.".to_string());
        }

        let mut result = String::from("Available stored outputs:\n\n");

        for output in &outputs {
            let age_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                .saturating_sub(output.timestamp);

            let age_str = if age_secs < 60 {
                format!("{}s ago", age_secs)
            } else if age_secs < 3600 {
                format!("{}m ago", age_secs / 60)
            } else {
                format!("{}h ago", age_secs / 3600)
            };

            let size_str = if output.size_bytes < 1024 {
                format!("{} B", output.size_bytes)
            } else {
                format!("{:.1} KB", output.size_bytes as f64 / 1024.0)
            };

            result.push_str(&format!(
                "- {} (tool: {}, {}, {})\n",
                output.ref_id, output.tool, size_str, age_str
            ));
        }

        result.push_str(&format!("\nTotal: {} outputs\n", outputs.len()));
        result.push_str("\nUse retrieve_output(ref_id, query) to get details.");

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retrieve_nonexistent() {
        let tool = RetrieveOutputTool::new();
        let args = RetrieveOutputArgs {
            ref_id: "nonexistent_12345".to_string(),
            query: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_outputs() {
        let tool = ListOutputsTool::new();
        let args = ListOutputsArgs {};

        let result = tool.call(args).await;
        assert!(result.is_ok());
    }
}
