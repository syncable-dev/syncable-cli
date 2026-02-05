//! Event producer API for emitting AG-UI events.
//!
//! This module provides the high-level API for agents to emit events to connected
//! frontends. It includes:
//!
//! - [`EventProducer`] trait - Core abstraction for event emission
//! - [`MessageStream`] - Helper for streaming text messages
//! - [`ToolCallStream`] - Helper for streaming tool calls
//! - [`ThinkingMessageStream`] - Helper for streaming thinking content
//! - [`ThinkingStep`] - Helper for thinking block boundaries (chain-of-thought)
//! - [`AgentSession`] - Manages run lifecycle and state
//!
//! # Example
//!
//! ```rust,ignore
//! use ag_ui_server::{transport::sse, AgentSession, MessageStream};
//!
//! async fn handle_request() -> impl IntoResponse {
//!     let (sender, handler) = sse::channel(32);
//!
//!     tokio::spawn(async move {
//!         let mut session = AgentSession::new(sender);
//!         session.start_run().await.unwrap();
//!
//!         // Stream a message
//!         let msg = MessageStream::start(session.producer()).await.unwrap();
//!         msg.content("Hello, ").await.unwrap();
//!         msg.content("world!").await.unwrap();
//!         msg.end().await.unwrap();
//!
//!         session.finish_run(None).await.unwrap();
//!     });
//!
//!     handler.into_response()
//! }
//! ```

use std::marker::PhantomData;

use ag_ui_core::{
    AgentState, Event, InterruptInfo, JsonValue, MessageId, RunErrorEvent, RunFinishedEvent,
    RunId, RunStartedEvent, TextMessageContentEvent, TextMessageEndEvent, TextMessageStartEvent,
    ThinkingEndEvent, ThinkingStartEvent, ThinkingTextMessageContentEvent,
    ThinkingTextMessageEndEvent, ThinkingTextMessageStartEvent, ThreadId, ToolCallArgsEvent,
    ToolCallEndEvent, ToolCallId, ToolCallStartEvent,
};
use async_trait::async_trait;

use crate::error::ServerError;
use crate::transport::SseSender;

/// Trait for producing AG-UI events.
///
/// Implementors of this trait can emit events to connected frontends
/// through various transport mechanisms (SSE, WebSocket, etc.).
///
/// # Example
///
/// ```rust,ignore
/// use ag_ui_server::EventProducer;
/// use ag_ui_core::{Event, RunErrorEvent};
///
/// async fn emit_error<P: EventProducer>(producer: &P) -> Result<(), ServerError> {
///     producer.emit(Event::RunError(RunErrorEvent::new("Something went wrong"))).await
/// }
/// ```
#[async_trait]
pub trait EventProducer<StateT: AgentState = JsonValue>: Send + Sync {
    /// Emit a single event to connected clients.
    ///
    /// Returns an error if the connection is closed or the event cannot be sent.
    async fn emit(&self, event: Event<StateT>) -> Result<(), ServerError>;

    /// Emit multiple events to connected clients.
    ///
    /// Events are sent in order. Stops and returns an error on the first failure.
    async fn emit_many(&self, events: Vec<Event<StateT>>) -> Result<(), ServerError> {
        for event in events {
            self.emit(event).await?;
        }
        Ok(())
    }

    /// Check if the connection is still open.
    ///
    /// Returns `false` if the client has disconnected.
    fn is_connected(&self) -> bool;
}

// Implement EventProducer for SseSender
#[async_trait]
impl<StateT: AgentState> EventProducer<StateT> for SseSender<StateT> {
    async fn emit(&self, event: Event<StateT>) -> Result<(), ServerError> {
        self.send(event)
            .await
            .map_err(|_| ServerError::Channel("SSE channel closed".into()))
    }

    fn is_connected(&self) -> bool {
        !self.is_closed()
    }
}

