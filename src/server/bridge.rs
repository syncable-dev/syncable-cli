//! Event Bridge - Converts agent events to AG-UI protocol events.
//!
//! This module provides the `EventBridge` which is the main integration
//! point between the syncable-cli agent and the AG-UI protocol.
//!
//! # Usage
//!
//! ```rust,ignore
//! let bridge = server.event_bridge();
//!
//! // Start a run
//! bridge.start_run().await;
//!
//! // Emit text message events
//! bridge.start_message().await;
//! bridge.emit_text_chunk("Hello, ").await;
//! bridge.emit_text_chunk("world!").await;
//! bridge.end_message().await;
//!
//! // Emit tool call events
//! let tool_id = bridge.start_tool_call("analyze", &args).await;
//! bridge.emit_tool_args_chunk(&tool_id, "partial args").await;
//! bridge.end_tool_call(&tool_id).await;
//!
//! // Finish the run
//! bridge.finish_run().await;
//! ```

use std::sync::Arc;

use syncable_ag_ui_core::{
    BaseEvent, Event, InterruptInfo, JsonValue, MessageId, Role, RunFinishedEvent,
    RunFinishedOutcome, RunId, RunStartedEvent, TextMessageContentEvent, TextMessageEndEvent,
    TextMessageStartEvent, ThreadId, ToolCallArgsEvent, ToolCallEndEvent, ToolCallId,
    ToolCallStartEvent,
};
use tokio::sync::{RwLock, broadcast};

/// Bridge between agent code and AG-UI protocol events.
///
/// This is the main interface for emitting events from agent code.
/// It handles the AG-UI protocol details like run IDs, message IDs,
/// and event sequencing.
#[derive(Clone)]
pub struct EventBridge {
    event_tx: broadcast::Sender<Event<JsonValue>>,
    thread_id: Arc<RwLock<ThreadId>>,
    run_id: Arc<RwLock<Option<RunId>>>,
    current_message_id: Arc<RwLock<Option<MessageId>>>,
    current_step_name: Arc<RwLock<Option<String>>>,
}

impl EventBridge {
    /// Creates a new event bridge.
    pub fn new(
        event_tx: broadcast::Sender<Event<JsonValue>>,
        thread_id: Arc<RwLock<ThreadId>>,
        run_id: Arc<RwLock<Option<RunId>>>,
    ) -> Self {
        Self {
            event_tx,
            thread_id,
            run_id,
            current_message_id: Arc::new(RwLock::new(None)),
            current_step_name: Arc::new(RwLock::new(None)),
        }
    }

    /// Emits an event to all connected clients.
    fn emit(&self, event: Event<JsonValue>) {
        // Ignore errors - clients may have disconnected
        let _ = self.event_tx.send(event);
    }

    // =========================================================================
    // Run Lifecycle
    // =========================================================================

    /// Starts a new agent run.
    ///
    /// Call this at the beginning of an agent interaction.
    pub async fn start_run(&self) {
        let thread_id = self.thread_id.read().await.clone();
        let run_id = RunId::random();

        // Store the run ID
        *self.run_id.write().await = Some(run_id.clone());

        self.emit(Event::RunStarted(RunStartedEvent {
            base: BaseEvent::with_current_timestamp(),
            thread_id,
            run_id,
        }));
    }

    /// Finishes the current run successfully.
    pub async fn finish_run(&self) {
        let thread_id = self.thread_id.read().await.clone();
        let run_id = self.run_id.write().await.take();
        let Some(run_id) = run_id else {
            return; // No active run
        };

        self.emit(Event::RunFinished(RunFinishedEvent {
            base: BaseEvent::with_current_timestamp(),
            thread_id,
            run_id,
            outcome: Some(RunFinishedOutcome::Success),
            result: None,
            interrupt: None,
        }));
    }

    /// Finishes the current run with an error.
    pub async fn finish_run_with_error(&self, message: &str) {
        let _run_id = self.run_id.write().await.take();

        self.emit(Event::RunError(syncable_ag_ui_core::RunErrorEvent {
            base: BaseEvent::with_current_timestamp(),
            message: message.to_string(),
            code: None,
        }));
    }

    // =========================================================================
    // Human-in-the-Loop Interrupts
    // =========================================================================

