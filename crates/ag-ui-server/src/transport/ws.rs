//! WebSocket Transport for AG-UI Events
//!
//! This module provides WebSocket transport for streaming AG-UI events to frontend clients.
//! It integrates with axum to provide WebSocket endpoints as an alternative to SSE.
//!
//! # Architecture
//!
//! The WebSocket transport uses a channel-based design similar to SSE:
//! - [`WsSender`] - Used by agent code to send events into the WebSocket stream
//! - [`WsHandler`] - Handles the WebSocket connection and streams events
//!
//! # Example
//!
//! ```rust,ignore
//! use ag_ui_server::transport::ws;
//! use ag_ui_core::{Event, TextMessageStartEvent, MessageId};
//! use axum::extract::ws::WebSocketUpgrade;
//!
//! async fn ws_endpoint(upgrade: WebSocketUpgrade) -> impl IntoResponse {
//!     let (sender, handler) = ws::channel::<serde_json::Value>(32);
//!
//!     // Spawn task to send events
//!     tokio::spawn(async move {
//!         let event = Event::TextMessageStart(
//!             TextMessageStartEvent::new(MessageId::random())
//!         );
//!         sender.send(event).await.ok();
//!     });
//!
//!     handler.into_response(upgrade)
//! }
//! ```
//!
//! # SSE vs WebSocket
//!
//! Choose WebSocket when:
//! - You need bidirectional communication (future AG-UI extensions)
//! - You want lower latency for high-frequency updates
//! - You need to work around SSE connection limits in browsers
//!
//! Choose SSE when:
//! - You only need server-to-client streaming (current AG-UI)
//! - You want automatic reconnection (built into EventSource)
//! - You need HTTP/2 multiplexing benefits

use std::time::Duration;

use ag_ui_core::{AgentState, Event, JsonValue};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::error::ServerError;

/// Default ping interval for WebSocket keep-alive (30 seconds).
pub const DEFAULT_PING_INTERVAL: Duration = Duration::from_secs(30);

/// Error type for WebSocket send operations.
#[derive(Debug, Clone)]
pub struct SendError<T>(pub T);

impl<T> std::fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebSocket channel closed")
    }
}

impl<T: std::fmt::Debug> std::error::Error for SendError<T> {}

/// Configuration for WebSocket connections.
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// Interval between ping messages for keep-alive.
    pub ping_interval: Duration,
    /// Whether to send ping messages.
    pub enable_ping: bool,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            ping_interval: DEFAULT_PING_INTERVAL,
            enable_ping: true,
        }
    }
}

impl WsConfig {
    /// Creates a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the ping interval.
    pub fn ping_interval(mut self, interval: Duration) -> Self {
        self.ping_interval = interval;
        self
    }

    /// Disables ping messages.
    pub fn disable_ping(mut self) -> Self {
        self.enable_ping = false;
        self
    }
}

/// Sender side of a WebSocket channel.
///
/// Use this to send AG-UI events that will be streamed to connected clients.
/// Events are serialized to JSON and sent as WebSocket text messages.
#[derive(Debug, Clone)]
pub struct WsSender<StateT: AgentState = JsonValue> {
    sender: mpsc::Sender<Event<StateT>>,
}

impl<StateT: AgentState> WsSender<StateT> {
    /// Sends an event to the WebSocket stream.
    ///
    /// Returns an error if the receiver has been dropped (client disconnected).
    pub async fn send(&self, event: Event<StateT>) -> Result<(), SendError<Event<StateT>>> {
        self.sender.send(event).await.map_err(|e| SendError(e.0))
    }

    /// Sends multiple events to the WebSocket stream.
    ///
    /// Stops and returns an error on the first failed send.
    pub async fn send_many(
        &self,
        events: impl IntoIterator<Item = Event<StateT>>,
    ) -> Result<(), SendError<Event<StateT>>> {
        for event in events {
            self.send(event).await?;
        }
        Ok(())
    }

    /// Tries to send an event without waiting.
    ///
    /// Returns an error if the channel is full or closed.
    pub fn try_send(&self, event: Event<StateT>) -> Result<(), SendError<Event<StateT>>> {
        self.sender
            .try_send(event)
            .map_err(|e| SendError(e.into_inner()))
    }

