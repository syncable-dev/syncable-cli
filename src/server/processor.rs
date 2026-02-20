//! Agent Processor - Routes frontend messages to agent for processing.
//!
//! This module provides the `AgentProcessor` which consumes messages from
//! the frontend (via WebSocket/POST) and processes them through the LLM,
//! emitting AG-UI events for the response.
//!
//! # Architecture
//!
//! ```text
//! Frontend → WebSocket/POST → message channel → AgentProcessor
//!                                                     ↓
//!                                              LLM (multi_turn with tools)
//!                                                     ↓
//!                                              EventBridge → SSE/WS → Frontend
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use syncable_ag_ui_core::{Role, RunId, ThreadId};
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Message as RigMessage;
use rig::completion::Prompt;
use rig::completion::message::{AssistantContent, UserContent};
use rig::one_or_many::OneOrMany;
use rig::providers::{anthropic, openai};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::{AgentMessage, EventBridge};
use crate::agent::prompts;
use crate::agent::tools::{
    // Core analysis tools
    AnalyzeTool,
    DclintTool,
    // Linting tools
    HadolintTool,
    HelmlintTool,
    K8sCostsTool,
    K8sDriftTool,
    // K8s tools
    K8sOptimizeTool,
    KubelintTool,
    ListDirectoryTool,
    ListOutputsTool,
    ReadFileTool,
    RetrieveOutputTool,
    SecurityScanTool,
    ShellTool,
    // Terraform tools
    TerraformFmtTool,
    TerraformInstallTool,
    TerraformValidateTool,
    VulnerabilitiesTool,
    // Web and retrieval tools
    WebFetchTool,
    // Write tools for generation
    WriteFileTool,
    WriteFilesTool,
};

use syncable_ag_ui_core::ToolCallId;
use syncable_ag_ui_core::state::StateManager;
use rig::agent::CancelSignal;
use rig::completion::{CompletionModel, CompletionResponse, Message as RigPromptMessage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Step status for generative UI progress display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Pending,
    Completed,
}

/// A step in the agent's execution for generative UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub description: String,
    pub status: StepStatus,
}

/// Result of a tool call for rich UI rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Name of the tool that was called.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub args: serde_json::Value,
    /// Result data from the tool (parsed JSON when possible).
    pub result: serde_json::Value,
    /// Whether the result is an error.
    #[serde(default)]
    pub is_error: bool,
}

/// Agent state for generative UI rendering.
///
/// This state is streamed to frontends via STATE_SNAPSHOT events
/// and can be rendered using CopilotKit's `useCoAgentStateRender`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentUiState {
    /// Steps showing progress of agent execution.
    pub steps: Vec<AgentStep>,
    /// Current tool being executed (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_tool: Option<String>,
    /// Tool results for rich UI rendering.
    #[serde(default)]
    pub tool_results: Vec<ToolResult>,
    /// Additional metadata for custom rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl AgentUiState {
    /// Creates a new empty agent state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a pending step.
    pub fn add_step(&mut self, description: impl Into<String>) {
        self.steps.push(AgentStep {
            description: description.into(),
            status: StepStatus::Pending,
        });
    }

    /// Marks a step as completed by index.
    pub fn complete_step(&mut self, index: usize) {
        if let Some(step) = self.steps.get_mut(index) {
            step.status = StepStatus::Completed;
        }
    }

    /// Marks the first pending step as completed.
    pub fn complete_current_step(&mut self) {
        for step in &mut self.steps {
            if step.status == StepStatus::Pending {
                step.status = StepStatus::Completed;
                break;
            }
        }
    }

    /// Sets the current tool being executed.
    pub fn set_current_tool(&mut self, tool: Option<String>) {
        self.current_tool = tool;
    }

    /// Adds a tool result for rich UI rendering.
    pub fn add_tool_result(
        &mut self,
        tool_name: String,
        args: serde_json::Value,
        result: serde_json::Value,
        is_error: bool,
    ) {
        self.tool_results.push(ToolResult {
            tool_name,
            args,
            result,
            is_error,
        });
    }

    /// Converts to JSON value for state events.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// Info about a tool call in progress.
#[derive(Clone)]
struct ToolCallInfo {
    id: ToolCallId,
    name: String,
    args: serde_json::Value,
}

/// AG-UI Hook for streaming tool call events and state updates to frontends.
///
/// This hook implements rig's PromptHook trait to capture tool calls
/// and emit AG-UI protocol events via the EventBridge.
/// It also maintains agent state for generative UI rendering.
#[derive(Clone)]
pub struct AgUiHook {
    event_bridge: EventBridge,
    /// Current tool call info for tracking (shared across async calls)
    current_tool_call: Arc<Mutex<Option<ToolCallInfo>>>,
    /// Agent state for generative UI (shared across async calls)
    state: Arc<Mutex<AgentUiState>>,
}