    /// Interrupt the current run for human-in-the-loop interaction.
    ///
    /// This emits a `RunFinished` event with `outcome: Interrupt`, signaling
    /// that the frontend should show approval UI and resume with user input.
    ///
    /// # Arguments
    /// * `reason` - Optional interrupt reason (e.g., "file_write", "deployment")
    /// * `payload` - Optional JSON payload with context for the approval UI
    pub async fn interrupt(&self, reason: Option<&str>, payload: Option<serde_json::Value>) {
        let thread_id = self.thread_id.read().await.clone();
        let run_id = self.run_id.write().await.take();
        let Some(run_id) = run_id else {
            return; // No active run
        };

        let mut info = InterruptInfo::new();
        if let Some(r) = reason {
            info = info.with_reason(r);
        }
        if let Some(p) = payload {
            info = info.with_payload(p);
        }

        self.emit(Event::RunFinished(RunFinishedEvent {
            base: BaseEvent::with_current_timestamp(),
            thread_id,
            run_id,
            outcome: Some(RunFinishedOutcome::Interrupt),
            result: None,
            interrupt: Some(info),
        }));
    }

    /// Interrupt with a tracking ID for correlation.
    ///
    /// The interrupt ID can be used by the client to correlate the resume
    /// request with the original interrupt.
    pub async fn interrupt_with_id(
        &self,
        id: &str,
        reason: Option<&str>,
        payload: Option<serde_json::Value>,
    ) {
        let thread_id = self.thread_id.read().await.clone();
        let run_id = self.run_id.write().await.take();
        let Some(run_id) = run_id else {
            return; // No active run
        };

        let mut info = InterruptInfo::new().with_id(id);
        if let Some(r) = reason {
            info = info.with_reason(r);
        }
        if let Some(p) = payload {
            info = info.with_payload(p);
        }

        self.emit(Event::RunFinished(RunFinishedEvent {
            base: BaseEvent::with_current_timestamp(),
            thread_id,
            run_id,
            outcome: Some(RunFinishedOutcome::Interrupt),
            result: None,
            interrupt: Some(info),
        }));
    }

    // =========================================================================
    // Text Messages (Agent Response)
    // =========================================================================

    /// Starts a new text message from the assistant.
    pub async fn start_message(&self) -> MessageId {
        let message_id = MessageId::random();
        *self.current_message_id.write().await = Some(message_id.clone());

        self.emit(Event::TextMessageStart(TextMessageStartEvent {
            base: BaseEvent::with_current_timestamp(),
            message_id: message_id.clone(),
            role: Role::Assistant,
        }));

        message_id
    }

    /// Emits a text chunk as part of the current message.
    pub async fn emit_text_chunk(&self, delta: &str) {
        let message_id = self.current_message_id.read().await.clone();
        if let Some(message_id) = message_id {
            self.emit(Event::TextMessageContent(
                TextMessageContentEvent::new_unchecked(message_id, delta),
            ));
        }
    }

    /// Ends the current text message.
    pub async fn end_message(&self) {
        let message_id = self.current_message_id.write().await.take();
        if let Some(message_id) = message_id {
            self.emit(Event::TextMessageEnd(TextMessageEndEvent {
                base: BaseEvent::with_current_timestamp(),
                message_id,
            }));
        }
    }

    /// Convenience: Emits a complete text message (start + content + end).
    pub async fn emit_message(&self, content: &str) {
        let _message_id = self.start_message().await;
        self.emit_text_chunk(content).await;
        self.end_message().await;
    }

    // =========================================================================
    // Tool Calls
    // =========================================================================

    /// Starts a tool call.
    ///
    /// Returns the tool call ID for use with subsequent events.
    pub async fn start_tool_call(&self, name: &str, args: &JsonValue) -> ToolCallId {
        let tool_call_id = ToolCallId::random();

        // Get current message ID or create one
        let message_id = {
            let mut current = self.current_message_id.write().await;
            if current.is_none() {
                *current = Some(MessageId::random());
            }
            current.clone().unwrap()
        };

        self.emit(Event::ToolCallStart(ToolCallStartEvent {
            base: BaseEvent::with_current_timestamp(),
            tool_call_id: tool_call_id.clone(),
            tool_call_name: name.to_string(),
            parent_message_id: Some(message_id),
        }));

        // Emit initial args if provided
        if !args.is_null() {
            if let Ok(args_str) = serde_json::to_string(args) {
                self.emit(Event::ToolCallArgs(ToolCallArgsEvent {
                    base: BaseEvent::with_current_timestamp(),
                    tool_call_id: tool_call_id.clone(),
                    delta: args_str,
                }));
            }
        }

        tool_call_id
    }

