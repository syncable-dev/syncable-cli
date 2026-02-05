//! Server-Sent Events (SSE) Transport
//!
//! This module provides SSE transport for streaming AG-UI events to frontend clients.
//! It integrates with axum to provide HTTP SSE endpoints.
//!
//! # Architecture
//!
//! The SSE transport uses a channel-based design:
//! - [`SseSender`] - Used by agent code to send events into the stream
//! - [`SseHandler`] - Converted to an axum SSE response for the HTTP endpoint
//!
//! # Example
//!
//! ```rust,ignore
//! use ag_ui_server::transport::sse;
//! use ag_ui_core::{Event, TextMessageStartEvent, MessageId};
//!
//! // Create a channel pair
//! let (sender, handler) = sse::channel::<serde_json::Value>(32);
//!
//! // In your axum handler, return the SSE response
//! async fn events_endpoint() -> impl IntoResponse {
//!     let (sender, handler) = sse::channel::<serde_json::Value>(32);
//!
//!     // Spawn task to send events
//!     tokio::spawn(async move {
//!         let event = Event::TextMessageStart(
//!             TextMessageStartEvent::new(MessageId::random())
//!         );
//!         sender.send(event).await.ok();
//!     });
//!
//!     handler.into_response()
//! }
//! ```

use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

use ag_ui_core::{AgentState, Event, JsonValue};
use axum::response::sse::{Event as AxumSseEvent, KeepAlive, Sse};
use axum::response::IntoResponse;
use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::error::ServerError;

/// Error type for SSE send operations.
#[derive(Debug, Clone)]
pub struct SendError<T>(pub T);

impl<T> std::fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "channel closed")
    }
}

impl<T: std::fmt::Debug> std::error::Error for SendError<T> {}

/// Sender side of an SSE channel.
///
/// Use this to send AG-UI events that will be streamed to connected clients.
/// Events are serialized to JSON and formatted as SSE data frames.
#[derive(Debug, Clone)]
pub struct SseSender<StateT: AgentState = JsonValue> {
    sender: mpsc::Sender<Event<StateT>>,
}

impl<StateT: AgentState> SseSender<StateT> {
    /// Sends an event to the SSE stream.
    ///
    /// Returns an error if the receiver has been dropped (client disconnected).
    pub async fn send(&self, event: Event<StateT>) -> Result<(), SendError<Event<StateT>>> {
        self.sender.send(event).await.map_err(|e| SendError(e.0))
    }

    /// Sends multiple events to the SSE stream.
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
        self.sender.try_send(event).map_err(|e| SendError(e.into_inner()))
    }

    /// Checks if the receiver is still connected.
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

/// Handler side of an SSE channel.
///
/// This is converted to an axum SSE response that streams events to the client.
/// Each event is serialized to JSON and sent as an SSE data frame.
pub struct SseHandler<StateT: AgentState = JsonValue> {
    receiver: mpsc::Receiver<Event<StateT>>,
}

impl<StateT: AgentState> SseHandler<StateT> {
    /// Converts this handler into an axum SSE response.
    ///
    /// The response will stream events as they are sent through the corresponding
    /// [`SseSender`]. The stream ends when the sender is dropped.
    pub fn into_response(self) -> impl IntoResponse {
        let stream = SseEventStream {
            inner: ReceiverStream::new(self.receiver),
        };

        Sse::new(stream).keep_alive(KeepAlive::default())
    }
}

/// Internal stream wrapper that converts Events to axum SSE events.
struct SseEventStream<StateT: AgentState> {
    inner: ReceiverStream<Event<StateT>>,
}

impl<StateT: AgentState> Stream for SseEventStream<StateT> {
    type Item = Result<AxumSseEvent, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(event)) => {
                // Serialize event to JSON
                let json = match serde_json::to_string(&event) {
                    Ok(json) => json,
                    Err(e) => {
                        // Log error and send error event
                        eprintln!("SSE serialization error: {}", e);
                        format!(r#"{{"type":"RUN_ERROR","message":"Serialization error: {}"}}"#, e)
                    }
                };

                // Create SSE event with the event type as the SSE event name
                let sse_event = AxumSseEvent::default()
                    .event(event.event_type().as_str())
                    .data(json);

                Poll::Ready(Some(Ok(sse_event)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Creates a new SSE channel pair.
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
/// A tuple of (`SseSender`, `SseHandler`) that are connected.
///
/// # Example
///
/// ```rust,ignore
/// let (sender, handler) = sse::channel::<serde_json::Value>(32);
/// ```
pub fn channel<StateT: AgentState>(buffer: usize) -> (SseSender<StateT>, SseHandler<StateT>) {
    let (tx, rx) = mpsc::channel(buffer);
    (SseSender { sender: tx }, SseHandler { receiver: rx })
}

/// Serializes an event to SSE format.
///
/// Returns the event formatted as `data: {json}\n\n`.
pub fn format_sse_event<StateT: AgentState>(event: &Event<StateT>) -> Result<String, ServerError> {
    let json = serde_json::to_string(event)
        .map_err(|e| ServerError::Serialization(e.to_string()))?;
    Ok(format!("data: {}\n\n", json))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ag_ui_core::{
        MessageId, RunErrorEvent, TextMessageContentEvent, TextMessageStartEvent,
    };

    #[tokio::test]
    async fn test_channel_creation() {
        let (sender, _handler) = channel::<JsonValue>(10);
        assert!(!sender.is_closed());
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
    fn test_format_sse_event() {
        let event: Event = Event::RunError(RunErrorEvent::new("test error"));
        let formatted = format_sse_event(&event).unwrap();

        assert!(formatted.starts_with("data: "));
        assert!(formatted.ends_with("\n\n"));
        assert!(formatted.contains("\"type\":\"RUN_ERROR\""));
        assert!(formatted.contains("\"message\":\"test error\""));
    }

    #[test]
    fn test_format_sse_event_with_complex_event() {
        let event: Event = Event::TextMessageStart(TextMessageStartEvent::new(MessageId::random()));
        let formatted = format_sse_event(&event).unwrap();

        assert!(formatted.contains("\"type\":\"TEXT_MESSAGE_START\""));
        assert!(formatted.contains("\"messageId\":"));
        assert!(formatted.contains("\"role\":\"assistant\""));
    }
}