impl AgUiHook {
    /// Creates a new AG-UI hook with the given event bridge.
    pub fn new(event_bridge: EventBridge) -> Self {
        Self {
            event_bridge,
            current_tool_call: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(AgentUiState::new())),
        }
    }

    /// Emits the current state as a STATE_SNAPSHOT event.
    async fn emit_state(&self) {
        let state = self.state.lock().await;
        self.event_bridge.emit_state_snapshot(state.to_json()).await;
    }

    /// Adds a step and emits state update.
    pub async fn add_step(&self, description: impl Into<String>) {
        {
            let mut state = self.state.lock().await;
            state.add_step(description);
        }
        self.emit_state().await;
    }

    /// Completes the current step and emits state update.
    pub async fn complete_current_step(&self) {
        {
            let mut state = self.state.lock().await;
            state.complete_current_step();
        }
        self.emit_state().await;
    }
}

impl<M> rig::agent::PromptHook<M> for AgUiHook
where
    M: CompletionModel,
{
    fn on_tool_call(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        args: &str,
        _cancel: CancelSignal,
    ) -> impl std::future::Future<Output = ()> + Send {
        let bridge = self.event_bridge.clone();
        let name = tool_name.to_string();
        let args_str = args.to_string();
        let current_call = Arc::clone(&self.current_tool_call);
        let state = Arc::clone(&self.state);

        async move {
            debug!(tool = %name, "AgUiHook: on_tool_call triggered");

            // Parse args as JSON for the event
            let args_json: serde_json::Value = serde_json::from_str(&args_str)
                .unwrap_or_else(|_| serde_json::json!({"raw": args_str}));

            // Update state for generative UI - add step for this tool
            {
                let mut s = state.lock().await;
                // Create human-readable description from tool name
                let description = match name.as_str() {
                    // Core analysis tools
                    "analyze_project" => "Analyzing project structure...".to_string(),
                    "read_file" => format!(
                        "Reading file: {}",
                        args_json
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("...")
                    ),
                    "list_directory" => format!(
                        "Listing directory: {}",
                        args_json
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("...")
                    ),
                    // Security tools
                    "security_scan" => "Running security scan...".to_string(),
                    "check_vulnerabilities" => "Checking for vulnerabilities...".to_string(),
                    // Linting tools
                    "hadolint" => "Linting Dockerfiles...".to_string(),
                    "dclint" => "Linting docker-compose files...".to_string(),
                    "kubelint" => "Linting Kubernetes manifests...".to_string(),
                    "helmlint" => "Linting Helm charts...".to_string(),
                    // K8s tools
                    "k8s_optimize" => "Analyzing Kubernetes resource optimization...".to_string(),
                    "k8s_costs" => "Calculating Kubernetes costs...".to_string(),
                    "k8s_drift" => "Detecting configuration drift...".to_string(),
                    // Terraform tools
                    "terraform_fmt" => "Formatting Terraform files...".to_string(),
                    "terraform_validate" => "Validating Terraform configuration...".to_string(),
                    "terraform_install" => "Installing Terraform...".to_string(),
                    // Web tools
                    "web_fetch" => format!(
                        "Fetching: {}",
                        args_json
                            .get("url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("...")
                    ),
                    // Retrieval tools
                    "retrieve_output" => "Retrieving stored output...".to_string(),
                    "list_outputs" => "Listing available outputs...".to_string(),
                    // Write tools
                    "write_file" => format!(
                        "Writing file: {}",
                        args_json
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("...")
                    ),
                    "write_files" => "Writing multiple files...".to_string(),
                    // Shell tool
                    "shell" => format!(
                        "Running command: {}",
                        args_json
                            .get("command")
                            .and_then(|v| v.as_str())
                            .map(|s| if s.len() > 50 {
                                format!("{}...", &s[..50])
                            } else {
                                s.to_string()
                            })
                            .unwrap_or("...".to_string())
                    ),
                    _ => format!("Running {}...", name.replace('_', " ")),
                };
                s.add_step(description);
                s.set_current_tool(Some(name.clone()));
            }

            // Emit state update for generative UI
            let s = state.lock().await;
            bridge.emit_state_snapshot(s.to_json()).await;
            drop(s);

            // Emit ToolCallStart event
            let tool_call_id = bridge.start_tool_call(&name, &args_json).await;

            // Store the tool call info (id, name, args) for the result handler
            let mut call_guard = current_call.lock().await;
            *call_guard = Some(ToolCallInfo {
                id: tool_call_id,
                name: name.clone(),
                args: args_json.clone(),
            });
        }
    }

    fn on_tool_result(
        &self,
        _tool_name: &str,
        _tool_call_id: Option<String>,
        _args: &str,
        result: &str,
        _cancel: CancelSignal,
    ) -> impl std::future::Future<Output = ()> + Send {
        let bridge = self.event_bridge.clone();
        let current_call = Arc::clone(&self.current_tool_call);
        let state = Arc::clone(&self.state);
        let result_str = result.to_string();

        async move {
            // Get and clear the stored tool call info
            let tool_call_info = {
                let mut call_guard = current_call.lock().await;
                call_guard.take()
            };

            // Parse result as JSON (if possible)
            let result_json: serde_json::Value = serde_json::from_str(&result_str)
                .unwrap_or_else(|_| serde_json::json!({"raw": result_str}));

            // Check if this looks like an error result
            // Only check for explicit error fields, not substring matches
            let is_error = result_json.get("error").is_some()
                || result_json.get("success").and_then(|v| v.as_bool()) == Some(false)
                || result_json.get("status").and_then(|v| v.as_str()) == Some("error")
                || result_json.get("status").and_then(|v| v.as_str()) == Some("ERROR");

            // Update state - mark current step as completed and add tool result
            {
                let mut s = state.lock().await;
                s.complete_current_step();
                s.set_current_tool(None);

                // Add tool result for rich UI rendering
                if let Some(ref info) = tool_call_info {
                    debug!(
                        tool = %info.name,
                        result_size = result_str.len(),
                        "AgUiHook: capturing tool result for UI"
                    );
                    s.add_tool_result(
                        info.name.clone(),
                        info.args.clone(),
                        result_json.clone(),
                        is_error,
                    );
                }
            }

            // Emit state update for generative UI
            let s = state.lock().await;
            bridge.emit_state_snapshot(s.to_json()).await;
            drop(s);

            // Emit ToolCallEnd event
            if let Some(info) = tool_call_info {
                bridge.end_tool_call(&info.id).await;
            }
        }
    }

    fn on_completion_response(
        &self,
        _prompt: &RigPromptMessage,
        _response: &CompletionResponse<M::Response>,
        _cancel: CancelSignal,
    ) -> impl std::future::Future<Output = ()> + Send {
        // No-op for AG-UI - we don't need to track usage here
        async {}
    }
}

/// Errors that can occur during message processing.
#[derive(Debug, thiserror::Error)]
pub enum ProcessorError {
    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),
    #[error("LLM completion failed: {0}")]
    CompletionFailed(String),
    #[error("Missing API key for provider: {0}")]
    MissingApiKey(String),
}