    /// Checks if the receiver is still connected.
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

/// Handler side of a WebSocket channel.
///
/// This handles the WebSocket connection and streams events from the sender.
pub struct WsHandler<StateT: AgentState = JsonValue> {
    receiver: mpsc::Receiver<Event<StateT>>,
    config: WsConfig,
}

impl<StateT: AgentState> WsHandler<StateT> {
    /// Converts a WebSocket upgrade into an axum response.
    ///
    /// The response will upgrade to WebSocket and stream events as they are
    /// sent through the corresponding [`WsSender`].
    pub fn into_response(self, upgrade: WebSocketUpgrade) -> impl IntoResponse {
        upgrade.on_upgrade(move |socket| self.handle_socket(socket))
    }

    /// Handles the WebSocket connection.
    async fn handle_socket(self, socket: WebSocket) {
        let (mut ws_sender, mut ws_receiver) = socket.split();
        let mut event_receiver = self.receiver;

        // Create ping interval if enabled
        let mut ping_interval = if self.config.enable_ping {
            Some(interval(self.config.ping_interval))
        } else {
            None
        };

        loop {
            tokio::select! {
                // Handle incoming events to send
                event = event_receiver.recv() => {
                    match event {
                        Some(event) => {
                            // Serialize event to JSON
                            let json = match serde_json::to_string(&event) {
                                Ok(json) => json,
                                Err(e) => {
                                    eprintln!("WebSocket serialization error: {}", e);
                                    continue;
                                }
                            };

                            // Send as text message
                            if ws_sender.send(Message::Text(json.into())).await.is_err() {
                                // Client disconnected
                                break;
                            }
                        }
                        None => {
                            // Event channel closed, send close frame and exit
                            let _ = ws_sender.send(Message::Close(None)).await;
                            break;
                        }
                    }
                }

                // Handle ping interval
                _ = async {
                    if let Some(ref mut interval) = ping_interval {
                        interval.tick().await;
                    } else {
                        // Never completes if ping disabled
                        std::future::pending::<()>().await;
                    }
                } => {
                    if ws_sender.send(Message::Ping(vec![].into())).await.is_err() {
                        break;
                    }
                }

                // Handle incoming WebSocket messages (for close/pong)
                msg = ws_receiver.next() => {
                    match msg {
                        Some(Ok(Message::Pong(_))) => {
                            // Pong received, connection is alive
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            // Client closed connection
                            break;
                        }
                        Some(Ok(_)) => {
                            // Ignore other message types (Text, Binary)
                            // AG-UI is unidirectional server->client
                        }
                        Some(Err(_)) => {
                            // WebSocket error
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// Creates a new WebSocket channel pair with default configuration.
///
/// The `buffer` parameter controls how many events can be queued before
/// sends will block (or fail for `try_send`).
///
/// # Arguments
///
/// * `buffer` - The capacity of the internal channel buffer
///
/// # Returns
///
/// A tuple of (`WsSender`, `WsHandler`) that are connected.
///
/// # Example
///
/// ```rust,ignore
/// let (sender, handler) = ws::channel::<serde_json::Value>(32);
/// ```
pub fn channel<StateT: AgentState>(buffer: usize) -> (WsSender<StateT>, WsHandler<StateT>) {
    channel_with_config(buffer, WsConfig::default())
}

/// Creates a new WebSocket channel pair with custom configuration.
///
/// # Arguments
///
/// * `buffer` - The capacity of the internal channel buffer
/// * `config` - WebSocket configuration options
///
/// # Returns
///
/// A tuple of (`WsSender`, `WsHandler`) that are connected.
///
/// # Example
///
/// ```rust,ignore
/// let config = WsConfig::new()
///     .ping_interval(Duration::from_secs(15))
///     .disable_ping();
/// let (sender, handler) = ws::channel_with_config::<serde_json::Value>(32, config);
/// ```
pub fn channel_with_config<StateT: AgentState>(
    buffer: usize,
    config: WsConfig,
) -> (WsSender<StateT>, WsHandler<StateT>) {
    let (tx, rx) = mpsc::channel(buffer);
    (
        WsSender { sender: tx },
        WsHandler {
            receiver: rx,
            config,
        },
    )
}

/// Serializes an event to a WebSocket text message.
///
/// Returns the JSON string suitable for sending as a WebSocket text frame.
pub fn format_ws_message<StateT: AgentState>(event: &Event<StateT>) -> Result<String, ServerError> {
    serde_json::to_string(event).map_err(|e| ServerError::Serialization(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ag_ui_core::{MessageId, RunErrorEvent, TextMessageContentEvent, TextMessageStartEvent};

    #[tokio::test]
    async fn test_channel_creation() {
        let (sender, _handler) = channel::<JsonValue>(10);
        assert!(!sender.is_closed());
    }

    #[tokio::test]
    async fn test_channel_with_config() {
        let config = WsConfig::new()
            .ping_interval(Duration::from_secs(10))
            .disable_ping();

        let (sender, handler) = channel_with_config::<JsonValue>(10, config);
        assert!(!sender.is_closed());
        assert!(!handler.config.enable_ping);
        assert_eq!(handler.config.ping_interval, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_send_event() {
        let (sender, mut handler) = channel::<JsonValue>(10);

        let event: Event = Event::TextMessageStart(TextMessageStartEvent::new(MessageId::random()));

        sender.send(event.clone()).await.unwrap();

        // Receive from the handler's receiver directly for testing
        let received = handler.receiver.recv().await.unwrap();
        assert_eq!(received.event_type(), event.event_type());
    }

    #[tokio::test]
    async fn test_send_many_events() {
        let (sender, mut handler) = channel::<JsonValue>(10);

        let events: Vec<Event> = vec![
            Event::TextMessageStart(TextMessageStartEvent::new(MessageId::random())),
            Event::TextMessageContent(TextMessageContentEvent::new_unchecked(
                MessageId::random(),
                "Hello",
            )),
            Event::RunError(RunErrorEvent::new("test error")),
        ];

        sender.send_many(events.clone()).await.unwrap();

        // Verify all events received
        for expected in &events {
            let received = handler.receiver.recv().await.unwrap();
            assert_eq!(received.event_type(), expected.event_type());
        }
    }

    #[tokio::test]
    async fn test_channel_close_detection() {
        let (sender, handler) = channel::<JsonValue>(10);

        // Drop the handler
        drop(handler);

        // Sender should detect closure
        assert!(sender.is_closed());

        // Send should fail
        let event: Event = Event::RunError(RunErrorEvent::new("test"));
        let result = sender.send(event).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_send() {
        let (sender, _handler) = channel::<JsonValue>(2);

        let event: Event = Event::RunError(RunErrorEvent::new("test"));

        // First two should succeed (buffer size is 2)
        assert!(sender.try_send(event.clone()).is_ok());
        assert!(sender.try_send(event.clone()).is_ok());

        // Third should fail (buffer full)
        assert!(sender.try_send(event).is_err());
    }

    #[test]
    fn test_format_ws_message() {
        let event: Event = Event::RunError(RunErrorEvent::new("test error"));
        let message = format_ws_message(&event).unwrap();

        assert!(message.contains("\"type\":\"RUN_ERROR\""));
        assert!(message.contains("\"message\":\"test error\""));
    }

    #[test]
    fn test_format_ws_message_complex() {
        let event: Event =
            Event::TextMessageStart(TextMessageStartEvent::new(MessageId::random()));
        let message = format_ws_message(&event).unwrap();

        assert!(message.contains("\"type\":\"TEXT_MESSAGE_START\""));
        assert!(message.contains("\"messageId\":"));
        assert!(message.contains("\"role\":\"assistant\""));
    }

    #[test]
    fn test_ws_config_default() {
        let config = WsConfig::default();
        assert!(config.enable_ping);
        assert_eq!(config.ping_interval, DEFAULT_PING_INTERVAL);
    }

    #[test]
    fn test_ws_config_builder() {
        let config = WsConfig::new()
            .ping_interval(Duration::from_secs(60))
            .disable_ping();

        assert!(!config.enable_ping);
        assert_eq!(config.ping_interval, Duration::from_secs(60));
    }

    #[test]
    fn test_send_error_display() {
        let error: SendError<i32> = SendError(42);
        assert_eq!(format!("{}", error), "WebSocket channel closed");
    }
}
