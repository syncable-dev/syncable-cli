//! AG-UI Server Integration
//!
//! This module provides the AG-UI protocol server for syncable-cli,
//! enabling frontend applications to connect and receive real-time
//! updates as the agent works.
//!
//! # Architecture
//!
//! ```text
//! Frontend (tanstack)
//!     ↓ SSE/WebSocket
//! AgUiServer (this module)
//!     ↓ Event Bridge
//! Agent (ToolDisplayHook)
//!     ↓
//! LLM Provider (OpenAI/Anthropic/Bedrock)
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use syncable_cli::server::{AgUiServer, AgUiConfig};
//!
//! // Start the AG-UI server
//! let config = AgUiConfig::default().port(9090);
//! let server = AgUiServer::new(config);
//! let event_sender = server.event_sender();
//!
//! // Run server in background
//! tokio::spawn(server.run());
//!
//! // In agent code, emit events
//! let bridge = server.event_bridge();
//! bridge.start_run().await;
//! let tool_id = bridge.start_tool_call("analyze", &args).await;
//! bridge.emit_text_chunk("Processing...").await;
//! bridge.end_tool_call(&tool_id).await;
//! bridge.finish_run().await;
//! ```

pub mod bridge;
pub mod processor;
pub mod routes;

use std::net::SocketAddr;
use std::sync::Arc;

use ag_ui_core::{Event, JsonValue, RunId, ThreadId};
use axum::{routing::{get, post}, Router};
use tokio::sync::{broadcast, mpsc, RwLock};

pub use bridge::EventBridge;
pub use processor::{AgentProcessor, ProcessorConfig, ThreadSession};

// Re-export types needed for message handling
pub use ag_ui_core::types::{Context, Message as AgUiMessage, RunAgentInput, Tool};

/// Message from frontend to agent processor.
/// Wraps RunAgentInput with optional response channel for acknowledgments.
#[derive(Debug, Clone)]
pub struct AgentMessage {
    /// The AG-UI protocol input from the frontend.
    pub input: RunAgentInput,
}

impl AgentMessage {
    /// Creates a new agent message from RunAgentInput.
    pub fn new(input: RunAgentInput) -> Self {
        Self { input }
    }
}

/// Configuration for the AG-UI server.
#[derive(Debug, Clone)]
pub struct AgUiConfig {
    /// Port to listen on.
    pub port: u16,
    /// Host address to bind to.
    pub host: String,
    /// Maximum number of concurrent connections.
    pub max_connections: usize,
    /// Whether to start the agent processor.
    pub enable_processor: bool,
    /// Configuration for the agent processor (if enabled).
    pub processor_config: Option<ProcessorConfig>,
}

impl Default for AgUiConfig {
    fn default() -> Self {
        Self {
            port: 9090,
            host: "127.0.0.1".to_string(),
            max_connections: 100,
            enable_processor: false,
            processor_config: None,
        }
    }
}

impl AgUiConfig {
    /// Creates a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the port number.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets the host address.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Enables or disables the agent processor.
    ///
    /// When enabled, the server will spawn an AgentProcessor that
    /// consumes messages from the message channel and processes them.
    pub fn with_processor(mut self, enable: bool) -> Self {
        self.enable_processor = enable;
        if enable && self.processor_config.is_none() {
            self.processor_config = Some(ProcessorConfig::default());
        }
        self
    }

    /// Sets the processor configuration.
    pub fn with_processor_config(mut self, config: ProcessorConfig) -> Self {
        self.processor_config = Some(config);
        self.enable_processor = true;
        self
    }
}

/// Shared state for the AG-UI server.
#[derive(Clone)]
pub struct ServerState {
    /// Broadcast channel for events (outgoing to clients).
    event_tx: broadcast::Sender<Event<JsonValue>>,
    /// Channel for incoming messages from frontends.
    message_tx: mpsc::Sender<AgentMessage>,
    /// Receiver stored in Arc for extraction (only one consumer).
    message_rx: Arc<RwLock<Option<mpsc::Receiver<AgentMessage>>>>,
    /// Current thread ID for the session.
    thread_id: Arc<RwLock<ThreadId>>,
    /// Current run ID (if agent is running).
    run_id: Arc<RwLock<Option<RunId>>>,
}