/// Configuration for the agent processor.
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// LLM provider to use (openai, anthropic, bedrock).
    pub provider: String,
    /// Model name/ID.
    pub model: String,
    /// Maximum number of tool call iterations.
    pub max_turns: usize,
    /// System prompt for agent behavior (if None, uses prompts module based on project_path).
    pub system_prompt: Option<String>,
    /// Project/workspace path for context-aware prompts and tools.
    pub project_path: PathBuf,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            max_turns: 50,
            system_prompt: None,
            project_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

impl ProcessorConfig {
    /// Creates a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the provider.
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = provider.into();
        self
    }

    /// Sets the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Sets the maximum number of turns.
    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns;
        self
    }

    /// Sets the system prompt (overrides auto-generated prompt).
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Sets the project path for context-aware prompts and tools.
    pub fn with_project_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.project_path = path.into();
        self
    }

    /// Gets the effective system prompt.
    /// If a custom prompt is set, returns that.
    /// Otherwise, generates appropriate prompt based on project_path.
    pub fn effective_system_prompt(&self, query: Option<&str>) -> String {
        if let Some(ref prompt) = self.system_prompt {
            return prompt.clone();
        }
        // Use analysis prompt by default (covers most use cases)
        // For generation tasks, the prompts module can detect and switch
        if let Some(q) = query {
            if prompts::is_code_development_query(q) {
                return prompts::get_code_development_prompt(&self.project_path);
            }
            if prompts::is_generation_query(q) {
                return prompts::get_devops_prompt(&self.project_path, Some(q));
            }
        }
        prompts::get_analysis_prompt(&self.project_path)
    }
}

/// Per-thread session state for conversation isolation.
#[derive(Debug)]
pub struct ThreadSession {
    /// Thread ID for this session.
    pub thread_id: ThreadId,
    /// Raw chat history for multi-turn conversations.
    pub history: Vec<RigMessage>,
    /// When this session was created.
    pub created_at: Instant,
    /// Number of turns in this session.
    pub turn_count: usize,
}

impl ThreadSession {
    /// Creates a new thread session.
    pub fn new(thread_id: ThreadId) -> Self {
        Self {
            thread_id,
            history: Vec::new(),
            created_at: Instant::now(),
            turn_count: 0,
        }
    }

