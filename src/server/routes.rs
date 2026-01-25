//! HTTP Routes for AG-UI Server
//!
//! This module provides the HTTP endpoints for AG-UI protocol:
//! - `/sse` - Server-Sent Events endpoint
//! - `/ws` - WebSocket endpoint
//! - `/health` - Health check endpoint

use std::convert::Infallible;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{
        sse::{Event as SseEvent, KeepAlive, Sse},
        Response,
    },
    Json,
};
use futures_util::{SinkExt, Stream, StreamExt};
use serde_json::json;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, warn};

use super::{AgentMessage, RunAgentInput, ServerState};

/// Health check endpoint.
pub async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "syncable-cli-agent",
        "protocol": "ag-ui"
    }))
}

/// SSE endpoint for streaming AG-UI events.
pub async fn sse_handler(
    State(state): State<ServerState>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let rx = state.subscribe();
    let stream = BroadcastStream::new(rx);

    let event_stream = stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                // Serialize event to JSON
                let json = serde_json::to_string(&event).ok()?;
                let event_type = event.event_type().as_str().to_string();

                Some(Ok(SseEvent::default()
                    .event(event_type)
                    .data(json)))
            }
            Err(_) => None, // Lagged, skip this event
        }
    });

    Sse::new(event_stream).keep_alive(KeepAlive::default())
}

/// WebSocket endpoint for streaming AG-UI events.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// Handles a WebSocket connection.
async fn handle_websocket(socket: WebSocket, state: ServerState) {
    let (mut sender, mut receiver) = socket.split();
    let mut event_rx = state.subscribe();
    let message_tx = state.message_sender();

    // Spawn task to send events to client
    let send_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break; // Client disconnected
                }
            }
        }
    });

    // Handle incoming messages from client
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Close(_)) => break,
                Ok(Message::Ping(_)) => {
                    // Pong is handled automatically by axum
                }
                Ok(Message::Text(text)) => {
                    // Parse as RunAgentInput and route to agent processor
                    match serde_json::from_str::<RunAgentInput>(&text) {
                        Ok(input) => {
                            debug!(
                                thread_id = %input.thread_id,
                                run_id = %input.run_id,
                                message_count = input.messages.len(),
                                "Received RunAgentInput via WebSocket"
                            );
                            let agent_msg = AgentMessage::new(input);
                            if let Err(e) = message_tx.send(agent_msg).await {
                                warn!("Failed to send message to agent processor: {}", e);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse WebSocket message as RunAgentInput: {}", e);
                            // Log but continue - don't crash the connection
                        }
                    }
                }
                Ok(Message::Binary(_)) => {
                    // Binary messages not supported yet
                    debug!("Received binary WebSocket message, ignoring");
                }
                Ok(Message::Pong(_)) => {
                    // Pong response, ignore
                }
                Err(e) => {
                    warn!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        let response = health().await;
        assert_eq!(response.0["status"], "ok");
        assert_eq!(response.0["protocol"], "ag-ui");
    }
}
