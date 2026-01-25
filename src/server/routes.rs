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

/// POST endpoint for receiving messages via HTTP.
///
/// Alternative to WebSocket for clients that prefer REST.
/// Accepts RunAgentInput JSON body and routes to agent processor.
pub async fn post_message(
    State(state): State<ServerState>,
    Json(input): Json<RunAgentInput>,
) -> Json<serde_json::Value> {
    let thread_id = input.thread_id.to_string();
    let run_id = input.run_id.to_string();

    debug!(
        thread_id = %thread_id,
        run_id = %run_id,
        message_count = input.messages.len(),
        "Received RunAgentInput via POST"
    );

    let message_tx = state.message_sender();
    let agent_msg = AgentMessage::new(input);

    match message_tx.send(agent_msg).await {
        Ok(_) => Json(json!({
            "status": "accepted",
            "thread_id": thread_id,
            "run_id": run_id
        })),
        Err(e) => {
            warn!("Failed to route message to agent processor: {}", e);
            Json(json!({
                "status": "error",
                "message": "Failed to route message to agent processor"
            }))
        }
    }
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
    use ag_ui_core::types::Message as AgUiProtocolMessage;
    use ag_ui_core::{RunId, ThreadId};
    use axum::extract::State;

    #[tokio::test]
    async fn test_health_endpoint() {
        let response = health().await;
        assert_eq!(response.0["status"], "ok");
        assert_eq!(response.0["protocol"], "ag-ui");
    }

    #[tokio::test]
    async fn test_post_message_accepted() {
        use crate::server::ServerState;

        let state = ServerState::new();
        let mut msg_rx = state.take_message_receiver().await.expect("Should get receiver");

        // Create RunAgentInput
        let thread_id = ThreadId::random();
        let run_id = RunId::random();
        let input = RunAgentInput::new(thread_id.clone(), run_id.clone())
            .with_messages(vec![AgUiProtocolMessage::new_user("Hello from POST")]);

        // Call post_message handler
        let response = post_message(State(state), Json(input)).await;

        // Verify response
        assert_eq!(response.0["status"], "accepted");
        assert_eq!(response.0["thread_id"], thread_id.to_string());
        assert_eq!(response.0["run_id"], run_id.to_string());

        // Verify message was routed
        let received = msg_rx.recv().await.expect("Should receive message");
        assert_eq!(received.input.messages.len(), 1);
    }
}
