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
pub mod routes;

use std::net::SocketAddr;
use std::sync::Arc;

use ag_ui_core::{Event, JsonValue, RunId, ThreadId};
use axum::{routing::get, Router};
use tokio::sync::{broadcast, RwLock};

pub use bridge::EventBridge;

/// Configuration for the AG-UI server.
#[derive(Debug, Clone)]
pub struct AgUiConfig {
    /// Port to listen on.
    pub port: u16,
    /// Host address to bind to.
    pub host: String,
    /// Maximum number of concurrent connections.
    pub max_connections: usize,
}

impl Default for AgUiConfig {
    fn default() -> Self {
        Self {
            port: 9090,
            host: "127.0.0.1".to_string(),
            max_connections: 100,
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
}

/// Shared state for the AG-UI server.
#[derive(Clone)]
pub struct ServerState {
    /// Broadcast channel for events.
    event_tx: broadcast::Sender<Event<JsonValue>>,
    /// Current thread ID for the session.
    thread_id: Arc<RwLock<ThreadId>>,
    /// Current run ID (if agent is running).
    run_id: Arc<RwLock<Option<RunId>>>,
}

impl ServerState {
    /// Creates new server state.
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        Self {
            event_tx,
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
    pub async fn run(self) -> Result<(), std::io::Error> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .expect("Invalid address");

        let app = Router::new()
            .route("/", get(routes::health))
            .route("/sse", get(routes::sse_handler))
            .route("/ws", get(routes::ws_handler))
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
}
