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
        IntoResponse, Response,
    },
    Json,
};
use futures_util::{SinkExt, Stream, StreamExt};
use serde::Deserialize;
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

/// Runtime info endpoint for CopilotKit.
///
/// CopilotKit expects this endpoint to return information about
/// available agents and actions. Agents must be an object/map with
/// agent ID as key, not an array.
pub async fn info() -> Json<serde_json::Value> {
    Json(json!({
        "version": "1.0.0",
        "agents": {
            "syncable": {
                "name": "syncable",
                "className": "HttpAgent",
                "description": "Syncable CLI Agent - Kubernetes and DevOps assistant"
            }
        },
        "actions": {},
        "audioFileTranscriptionEnabled": false
    }))
}

/// CopilotKit request body structure.
/// CopilotKit wraps requests in an envelope with method, params, and body.
#[derive(Debug, Clone, Deserialize)]
pub struct CopilotKitRequest {
    /// The method being called (e.g., "agent/run")
    pub method: Option<String>,
    /// Method parameters
    pub params: Option<CopilotKitParams>,
    /// The actual request body
    pub body: Option<CopilotKitBody>,
    /// Direct fields for RunAgentInput format (non-envelope)
    #[serde(rename = "threadId")]
    pub thread_id: Option<String>,
    #[serde(rename = "runId")]
    pub run_id: Option<String>,
    pub messages: Option<Vec<serde_json::Value>>,
    pub tools: Option<Vec<serde_json::Value>>,
    pub context: Option<Vec<serde_json::Value>>,
    pub state: Option<serde_json::Value>,
    #[serde(rename = "forwardedProps")]
    pub forwarded_props: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CopilotKitParams {
    #[serde(rename = "agentId")]
    pub agent_id: Option<String>,
    #[serde(rename = "threadId")]
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CopilotKitBody {
    pub messages: Option<Vec<serde_json::Value>>,
    #[serde(rename = "threadId")]
    pub thread_id: Option<String>,
    #[serde(rename = "runId")]
    pub run_id: Option<String>,
    pub tools: Option<Vec<serde_json::Value>>,
    pub context: Option<Vec<serde_json::Value>>,
    pub state: Option<serde_json::Value>,
    #[serde(rename = "forwardedProps")]
    pub forwarded_props: Option<serde_json::Value>,
}

/// POST endpoint for receiving messages via HTTP.
///
/// Accepts both CopilotKit envelope format and direct RunAgentInput format.
/// Routes messages to the agent processor and returns an SSE stream of events.
/// Also handles CopilotKit's "info" method requests.
pub async fn post_message(
    State(state): State<ServerState>,
    Json(raw): Json<serde_json::Value>,
) -> Response {
    debug!("Received POST request body: {}", serde_json::to_string_pretty(&raw).unwrap_or_default());

    // Try to parse as CopilotKit request
    let copilot_req: Result<CopilotKitRequest, _> = serde_json::from_value(raw.clone());

    // Track original thread/run IDs for response (may not be valid UUIDs)
    let (input, original_thread_id, original_run_id) = match copilot_req {
        Ok(req) => {
            // Check if this is an envelope format (has method field)
            if let Some(ref method) = req.method {
                debug!("Detected CopilotKit envelope format, method: {:?}", method);

                // Handle "info" method - return runtime info
                if method == "info" {
                    return Json(json!({
                        "version": "1.0.0",
                        "agents": {
                            "syncable": {
                                "name": "syncable",
                                "className": "HttpAgent",
                                "description": "Syncable CLI Agent - Kubernetes and DevOps assistant"
                            }
                        },
                        "actions": {},
                        "audioFileTranscriptionEnabled": false
                    })).into_response();
                }

                // Extract from envelope body
                let body = req.body.unwrap_or(CopilotKitBody {
                    messages: None,
                    thread_id: None,
                    run_id: None,
                    tools: None,
                    context: None,
                    state: None,
                    forwarded_props: None,
                });

                let thread_id_str = body.thread_id
                    .or(req.params.as_ref().and_then(|p| p.thread_id.clone()))
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let run_id_str = body.run_id
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                // Parse IDs, falling back to random if invalid UUID
                let thread_id: ag_ui_core::ThreadId = thread_id_str.parse()
                    .unwrap_or_else(|_| ag_ui_core::ThreadId::random());
                let run_id: ag_ui_core::RunId = run_id_str.parse()
                    .unwrap_or_else(|_| ag_ui_core::RunId::random());

                // Convert messages from JSON to Message type
                let messages = convert_messages(body.messages.unwrap_or_default());
                let tools = convert_tools(body.tools.unwrap_or_default());
                let context = convert_context(body.context.unwrap_or_default());

                let input = RunAgentInput::new(thread_id, run_id)
                    .with_messages(messages)
                    .with_tools(tools)
                    .with_context(context)
                    .with_state(body.state.unwrap_or(serde_json::Value::Null))
                    .with_forwarded_props(body.forwarded_props.unwrap_or(serde_json::Value::Null));

                (input, thread_id_str, run_id_str)
            } else if req.thread_id.is_some() || req.messages.is_some() {
                // Direct RunAgentInput format
                debug!("Detected direct RunAgentInput format");

                let thread_id_str = req.thread_id
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let run_id_str = req.run_id
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                // Parse IDs, falling back to random if invalid UUID
                let thread_id: ag_ui_core::ThreadId = thread_id_str.parse()
                    .unwrap_or_else(|_| ag_ui_core::ThreadId::random());
                let run_id: ag_ui_core::RunId = run_id_str.parse()
                    .unwrap_or_else(|_| ag_ui_core::RunId::random());

                let messages = convert_messages(req.messages.unwrap_or_default());
                let tools = convert_tools(req.tools.unwrap_or_default());
                let context = convert_context(req.context.unwrap_or_default());

                let input = RunAgentInput::new(thread_id, run_id)
                    .with_messages(messages)
                    .with_tools(tools)
                    .with_context(context)
                    .with_state(req.state.unwrap_or(serde_json::Value::Null))
                    .with_forwarded_props(req.forwarded_props.unwrap_or(serde_json::Value::Null));

                (input, thread_id_str, run_id_str)
            } else {
                warn!("Could not parse request format: {:?}", raw);
                return Json(json!({
                    "status": "error",
                    "message": "Invalid request format"
                })).into_response();
            }
        }
        Err(e) => {
            warn!("Failed to parse request: {}", e);
            return Json(json!({
                "status": "error",
                "message": format!("Failed to parse request: {}", e)
            })).into_response();
        }
    };

    // Use original string IDs for response (preserves non-UUID IDs like "thread-123")
    let thread_id = original_thread_id;
    let run_id = original_run_id;

    debug!(
        thread_id = %thread_id,
        run_id = %run_id,
        message_count = input.messages.len(),
        "Processed RunAgentInput via POST"
    );

    // Subscribe to events BEFORE sending message to avoid race condition
    let mut event_rx = state.subscribe();

    let message_tx = state.message_sender();
    let agent_msg = AgentMessage::new(input);

    if let Err(e) = message_tx.send(agent_msg).await {
        warn!("Failed to route message to agent processor: {}", e);
        return Json(json!({
            "status": "error",
            "message": "Failed to route message to agent processor"
        })).into_response();
    }

    // Create SSE stream that filters events and ends on RunFinished/RunError
    let stream = async_stream::stream! {
        use ag_ui_core::Event;

        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let is_terminal = matches!(&event, Event::RunFinished(_) | Event::RunError(_));

                    // Serialize event to JSON
                    if let Ok(json) = serde_json::to_string(&event) {
                        let event_type = event.event_type().as_str().to_string();
                        yield Ok::<_, Infallible>(SseEvent::default()
                            .event(event_type)
                            .data(json));
                    }

                    // Stop streaming after terminal event
                    if is_terminal {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    // Missed some events, continue
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // Channel closed, stop streaming
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}

/// Convert JSON messages to AG-UI Message type
fn convert_messages(raw_messages: Vec<serde_json::Value>) -> Vec<ag_ui_core::types::Message> {
    use ag_ui_core::MessageId;

    raw_messages
        .into_iter()
        .filter_map(|msg| {
            let role = msg.get("role")?.as_str()?;
            let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
            let id_str = msg.get("id")
                .and_then(|i| i.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            // Parse ID, falling back to random if invalid UUID format
            let id: MessageId = id_str.parse().unwrap_or_else(|_| MessageId::random());

            match role {
                "user" => Some(ag_ui_core::types::Message::User {
                    id,
                    content: content.to_string(),
                    name: msg.get("name").and_then(|n| n.as_str()).map(String::from),
                }),
                "assistant" => Some(ag_ui_core::types::Message::Assistant {
                    id,
                    content: Some(content.to_string()),
                    name: msg.get("name").and_then(|n| n.as_str()).map(String::from),
                    tool_calls: None,
                }),
                "system" => Some(ag_ui_core::types::Message::System {
                    id,
                    content: content.to_string(),
                    name: msg.get("name").and_then(|n| n.as_str()).map(String::from),
                }),
                _ => {
                    debug!("Unknown message role: {}", role);
                    None
                }
            }
        })
        .collect()
}

/// Convert JSON tools to AG-UI Tool type
fn convert_tools(raw_tools: Vec<serde_json::Value>) -> Vec<ag_ui_core::types::Tool> {
    raw_tools
        .into_iter()
        .filter_map(|tool| {
            let name = tool.get("name")?.as_str()?.to_string();
            let description = tool.get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let parameters = tool.get("parameters")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            Some(ag_ui_core::types::Tool::new(name, description, parameters))
        })
        .collect()
}

/// Convert JSON context to AG-UI Context type
fn convert_context(raw_context: Vec<serde_json::Value>) -> Vec<ag_ui_core::types::Context> {
    raw_context
        .into_iter()
        .filter_map(|ctx| {
            let description = ctx.get("description")?.as_str()?.to_string();
            let value = ctx.get("value")?.as_str()?.to_string();
            Some(ag_ui_core::types::Context::new(description, value))
        })
        .collect()
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
        use http::StatusCode;

        let state = ServerState::new();
        let mut msg_rx = state.take_message_receiver().await.expect("Should get receiver");

        // Create RunAgentInput as JSON value
        let thread_id = ThreadId::random();
        let run_id = RunId::random();
        let input_json = json!({
            "threadId": thread_id.to_string(),
            "runId": run_id.to_string(),
            "messages": [{
                "id": "msg-1",
                "role": "user",
                "content": "Hello from POST"
            }],
            "tools": [],
            "context": [],
            "state": null,
            "forwardedProps": null
        });

        // Call post_message handler with raw JSON
        let response = post_message(State(state), Json(input_json)).await;

        // Verify response is SSE stream (HTTP 200)
        assert_eq!(response.status(), StatusCode::OK);

        // Verify message was routed
        let received = msg_rx.recv().await.expect("Should receive message");
        assert_eq!(received.input.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_post_message_copilotkit_envelope() {
        use crate::server::ServerState;
        use http::StatusCode;

        let state = ServerState::new();
        let mut msg_rx = state.take_message_receiver().await.expect("Should get receiver");

        // Create CopilotKit envelope format
        let input_json = json!({
            "method": "agent/run",
            "params": { "agentId": "syncable" },
            "body": {
                "threadId": "thread-123",
                "messages": [{
                    "id": "msg-1",
                    "role": "user",
                    "content": "Hello from CopilotKit"
                }]
            }
        });

        // Call post_message handler
        let response = post_message(State(state), Json(input_json)).await;

        // Verify response is SSE stream (HTTP 200)
        assert_eq!(response.status(), StatusCode::OK);

        // Verify message was routed
        let received = msg_rx.recv().await.expect("Should receive message");
        assert_eq!(received.input.messages.len(), 1);
    }
}