/// Helper for streaming a text message piece by piece.
///
/// This struct manages the lifecycle of a streaming text message, automatically
/// generating message IDs and emitting the appropriate events.
///
/// # Example
///
/// ```rust,ignore
/// let msg = MessageStream::start(&producer).await?;
/// msg.content("Hello, ").await?;
/// msg.content("world!").await?;
/// let message_id = msg.end().await?;
/// ```
pub struct MessageStream<'a, P: EventProducer<StateT>, StateT: AgentState = JsonValue> {
    producer: &'a P,
    message_id: MessageId,
    _state: PhantomData<StateT>,
}

impl<'a, P: EventProducer<StateT>, StateT: AgentState> MessageStream<'a, P, StateT> {
    /// Start a new message stream.
    ///
    /// Emits a `TextMessageStart` event with a randomly generated message ID.
    pub async fn start(producer: &'a P) -> Result<Self, ServerError> {
        let message_id = MessageId::random();
        producer
            .emit(Event::TextMessageStart(TextMessageStartEvent::new(
                message_id.clone(),
            )))
            .await?;
        Ok(Self {
            producer,
            message_id,
            _state: PhantomData,
        })
    }

    /// Start a new message stream with a specific message ID.
    pub async fn start_with_id(
        producer: &'a P,
        message_id: MessageId,
    ) -> Result<Self, ServerError> {
        producer
            .emit(Event::TextMessageStart(TextMessageStartEvent::new(
                message_id.clone(),
            )))
            .await?;
        Ok(Self {
            producer,
            message_id,
            _state: PhantomData,
        })
    }

    /// Append content to the message.
    ///
    /// Emits a `TextMessageContent` event with the given delta.
    /// Empty deltas are silently ignored.
    pub async fn content(&self, delta: impl Into<String>) -> Result<(), ServerError> {
        let delta = delta.into();
        if delta.is_empty() {
            return Ok(());
        }
        self.producer
            .emit(Event::TextMessageContent(
                TextMessageContentEvent::new_unchecked(self.message_id.clone(), delta),
            ))
            .await
    }

    /// End the message stream.
    ///
    /// Emits a `TextMessageEnd` event and returns the message ID.
    /// Consumes the stream to prevent further content being added.
    pub async fn end(self) -> Result<MessageId, ServerError> {
        self.producer
            .emit(Event::TextMessageEnd(TextMessageEndEvent::new(
                self.message_id.clone(),
            )))
            .await?;
        Ok(self.message_id)
    }

    /// Get the message ID for this stream.
    pub fn message_id(&self) -> &MessageId {
        &self.message_id
    }
}

/// Helper for streaming a tool call with arguments.
///
/// This struct manages the lifecycle of a streaming tool call, automatically
/// generating tool call IDs and emitting the appropriate events.
///
/// # Example
///
/// ```rust,ignore
/// let call = ToolCallStream::start(&producer, "get_weather").await?;
/// call.args(r#"{"location": "#).await?;
/// call.args(r#""New York"}"#).await?;
/// let tool_call_id = call.end().await?;
/// ```
pub struct ToolCallStream<'a, P: EventProducer<StateT>, StateT: AgentState = JsonValue> {
    producer: &'a P,
    tool_call_id: ToolCallId,
    _state: PhantomData<StateT>,
}

impl<'a, P: EventProducer<StateT>, StateT: AgentState> ToolCallStream<'a, P, StateT> {
    /// Start a new tool call stream.
    ///
    /// Emits a `ToolCallStart` event with the given tool name and a randomly
    /// generated tool call ID.
    pub async fn start(producer: &'a P, name: impl Into<String>) -> Result<Self, ServerError> {
        let tool_call_id = ToolCallId::random();
        producer
            .emit(Event::ToolCallStart(ToolCallStartEvent::new(
                tool_call_id.clone(),
                name,
            )))
            .await?;
        Ok(Self {
            producer,
            tool_call_id,
            _state: PhantomData,
        })
    }