    /// Emits a chunk of tool call arguments (for streaming args).
    pub async fn emit_tool_args_chunk(&self, tool_call_id: &ToolCallId, delta: &str) {
        self.emit(Event::ToolCallArgs(ToolCallArgsEvent {
            base: BaseEvent::with_current_timestamp(),
            tool_call_id: tool_call_id.clone(),
            delta: delta.to_string(),
        }));
    }

    /// Ends a tool call.
    ///
    /// Note: Tool results are handled separately via messages in AG-UI protocol.
    pub async fn end_tool_call(&self, tool_call_id: &ToolCallId) {
        self.emit(Event::ToolCallEnd(ToolCallEndEvent {
            base: BaseEvent::with_current_timestamp(),
            tool_call_id: tool_call_id.clone(),
        }));
    }

    /// Convenience: Emits a complete tool call (start + end).
    pub async fn emit_tool_call(&self, name: &str, args: &JsonValue) {
        let tool_call_id = self.start_tool_call(name, args).await;
        self.end_tool_call(&tool_call_id).await;
    }

    // =========================================================================
    // State Updates
    // =========================================================================

    /// Emits a state snapshot.
    pub async fn emit_state_snapshot(&self, state: JsonValue) {
        self.emit(Event::StateSnapshot(
            syncable_ag_ui_core::StateSnapshotEvent {
                base: BaseEvent::with_current_timestamp(),
                snapshot: state,
            },
        ));
    }

    /// Emits a state delta (JSON Patch).
    pub async fn emit_state_delta(&self, delta: Vec<JsonValue>) {
        self.emit(Event::StateDelta(syncable_ag_ui_core::StateDeltaEvent {
            base: BaseEvent::with_current_timestamp(),
            delta,
        }));
    }

    // =========================================================================
    // Thinking/Progress
    // =========================================================================

    /// Starts a thinking/processing step.
    pub async fn start_thinking(&self, title: Option<&str>) {
        self.emit(Event::ThinkingStart(
            syncable_ag_ui_core::ThinkingStartEvent {
                base: BaseEvent::with_current_timestamp(),
                title: title.map(|s| s.to_string()),
            },
        ));
    }

    /// Ends the current thinking step.
    pub async fn end_thinking(&self) {
        self.emit(Event::ThinkingEnd(syncable_ag_ui_core::ThinkingEndEvent {
            base: BaseEvent::with_current_timestamp(),
        }));
    }

    /// Starts a step in the agent workflow.
    pub async fn start_step(&self, name: &str) {
        *self.current_step_name.write().await = Some(name.to_string());
        self.emit(Event::StepStarted(syncable_ag_ui_core::StepStartedEvent {
            base: BaseEvent::with_current_timestamp(),
            step_name: name.to_string(),
        }));
    }

    /// Ends the current step.
    pub async fn end_step(&self) {
        let step_name = self
            .current_step_name
            .write()
            .await
            .take()
            .unwrap_or_else(|| "unknown".to_string());
        self.emit(Event::StepFinished(
            syncable_ag_ui_core::StepFinishedEvent {
                base: BaseEvent::with_current_timestamp(),
                step_name,
            },
        ));
    }

    // =========================================================================
    // Custom Events
    // =========================================================================

