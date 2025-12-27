//! MCP Client for IDE Communication
//!
//! Connects to the IDE's MCP server via HTTP SSE and provides methods
//! for opening diffs and receiving notifications.

use super::detect::{IdeInfo, IdeProcessInfo, detect_ide, get_ide_process_info};
use super::types::*;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// Result of a diff operation
#[derive(Debug, Clone)]
pub enum DiffResult {
    /// User accepted the diff, possibly with edits
    Accepted { content: String },
    /// User rejected the diff
    Rejected,
}

/// IDE connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
}

/// Errors that can occur during IDE operations
#[derive(Debug, thiserror::Error)]
pub enum IdeError {
    #[error("IDE not detected")]
    NotDetected,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("No response received")]
    NoResponse,
    #[error("Operation cancelled")]
    Cancelled,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// MCP Client for IDE communication
#[derive(Debug)]
pub struct IdeClient {
    /// HTTP client
    http_client: reqwest::Client,
    /// Connection state
    status: Arc<Mutex<ConnectionStatus>>,
    /// Detected IDE info
    ide_info: Option<IdeInfo>,
    /// IDE process info
    process_info: Option<IdeProcessInfo>,
    /// Server port
    port: Option<u16>,
    /// Auth token
    auth_token: Option<String>,
    /// Session ID for MCP
    session_id: Arc<Mutex<Option<String>>>,
    /// Request ID counter
    request_id: Arc<Mutex<u64>>,
    /// Pending diff responses
    diff_responses: Arc<Mutex<HashMap<String, oneshot::Sender<DiffResult>>>>,
    /// SSE event receiver
    sse_receiver: Option<mpsc::Receiver<JsonRpcNotification>>,
}

impl IdeClient {
    /// Create a new IDE client (does not connect automatically)
    pub async fn new() -> Self {
        let process_info = get_ide_process_info().await;
        let ide_info = detect_ide(process_info.as_ref());

        Self {
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            status: Arc::new(Mutex::new(ConnectionStatus::Disconnected)),
            ide_info,
            process_info,
            port: None,
            auth_token: None,
            session_id: Arc::new(Mutex::new(None)),
            request_id: Arc::new(Mutex::new(0)),
            diff_responses: Arc::new(Mutex::new(HashMap::new())),
            sse_receiver: None,
        }
    }

    /// Check if IDE integration is available
    pub fn is_ide_available(&self) -> bool {
        self.ide_info.is_some()
    }

    /// Get the detected IDE name
    pub fn ide_name(&self) -> Option<&str> {
        self.ide_info.as_ref().map(|i| i.display_name.as_str())
    }

    /// Check if connected to IDE
    pub fn is_connected(&self) -> bool {
        *self.status.lock().unwrap() == ConnectionStatus::Connected
    }

    /// Get connection status
    pub fn status(&self) -> ConnectionStatus {
        self.status.lock().unwrap().clone()
    }

    /// Try to connect to the IDE server
    pub async fn connect(&mut self) -> Result<(), IdeError> {
        if self.ide_info.is_none() {
            return Err(IdeError::NotDetected);
        }

        *self.status.lock().unwrap() = ConnectionStatus::Connecting;

        // Try to read connection config from file
        if let Some(config) = self.read_connection_config().await {
            self.port = Some(config.port);
            self.auth_token = config.auth_token.clone();

            // Try to establish connection
            if self.establish_connection().await.is_ok() {
                *self.status.lock().unwrap() = ConnectionStatus::Connected;
                return Ok(());
            }
        }

        // Try environment variables as fallback
        if let Ok(port_str) = env::var("SYNCABLE_CLI_IDE_SERVER_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                self.port = Some(port);
                self.auth_token = env::var("SYNCABLE_CLI_IDE_AUTH_TOKEN").ok();

                if self.establish_connection().await.is_ok() {
                    *self.status.lock().unwrap() = ConnectionStatus::Connected;
                    return Ok(());
                }
            }
        }