    /// Start a new tool call stream with a specific tool call ID.
    pub async fn start_with_id(
        producer: &'a P,
        tool_call_id: ToolCallId,
        name: impl Into<String>,
    ) -> Result<Self, ServerError> {
        producer
            .emit(Event::ToolCallStart(ToolCallStartEvent::new(
                tool_call_id.clone(),
                name,
            )))
            .await?;
        Ok(Self {
            producer,
            tool_call_id,
            _state: PhantomData,
        })
    }

    /// Stream an argument chunk.
    ///
    /// Emits a `ToolCallArgs` event with the given delta.
    pub async fn args(&self, delta: impl Into<String>) -> Result<(), ServerError> {
        self.producer
            .emit(Event::ToolCallArgs(ToolCallArgsEvent::new(
                self.tool_call_id.clone(),
                delta,
            )))
            .await
    }

    /// End the tool call stream.
    ///
    /// Emits a `ToolCallEnd` event and returns the tool call ID.
    /// Consumes the stream to prevent further args being added.
    pub async fn end(self) -> Result<ToolCallId, ServerError> {
        self.producer
            .emit(Event::ToolCallEnd(ToolCallEndEvent::new(
                self.tool_call_id.clone(),
            )))
            .await?;
        Ok(self.tool_call_id)
    }

    /// Get the tool call ID for this stream.
    pub fn tool_call_id(&self) -> &ToolCallId {
        &self.tool_call_id
    }
}

/// Helper for streaming thinking content (extended thinking / chain-of-thought).
///
/// This struct manages the lifecycle of streaming thinking content. Unlike
/// [`MessageStream`], thinking messages don't have IDs as they're ephemeral.
///
/// # Example
///
/// ```rust,ignore
/// let thinking = ThinkingMessageStream::start(&producer).await?;
/// thinking.content("Let me analyze this...").await?;
/// thinking.content("The key factors are...").await?;
/// thinking.end().await?;
/// ```
pub struct ThinkingMessageStream<'a, P: EventProducer<StateT>, StateT: AgentState = JsonValue> {
    producer: &'a P,
    _state: PhantomData<StateT>,
}

impl<'a, P: EventProducer<StateT>, StateT: AgentState> ThinkingMessageStream<'a, P, StateT> {
    /// Start a new thinking message stream.
    ///
    /// Emits a `ThinkingTextMessageStart` event.
    pub async fn start(producer: &'a P) -> Result<Self, ServerError> {
        producer
            .emit(Event::ThinkingTextMessageStart(
                ThinkingTextMessageStartEvent::new(),
            ))
            .await?;
        Ok(Self {
            producer,
            _state: PhantomData,
        })
    }

    /// Append content to the thinking message.
    ///
    /// Emits a `ThinkingTextMessageContent` event with the given delta.
    /// Unlike regular messages, empty deltas are allowed for thinking content.
    pub async fn content(&self, delta: impl Into<String>) -> Result<(), ServerError> {
        self.producer
            .emit(Event::ThinkingTextMessageContent(
                ThinkingTextMessageContentEvent::new(delta),
            ))
            .await
    }

    /// End the thinking message stream.
    ///
    /// Emits a `ThinkingTextMessageEnd` event.
    /// Consumes the stream to prevent further content being added.
    pub async fn end(self) -> Result<(), ServerError> {
        self.producer
            .emit(Event::ThinkingTextMessageEnd(
                ThinkingTextMessageEndEvent::new(),
            ))
            .await
    }
}

/// Helper for managing thinking block boundaries (chain-of-thought steps).
///
/// This struct wraps a thinking block with `ThinkingStart` and `ThinkingEnd` events.
/// Inside a thinking step, you can emit thinking content using [`ThinkingMessageStream`].
///
/// # Example
///
/// ```rust,ignore
/// // Start a thinking step with optional title
/// let step = ThinkingStep::start(&producer, Some("Analyzing user query")).await?;
///
/// // Emit thinking content inside the step
/// let thinking = ThinkingMessageStream::start(step.producer()).await?;
/// thinking.content("First, let me consider...").await?;
/// thinking.end().await?;
///
/// // End the thinking step
/// step.end().await?;
/// ```
pub struct ThinkingStep<'a, P: EventProducer<StateT>, StateT: AgentState = JsonValue> {
    producer: &'a P,
    _state: PhantomData<StateT>,
}

