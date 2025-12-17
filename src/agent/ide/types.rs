//! MCP Protocol Types
//!
//! JSON-RPC and MCP notification types for IDE communication.

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

/// JSON-RPC notification (no id)
#[derive(Debug, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
}

/// MCP Initialize request parameters
#[derive(Debug, Serialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
    pub capabilities: ClientCapabilities,
}

#[derive(Debug, Serialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct ClientCapabilities {
    // Empty for now, can be extended
}

/// MCP Tool call parameters
#[derive(Debug, Serialize)]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// MCP Tool call result
#[derive(Debug, Deserialize)]
pub struct ToolCallResult {
    #[serde(default)]
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

#[derive(Debug, Deserialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(default)]
    pub text: Option<String>,
}

/// IDE Diff Accepted notification parameters
#[derive(Debug, Deserialize)]
pub struct IdeDiffAcceptedParams {
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub content: String,
}

/// IDE Diff Rejected notification parameters
#[derive(Debug, Deserialize)]
pub struct IdeDiffRejectedParams {
    #[serde(rename = "filePath")]
    pub file_path: String,
}

/// IDE Diff Closed notification parameters (for backwards compatibility)
#[derive(Debug, Deserialize)]
pub struct IdeDiffClosedParams {
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(default)]
    pub content: Option<String>,
}

/// IDE Context notification parameters
#[derive(Debug, Deserialize)]
pub struct IdeContextParams {
    #[serde(rename = "workspaceState", default)]
    pub workspace_state: Option<WorkspaceState>,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceState {
    #[serde(rename = "openFiles", default)]
    pub open_files: Vec<OpenFile>,
    #[serde(rename = "isTrusted", default)]
    pub is_trusted: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct OpenFile {
    pub path: String,
    pub timestamp: u64,
    #[serde(rename = "isActive", default)]
    pub is_active: bool,
    #[serde(rename = "selectedText", default)]
    pub selected_text: Option<String>,
}

/// Connection config read from port file
#[derive(Debug, Deserialize)]
pub struct ConnectionConfig {
    pub port: u16,
    #[serde(rename = "workspacePath", default)]
    pub workspace_path: Option<String>,
    #[serde(rename = "authToken", default)]
    pub auth_token: Option<String>,
}

/// Open diff request arguments
#[derive(Debug, Serialize)]
pub struct OpenDiffArgs {
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "newContent")]
    pub new_content: String,
}

/// Close diff request arguments
#[derive(Debug, Serialize)]
pub struct CloseDiffArgs {
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "suppressNotification", skip_serializing_if = "Option::is_none")]
    pub suppress_notification: Option<bool>,
}