    /// Emits a custom event.
    pub async fn emit_custom(&self, name: &str, value: JsonValue) {
        self.emit(Event::Custom(syncable_ag_ui_core::CustomEvent {
            base: BaseEvent::with_current_timestamp(),
            name: name.to_string(),
            value,
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_bridge() -> EventBridge {
        let (tx, _) = broadcast::channel(100);
        EventBridge::new(
            tx,
            Arc::new(RwLock::new(ThreadId::random())),
            Arc::new(RwLock::new(None)),
        )
    }

    #[tokio::test]
    async fn test_start_and_finish_run() {
        let bridge = create_bridge();

        bridge.start_run().await;
        assert!(bridge.run_id.read().await.is_some());

        bridge.finish_run().await;
        assert!(bridge.run_id.read().await.is_none());
    }

    #[tokio::test]
    async fn test_message_lifecycle() {
        let bridge = create_bridge();

        let _msg_id = bridge.start_message().await;
        assert!(bridge.current_message_id.read().await.is_some());

        bridge.emit_text_chunk("Hello").await;
        bridge.end_message().await;

        assert!(bridge.current_message_id.read().await.is_none());
    }

    #[tokio::test]
    async fn test_emit_complete_message() {
        let bridge = create_bridge();
        bridge.emit_message("Hello, world!").await;
        // Should not panic
    }

    #[tokio::test]
    async fn test_tool_call() {
        let bridge = create_bridge();

        let tool_id = bridge
            .start_tool_call("test", &serde_json::json!({"key": "value"}))
            .await;
        bridge.emit_tool_args_chunk(&tool_id, "more args").await;
        bridge.end_tool_call(&tool_id).await;
        // Should not panic
    }

    #[tokio::test]
    async fn test_interrupt() {
        let bridge = create_bridge();

        bridge.start_run().await;
        assert!(bridge.run_id.read().await.is_some());

        bridge.interrupt(Some("file_write"), None).await;
        // Run ID should be cleared after interrupt
        assert!(bridge.run_id.read().await.is_none());
    }

    #[tokio::test]
    async fn test_interrupt_with_payload() {
        let bridge = create_bridge();

        bridge.start_run().await;
        bridge
            .interrupt(
                Some("deployment"),
                Some(serde_json::json!({"file": "main.rs", "action": "write"})),
            )
            .await;
        assert!(bridge.run_id.read().await.is_none());
    }

    #[tokio::test]
    async fn test_interrupt_with_id() {
        let bridge = create_bridge();

        bridge.start_run().await;
        bridge
            .interrupt_with_id("int-123", Some("deployment"), None)
            .await;
        assert!(bridge.run_id.read().await.is_none());
    }

    #[tokio::test]
    async fn test_interrupt_without_run() {
        let bridge = create_bridge();

        // Interrupt without an active run should do nothing (not panic)
        bridge.interrupt(Some("test"), None).await;
    }

    #[tokio::test]
    async fn test_events_received_by_subscriber() {
        let (tx, mut rx) = broadcast::channel(100);
        let bridge = EventBridge::new(
            tx,
            Arc::new(RwLock::new(ThreadId::random())),
            Arc::new(RwLock::new(None)),
        );

        // Start a run
        bridge.start_run().await;

        // Receive the RunStarted event
        let event = rx.recv().await.expect("Should receive event");
        match event {
            Event::RunStarted(_) => {}
            _ => panic!("Expected RunStarted event"),
        }

        // Emit a message
        bridge.emit_message("Hello").await;

        // Should receive TextMessageStart, TextMessageContent, TextMessageEnd
        let event = rx.recv().await.expect("Should receive event");
        match event {
            Event::TextMessageStart(_) => {}
            _ => panic!("Expected TextMessageStart"),
        }

        let event = rx.recv().await.expect("Should receive event");
        match event {
            Event::TextMessageContent(_) => {}
            _ => panic!("Expected TextMessageContent"),
        }

        let event = rx.recv().await.expect("Should receive event");
        match event {
            Event::TextMessageEnd(_) => {}
            _ => panic!("Expected TextMessageEnd"),
        }
    }

    #[tokio::test]
    async fn test_step_and_thinking_events() {
        let (tx, mut rx) = broadcast::channel(100);
        let bridge = EventBridge::new(
            tx,
            Arc::new(RwLock::new(ThreadId::random())),
            Arc::new(RwLock::new(None)),
        );

        bridge.start_step("processing").await;
        let event = rx.recv().await.expect("Should receive event");
        match event {
            Event::StepStarted(_) => {}
            _ => panic!("Expected StepStarted"),
        }

        bridge.start_thinking(Some("Analyzing")).await;
        let event = rx.recv().await.expect("Should receive event");
        match event {
            Event::ThinkingStart(_) => {}
            _ => panic!("Expected ThinkingStart"),
        }

        bridge.end_thinking().await;
        let event = rx.recv().await.expect("Should receive event");
        match event {
            Event::ThinkingEnd(_) => {}
            _ => panic!("Expected ThinkingEnd"),
        }

        bridge.end_step().await;
        let event = rx.recv().await.expect("Should receive event");
        match event {
            Event::StepFinished(_) => {}
            _ => panic!("Expected StepFinished"),
        }
    }

    #[tokio::test]
    async fn test_state_snapshot_event() {
        let (tx, mut rx) = broadcast::channel(100);
        let bridge = EventBridge::new(
            tx,
            Arc::new(RwLock::new(ThreadId::random())),
            Arc::new(RwLock::new(None)),
        );

        let state = serde_json::json!({
            "model": "gpt-4",
            "turn_count": 5
        });

        bridge.emit_state_snapshot(state).await;

        let event = rx.recv().await.expect("Should receive event");
        match event {
            Event::StateSnapshot(e) => {
                assert_eq!(e.snapshot["model"], "gpt-4");
                assert_eq!(e.snapshot["turn_count"], 5);
            }
            _ => panic!("Expected StateSnapshot"),
        }
    }
}