impl<'a, P: EventProducer<StateT>, StateT: AgentState> ThinkingStep<'a, P, StateT> {
    /// Start a new thinking step.
    ///
    /// Emits a `ThinkingStart` event with an optional title.
    pub async fn start(
        producer: &'a P,
        title: Option<impl Into<String>>,
    ) -> Result<Self, ServerError> {
        let event = if let Some(t) = title {
            ThinkingStartEvent::new().with_title(t)
        } else {
            ThinkingStartEvent::new()
        };
        producer.emit(Event::ThinkingStart(event)).await?;
        Ok(Self {
            producer,
            _state: PhantomData,
        })
    }

    /// End the thinking step.
    ///
    /// Emits a `ThinkingEnd` event.
    /// Consumes the step to prevent reuse.
    pub async fn end(self) -> Result<(), ServerError> {
        self.producer
            .emit(Event::ThinkingEnd(ThinkingEndEvent::new()))
            .await
    }

    /// Get a reference to the underlying producer.
    ///
    /// Use this to create [`ThinkingMessageStream`] instances inside the step.
    pub fn producer(&self) -> &'a P {
        self.producer
    }
}

/// Manages an agent session with run lifecycle events.
///
/// This struct provides high-level management of agent runs, including
/// starting, finishing, and error handling.
///
/// # Example
///
/// ```rust,ignore
/// let mut session = AgentSession::new(sender);
///
/// // Start a run
/// let run_id = session.start_run().await?;
///
/// // Do work...
///
/// // Finish the run
/// session.finish_run(Some(json!({"result": "success"}))).await?;
/// ```
pub struct AgentSession<P: EventProducer<StateT>, StateT: AgentState = JsonValue> {
    producer: P,
    thread_id: ThreadId,
    current_run: Option<RunId>,
    _state: PhantomData<StateT>,
}

impl<P: EventProducer<StateT>, StateT: AgentState> AgentSession<P, StateT> {
    /// Create a new session with the given producer.
    ///
    /// Generates a random thread ID for the session.
    pub fn new(producer: P) -> Self {
        Self {
            producer,
            thread_id: ThreadId::random(),
            current_run: None,
            _state: PhantomData,
        }
    }

    /// Create a new session with a specific thread ID.
    pub fn with_thread_id(producer: P, thread_id: ThreadId) -> Self {
        Self {
            producer,
            thread_id,
            current_run: None,
            _state: PhantomData,
        }
    }

    /// Start a new run.
    ///
    /// Emits a `RunStarted` event and stores the run ID.
    /// Returns an error if a run is already in progress.
    pub async fn start_run(&mut self) -> Result<RunId, ServerError> {
        if self.current_run.is_some() {
            return Err(ServerError::Channel("Run already in progress".into()));
        }
        let run_id = RunId::random();
        self.producer
            .emit(Event::RunStarted(RunStartedEvent::new(
                self.thread_id.clone(),
                run_id.clone(),
            )))
            .await?;
        self.current_run = Some(run_id.clone());
        Ok(run_id)
    }

    /// Finish the current run.
    ///
    /// Emits a `RunFinished` event with an optional result.
    /// Does nothing if no run is in progress.
    pub async fn finish_run(&mut self, result: Option<JsonValue>) -> Result<(), ServerError> {
        if let Some(run_id) = self.current_run.take() {
            let mut event = RunFinishedEvent::new(self.thread_id.clone(), run_id);
            if let Some(r) = result {
                event = event.with_result(r);
            }
            self.producer.emit(Event::RunFinished(event)).await?;
        }
        Ok(())
    }