        *self.status.lock().unwrap() = ConnectionStatus::Disconnected;
        Err(IdeError::ConnectionFailed(
            "Could not connect to IDE companion extension".to_string(),
        ))
    }

    /// Read connection config from port file
    /// Supports both Syncable and Gemini CLI companion extensions
    async fn read_connection_config(&self) -> Option<ConnectionConfig> {
        let temp_dir = env::temp_dir();

        // Debug: show where we're looking
        if cfg!(debug_assertions) || env::var("SYNCABLE_DEBUG").is_ok() {
            eprintln!(
                "[IDE Debug] Looking for port files in temp_dir: {:?}",
                temp_dir
            );
        }

        // Try Syncable extension first - scan all port files, match by workspace
        let syncable_port_dir = temp_dir.join("syncable").join("ide");
        if cfg!(debug_assertions) || env::var("SYNCABLE_DEBUG").is_ok() {
            eprintln!(
                "[IDE Debug] Checking Syncable dir: {:?} (exists: {})",
                syncable_port_dir,
                syncable_port_dir.exists()
            );
        }
        if let Some(config) =
            self.find_port_file_by_workspace(&syncable_port_dir, "syncable-ide-server")
        {
            if cfg!(debug_assertions) || env::var("SYNCABLE_DEBUG").is_ok() {
                eprintln!("[IDE Debug] Found Syncable config: port={}", config.port);
            }
            return Some(config);
        }

        // Try Gemini CLI extension (for compatibility)
        let gemini_port_dir = temp_dir.join("gemini").join("ide");
        if cfg!(debug_assertions) || env::var("SYNCABLE_DEBUG").is_ok() {
            eprintln!(
                "[IDE Debug] Checking Gemini dir: {:?} (exists: {})",
                gemini_port_dir,
                gemini_port_dir.exists()
            );
        }
        if let Some(config) =
            self.find_port_file_by_workspace(&gemini_port_dir, "gemini-ide-server")
        {
            if cfg!(debug_assertions) || env::var("SYNCABLE_DEBUG").is_ok() {
                eprintln!("[IDE Debug] Found Gemini config: port={}", config.port);
            }
            return Some(config);
        }

        if cfg!(debug_assertions) || env::var("SYNCABLE_DEBUG").is_ok() {
            eprintln!("[IDE Debug] No port file found in either location");
        }
        None
    }

    /// Find a port file in a directory by scanning all files and matching workspace path
    fn find_port_file_by_workspace(&self, dir: &PathBuf, prefix: &str) -> Option<ConnectionConfig> {
        let entries = fs::read_dir(dir).ok()?;

        let debug = cfg!(debug_assertions) || env::var("SYNCABLE_DEBUG").is_ok();

        for entry in entries.flatten() {
            let filename = entry.file_name().to_string_lossy().to_string();
            // Match any file starting with the prefix and ending with .json
            if filename.starts_with(prefix) && filename.ends_with(".json") {
                if debug {
                    eprintln!("[IDE Debug] Found port file: {:?}", entry.path());
                }
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(config) = serde_json::from_str::<ConnectionConfig>(&content) {
                        if debug {
                            eprintln!(
                                "[IDE Debug] Config workspace_path: {:?}",
                                config.workspace_path
                            );
                        }
                        if self.validate_workspace_path(&config.workspace_path) {
                            return Some(config);
                        } else if debug {
                            let cwd = env::current_dir().ok();
                            eprintln!("[IDE Debug] Workspace path did not match cwd: {:?}", cwd);
                        }
                    }
                }
            }
        }
        None
    }

    /// Validate that the workspace path matches our current directory
    fn validate_workspace_path(&self, workspace_path: &Option<String>) -> bool {
        let Some(ws_path) = workspace_path else {
            return false;
        };

        if ws_path.is_empty() {
            return false;
        }

        let cwd = match env::current_dir() {
            Ok(p) => p,
            Err(_) => return false,
        };

        // Check if cwd is within any of the workspace paths
        for path in ws_path.split(std::path::MAIN_SEPARATOR) {
            let ws = PathBuf::from(path);
            if cwd.starts_with(&ws) || ws.starts_with(&cwd) {
                return true;
            }
        }

        false
    }

    /// Establish HTTP connection and initialize MCP session
    async fn establish_connection(&mut self) -> Result<(), IdeError> {
        let port = self
            .port
            .ok_or(IdeError::ConnectionFailed("No port".to_string()))?;
        let url = format!("http://127.0.0.1:{}/mcp", port);

        // Build initialize request
        let init_request = JsonRpcRequest::new(
            self.next_request_id(),
            "initialize",
            serde_json::to_value(InitializeParams {
                protocol_version: "2024-11-05".to_string(),
                client_info: ClientInfo {
                    name: "syncable-cli".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
                capabilities: ClientCapabilities {},
            })
            .unwrap(),
        );

        // Send initialize request
        let mut request = self
            .http_client
            .post(&url)
            .header("Accept", "application/json, text/event-stream")
            .json(&init_request);

        if let Some(token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| IdeError::ConnectionFailed(e.to_string()))?;

        // Get session ID from response header
        if let Some(session_id) = response.headers().get("mcp-session-id") {
            if let Ok(id) = session_id.to_str() {
                *self.session_id.lock().unwrap() = Some(id.to_string());
            }
        }

        // Parse response (SSE format: "event: message\ndata: {json}")
        let response_text = response
            .text()
            .await
            .map_err(|e| IdeError::ConnectionFailed(e.to_string()))?;

        let response_data: JsonRpcResponse =
            Self::parse_sse_response(&response_text).map_err(IdeError::ConnectionFailed)?;

        if response_data.error.is_some() {
            return Err(IdeError::ConnectionFailed(
                response_data.error.map(|e| e.message).unwrap_or_default(),
            ));
        }

        Ok(())
    }

    /// Parse SSE response format to extract JSON
    fn parse_sse_response(text: &str) -> Result<JsonRpcResponse, String> {
        // SSE format: "event: message\ndata: {json}\n\n"
        for line in text.lines() {
            if let Some(json_str) = line.strip_prefix("data: ") {
                return serde_json::from_str(json_str)
                    .map_err(|e| format!("Failed to parse JSON: {}", e));
            }
        }
        // Fallback: try parsing entire response as JSON (for non-SSE responses)
        serde_json::from_str(text).map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Get next request ID
    fn next_request_id(&self) -> u64 {
        let mut id = self.request_id.lock().unwrap();
        *id += 1;
        *id
    }

    /// Send an MCP request
    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<JsonRpcResponse, IdeError> {
        let port = self
            .port
            .ok_or(IdeError::ConnectionFailed("Not connected".to_string()))?;
        let url = format!("http://127.0.0.1:{}/mcp", port);

        let request = JsonRpcRequest::new(self.next_request_id(), method, params);

        let mut http_request = self
            .http_client
            .post(&url)
            .header("Accept", "application/json, text/event-stream")
            .json(&request);

        if let Some(token) = &self.auth_token {
            http_request = http_request.header("Authorization", format!("Bearer {}", token));
        }

        if let Some(session_id) = &*self.session_id.lock().unwrap() {
            http_request = http_request.header("mcp-session-id", session_id);
        }

        let response = http_request
            .send()
            .await
            .map_err(|e| IdeError::RequestFailed(e.to_string()))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| IdeError::RequestFailed(e.to_string()))?;

        Self::parse_sse_response(&response_text).map_err(IdeError::RequestFailed)
    }

    /// Open a diff view in the IDE
    ///
    /// This sends the file path and new content to the IDE, which will show
    /// a diff view. The method returns when the user accepts or rejects the diff.
    pub async fn open_diff(
        &self,
        file_path: &str,
        new_content: &str,
    ) -> Result<DiffResult, IdeError> {
        if !self.is_connected() {
            return Err(IdeError::ConnectionFailed(
                "Not connected to IDE".to_string(),
            ));
        }

        let params = serde_json::to_value(ToolCallParams {
            name: "openDiff".to_string(),
            arguments: serde_json::to_value(OpenDiffArgs {
                file_path: file_path.to_string(),
                new_content: new_content.to_string(),
            })
            .unwrap(),
        })
        .unwrap();

        // Create a channel to receive the diff result
        let (tx, rx) = oneshot::channel();
        {
            let mut responses = self.diff_responses.lock().unwrap();
            responses.insert(file_path.to_string(), tx);
        }

        // Send the openDiff request
        let response = self.send_request("tools/call", params).await;

        if let Err(e) = response {
            // Remove the pending response
            let mut responses = self.diff_responses.lock().unwrap();
            responses.remove(file_path);
            return Err(e);
        }

        // Wait for the notification (with timeout)
        match tokio::time::timeout(Duration::from_secs(300), rx).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => Err(IdeError::Cancelled),
            Err(_) => {
                // Timeout - remove pending response
                let mut responses = self.diff_responses.lock().unwrap();
                responses.remove(file_path);
                Err(IdeError::NoResponse)
            }
        }
    }

    /// Close a diff view in the IDE
    pub async fn close_diff(&self, file_path: &str) -> Result<Option<String>, IdeError> {
        if !self.is_connected() {
            return Err(IdeError::ConnectionFailed(
                "Not connected to IDE".to_string(),
            ));
        }

        let params = serde_json::to_value(ToolCallParams {
            name: "closeDiff".to_string(),
            arguments: serde_json::to_value(CloseDiffArgs {
                file_path: file_path.to_string(),
                suppress_notification: Some(false),
            })
            .unwrap(),
        })
        .unwrap();

        let response = self.send_request("tools/call", params).await?;

        // Parse the response to get content if available
        if let Some(result) = response.result {
            if let Ok(tool_result) = serde_json::from_value::<ToolCallResult>(result) {
                for content in tool_result.content {
                    if content.content_type == "text" {
                        if let Some(text) = content.text {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                                if let Some(content) =
                                    parsed.get("content").and_then(|c| c.as_str())
                                {
                                    return Ok(Some(content.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Handle an incoming notification from the IDE
    pub fn handle_notification(&self, notification: JsonRpcNotification) {
        match notification.method.as_str() {
            "ide/diffAccepted" => {
                if let Ok(params) =
                    serde_json::from_value::<IdeDiffAcceptedParams>(notification.params)
                {
                    let mut responses = self.diff_responses.lock().unwrap();
                    if let Some(tx) = responses.remove(&params.file_path) {
                        let _ = tx.send(DiffResult::Accepted {
                            content: params.content,
                        });
                    }
                }
            }
            "ide/diffRejected" | "ide/diffClosed" => {
                if let Ok(params) =
                    serde_json::from_value::<IdeDiffRejectedParams>(notification.params)
                {
                    let mut responses = self.diff_responses.lock().unwrap();
                    if let Some(tx) = responses.remove(&params.file_path) {
                        let _ = tx.send(DiffResult::Rejected);
                    }
                }
            }
            "ide/contextUpdate" => {
                // Handle IDE context updates (e.g., open files)
                // This could be used to show relevant context in the agent
            }
            _ => {
                // Unknown notification
            }
        }
    }

    /// Get diagnostics from the IDE's language servers
    ///
    /// This queries the IDE for all diagnostic messages (errors, warnings, etc.)
    /// from the active language servers (rust-analyzer, ESLint, TypeScript, etc.)
    ///
    /// If `file_path` is provided, returns diagnostics only for that file.
    /// Otherwise returns all diagnostics across the workspace.
    pub async fn get_diagnostics(
        &self,
        file_path: Option<&str>,
    ) -> Result<DiagnosticsResponse, IdeError> {
        if !self.is_connected() {
            return Err(IdeError::ConnectionFailed(
                "Not connected to IDE".to_string(),
            ));
        }

        let params = serde_json::to_value(ToolCallParams {
            name: "getDiagnostics".to_string(),
            arguments: serde_json::to_value(GetDiagnosticsArgs {
                uri: file_path.map(|p| format!("file://{}", p)),
            })
            .unwrap(),
        })
        .unwrap();

        let response = self.send_request("tools/call", params).await?;

        // Parse the response
        if let Some(result) = response.result {
            if let Ok(tool_result) = serde_json::from_value::<ToolCallResult>(result) {
                // Look for the text content with diagnostics
                for content in tool_result.content {
                    if content.content_type == "text" {
                        if let Some(text) = content.text {
                            // Try to parse as DiagnosticsResponse
                            if let Ok(diag_response) =
                                serde_json::from_str::<DiagnosticsResponse>(&text)
                            {
                                return Ok(diag_response);
                            }
                            // Try parsing as raw array of diagnostics
                            if let Ok(diagnostics) = serde_json::from_str::<Vec<Diagnostic>>(&text)
                            {
                                let total_errors = diagnostics
                                    .iter()
                                    .filter(|d| d.severity == DiagnosticSeverity::Error)
                                    .count()
                                    as u32;
                                let total_warnings = diagnostics
                                    .iter()
                                    .filter(|d| d.severity == DiagnosticSeverity::Warning)
                                    .count()
                                    as u32;
                                return Ok(DiagnosticsResponse {
                                    diagnostics,
                                    total_errors,
                                    total_warnings,
                                });
                            }
                        }
                    }
                }
            }
        }

        // No diagnostics found - return empty response
        Ok(DiagnosticsResponse {
            diagnostics: Vec::new(),
            total_errors: 0,
            total_warnings: 0,
        })
    }

    /// Disconnect from the IDE
    pub async fn disconnect(&mut self) {
        // Close any pending diffs
        let pending: Vec<String> = {
            let responses = self.diff_responses.lock().unwrap();
            responses.keys().cloned().collect()
        };

        for file_path in pending {
            let _ = self.close_diff(&file_path).await;
        }

        *self.status.lock().unwrap() = ConnectionStatus::Disconnected;
        *self.session_id.lock().unwrap() = None;
    }
}

impl Default for IdeClient {
    fn default() -> Self {
        // Create with blocking runtime for sync context
        tokio::runtime::Handle::current().block_on(Self::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ide_client_creation() {
        let client = IdeClient::new().await;
        assert!(!client.is_connected());
    }

    #[test]
    fn test_diff_result() {
        let accepted = DiffResult::Accepted {
            content: "test".to_string(),
        };
        match accepted {
            DiffResult::Accepted { content } => assert_eq!(content, "test"),
            _ => panic!("Expected Accepted"),
        }
    }
}