    /// Adds a user message to history.
    pub fn add_user_message(&mut self, content: &str) {
        self.history.push(RigMessage::User {
            content: OneOrMany::one(UserContent::text(content)),
        });
    }

    /// Adds an assistant message to history.
    pub fn add_assistant_message(&mut self, content: &str) {
        self.history.push(RigMessage::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::text(content)),
        });
        self.turn_count += 1;
    }

    /// Injects context that appears at start of conversation.
    /// This is useful for adding system-level context to the conversation.
    pub fn inject_context(&mut self, context: &str) {
        // Add as a system-like user message at the beginning
        // (rig doesn't have a System variant, so we use User with context prefix)
        let context_msg = RigMessage::User {
            content: OneOrMany::one(UserContent::text(format!("[Context]: {}", context))),
        };
        self.history.insert(0, context_msg);
    }
}

/// Processes frontend messages through the LLM agent.
///
/// The processor maintains per-thread sessions for conversation isolation
/// and emits AG-UI events via the EventBridge during processing.
pub struct AgentProcessor {
    /// Receiver for messages from frontend.
    message_rx: mpsc::Receiver<AgentMessage>,
    /// Event bridge for emitting AG-UI events.
    event_bridge: EventBridge,
    /// Per-thread session state.
    sessions: HashMap<ThreadId, ThreadSession>,
    /// Processor configuration.
    config: ProcessorConfig,
}

impl AgentProcessor {
    /// Creates a new agent processor.
    ///
    /// # Arguments
    /// * `message_rx` - Receiver for messages from frontend
    /// * `event_bridge` - Bridge for emitting AG-UI events
    /// * `config` - Processor configuration
    pub fn new(
        message_rx: mpsc::Receiver<AgentMessage>,
        event_bridge: EventBridge,
        config: ProcessorConfig,
    ) -> Self {
        Self {
            message_rx,
            event_bridge,
            sessions: HashMap::new(),
            config,
        }
    }

    /// Creates a processor with default configuration.
    pub fn with_defaults(
        message_rx: mpsc::Receiver<AgentMessage>,
        event_bridge: EventBridge,
    ) -> Self {
        Self::new(message_rx, event_bridge, ProcessorConfig::default())
    }

    /// Gets or creates a session for the given thread ID.
    fn get_or_create_session(&mut self, thread_id: &ThreadId) -> &mut ThreadSession {
        self.sessions
            .entry(thread_id.clone())
            .or_insert_with(|| ThreadSession::new(thread_id.clone()))
    }

    /// Gets the current session count.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Gets the configuration.
    pub fn config(&self) -> &ProcessorConfig {
        &self.config
    }

    /// Extracts the user message content from RunAgentInput messages.
    ///
    /// Returns the last user message content, or None if no user messages.
    fn extract_user_input(&self, messages: &[syncable_ag_ui_core::types::Message]) -> Option<String> {
        // Find the last user message and extract its content
        messages
            .iter()
            .rev()
            .find(|m| m.role() == Role::User)
            .and_then(|m| m.content().map(|s| s.to_string()))
    }

    /// Runs the message processing loop.
    ///
    /// This method consumes messages from the channel and processes each one
    /// through the agent. It runs until the channel is closed.
    pub async fn run(&mut self) {
        info!("AgentProcessor starting message processing loop");

        while let Some(msg) = self.message_rx.recv().await {
            let input = msg.input;
            let thread_id = input.thread_id.clone();
            let run_id = input.run_id.clone();

            debug!(
                thread_id = %thread_id,
                run_id = %run_id,
                message_count = input.messages.len(),
                "Received message from frontend"
            );

            // Check for dynamic provider/model/apiKey from forwardedProps
            self.apply_forwarded_props(&input.forwarded_props);

            // Extract user input from messages
            match self.extract_user_input(&input.messages) {
                Some(user_input) => {
                    self.process_message(thread_id, run_id, user_input).await;
                }
                None => {
                    debug!(
                        thread_id = %thread_id,
                        "No user message found in input, skipping"
                    );
                    // Emit error event
                    self.event_bridge.start_run().await;
                    self.event_bridge
                        .finish_run_with_error("No user message found in input")
                        .await;
                }
            }
        }

        info!("AgentProcessor message channel closed, shutting down");
    }