    /// Signal a run error.
    ///
    /// Emits a `RunError` event and clears the current run.
    pub async fn run_error(&mut self, message: impl Into<String>) -> Result<(), ServerError> {
        self.current_run = None;
        self.producer
            .emit(Event::RunError(RunErrorEvent::new(message)))
            .await
    }

    /// Signal a run error with an error code.
    pub async fn run_error_with_code(
        &mut self,
        message: impl Into<String>,
        code: impl Into<String>,
    ) -> Result<(), ServerError> {
        self.current_run = None;
        self.producer
            .emit(Event::RunError(
                RunErrorEvent::new(message).with_code(code),
            ))
            .await
    }

    /// Get a reference to the underlying producer.
    pub fn producer(&self) -> &P {
        &self.producer
    }

    /// Get the thread ID for this session.
    pub fn thread_id(&self) -> &ThreadId {
        &self.thread_id
    }

    /// Get the current run ID, if any.
    pub fn run_id(&self) -> Option<&RunId> {
        self.current_run.as_ref()
    }

    /// Check if a run is currently in progress.
    pub fn is_running(&self) -> bool {
        self.current_run.is_some()
    }

    /// Check if the connection is still open.
    pub fn is_connected(&self) -> bool {
        self.producer.is_connected()
    }

    /// Start a thinking step.
    ///
    /// Convenience method that creates a [`ThinkingStep`] using this session's producer.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let step = session.start_thinking(Some("Planning response")).await?;
    /// // ... emit thinking content ...
    /// step.end().await?;
    /// ```
    pub async fn start_thinking(
        &self,
        title: Option<impl Into<String>>,
    ) -> Result<ThinkingStep<'_, P, StateT>, ServerError> {
        ThinkingStep::start(&self.producer, title).await
    }

    /// Interrupt the current run for human-in-the-loop interaction.
    ///
    /// Finishes the run with an interrupt outcome, signaling that human input
    /// is required before the agent can continue. The client should display
    /// appropriate UI based on the interrupt info and resume with user input.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// session.start_run().await?;
    ///
    /// // Request human approval
    /// session.interrupt(
    ///     Some("human_approval"),
    ///     Some(serde_json::json!({"action": "send_email", "to": "user@example.com"}))
    /// ).await?;
    /// ```
    pub async fn interrupt(
        &mut self,
        reason: Option<impl Into<String>>,
        payload: Option<JsonValue>,
    ) -> Result<(), ServerError> {
        let run_id = self.current_run.take();
        if let Some(run_id) = run_id {
            let mut info = InterruptInfo::new();
            if let Some(r) = reason {
                info = info.with_reason(r);
            }
            if let Some(p) = payload {
                info = info.with_payload(p);
            }

            let event = RunFinishedEvent::new(self.thread_id.clone(), run_id).with_interrupt(info);
            self.producer.emit(Event::RunFinished(event)).await?;
        }
        Ok(())
    }

    /// Interrupt with a specific interrupt ID for tracking.
    ///
    /// The interrupt ID can be used by the client to correlate the resume
    /// request with the original interrupt.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// session.start_run().await?;
    ///
    /// // Request approval with tracking ID
    /// session.interrupt_with_id(
    ///     "approval-001",
    ///     Some("database_modification"),
    ///     Some(serde_json::json!({"query": "DELETE FROM users WHERE inactive"}))
    /// ).await?;
    /// ```
    pub async fn interrupt_with_id(
        &mut self,
        id: impl Into<String>,
        reason: Option<impl Into<String>>,
        payload: Option<JsonValue>,
    ) -> Result<(), ServerError> {
        let run_id = self.current_run.take();
        if let Some(run_id) = run_id {
            let mut info = InterruptInfo::new().with_id(id);
            if let Some(r) = reason {
                info = info.with_reason(r);
            }
            if let Some(p) = payload {
                info = info.with_payload(p);
            }

            let event = RunFinishedEvent::new(self.thread_id.clone(), run_id).with_interrupt(info);
            self.producer.emit(Event::RunFinished(event)).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Mock producer for testing
    struct MockProducer {
        events: Arc<Mutex<Vec<Event>>>,
        connected: bool,
    }

    impl MockProducer {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                connected: true,
            }
        }

        fn events(&self) -> Vec<Event> {
            self.events.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl EventProducer for MockProducer {
        async fn emit(&self, event: Event) -> Result<(), ServerError> {
            if !self.connected {
                return Err(ServerError::Channel("disconnected".into()));
            }
            self.events.lock().unwrap().push(event);
            Ok(())
        }

        fn is_connected(&self) -> bool {
            self.connected
        }
    }

    #[tokio::test]
    async fn test_event_producer_emit() {
        let producer = MockProducer::new();

        producer
            .emit(Event::RunError(RunErrorEvent::new("test")))
            .await
            .unwrap();

        let events = producer.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::RunError(_)));
    }

    #[tokio::test]
    async fn test_event_producer_emit_many() {
        let producer = MockProducer::new();

        producer
            .emit_many(vec![
                Event::RunError(RunErrorEvent::new("error1")),
                Event::RunError(RunErrorEvent::new("error2")),
            ])
            .await
            .unwrap();

        let events = producer.events();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_message_stream() {
        let producer = MockProducer::new();

        let msg = MessageStream::start(&producer).await.unwrap();
        msg.content("Hello, ").await.unwrap();
        msg.content("world!").await.unwrap();
        let _message_id = msg.end().await.unwrap();

        let events = producer.events();
        assert_eq!(events.len(), 4); // start + 2 content + end

        assert!(matches!(events[0], Event::TextMessageStart(_)));
        assert!(matches!(events[1], Event::TextMessageContent(_)));
        assert!(matches!(events[2], Event::TextMessageContent(_)));
        assert!(matches!(events[3], Event::TextMessageEnd(_)));
    }

    #[tokio::test]
    async fn test_message_stream_empty_content_ignored() {
        let producer = MockProducer::new();

        let msg = MessageStream::start(&producer).await.unwrap();
        msg.content("").await.unwrap(); // Should be ignored
        msg.content("Hello").await.unwrap();
        msg.end().await.unwrap();

        let events = producer.events();
        assert_eq!(events.len(), 3); // start + 1 content + end (empty ignored)
    }

    #[tokio::test]
    async fn test_tool_call_stream() {
        let producer = MockProducer::new();

        let call = ToolCallStream::start(&producer, "get_weather").await.unwrap();
        call.args(r#"{"location": "#).await.unwrap();
        call.args(r#""NYC"}"#).await.unwrap();
        let _tool_call_id = call.end().await.unwrap();

        let events = producer.events();
        assert_eq!(events.len(), 4); // start + 2 args + end

        assert!(matches!(events[0], Event::ToolCallStart(_)));
        assert!(matches!(events[1], Event::ToolCallArgs(_)));
        assert!(matches!(events[2], Event::ToolCallArgs(_)));
        assert!(matches!(events[3], Event::ToolCallEnd(_)));
    }

    #[tokio::test]
    async fn test_agent_session_run_lifecycle() {
        let producer = MockProducer::new();
        let mut session = AgentSession::new(producer);

        assert!(!session.is_running());

        // Start run
        let run_id = session.start_run().await.unwrap();
        assert!(session.is_running());
        assert_eq!(session.run_id(), Some(&run_id));

        // Finish run
        session.finish_run(None).await.unwrap();
        assert!(!session.is_running());
        assert_eq!(session.run_id(), None);

        let events = session.producer().events();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], Event::RunStarted(_)));
        assert!(matches!(events[1], Event::RunFinished(_)));
    }

    #[tokio::test]
    async fn test_agent_session_run_error() {
        let producer = MockProducer::new();
        let mut session = AgentSession::new(producer);

        session.start_run().await.unwrap();
        session.run_error("Something went wrong").await.unwrap();

        assert!(!session.is_running());

        let events = session.producer().events();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], Event::RunStarted(_)));
        assert!(matches!(events[1], Event::RunError(_)));
    }

    #[tokio::test]
    async fn test_agent_session_double_start_error() {
        let producer = MockProducer::new();
        let mut session = AgentSession::new(producer);

        session.start_run().await.unwrap();
        let result = session.start_run().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_agent_session_finish_without_run() {
        let producer = MockProducer::new();
        let mut session = AgentSession::new(producer);

        // Should not error, just do nothing
        session.finish_run(None).await.unwrap();

        let events = session.producer().events();
        assert!(events.is_empty());
    }

    // =========================================================================
    // Thinking Message Stream Tests
    // =========================================================================

    #[tokio::test]
    async fn test_thinking_message_stream() {
        let producer = MockProducer::new();

        let thinking = ThinkingMessageStream::start(&producer).await.unwrap();
        thinking.content("Let me analyze...").await.unwrap();
        thinking.content("The answer is...").await.unwrap();
        thinking.end().await.unwrap();

        let events = producer.events();
        assert_eq!(events.len(), 4); // start + 2 content + end

        assert!(matches!(events[0], Event::ThinkingTextMessageStart(_)));
        assert!(matches!(events[1], Event::ThinkingTextMessageContent(_)));
        assert!(matches!(events[2], Event::ThinkingTextMessageContent(_)));
        assert!(matches!(events[3], Event::ThinkingTextMessageEnd(_)));
    }

    #[tokio::test]
    async fn test_thinking_message_stream_empty_content_allowed() {
        let producer = MockProducer::new();

        let thinking = ThinkingMessageStream::start(&producer).await.unwrap();
        thinking.content("").await.unwrap(); // Empty is allowed for thinking
        thinking.content("Thinking...").await.unwrap();
        thinking.end().await.unwrap();

        let events = producer.events();
        // Empty content is emitted (unlike regular MessageStream)
        assert_eq!(events.len(), 4); // start + empty + content + end
    }

    // =========================================================================
    // Thinking Step Tests
    // =========================================================================

    #[tokio::test]
    async fn test_thinking_step() {
        let producer = MockProducer::new();

        let step = ThinkingStep::start(&producer, None::<String>).await.unwrap();
        step.end().await.unwrap();

        let events = producer.events();
        assert_eq!(events.len(), 2); // start + end

        assert!(matches!(events[0], Event::ThinkingStart(_)));
        assert!(matches!(events[1], Event::ThinkingEnd(_)));
    }

    #[tokio::test]
    async fn test_thinking_step_with_title() {
        let producer = MockProducer::new();

        let step = ThinkingStep::start(&producer, Some("Analyzing query"))
            .await
            .unwrap();
        step.end().await.unwrap();

        let events = producer.events();
        assert_eq!(events.len(), 2);

        if let Event::ThinkingStart(start) = &events[0] {
            assert_eq!(start.title, Some("Analyzing query".to_string()));
        } else {
            panic!("Expected ThinkingStart event");
        }
    }

    #[tokio::test]
    async fn test_thinking_step_with_content() {
        let producer = MockProducer::new();

        let step = ThinkingStep::start(&producer, Some("Planning"))
            .await
            .unwrap();

        // Emit thinking content inside the step
        let thinking = ThinkingMessageStream::start(step.producer()).await.unwrap();
        thinking.content("First, consider...").await.unwrap();
        thinking.end().await.unwrap();

        step.end().await.unwrap();

        let events = producer.events();
        assert_eq!(events.len(), 5); // ThinkingStart + TextStart + content + TextEnd + ThinkingEnd

        assert!(matches!(events[0], Event::ThinkingStart(_)));
        assert!(matches!(events[1], Event::ThinkingTextMessageStart(_)));
        assert!(matches!(events[2], Event::ThinkingTextMessageContent(_)));
        assert!(matches!(events[3], Event::ThinkingTextMessageEnd(_)));
        assert!(matches!(events[4], Event::ThinkingEnd(_)));
    }

    // =========================================================================
    // AgentSession Thinking Tests
    // =========================================================================

    #[tokio::test]
    async fn test_agent_session_start_thinking() {
        let producer = MockProducer::new();
        let session = AgentSession::new(producer);

        let step = session.start_thinking(Some("Reasoning")).await.unwrap();
        step.end().await.unwrap();

        let events = session.producer().events();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], Event::ThinkingStart(_)));
        assert!(matches!(events[1], Event::ThinkingEnd(_)));
    }

    #[tokio::test]
    async fn test_agent_session_start_thinking_no_title() {
        let producer = MockProducer::new();
        let session = AgentSession::new(producer);

        let step = session.start_thinking(None::<String>).await.unwrap();
        step.end().await.unwrap();

        let events = session.producer().events();
        assert_eq!(events.len(), 2);

        if let Event::ThinkingStart(start) = &events[0] {
            assert!(start.title.is_none());
        } else {
            panic!("Expected ThinkingStart event");
        }
    }

    // =========================================================================
    // AgentSession Interrupt Tests
    // =========================================================================

    #[tokio::test]
    async fn test_agent_session_interrupt() {
        use ag_ui_core::RunFinishedOutcome;

        let producer = MockProducer::new();
        let mut session = AgentSession::new(producer);

        session.start_run().await.unwrap();
        session
            .interrupt(
                Some("human_approval"),
                Some(serde_json::json!({"action": "send_email"})),
            )
            .await
            .unwrap();

        // Run should be cleared after interrupt
        assert!(!session.is_running());

        let events = session.producer().events();
        assert_eq!(events.len(), 2); // RunStarted + RunFinished(interrupt)

        assert!(matches!(events[0], Event::RunStarted(_)));

        if let Event::RunFinished(finished) = &events[1] {
            assert_eq!(finished.outcome, Some(RunFinishedOutcome::Interrupt));
            assert!(finished.interrupt.is_some());
            let info = finished.interrupt.as_ref().unwrap();
            assert_eq!(info.reason, Some("human_approval".to_string()));
            assert!(info.payload.is_some());
        } else {
            panic!("Expected RunFinished event");
        }
    }

    #[tokio::test]
    async fn test_agent_session_interrupt_with_id() {
        use ag_ui_core::RunFinishedOutcome;

        let producer = MockProducer::new();
        let mut session = AgentSession::new(producer);

        session.start_run().await.unwrap();
        session
            .interrupt_with_id(
                "approval-001",
                Some("database_modification"),
                Some(serde_json::json!({"query": "DELETE FROM users"})),
            )
            .await
            .unwrap();

        assert!(!session.is_running());

        let events = session.producer().events();
        assert_eq!(events.len(), 2);

        if let Event::RunFinished(finished) = &events[1] {
            assert_eq!(finished.outcome, Some(RunFinishedOutcome::Interrupt));
            let info = finished.interrupt.as_ref().unwrap();
            assert_eq!(info.id, Some("approval-001".to_string()));
            assert_eq!(info.reason, Some("database_modification".to_string()));
        } else {
            panic!("Expected RunFinished event");
        }
    }

    #[tokio::test]
    async fn test_agent_session_interrupt_without_run() {
        let producer = MockProducer::new();
        let mut session = AgentSession::new(producer);

        // Interrupt without an active run should do nothing
        session
            .interrupt(Some("test"), None)
            .await
            .unwrap();

        let events = session.producer().events();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_agent_session_interrupt_minimal() {
        let producer = MockProducer::new();
        let mut session = AgentSession::new(producer);

        session.start_run().await.unwrap();

        // Interrupt with no reason or payload
        session
            .interrupt(None::<String>, None)
            .await
            .unwrap();

        let events = session.producer().events();
        assert_eq!(events.len(), 2);

        if let Event::RunFinished(finished) = &events[1] {
            let info = finished.interrupt.as_ref().unwrap();
            assert!(info.id.is_none());
            assert!(info.reason.is_none());
            assert!(info.payload.is_none());
        } else {
            panic!("Expected RunFinished event");
        }
    }
}