impl ServerState {
    /// Creates new server state.
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        let (message_tx, message_rx) = mpsc::channel(100);
        Self {
            event_tx,
            message_tx,
            message_rx: Arc::new(RwLock::new(Some(message_rx))),
            thread_id: Arc::new(RwLock::new(ThreadId::random())),
            run_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Gets the event sender for emitting events.
    pub fn event_sender(&self) -> EventBridge {
        EventBridge::new(
            self.event_tx.clone(),
            Arc::clone(&self.thread_id),
            Arc::clone(&self.run_id),
        )
    }

    /// Subscribes to the event stream.
    pub fn subscribe(&self) -> broadcast::Receiver<Event<JsonValue>> {
        self.event_tx.subscribe()
    }

    /// Gets the message sender for routing incoming messages.
    pub fn message_sender(&self) -> mpsc::Sender<AgentMessage> {
        self.message_tx.clone()
    }

    /// Takes the message receiver (can only be called once).
    ///
    /// This is used by the agent processor to receive messages from frontends.
    /// Returns None if the receiver has already been taken.
    pub async fn take_message_receiver(&self) -> Option<mpsc::Receiver<AgentMessage>> {
        self.message_rx.write().await.take()
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// The AG-UI server that enables frontend connectivity.
pub struct AgUiServer {
    config: AgUiConfig,
    state: ServerState,
}

impl AgUiServer {
    /// Creates a new AG-UI server with the given configuration.
    pub fn new(config: AgUiConfig) -> Self {
        Self {
            config,
            state: ServerState::new(),
        }
    }

    /// Creates a new server with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(AgUiConfig::default())
    }

    /// Gets the event bridge for emitting events from agent code.
    pub fn event_bridge(&self) -> EventBridge {
        self.state.event_sender()
    }

    /// Gets the server state for sharing with routes.
    pub fn state(&self) -> ServerState {
        self.state.clone()
    }

    /// Runs the AG-UI server.
    ///
    /// This method blocks until the server is shut down.
    /// If the processor is enabled in config, it will be spawned as a background task.
    pub async fn run(self) -> Result<(), std::io::Error> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .expect("Invalid address");

        // Optionally start the agent processor
        if self.config.enable_processor {
            let processor_config = self.config.processor_config.clone()
                .unwrap_or_default();

            if let Some(msg_rx) = self.state.take_message_receiver().await {
                let event_bridge = self.state.event_sender();
                let mut processor = AgentProcessor::new(msg_rx, event_bridge, processor_config);

                // Spawn processor in background
                tokio::spawn(async move {
                    processor.run().await;
                });

                println!("Agent processor started");
            }
        }

        let app = Router::new()
            .route("/", get(routes::health))
            .route("/sse", get(routes::sse_handler))
            .route("/ws", get(routes::ws_handler))
            .route("/message", post(routes::post_message))
            .route("/health", get(routes::health))
            .with_state(self.state);

        println!("AG-UI server listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await
    }

    /// Returns the address the server will listen on.
    pub fn addr(&self) -> String {
        format!("{}:{}", self.config.host, self.config.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = AgUiConfig::default();
        assert_eq!(config.port, 9090);
        assert_eq!(config.host, "127.0.0.1");
    }

    #[test]
    fn test_config_builder() {
        let config = AgUiConfig::new()
            .port(8080)
            .host("0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
    }

    #[test]
    fn test_server_state_new() {
        let state = ServerState::new();
        let _bridge = state.event_sender();
        let _rx = state.subscribe();
    }

    #[test]
    fn test_server_addr() {
        let server = AgUiServer::with_defaults();
        assert_eq!(server.addr(), "127.0.0.1:9090");
    }

    #[test]
    fn test_event_bridge_from_state() {
        let state = ServerState::new();
        let bridge1 = state.event_sender();
        let bridge2 = state.event_sender();

        // Both bridges should share the same channel
        // (they'll both send to the same subscribers)
        let _ = state.subscribe();

        // Just verify we can create multiple bridges without panic
        drop(bridge1);
        drop(bridge2);
    }

    #[tokio::test]
    async fn test_server_event_flow() {
        use ag_ui_core::Event;

        let state = ServerState::new();
        let bridge = state.event_sender();
        let mut rx = state.subscribe();

        // Start a run
        bridge.start_run().await;

        // Receive the event
        let event = rx.recv().await.expect("Should receive RunStarted");
        assert!(matches!(event, Event::RunStarted(_)));
    }

    #[tokio::test]
    async fn test_message_channel() {
        use ag_ui_core::types::{RunAgentInput, Message};

        let state = ServerState::new();
        let msg_tx = state.message_sender();
        let mut msg_rx = state.take_message_receiver().await.expect("Should get receiver");

        // Create a RunAgentInput using builder pattern
        let input = RunAgentInput::new(ThreadId::random(), RunId::random())
            .with_messages(vec![Message::new_user("Hello agent")]);

        // Send message
        let agent_msg = AgentMessage::new(input);
        msg_tx.send(agent_msg).await.expect("Should send");

        // Receive message
        let received = msg_rx.recv().await.expect("Should receive message");
        assert_eq!(received.input.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_message_receiver_only_once() {
        let state = ServerState::new();

        // First take succeeds
        let rx1 = state.take_message_receiver().await;
        assert!(rx1.is_some());

        // Second take fails
        let rx2 = state.take_message_receiver().await;
        assert!(rx2.is_none());
    }

    #[test]
    fn test_config_with_processor() {
        let config = AgUiConfig::new().with_processor(true);
        assert!(config.enable_processor);
        assert!(config.processor_config.is_some());
    }

    #[test]
    fn test_config_with_processor_config() {
        let processor_config = ProcessorConfig::new()
            .with_provider("anthropic")
            .with_model("claude-3-sonnet");

        let config = AgUiConfig::new()
            .with_processor_config(processor_config);

        assert!(config.enable_processor);
        let proc_config = config.processor_config.unwrap();
        assert_eq!(proc_config.provider, "anthropic");
        assert_eq!(proc_config.model, "claude-3-sonnet");
    }

    #[tokio::test]
    async fn test_processor_integration_with_state() {
        use ag_ui_core::types::{Message, RunAgentInput};
        use ag_ui_core::Event;

        // Create state and get components
        let state = ServerState::new();
        let msg_tx = state.message_sender();
        let mut event_rx = state.subscribe();
        let msg_rx = state.take_message_receiver().await.expect("Should get receiver");

        // Create and spawn processor
        let event_bridge = state.event_sender();
        let mut processor = AgentProcessor::with_defaults(msg_rx, event_bridge);

        let handle = tokio::spawn(async move {
            processor.run().await;
        });

        // Send a message
        let thread_id = ThreadId::random();
        let run_id = RunId::random();
        let input = RunAgentInput::new(thread_id.clone(), run_id.clone())
            .with_messages(vec![Message::new_user("Integration test message")]);

        msg_tx.send(AgentMessage::new(input)).await.expect("Should send");

        // Verify events are emitted
        let event = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            event_rx.recv()
        ).await.expect("Should receive in time").expect("Should have event");

        assert!(matches!(event, Event::RunStarted(_)));

        // Stop processor by dropping sender
        drop(msg_tx);

        // Wait for processor to finish
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            handle
        ).await;
    }
}