    /// Apply settings from forwardedProps (provider, model, apiKey, awsRegion)
    fn apply_forwarded_props(&mut self, forwarded_props: &serde_json::Value) {
        if let Some(obj) = forwarded_props.as_object() {
            // Update provider
            if let Some(provider) = obj.get("provider").and_then(|v| v.as_str()) {
                if !provider.is_empty() {
                    debug!(provider = %provider, "Applying provider from forwardedProps");
                    self.config.provider = provider.to_string();
                }
            }

            // Update model
            if let Some(model) = obj.get("model").and_then(|v| v.as_str()) {
                if !model.is_empty() {
                    debug!(model = %model, "Applying model from forwardedProps");
                    self.config.model = model.to_string();
                }
            }

            // Update API key (set in environment for the provider client)
            if let Some(api_key) = obj.get("apiKey").and_then(|v| v.as_str()) {
                if !api_key.is_empty() {
                    let provider = self.config.provider.to_lowercase();
                    match provider.as_str() {
                        "openai" => {
                            debug!("Setting OPENAI_API_KEY from forwardedProps");
                            // SAFETY: Single-threaded CLI context
                            unsafe {
                                std::env::set_var("OPENAI_API_KEY", api_key);
                            }
                        }
                        "anthropic" => {
                            debug!("Setting ANTHROPIC_API_KEY from forwardedProps");
                            unsafe {
                                std::env::set_var("ANTHROPIC_API_KEY", api_key);
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Update AWS region for Bedrock
            if let Some(region) = obj.get("awsRegion").and_then(|v| v.as_str()) {
                if !region.is_empty() {
                    debug!(region = %region, "Setting AWS_REGION from forwardedProps");
                    unsafe {
                        std::env::set_var("AWS_REGION", region);
                    }
                }
            }
        }
    }

    /// Processes a single message through the agent.
    ///
    /// This is the core processing method that:
    /// 1. Emits RunStarted
    /// 2. Processes through LLM
    /// 3. Emits TextMessage events
    /// 4. Updates session history
    /// 5. Emits RunFinished
    async fn process_message(&mut self, thread_id: ThreadId, _run_id: RunId, user_input: String) {
        info!(
            thread_id = %thread_id,
            input_len = user_input.len(),
            provider = %self.config.provider,
            model = %self.config.model,
            "Processing message through LLM"
        );

        // Get or create session
        let session = self.get_or_create_session(&thread_id);
        session.add_user_message(&user_input);

        // Emit run started
        self.event_bridge.start_run().await;

        // Start thinking
        self.event_bridge.start_thinking(Some("Thinking")).await;

        // Call LLM based on provider
        let response = self.call_llm(&thread_id, &user_input).await;

        self.event_bridge.end_thinking().await;

        match response {
            Ok(response_text) => {
                // Emit the response as text message
                self.event_bridge.start_message().await;

                // Stream the response in chunks for better UX
                for chunk in response_text.chars().collect::<Vec<_>>().chunks(50) {
                    let chunk_str: String = chunk.iter().collect();
                    self.event_bridge.emit_text_chunk(&chunk_str).await;
                }

                self.event_bridge.end_message().await;

                // Update session with assistant response
                let session = self.get_or_create_session(&thread_id);
                session.add_assistant_message(&response_text);

                debug!(
                    thread_id = %thread_id,
                    turn_count = session.turn_count,
                    response_len = response_text.len(),
                    "Message processed successfully"
                );

                // Finish the run
                self.event_bridge.finish_run().await;
            }
            Err(e) => {
                error!(
                    thread_id = %thread_id,
                    error = %e,
                    "LLM call failed"
                );
                self.event_bridge
                    .finish_run_with_error(&e.to_string())
                    .await;
            }
        }
    }

    /// Calls the LLM with the user input and conversation history.
    async fn call_llm(
        &mut self,
        thread_id: &ThreadId,
        user_input: &str,
    ) -> Result<String, ProcessorError> {
        // Clone config values and event_bridge to avoid borrow conflicts
        // Use query-aware system prompt for better context
        let preamble = self.config.effective_system_prompt(Some(user_input));
        let provider = self.config.provider.to_lowercase();
        let model = self.config.model.clone();
        let max_turns = self.config.max_turns;
        let project_path = self.config.project_path.clone();
        let event_bridge = self.event_bridge.clone();

        // Get mutable reference to session history
        let session = self.get_or_create_session(thread_id);
        let history = &mut session.history;

        debug!(
            provider = %provider,
            model = %model,
            project_path = %project_path.display(),
            history_len = history.len(),
            "Calling LLM with tools"
        );

        match provider.as_str() {
            "openai" => {
                // Check for API key
                if std::env::var("OPENAI_API_KEY").is_err() {
                    warn!("OPENAI_API_KEY not set");
                    return Err(ProcessorError::MissingApiKey("OPENAI_API_KEY".to_string()));
                }

                // Create AG-UI hook for streaming tool events to frontend
                let hook = AgUiHook::new(event_bridge.clone());

                let client = openai::Client::from_env();
                let agent = client
                    .agent(model)
                    .preamble(&preamble)
                    .max_tokens(4096)
                    // Core tools for file exploration and analysis
                    .tool(AnalyzeTool::new(project_path.clone()))
                    .tool(ReadFileTool::new(project_path.clone()))
                    .tool(ListDirectoryTool::new(project_path.clone()))
                    // Security and linting tools
                    .tool(SecurityScanTool::new(project_path.clone()))
                    .tool(VulnerabilitiesTool::new(project_path.clone()))
                    .tool(HadolintTool::new(project_path.clone()))
                    .tool(DclintTool::new(project_path.clone()))
                    .tool(KubelintTool::new(project_path.clone()))
                    .tool(HelmlintTool::new(project_path.clone()))
                    // K8s optimization and analysis tools
                    .tool(K8sOptimizeTool::new(project_path.clone()))
                    .tool(K8sCostsTool::new(project_path.clone()))
                    .tool(K8sDriftTool::new(project_path.clone()))
                    // Terraform tools
                    .tool(TerraformFmtTool::new(project_path.clone()))
                    .tool(TerraformValidateTool::new(project_path.clone()))
                    .tool(TerraformInstallTool::new())
                    // Web and retrieval tools
                    .tool(WebFetchTool::new())
                    .tool(RetrieveOutputTool::new())
                    .tool(ListOutputsTool::new())
                    // Write and shell tools for generation
                    .tool(WriteFileTool::new(project_path.clone()))
                    .tool(WriteFilesTool::new(project_path.clone()))
                    .tool(ShellTool::new(project_path.clone()))
                    .build();

                agent
                    .prompt(user_input)
                    .with_history(history)
                    .with_hook(hook)  // AG-UI hook for streaming tool events
                    .multi_turn(max_turns)
                    .await
                    .map_err(|e| ProcessorError::CompletionFailed(e.to_string()))
            }
            "anthropic" => {
                // Check for API key
                if std::env::var("ANTHROPIC_API_KEY").is_err() {
                    warn!("ANTHROPIC_API_KEY not set");
                    return Err(ProcessorError::MissingApiKey(
                        "ANTHROPIC_API_KEY".to_string(),
                    ));
                }

                // Need fresh hook for anthropic (hook may be consumed by openai path)
                let hook = AgUiHook::new(event_bridge.clone());

                let client = anthropic::Client::from_env();
                let agent = client
                    .agent(model)
                    .preamble(&preamble)
                    .max_tokens(4096)
                    // Core tools for file exploration and analysis
                    .tool(AnalyzeTool::new(project_path.clone()))
                    .tool(ReadFileTool::new(project_path.clone()))
                    .tool(ListDirectoryTool::new(project_path.clone()))
                    // Security and linting tools
                    .tool(SecurityScanTool::new(project_path.clone()))
                    .tool(VulnerabilitiesTool::new(project_path.clone()))
                    .tool(HadolintTool::new(project_path.clone()))
                    .tool(DclintTool::new(project_path.clone()))
                    .tool(KubelintTool::new(project_path.clone()))
                    .tool(HelmlintTool::new(project_path.clone()))
                    // K8s optimization and analysis tools
                    .tool(K8sOptimizeTool::new(project_path.clone()))
                    .tool(K8sCostsTool::new(project_path.clone()))
                    .tool(K8sDriftTool::new(project_path.clone()))
                    // Terraform tools
                    .tool(TerraformFmtTool::new(project_path.clone()))
                    .tool(TerraformValidateTool::new(project_path.clone()))
                    .tool(TerraformInstallTool::new())
                    // Web and retrieval tools
                    .tool(WebFetchTool::new())
                    .tool(RetrieveOutputTool::new())
                    .tool(ListOutputsTool::new())
                    // Write and shell tools for generation
                    .tool(WriteFileTool::new(project_path.clone()))
                    .tool(WriteFilesTool::new(project_path.clone()))
                    .tool(ShellTool::new(project_path.clone()))
                    .build();

                agent
                    .prompt(user_input)
                    .with_history(history)
                    .with_hook(hook)  // AG-UI hook for streaming tool events
                    .multi_turn(max_turns)
                    .await
                    .map_err(|e| ProcessorError::CompletionFailed(e.to_string()))
            }
            "bedrock" | "aws" | "aws-bedrock" => {
                // Need fresh hook for bedrock
                let hook = AgUiHook::new(event_bridge.clone());

                // Bedrock uses AWS credentials from environment
                let client = crate::bedrock::client::Client::from_env();
                let agent = client
                    .agent(model)
                    .preamble(&preamble)
                    .max_tokens(4096)
                    // Core tools for file exploration and analysis
                    .tool(AnalyzeTool::new(project_path.clone()))
                    .tool(ReadFileTool::new(project_path.clone()))
                    .tool(ListDirectoryTool::new(project_path.clone()))
                    // Security and linting tools
                    .tool(SecurityScanTool::new(project_path.clone()))
                    .tool(VulnerabilitiesTool::new(project_path.clone()))
                    .tool(HadolintTool::new(project_path.clone()))
                    .tool(DclintTool::new(project_path.clone()))
                    .tool(KubelintTool::new(project_path.clone()))
                    .tool(HelmlintTool::new(project_path.clone()))
                    // K8s optimization and analysis tools
                    .tool(K8sOptimizeTool::new(project_path.clone()))
                    .tool(K8sCostsTool::new(project_path.clone()))
                    .tool(K8sDriftTool::new(project_path.clone()))
                    // Terraform tools
                    .tool(TerraformFmtTool::new(project_path.clone()))
                    .tool(TerraformValidateTool::new(project_path.clone()))
                    .tool(TerraformInstallTool::new())
                    // Web and retrieval tools
                    .tool(WebFetchTool::new())
                    .tool(RetrieveOutputTool::new())
                    .tool(ListOutputsTool::new())
                    // Write and shell tools for generation
                    .tool(WriteFileTool::new(project_path.clone()))
                    .tool(WriteFilesTool::new(project_path.clone()))
                    .tool(ShellTool::new(project_path))
                    .build();

                agent
                    .prompt(user_input)
                    .with_history(history)
                    .with_hook(hook)  // AG-UI hook for streaming tool events
                    .multi_turn(max_turns)
                    .await
                    .map_err(|e| ProcessorError::CompletionFailed(e.to_string()))
            }
            _ => Err(ProcessorError::UnsupportedProvider(provider.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tokio::sync::broadcast;

    fn create_test_processor() -> (AgentProcessor, mpsc::Sender<AgentMessage>) {
        let (msg_tx, msg_rx) = mpsc::channel(100);
        let (event_tx, _) = broadcast::channel(100);
        let bridge = EventBridge::new(
            event_tx,
            Arc::new(RwLock::new(ThreadId::random())),
            Arc::new(RwLock::new(None)),
        );
        let processor = AgentProcessor::with_defaults(msg_rx, bridge);
        (processor, msg_tx)
    }

    #[test]
    fn test_processor_config_default() {
        let config = ProcessorConfig::default();
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-4o");
        assert_eq!(config.max_turns, 50);
    }

    #[test]
    fn test_processor_config_builder() {
        let config = ProcessorConfig::new()
            .with_provider("anthropic")
            .with_model("claude-3-opus")
            .with_max_turns(100);
        assert_eq!(config.provider, "anthropic");
        assert_eq!(config.model, "claude-3-opus");
        assert_eq!(config.max_turns, 100);
    }

    #[test]
    fn test_processor_config_system_prompt() {
        // Default system prompt (uses analysis prompt from prompts module)
        let config = ProcessorConfig::default();
        assert!(config.system_prompt.is_none());
        // Analysis prompt contains agent identity section
        assert!(
            config
                .effective_system_prompt(None)
                .contains("DevOps/Platform Engineer")
        );

        // Custom system prompt overrides auto-generated
        let config = ProcessorConfig::new().with_system_prompt("You are a DevOps expert.");
        assert_eq!(
            config.system_prompt,
            Some("You are a DevOps expert.".to_string())
        );
        assert_eq!(
            config.effective_system_prompt(None),
            "You are a DevOps expert."
        );
    }

    #[test]
    fn test_thread_session_inject_context() {
        let mut session = ThreadSession::new(ThreadId::random());

        // Add some messages first
        session.add_user_message("Hello");
        session.add_assistant_message("Hi there!");
        assert_eq!(session.history.len(), 2);

        // Inject context - should be at the beginning
        session.inject_context("Working on project: my-app");
        assert_eq!(session.history.len(), 3);

        // Verify the context message is at the start (index 0)
        if let RigMessage::User { content } = &session.history[0] {
            let content_str = format!("{:?}", content);
            assert!(content_str.contains("[Context]"));
            assert!(content_str.contains("my-app"));
        } else {
            panic!("Expected User message at index 0");
        }
    }

    #[test]
    fn test_thread_session_new() {
        let thread_id = ThreadId::random();
        let session = ThreadSession::new(thread_id.clone());
        assert_eq!(session.thread_id, thread_id);
        assert!(session.history.is_empty());
        assert_eq!(session.turn_count, 0);
    }

    #[test]
    fn test_thread_session_add_messages() {
        let mut session = ThreadSession::new(ThreadId::random());

        session.add_user_message("Hello");
        assert_eq!(session.history.len(), 1);
        assert_eq!(session.turn_count, 0); // User message doesn't increment turn

        session.add_assistant_message("Hi there!");
        assert_eq!(session.history.len(), 2);
        assert_eq!(session.turn_count, 1); // Assistant message increments turn
    }

    #[test]
    fn test_processor_creation() {
        let (processor, _tx) = create_test_processor();
        assert_eq!(processor.session_count(), 0);
        assert_eq!(processor.config().provider, "openai");
    }

    #[test]
    fn test_get_or_create_session() {
        let (mut processor, _tx) = create_test_processor();
        let thread_id = ThreadId::random();

        // First call creates new session
        let session = processor.get_or_create_session(&thread_id);
        assert_eq!(session.turn_count, 0);

        // Add a message
        session.add_user_message("test");

        // Second call returns same session
        let session = processor.get_or_create_session(&thread_id);
        assert_eq!(session.history.len(), 1);
    }

    #[tokio::test]
    async fn test_process_message() {
        let (mut processor, _tx) = create_test_processor();
        let thread_id = ThreadId::random();
        let run_id = RunId::random();

        processor
            .process_message(thread_id.clone(), run_id, "Hello, agent!".to_string())
            .await;

        // Check session was created and user message was added
        assert_eq!(processor.session_count(), 1);
        let session = processor.sessions.get(&thread_id).unwrap();

        // User message should always be added
        assert!(
            session.history.len() >= 1,
            "User message should be in history"
        );

        // If API keys are available, turn_count and history should include assistant response
        // If not, the LLM call fails gracefully and only user message is present
        if std::env::var("OPENAI_API_KEY").is_ok() {
            // With API key, expect full conversation
            assert_eq!(session.turn_count, 1);
            assert_eq!(session.history.len(), 2); // user + assistant
        } else {
            // Without API key, LLM call fails - only user message present
            assert_eq!(session.turn_count, 0);
            assert_eq!(session.history.len(), 1); // just user
        }
    }

    #[tokio::test]
    async fn test_run_processes_messages() {
        use syncable_ag_ui_core::Event;
        use syncable_ag_ui_core::types::{Message as AgUiProtocolMessage, RunAgentInput};
        use tokio::sync::broadcast;

        let (msg_tx, msg_rx) = mpsc::channel(100);
        let (event_tx, mut event_rx) = broadcast::channel(100);

        let bridge = EventBridge::new(
            event_tx,
            Arc::new(RwLock::new(ThreadId::random())),
            Arc::new(RwLock::new(None)),
        );

        let mut processor = AgentProcessor::with_defaults(msg_rx, bridge);

        // Spawn processor in background
        let handle = tokio::spawn(async move {
            processor.run().await;
        });

        // Send a message
        let thread_id = ThreadId::random();
        let run_id = RunId::random();
        let input = RunAgentInput::new(thread_id.clone(), run_id.clone())
            .with_messages(vec![AgUiProtocolMessage::new_user("Hello from test")]);

        let agent_msg = super::super::AgentMessage::new(input);
        msg_tx.send(agent_msg).await.expect("Should send");

        // Verify we receive RunStarted event
        let event = tokio::time::timeout(std::time::Duration::from_millis(100), event_rx.recv())
            .await
            .expect("Should receive event in time")
            .expect("Should have event");

        assert!(matches!(event, Event::RunStarted(_)));

        // Drop sender to close channel and stop processor
        drop(msg_tx);

        // Wait for processor to finish
        tokio::time::timeout(std::time::Duration::from_millis(100), handle)
            .await
            .expect("Processor should finish")
            .expect("Should not panic");
    }

    #[tokio::test]
    async fn test_run_handles_empty_messages() {
        use syncable_ag_ui_core::Event;
        use syncable_ag_ui_core::types::RunAgentInput;
        use tokio::sync::broadcast;

        let (msg_tx, msg_rx) = mpsc::channel(100);
        let (event_tx, mut event_rx) = broadcast::channel(100);

        let bridge = EventBridge::new(
            event_tx,
            Arc::new(RwLock::new(ThreadId::random())),
            Arc::new(RwLock::new(None)),
        );

        let mut processor = AgentProcessor::with_defaults(msg_rx, bridge);

        // Spawn processor in background
        let handle = tokio::spawn(async move {
            processor.run().await;
        });

        // Send a message with no user content
        let thread_id = ThreadId::random();
        let run_id = RunId::random();
        let input = RunAgentInput::new(thread_id.clone(), run_id.clone());
        // Note: no messages added

        let agent_msg = super::super::AgentMessage::new(input);
        msg_tx.send(agent_msg).await.expect("Should send");

        // Should receive RunStarted then RunError
        let event = tokio::time::timeout(std::time::Duration::from_millis(100), event_rx.recv())
            .await
            .expect("Should receive event")
            .expect("Should have event");

        assert!(matches!(event, Event::RunStarted(_)));

        let event = tokio::time::timeout(std::time::Duration::from_millis(100), event_rx.recv())
            .await
            .expect("Should receive event")
            .expect("Should have event");

        assert!(matches!(event, Event::RunError(_)));

        drop(msg_tx);
        let _ = tokio::time::timeout(std::time::Duration::from_millis(100), handle).await;
    }
}
