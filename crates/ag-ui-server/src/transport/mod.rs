//! Transport Layer for AG-UI Events
//!
//! This module provides transport implementations for streaming AG-UI events
//! to frontend clients:
//!
//! - **SSE (Server-Sent Events)**: HTTP-based unidirectional streaming via [`sse`]
//! - **WebSocket**: Bidirectional WebSocket transport via [`ws`]
//!
//! # SSE Example
//!
//! ```rust,ignore
//! use ag_ui_server::transport::sse;
//! use ag_ui_core::{Event, RunErrorEvent};
//!
//! // Create channel pair
//! let (sender, handler) = sse::channel::<serde_json::Value>(32);
//!
//! // Send events from agent code
//! sender.send(Event::RunError(RunErrorEvent::new("error"))).await?;
//!
//! // Return handler as axum response
//! handler.into_response()
//! ```
//!
//! # WebSocket Example
//!
//! ```rust,ignore
//! use ag_ui_server::transport::ws;
//! use ag_ui_core::{Event, RunErrorEvent};
//! use axum::extract::ws::WebSocketUpgrade;
//!
//! async fn ws_endpoint(upgrade: WebSocketUpgrade) -> impl IntoResponse {
//!     let (sender, handler) = ws::channel::<serde_json::Value>(32);
//!
//!     tokio::spawn(async move {
//!         sender.send(Event::RunError(RunErrorEvent::new("error"))).await.ok();
//!     });
//!
//!     handler.into_response(upgrade)
//! }
//! ```
//!
//! # Choosing Between SSE and WebSocket
//!
//! | Feature | SSE | WebSocket |
//! |---------|-----|-----------|
//! | Direction | Server → Client | Bidirectional |
//! | Auto-reconnect | Built-in (EventSource) | Manual |
//! | HTTP/2 multiplexing | Yes | No |
//! | Binary data | No (text only) | Yes |
//! | Browser connection limit | Per-domain | Per-domain |

pub mod sse;
pub mod ws;

// Re-export SSE types (default transport)
pub use sse::{channel, format_sse_event, SendError, SseHandler, SseSender};

// Re-export WebSocket types with ws_ prefix to avoid conflicts
pub use ws::{
    channel as ws_channel, channel_with_config as ws_channel_with_config,
    format_ws_message, SendError as WsSendError, WsConfig, WsHandler, WsSender,
    DEFAULT_PING_INTERVAL,
};
