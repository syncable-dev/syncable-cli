//! AG-UI Event Types
//!
//! This module defines all AG-UI protocol event types including:
//! - Text message events (start, content, end, chunk)
//! - Thinking text message events
//! - Tool call events (start, args, end, result)
//! - State events (snapshot, delta)
//! - Run lifecycle events (started, finished, error)
//! - Step events (started, finished)
//! - Custom and raw events

use crate::state::AgentState;
use crate::types::{Message, MessageId, Role, RunId, ThreadId, ToolCallId};
use crate::JsonValue;
use serde::{Deserialize, Serialize};

/// Event types for the AG-UI protocol.
///
/// This enum enumerates all possible event types in the protocol.
/// Event types are serialized using SCREAMING_SNAKE_CASE (e.g., `TEXT_MESSAGE_START`).
///
/// # Event Categories
///
/// - **Text Messages**: `TextMessageStart`, `TextMessageContent`, `TextMessageEnd`, `TextMessageChunk`
/// - **Thinking Messages**: `ThinkingTextMessageStart`, `ThinkingTextMessageContent`, `ThinkingTextMessageEnd`
/// - **Tool Calls**: `ToolCallStart`, `ToolCallArgs`, `ToolCallEnd`, `ToolCallChunk`, `ToolCallResult`
/// - **Thinking Steps**: `ThinkingStart`, `ThinkingEnd`
/// - **State**: `StateSnapshot`, `StateDelta`
/// - **Messages**: `MessagesSnapshot`
/// - **Run Lifecycle**: `RunStarted`, `RunFinished`, `RunError`
/// - **Step Lifecycle**: `StepStarted`, `StepFinished`
/// - **Other**: `Raw`, `Custom`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    /// Start of a text message from the assistant.
    TextMessageStart,
    /// Content chunk of a text message (streaming delta).
    TextMessageContent,
    /// End of a text message.
    TextMessageEnd,
    /// Complete text message chunk (non-streaming alternative).
    TextMessageChunk,
    /// Start of a thinking text message (extended thinking).
    ThinkingTextMessageStart,
    /// Content chunk of a thinking text message.
    ThinkingTextMessageContent,
    /// End of a thinking text message.
    ThinkingTextMessageEnd,
    /// Start of a tool call.
    ToolCallStart,
    /// Arguments chunk for a tool call (streaming).
    ToolCallArgs,
    /// End of a tool call.
    ToolCallEnd,
    /// Complete tool call chunk (non-streaming alternative).
    ToolCallChunk,
    /// Result of a tool call execution.
    ToolCallResult,
    /// Start of a thinking step (chain-of-thought).
    ThinkingStart,
    /// End of a thinking step.
    ThinkingEnd,
    /// Complete state snapshot.
    StateSnapshot,
    /// Incremental state update (JSON Patch RFC 6902).
    StateDelta,
    /// Complete messages snapshot.
    MessagesSnapshot,
    /// Complete activity snapshot.
    ActivitySnapshot,
    /// Incremental activity update (JSON Patch RFC 6902).
    ActivityDelta,
    /// Raw event from the underlying provider.
    Raw,
    /// Custom application-specific event.
    Custom,
    /// Agent run has started.
    RunStarted,
    /// Agent run has finished successfully.
    RunFinished,
    /// Agent run encountered an error.
    RunError,
    /// A step within a run has started.
    StepStarted,
    /// A step within a run has finished.
    StepFinished,
}

impl EventType {
    /// Returns the string representation of the event type.
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::TextMessageStart => "TEXT_MESSAGE_START",
            EventType::TextMessageContent => "TEXT_MESSAGE_CONTENT",
            EventType::TextMessageEnd => "TEXT_MESSAGE_END",
            EventType::TextMessageChunk => "TEXT_MESSAGE_CHUNK",
            EventType::ThinkingTextMessageStart => "THINKING_TEXT_MESSAGE_START",
            EventType::ThinkingTextMessageContent => "THINKING_TEXT_MESSAGE_CONTENT",
            EventType::ThinkingTextMessageEnd => "THINKING_TEXT_MESSAGE_END",
            EventType::ToolCallStart => "TOOL_CALL_START",
            EventType::ToolCallArgs => "TOOL_CALL_ARGS",
            EventType::ToolCallEnd => "TOOL_CALL_END",
            EventType::ToolCallChunk => "TOOL_CALL_CHUNK",
            EventType::ToolCallResult => "TOOL_CALL_RESULT",
            EventType::ThinkingStart => "THINKING_START",
            EventType::ThinkingEnd => "THINKING_END",
            EventType::StateSnapshot => "STATE_SNAPSHOT",
            EventType::StateDelta => "STATE_DELTA",
            EventType::MessagesSnapshot => "MESSAGES_SNAPSHOT",
            EventType::ActivitySnapshot => "ACTIVITY_SNAPSHOT",
            EventType::ActivityDelta => "ACTIVITY_DELTA",
            EventType::Raw => "RAW",
            EventType::Custom => "CUSTOM",
            EventType::RunStarted => "RUN_STARTED",
            EventType::RunFinished => "RUN_FINISHED",
            EventType::RunError => "RUN_ERROR",
            EventType::StepStarted => "STEP_STARTED",
            EventType::StepFinished => "STEP_FINISHED",
        }
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Base event structure for all AG-UI protocol events.
///
/// Contains common fields that are present in all event types.
/// Individual event structs flatten this into their structure.
///
/// # Fields
///
/// - `timestamp`: Optional Unix timestamp in milliseconds since epoch
/// - `raw_event`: Optional raw event from the underlying provider (for debugging/passthrough)
///
/// # Example
///
/// ```rust
/// use ag_ui_core::event::BaseEvent;
///
/// let base = BaseEvent {
///     timestamp: Some(1706123456789.0),
///     raw_event: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BaseEvent {
    /// Unix timestamp in milliseconds since epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<f64>,
    /// Raw event from the underlying provider (for debugging/passthrough).
    #[serde(rename = "rawEvent", skip_serializing_if = "Option::is_none")]
    pub raw_event: Option<JsonValue>,
}

impl BaseEvent {
    /// Creates a new empty BaseEvent.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a BaseEvent with the current timestamp.
    pub fn with_current_timestamp() -> Self {
        Self {
            timestamp: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as f64)
                    .unwrap_or(0.0),
            ),
            raw_event: None,
        }
    }

    /// Sets the timestamp for this event.
    pub fn timestamp(mut self, timestamp: f64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the raw event for this event.
    pub fn raw_event(mut self, raw_event: JsonValue) -> Self {
        self.raw_event = Some(raw_event);
        self
    }
}

/// Validation errors for AG-UI protocol events.
///
/// These errors occur when event data fails validation rules.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EventValidationError {
    /// Delta content must not be empty.
    #[error("Delta must not be an empty string")]
    EmptyDelta,
    /// Event format is invalid.
    #[error("Invalid event format: {0}")]
    InvalidFormat(String),
    /// Required field is missing.
    #[error("Missing required field: {0}")]
    MissingField(String),
    /// Event type mismatch.
    #[error("Event type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Expected event type.
        expected: String,
        /// Actual event type.
        actual: String,
    },
}

// =============================================================================
// Text Message Events
// =============================================================================

/// Event indicating the start of a text message.
///
/// This event is sent when the agent begins generating a text message.
/// The message_id identifies this message throughout the streaming process.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::{MessageId, Role};
/// use ag_ui_core::event::TextMessageStartEvent;
///
/// let event = TextMessageStartEvent::new(MessageId::random());
/// assert_eq!(event.role, Role::Assistant);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextMessageStartEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Unique identifier for this message.
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    /// The role of the message sender (always Assistant for new messages).
    pub role: Role,
}

impl TextMessageStartEvent {
    /// Creates a new TextMessageStartEvent with the given message ID.
    pub fn new(message_id: impl Into<MessageId>) -> Self {
        Self {
            base: BaseEvent::default(),
            message_id: message_id.into(),
            role: Role::Assistant,
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }

    /// Sets the raw event for this event.
    pub fn with_raw_event(mut self, raw_event: JsonValue) -> Self {
        self.base.raw_event = Some(raw_event);
        self
    }
}

/// Event containing a piece of text message content.
///
/// This event is sent for each chunk of content as the agent generates a message.
/// The delta field contains the new text to append to the message.
///
/// # Validation
///
/// The delta must not be empty. Use `new()` which returns a Result to validate,
/// or `new_unchecked()` if you've already validated the input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextMessageContentEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// The message ID this content belongs to.
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    /// The text content delta to append.
    pub delta: String,
}

impl TextMessageContentEvent {
    /// Creates a new TextMessageContentEvent with validation.
    ///
    /// Returns an error if delta is empty.
    pub fn new(
        message_id: impl Into<MessageId>,
        delta: impl Into<String>,
    ) -> Result<Self, EventValidationError> {
        let delta = delta.into();
        if delta.is_empty() {
            return Err(EventValidationError::EmptyDelta);
        }
        Ok(Self {
            base: BaseEvent::default(),
            message_id: message_id.into(),
            delta,
        })
    }

    /// Creates a new TextMessageContentEvent without validation.
    ///
    /// Use this only if you've already validated the delta is not empty.
    pub fn new_unchecked(message_id: impl Into<MessageId>, delta: impl Into<String>) -> Self {
        Self {
            base: BaseEvent::default(),
            message_id: message_id.into(),
            delta: delta.into(),
        }
    }

    /// Validates this event's data.
    pub fn validate(&self) -> Result<(), EventValidationError> {
        if self.delta.is_empty() {
            return Err(EventValidationError::EmptyDelta);
        }
        Ok(())
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

/// Event indicating the end of a text message.
///
/// This event is sent when the agent completes a text message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextMessageEndEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// The message ID that has completed.
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
}

impl TextMessageEndEvent {
    /// Creates a new TextMessageEndEvent.
    pub fn new(message_id: impl Into<MessageId>) -> Self {
        Self {
            base: BaseEvent::default(),
            message_id: message_id.into(),
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

/// Event containing a chunk of text message content.
///
/// This event combines start, content, and potentially end information in a single event.
/// Used as a non-streaming alternative where all fields may be optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextMessageChunkEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Optional message ID (may be omitted for continuation chunks).
    #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
    pub message_id: Option<MessageId>,
    /// The role of the message sender.
    pub role: Role,
    /// Optional text content delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
}

impl TextMessageChunkEvent {
    /// Creates a new TextMessageChunkEvent with the given role.
    pub fn new(role: Role) -> Self {
        Self {
            base: BaseEvent::default(),
            message_id: None,
            role,
            delta: None,
        }
    }

    /// Sets the message ID for this event.
    pub fn with_message_id(mut self, message_id: impl Into<MessageId>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    /// Sets the delta for this event.
    pub fn with_delta(mut self, delta: impl Into<String>) -> Self {
        self.delta = Some(delta.into());
        self
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

// =============================================================================
// Thinking Text Message Events
// =============================================================================

/// Event indicating the start of a thinking text message.
///
/// This event is sent when the agent begins generating internal thinking content
/// (extended thinking / chain-of-thought). Unlike regular messages, thinking
/// messages don't have a message ID as they're ephemeral.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingTextMessageStartEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
}

impl ThinkingTextMessageStartEvent {
    /// Creates a new ThinkingTextMessageStartEvent.
    pub fn new() -> Self {
        Self {
            base: BaseEvent::default(),
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

impl Default for ThinkingTextMessageStartEvent {
    fn default() -> Self {
        Self::new()
    }
}

/// Event containing a piece of thinking text message content.
///
/// This event contains chunks of the agent's internal thinking process.
/// Unlike regular content events, thinking content doesn't validate for
/// empty delta as it may represent the start of a stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingTextMessageContentEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// The thinking content delta.
    pub delta: String,
}

impl ThinkingTextMessageContentEvent {
    /// Creates a new ThinkingTextMessageContentEvent.
    pub fn new(delta: impl Into<String>) -> Self {
        Self {
            base: BaseEvent::default(),
            delta: delta.into(),
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

/// Event indicating the end of a thinking text message.
///
/// This event is sent when the agent completes its internal thinking process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingTextMessageEndEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
}

impl ThinkingTextMessageEndEvent {
    /// Creates a new ThinkingTextMessageEndEvent.
    pub fn new() -> Self {
        Self {
            base: BaseEvent::default(),
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

impl Default for ThinkingTextMessageEndEvent {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tool Call Events
// =============================================================================

/// Event indicating the start of a tool call.
///
/// This event is sent when the agent begins calling a tool with specific parameters.
/// The tool_call_id identifies this call throughout the streaming process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallStartEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Unique identifier for this tool call.
    #[serde(rename = "toolCallId")]
    pub tool_call_id: ToolCallId,
    /// Name of the tool being called.
    #[serde(rename = "toolCallName")]
    pub tool_call_name: String,
    /// Optional parent message ID if this call is part of a message.
    #[serde(rename = "parentMessageId", skip_serializing_if = "Option::is_none")]
    pub parent_message_id: Option<MessageId>,
}

impl ToolCallStartEvent {
    /// Creates a new ToolCallStartEvent.
    pub fn new(tool_call_id: impl Into<ToolCallId>, tool_call_name: impl Into<String>) -> Self {
        Self {
            base: BaseEvent::default(),
            tool_call_id: tool_call_id.into(),
            tool_call_name: tool_call_name.into(),
            parent_message_id: None,
        }
    }

    /// Sets the parent message ID.
    pub fn with_parent_message_id(mut self, message_id: impl Into<MessageId>) -> Self {
        self.parent_message_id = Some(message_id.into());
        self
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

/// Event containing tool call arguments.
///
/// This event contains chunks of the arguments being passed to a tool.
/// Arguments are streamed as JSON string deltas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallArgsEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// The tool call ID this argument chunk belongs to.
    #[serde(rename = "toolCallId")]
    pub tool_call_id: ToolCallId,
    /// The argument delta (JSON string chunk).
    pub delta: String,
}

impl ToolCallArgsEvent {
    /// Creates a new ToolCallArgsEvent.
    pub fn new(tool_call_id: impl Into<ToolCallId>, delta: impl Into<String>) -> Self {
        Self {
            base: BaseEvent::default(),
            tool_call_id: tool_call_id.into(),
            delta: delta.into(),
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

/// Event indicating the end of a tool call.
///
/// This event is sent when the agent completes sending arguments to a tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallEndEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// The tool call ID that has completed.
    #[serde(rename = "toolCallId")]
    pub tool_call_id: ToolCallId,
}

impl ToolCallEndEvent {
    /// Creates a new ToolCallEndEvent.
    pub fn new(tool_call_id: impl Into<ToolCallId>) -> Self {
        Self {
            base: BaseEvent::default(),
            tool_call_id: tool_call_id.into(),
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

/// Event containing a chunk of tool call content.
///
/// This event combines start, args, and potentially end information in a single event.
/// Used as a non-streaming alternative where all fields may be optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallChunkEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Optional tool call ID (may be omitted for continuation chunks).
    #[serde(rename = "toolCallId", skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<ToolCallId>,
    /// Optional tool name.
    #[serde(rename = "toolCallName", skip_serializing_if = "Option::is_none")]
    pub tool_call_name: Option<String>,
    /// Optional parent message ID.
    #[serde(rename = "parentMessageId", skip_serializing_if = "Option::is_none")]
    pub parent_message_id: Option<MessageId>,
    /// Optional argument delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
}

impl ToolCallChunkEvent {
    /// Creates a new empty ToolCallChunkEvent.
    pub fn new() -> Self {
        Self {
            base: BaseEvent::default(),
            tool_call_id: None,
            tool_call_name: None,
            parent_message_id: None,
            delta: None,
        }
    }

    /// Sets the tool call ID.
    pub fn with_tool_call_id(mut self, tool_call_id: impl Into<ToolCallId>) -> Self {
        self.tool_call_id = Some(tool_call_id.into());
        self
    }

    /// Sets the tool call name.
    pub fn with_tool_call_name(mut self, name: impl Into<String>) -> Self {
        self.tool_call_name = Some(name.into());
        self
    }

    /// Sets the parent message ID.
    pub fn with_parent_message_id(mut self, message_id: impl Into<MessageId>) -> Self {
        self.parent_message_id = Some(message_id.into());
        self
    }

    /// Sets the delta.
    pub fn with_delta(mut self, delta: impl Into<String>) -> Self {
        self.delta = Some(delta.into());
        self
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

impl Default for ToolCallChunkEvent {
    fn default() -> Self {
        Self::new()
    }
}

/// Event containing the result of a tool call.
///
/// This event is sent when a tool has completed execution and returns its result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallResultEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Message ID for the result message.
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    /// The tool call ID this result corresponds to.
    #[serde(rename = "toolCallId")]
    pub tool_call_id: ToolCallId,
    /// The result content.
    pub content: String,
    /// Role (always Tool).
    #[serde(default = "Role::tool")]
    pub role: Role,
}

impl ToolCallResultEvent {
    /// Creates a new ToolCallResultEvent.
    pub fn new(
        message_id: impl Into<MessageId>,
        tool_call_id: impl Into<ToolCallId>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            base: BaseEvent::default(),
            message_id: message_id.into(),
            tool_call_id: tool_call_id.into(),
            content: content.into(),
            role: Role::Tool,
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

// =============================================================================
// Run Lifecycle Events
// =============================================================================

/// Event indicating that a run has started.
///
/// This event is sent when an agent run begins execution within a specific thread.
/// A run represents a single agent execution that may produce multiple events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunStartedEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// The thread ID this run belongs to.
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    /// Unique identifier for this run.
    #[serde(rename = "runId")]
    pub run_id: RunId,
}

impl RunStartedEvent {
    /// Creates a new RunStartedEvent.
    pub fn new(thread_id: impl Into<ThreadId>, run_id: impl Into<RunId>) -> Self {
        Self {
            base: BaseEvent::default(),
            thread_id: thread_id.into(),
            run_id: run_id.into(),
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

/// Outcome of a run finishing.
///
/// Used to indicate whether a run completed successfully or was interrupted
/// for human-in-the-loop interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunFinishedOutcome {
    /// Run completed successfully.
    Success,
    /// Run was interrupted and requires human input to continue.
    Interrupt,
}

impl Default for RunFinishedOutcome {
    fn default() -> Self {
        Self::Success
    }
}

/// Information about a run interrupt.
///
/// When a run finishes with `outcome == Interrupt`, this struct contains
/// information about why the interrupt occurred and what input is needed.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::InterruptInfo;
///
/// let info = InterruptInfo::new()
///     .with_id("approval-001")
///     .with_reason("human_approval")
///     .with_payload(serde_json::json!({
///         "action": "DELETE",
///         "table": "users",
///         "affectedRows": 42
///     }));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InterruptInfo {
    /// Optional identifier for tracking this interrupt across resume.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Optional reason describing why the interrupt occurred.
    /// Common values: "human_approval", "upload_required", "policy_hold"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Optional payload with context for the interrupt UI.
    /// Contains arbitrary JSON data for rendering approval forms, proposals, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<JsonValue>,
}

impl InterruptInfo {
    /// Creates a new empty InterruptInfo.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the interrupt ID.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the interrupt reason.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Sets the interrupt payload.
    pub fn with_payload(mut self, payload: JsonValue) -> Self {
        self.payload = Some(payload);
        self
    }
}

/// Event indicating that a run has finished.
///
/// This event is sent when an agent run completes, either successfully or
/// with an interrupt requiring human input.
///
/// # Interrupt Flow
///
/// When `outcome == Interrupt`, the agent indicates that on the next run,
/// a value needs to be provided via `RunAgentInput.resume` to continue.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::{RunFinishedEvent, RunFinishedOutcome, InterruptInfo, ThreadId, RunId};
///
/// // Success case
/// let success = RunFinishedEvent::new(ThreadId::random(), RunId::random())
///     .with_result(serde_json::json!({"status": "done"}));
///
/// // Interrupt case
/// let interrupt = RunFinishedEvent::new(ThreadId::random(), RunId::random())
///     .with_interrupt(
///         InterruptInfo::new()
///             .with_reason("human_approval")
///             .with_payload(serde_json::json!({"action": "send_email"}))
///     );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunFinishedEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// The thread ID this run belongs to.
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    /// The run ID that finished.
    #[serde(rename = "runId")]
    pub run_id: RunId,
    /// Outcome of the run. Optional for backward compatibility.
    /// When omitted, outcome is inferred: if interrupt is present, it's Interrupt; otherwise Success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<RunFinishedOutcome>,
    /// Optional result value from the run.
    /// Present when outcome is Success (or omitted with no interrupt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<JsonValue>,
    /// Optional interrupt information.
    /// Present when outcome is Interrupt (or omitted with interrupt present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupt: Option<InterruptInfo>,
}

impl RunFinishedEvent {
    /// Creates a new RunFinishedEvent with Success outcome.
    pub fn new(thread_id: impl Into<ThreadId>, run_id: impl Into<RunId>) -> Self {
        Self {
            base: BaseEvent::default(),
            thread_id: thread_id.into(),
            run_id: run_id.into(),
            outcome: None,
            result: None,
            interrupt: None,
        }
    }

    /// Sets the outcome explicitly.
    pub fn with_outcome(mut self, outcome: RunFinishedOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    /// Sets the result for this event (implies Success outcome).
    pub fn with_result(mut self, result: JsonValue) -> Self {
        self.result = Some(result);
        self
    }

    /// Sets the interrupt info (implies Interrupt outcome).
    pub fn with_interrupt(mut self, interrupt: InterruptInfo) -> Self {
        self.outcome = Some(RunFinishedOutcome::Interrupt);
        self.interrupt = Some(interrupt);
        self
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }

    /// Returns the effective outcome of this event.
    ///
    /// If outcome is explicitly set, returns that. Otherwise:
    /// - If interrupt is present, returns Interrupt
    /// - Otherwise, returns Success
    pub fn effective_outcome(&self) -> RunFinishedOutcome {
        self.outcome.unwrap_or_else(|| {
            if self.interrupt.is_some() {
                RunFinishedOutcome::Interrupt
            } else {
                RunFinishedOutcome::Success
            }
        })
    }
}

/// Event indicating that a run has encountered an error.
///
/// This event is sent when an agent run fails with an error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunErrorEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Error message describing what went wrong.
    pub message: String,
    /// Optional error code for programmatic handling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl RunErrorEvent {
    /// Creates a new RunErrorEvent.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            base: BaseEvent::default(),
            message: message.into(),
            code: None,
        }
    }

    /// Sets the error code.
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

// =============================================================================
// Step Events
// =============================================================================

/// Event indicating that a step has started.
///
/// This event is sent when a specific named step within a run begins execution.
/// Steps allow tracking progress through multi-stage agent workflows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepStartedEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Name of the step that started.
    #[serde(rename = "stepName")]
    pub step_name: String,
}

impl StepStartedEvent {
    /// Creates a new StepStartedEvent.
    pub fn new(step_name: impl Into<String>) -> Self {
        Self {
            base: BaseEvent::default(),
            step_name: step_name.into(),
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

/// Event indicating that a step has finished.
///
/// This event is sent when a specific named step within a run completes execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepFinishedEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Name of the step that finished.
    #[serde(rename = "stepName")]
    pub step_name: String,
}

impl StepFinishedEvent {
    /// Creates a new StepFinishedEvent.
    pub fn new(step_name: impl Into<String>) -> Self {
        Self {
            base: BaseEvent::default(),
            step_name: step_name.into(),
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

// =============================================================================
// State Events
// =============================================================================

/// Event containing a complete state snapshot.
///
/// This event is sent to provide the full current state of the agent.
/// The state is generic over `StateT` which must implement `AgentState`.
///
/// # Type Parameter
///
/// - `StateT`: The type of state, defaults to `JsonValue` for flexibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(deserialize = ""))]
pub struct StateSnapshotEvent<StateT: AgentState = JsonValue> {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// The complete state snapshot.
    pub snapshot: StateT,
}

impl<StateT: AgentState> StateSnapshotEvent<StateT> {
    /// Creates a new StateSnapshotEvent with the given state.
    pub fn new(snapshot: StateT) -> Self {
        Self {
            base: BaseEvent::default(),
            snapshot,
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

impl<StateT: AgentState + Default> Default for StateSnapshotEvent<StateT> {
    fn default() -> Self {
        Self {
            base: BaseEvent::default(),
            snapshot: StateT::default(),
        }
    }
}

/// Event containing incremental state updates as JSON Patch operations.
///
/// This event is sent to update state incrementally using RFC 6902 JSON Patch format.
/// The delta is a vector of patch operations (add, remove, replace, move, copy, test).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateDeltaEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// JSON Patch operations per RFC 6902.
    pub delta: Vec<JsonValue>,
}

impl StateDeltaEvent {
    /// Creates a new StateDeltaEvent with the given patch operations.
    pub fn new(delta: Vec<JsonValue>) -> Self {
        Self {
            base: BaseEvent::default(),
            delta,
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

impl Default for StateDeltaEvent {
    fn default() -> Self {
        Self {
            base: BaseEvent::default(),
            delta: Vec::new(),
        }
    }
}

/// Event containing a complete snapshot of all messages.
///
/// This event is sent to provide the full message history to the client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessagesSnapshotEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Complete list of messages.
    pub messages: Vec<Message>,
}

impl MessagesSnapshotEvent {
    /// Creates a new MessagesSnapshotEvent with the given messages.
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            base: BaseEvent::default(),
            messages,
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

impl Default for MessagesSnapshotEvent {
    fn default() -> Self {
        Self {
            base: BaseEvent::default(),
            messages: Vec::new(),
        }
    }
}

// =============================================================================
// Activity Events
// =============================================================================

/// Event containing a complete activity snapshot.
///
/// This event creates a new activity message or replaces an existing one.
/// Activity messages track structured agent activities like planning or research.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::event::ActivitySnapshotEvent;
/// use ag_ui_core::MessageId;
/// use serde_json::json;
///
/// let event = ActivitySnapshotEvent::new(
///     MessageId::random(),
///     "PLAN",
///     json!({"steps": ["research", "implement", "test"]}),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivitySnapshotEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// The message ID for this activity.
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    /// The type of activity (e.g., "PLAN", "RESEARCH").
    #[serde(rename = "activityType")]
    pub activity_type: String,
    /// The activity content as a flexible JSON object.
    pub content: JsonValue,
    /// Whether to replace the existing activity content (default: true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace: Option<bool>,
}

impl ActivitySnapshotEvent {
    /// Creates a new ActivitySnapshotEvent with the given message ID, type, and content.
    pub fn new(
        message_id: impl Into<MessageId>,
        activity_type: impl Into<String>,
        content: JsonValue,
    ) -> Self {
        Self {
            base: BaseEvent::default(),
            message_id: message_id.into(),
            activity_type: activity_type.into(),
            content,
            replace: None,
        }
    }

    /// Sets whether to replace the existing activity content.
    pub fn with_replace(mut self, replace: bool) -> Self {
        self.replace = Some(replace);
        self
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

/// Event containing an incremental activity update.
///
/// This event applies a JSON Patch (RFC 6902) to an existing activity's content.
/// Use this for efficient partial updates to activity content.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::event::ActivityDeltaEvent;
/// use ag_ui_core::MessageId;
/// use serde_json::json;
///
/// let event = ActivityDeltaEvent::new(
///     MessageId::random(),
///     "PLAN",
///     vec![json!({"op": "add", "path": "/steps/-", "value": "deploy"})],
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityDeltaEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// The message ID for this activity.
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    /// The type of activity (e.g., "PLAN", "RESEARCH").
    #[serde(rename = "activityType")]
    pub activity_type: String,
    /// JSON Patch operations (RFC 6902) to apply to the content.
    pub patch: Vec<JsonValue>,
}

impl ActivityDeltaEvent {
    /// Creates a new ActivityDeltaEvent with the given message ID, type, and patch.
    pub fn new(
        message_id: impl Into<MessageId>,
        activity_type: impl Into<String>,
        patch: Vec<JsonValue>,
    ) -> Self {
        Self {
            base: BaseEvent::default(),
            message_id: message_id.into(),
            activity_type: activity_type.into(),
            patch,
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

// =============================================================================
// Thinking Step Events
// =============================================================================

/// Event indicating that a thinking step has started.
///
/// This event is sent when the agent begins a chain-of-thought reasoning step.
/// Unlike ThinkingTextMessage events (which contain actual thinking content),
/// this event marks the boundary of a thinking block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingStartEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Optional title for the thinking step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl ThinkingStartEvent {
    /// Creates a new ThinkingStartEvent.
    pub fn new() -> Self {
        Self {
            base: BaseEvent::default(),
            title: None,
        }
    }

    /// Sets the title for this thinking step.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

impl Default for ThinkingStartEvent {
    fn default() -> Self {
        Self::new()
    }
}

/// Event indicating that a thinking step has ended.
///
/// This event is sent when the agent completes a chain-of-thought reasoning step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingEndEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
}

impl ThinkingEndEvent {
    /// Creates a new ThinkingEndEvent.
    pub fn new() -> Self {
        Self {
            base: BaseEvent::default(),
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

impl Default for ThinkingEndEvent {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Special Events
// =============================================================================

/// Event containing raw data from the underlying provider.
///
/// This event is sent to pass through raw provider-specific data that
/// doesn't fit into other event types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// The raw event data.
    pub event: JsonValue,
    /// Optional source identifier for the raw event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl RawEvent {
    /// Creates a new RawEvent with the given event data.
    pub fn new(event: JsonValue) -> Self {
        Self {
            base: BaseEvent::default(),
            event,
            source: None,
        }
    }

    /// Sets the source identifier.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

/// Event for custom application-specific data.
///
/// This event allows sending arbitrary named events with custom payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomEvent {
    /// Common event fields (timestamp, rawEvent).
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Name of the custom event.
    pub name: String,
    /// Custom event payload.
    pub value: JsonValue,
}

impl CustomEvent {
    /// Creates a new CustomEvent with the given name and value.
    pub fn new(name: impl Into<String>, value: JsonValue) -> Self {
        Self {
            base: BaseEvent::default(),
            name: name.into(),
            value,
        }
    }

    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}

// =============================================================================
// Event Union
// =============================================================================

/// Union of all possible events in the Agent User Interaction Protocol.
///
/// This enum represents any event that can be sent or received in the AG-UI protocol.
/// Events are serialized with a `type` discriminant in SCREAMING_SNAKE_CASE format.
///
/// # Type Parameter
///
/// - `StateT`: The type of state for `StateSnapshot` events, defaults to `JsonValue`.
///
/// # Serialization
///
/// Events are serialized as JSON objects with a `type` field indicating the variant:
/// ```json
/// {"type": "TEXT_MESSAGE_START", "messageId": "...", "role": "assistant"}
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE", bound(deserialize = ""))]
pub enum Event<StateT: AgentState = JsonValue> {
    /// Start of a text message from the assistant.
    TextMessageStart(TextMessageStartEvent),
    /// Content chunk of a text message (streaming delta).
    TextMessageContent(TextMessageContentEvent),
    /// End of a text message.
    TextMessageEnd(TextMessageEndEvent),
    /// Complete text message chunk (non-streaming alternative).
    TextMessageChunk(TextMessageChunkEvent),
    /// Start of a thinking text message (extended thinking).
    ThinkingTextMessageStart(ThinkingTextMessageStartEvent),
    /// Content chunk of a thinking text message.
    ThinkingTextMessageContent(ThinkingTextMessageContentEvent),
    /// End of a thinking text message.
    ThinkingTextMessageEnd(ThinkingTextMessageEndEvent),
    /// Start of a tool call.
    ToolCallStart(ToolCallStartEvent),
    /// Arguments chunk for a tool call (streaming).
    ToolCallArgs(ToolCallArgsEvent),
    /// End of a tool call.
    ToolCallEnd(ToolCallEndEvent),
    /// Complete tool call chunk (non-streaming alternative).
    ToolCallChunk(ToolCallChunkEvent),
    /// Result of a tool call execution.
    ToolCallResult(ToolCallResultEvent),
    /// Start of a thinking step (chain-of-thought boundary).
    ThinkingStart(ThinkingStartEvent),
    /// End of a thinking step.
    ThinkingEnd(ThinkingEndEvent),
    /// Complete state snapshot.
    StateSnapshot(StateSnapshotEvent<StateT>),
    /// Incremental state update (JSON Patch).
    StateDelta(StateDeltaEvent),
    /// Complete messages snapshot.
    MessagesSnapshot(MessagesSnapshotEvent),
    /// Complete activity snapshot.
    ActivitySnapshot(ActivitySnapshotEvent),
    /// Incremental activity update (JSON Patch).
    ActivityDelta(ActivityDeltaEvent),
    /// Raw event from the underlying provider.
    Raw(RawEvent),
    /// Custom application-specific event.
    Custom(CustomEvent),
    /// Agent run has started.
    RunStarted(RunStartedEvent),
    /// Agent run has finished successfully.
    RunFinished(RunFinishedEvent),
    /// Agent run encountered an error.
    RunError(RunErrorEvent),
    /// A step within a run has started.
    StepStarted(StepStartedEvent),
    /// A step within a run has finished.
    StepFinished(StepFinishedEvent),
}

impl<StateT: AgentState> Event<StateT> {
    /// Returns the event type for this event.
    pub fn event_type(&self) -> EventType {
        match self {
            Event::TextMessageStart(_) => EventType::TextMessageStart,
            Event::TextMessageContent(_) => EventType::TextMessageContent,
            Event::TextMessageEnd(_) => EventType::TextMessageEnd,
            Event::TextMessageChunk(_) => EventType::TextMessageChunk,
            Event::ThinkingTextMessageStart(_) => EventType::ThinkingTextMessageStart,
            Event::ThinkingTextMessageContent(_) => EventType::ThinkingTextMessageContent,
            Event::ThinkingTextMessageEnd(_) => EventType::ThinkingTextMessageEnd,
            Event::ToolCallStart(_) => EventType::ToolCallStart,
            Event::ToolCallArgs(_) => EventType::ToolCallArgs,
            Event::ToolCallEnd(_) => EventType::ToolCallEnd,
            Event::ToolCallChunk(_) => EventType::ToolCallChunk,
            Event::ToolCallResult(_) => EventType::ToolCallResult,
            Event::ThinkingStart(_) => EventType::ThinkingStart,
            Event::ThinkingEnd(_) => EventType::ThinkingEnd,
            Event::StateSnapshot(_) => EventType::StateSnapshot,
            Event::StateDelta(_) => EventType::StateDelta,
            Event::MessagesSnapshot(_) => EventType::MessagesSnapshot,
            Event::ActivitySnapshot(_) => EventType::ActivitySnapshot,
            Event::ActivityDelta(_) => EventType::ActivityDelta,
            Event::Raw(_) => EventType::Raw,
            Event::Custom(_) => EventType::Custom,
            Event::RunStarted(_) => EventType::RunStarted,
            Event::RunFinished(_) => EventType::RunFinished,
            Event::RunError(_) => EventType::RunError,
            Event::StepStarted(_) => EventType::StepStarted,
            Event::StepFinished(_) => EventType::StepFinished,
        }
    }

    /// Returns the timestamp of this event if available.
    pub fn timestamp(&self) -> Option<f64> {
        match self {
            Event::TextMessageStart(e) => e.base.timestamp,
            Event::TextMessageContent(e) => e.base.timestamp,
            Event::TextMessageEnd(e) => e.base.timestamp,
            Event::TextMessageChunk(e) => e.base.timestamp,
            Event::ThinkingTextMessageStart(e) => e.base.timestamp,
            Event::ThinkingTextMessageContent(e) => e.base.timestamp,
            Event::ThinkingTextMessageEnd(e) => e.base.timestamp,
            Event::ToolCallStart(e) => e.base.timestamp,
            Event::ToolCallArgs(e) => e.base.timestamp,
            Event::ToolCallEnd(e) => e.base.timestamp,
            Event::ToolCallChunk(e) => e.base.timestamp,
            Event::ToolCallResult(e) => e.base.timestamp,
            Event::ThinkingStart(e) => e.base.timestamp,
            Event::ThinkingEnd(e) => e.base.timestamp,
            Event::StateSnapshot(e) => e.base.timestamp,
            Event::StateDelta(e) => e.base.timestamp,
            Event::MessagesSnapshot(e) => e.base.timestamp,
            Event::ActivitySnapshot(e) => e.base.timestamp,
            Event::ActivityDelta(e) => e.base.timestamp,
            Event::Raw(e) => e.base.timestamp,
            Event::Custom(e) => e.base.timestamp,
            Event::RunStarted(e) => e.base.timestamp,
            Event::RunFinished(e) => e.base.timestamp,
            Event::RunError(e) => e.base.timestamp,
            Event::StepStarted(e) => e.base.timestamp,
            Event::StepFinished(e) => e.base.timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_serialization() {
        let event_type = EventType::TextMessageStart;
        let json = serde_json::to_string(&event_type).unwrap();
        assert_eq!(json, "\"TEXT_MESSAGE_START\"");

        let event_type = EventType::ToolCallArgs;
        let json = serde_json::to_string(&event_type).unwrap();
        assert_eq!(json, "\"TOOL_CALL_ARGS\"");

        let event_type = EventType::StateSnapshot;
        let json = serde_json::to_string(&event_type).unwrap();
        assert_eq!(json, "\"STATE_SNAPSHOT\"");
    }

    #[test]
    fn test_event_type_deserialization() {
        let event_type: EventType = serde_json::from_str("\"RUN_STARTED\"").unwrap();
        assert_eq!(event_type, EventType::RunStarted);

        let event_type: EventType = serde_json::from_str("\"THINKING_TEXT_MESSAGE_CONTENT\"").unwrap();
        assert_eq!(event_type, EventType::ThinkingTextMessageContent);
    }

    #[test]
    fn test_event_type_as_str() {
        assert_eq!(EventType::TextMessageStart.as_str(), "TEXT_MESSAGE_START");
        assert_eq!(EventType::RunFinished.as_str(), "RUN_FINISHED");
        assert_eq!(EventType::Custom.as_str(), "CUSTOM");
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(format!("{}", EventType::TextMessageStart), "TEXT_MESSAGE_START");
        assert_eq!(format!("{}", EventType::StateDelta), "STATE_DELTA");
    }

    #[test]
    fn test_base_event_serialization() {
        let event = BaseEvent {
            timestamp: Some(1706123456789.0),
            raw_event: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"timestamp\":1706123456789.0"));
        assert!(!json.contains("rawEvent")); // skipped when None
    }

    #[test]
    fn test_base_event_with_raw_event() {
        let event = BaseEvent {
            timestamp: None,
            raw_event: Some(serde_json::json!({"provider": "openai"})),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"rawEvent\""));
        assert!(json.contains("\"provider\":\"openai\""));
    }

    #[test]
    fn test_base_event_builder() {
        let event = BaseEvent::new()
            .timestamp(1234567890.0)
            .raw_event(serde_json::json!({"test": true}));

        assert_eq!(event.timestamp, Some(1234567890.0));
        assert!(event.raw_event.is_some());
    }

    #[test]
    fn test_event_validation_error_display() {
        let error = EventValidationError::EmptyDelta;
        assert_eq!(error.to_string(), "Delta must not be an empty string");

        let error = EventValidationError::InvalidFormat("bad json".to_string());
        assert_eq!(error.to_string(), "Invalid event format: bad json");

        let error = EventValidationError::MissingField("message_id".to_string());
        assert_eq!(error.to_string(), "Missing required field: message_id");

        let error = EventValidationError::TypeMismatch {
            expected: "TEXT_MESSAGE_START".to_string(),
            actual: "RUN_STARTED".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Event type mismatch: expected TEXT_MESSAGE_START, got RUN_STARTED"
        );
    }

    #[test]
    fn test_event_validation_error_is_std_error() {
        fn requires_error<E: std::error::Error>(_: E) {}
        requires_error(EventValidationError::EmptyDelta);
    }

    #[test]
    fn test_all_event_types_roundtrip() {
        let all_types = [
            EventType::TextMessageStart,
            EventType::TextMessageContent,
            EventType::TextMessageEnd,
            EventType::TextMessageChunk,
            EventType::ThinkingTextMessageStart,
            EventType::ThinkingTextMessageContent,
            EventType::ThinkingTextMessageEnd,
            EventType::ToolCallStart,
            EventType::ToolCallArgs,
            EventType::ToolCallEnd,
            EventType::ToolCallChunk,
            EventType::ToolCallResult,
            EventType::ThinkingStart,
            EventType::ThinkingEnd,
            EventType::StateSnapshot,
            EventType::StateDelta,
            EventType::MessagesSnapshot,
            EventType::ActivitySnapshot,
            EventType::ActivityDelta,
            EventType::Raw,
            EventType::Custom,
            EventType::RunStarted,
            EventType::RunFinished,
            EventType::RunError,
            EventType::StepStarted,
            EventType::StepFinished,
        ];

        for event_type in all_types {
            let json = serde_json::to_string(&event_type).unwrap();
            let parsed: EventType = serde_json::from_str(&json).unwrap();
            assert_eq!(event_type, parsed);
        }
    }

    // =========================================================================
    // Text Message Event Tests
    // =========================================================================

    #[test]
    fn test_text_message_start_event() {
        use crate::types::{MessageId, Role};

        let event = TextMessageStartEvent::new(MessageId::random());
        assert_eq!(event.role, Role::Assistant);

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"messageId\""));
        assert!(json.contains("\"role\":\"assistant\""));
    }

    #[test]
    fn test_text_message_start_event_with_timestamp() {
        use crate::types::MessageId;

        let event = TextMessageStartEvent::new(MessageId::random()).with_timestamp(1234567890.0);
        assert_eq!(event.base.timestamp, Some(1234567890.0));
    }

    #[test]
    fn test_text_message_content_event_validation() {
        use crate::types::MessageId;

        // Valid delta
        let result = TextMessageContentEvent::new(MessageId::random(), "Hello");
        assert!(result.is_ok());

        // Empty delta should fail
        let result = TextMessageContentEvent::new(MessageId::random(), "");
        assert!(matches!(result, Err(EventValidationError::EmptyDelta)));
    }

    #[test]
    fn test_text_message_content_event_validate_method() {
        use crate::types::MessageId;

        let event = TextMessageContentEvent::new_unchecked(MessageId::random(), "");
        assert!(matches!(event.validate(), Err(EventValidationError::EmptyDelta)));

        let event = TextMessageContentEvent::new_unchecked(MessageId::random(), "Hello");
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_text_message_content_event_serialization() {
        use crate::types::MessageId;

        let event = TextMessageContentEvent::new(MessageId::random(), "Hello, world!").unwrap();
        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains("\"messageId\""));
        assert!(json.contains("\"delta\":\"Hello, world!\""));
    }

    #[test]
    fn test_text_message_end_event() {
        use crate::types::MessageId;

        let msg_id = MessageId::random();
        let event = TextMessageEndEvent::new(msg_id.clone());

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"messageId\""));
    }

    #[test]
    fn test_text_message_chunk_event() {
        use crate::types::{MessageId, Role};

        let event = TextMessageChunkEvent::new(Role::Assistant)
            .with_message_id(MessageId::random())
            .with_delta("chunk content");

        assert!(event.message_id.is_some());
        assert_eq!(event.delta, Some("chunk content".to_string()));

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"messageId\""));
        assert!(json.contains("\"delta\":\"chunk content\""));
    }

    #[test]
    fn test_text_message_chunk_event_skips_none() {
        use crate::types::Role;

        let event = TextMessageChunkEvent::new(Role::Assistant);
        let json = serde_json::to_string(&event).unwrap();

        // Should not contain optional fields when None
        assert!(!json.contains("\"messageId\""));
        assert!(!json.contains("\"delta\""));
        assert!(json.contains("\"role\":\"assistant\""));
    }

    // =========================================================================
    // Thinking Text Message Event Tests
    // =========================================================================

    #[test]
    fn test_thinking_text_message_start_event() {
        let event = ThinkingTextMessageStartEvent::new();
        let json = serde_json::to_string(&event).unwrap();

        // Should be minimal - just empty object or with timestamp if set
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_thinking_text_message_start_event_with_timestamp() {
        let event = ThinkingTextMessageStartEvent::new().with_timestamp(1234567890.0);
        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains("\"timestamp\":1234567890.0"));
    }

    #[test]
    fn test_thinking_text_message_content_event() {
        let event = ThinkingTextMessageContentEvent::new("Let me think about this...");

        assert_eq!(event.delta, "Let me think about this...");

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"delta\":\"Let me think about this...\""));
    }

    #[test]
    fn test_thinking_text_message_content_event_allows_empty() {
        // Unlike TextMessageContentEvent, ThinkingTextMessageContentEvent allows empty delta
        let event = ThinkingTextMessageContentEvent::new("");
        assert_eq!(event.delta, "");
    }

    #[test]
    fn test_thinking_text_message_end_event() {
        let event = ThinkingTextMessageEndEvent::new();
        let json = serde_json::to_string(&event).unwrap();

        // Should be minimal
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_thinking_text_message_events_default() {
        let start = ThinkingTextMessageStartEvent::default();
        let end = ThinkingTextMessageEndEvent::default();

        assert!(start.base.timestamp.is_none());
        assert!(end.base.timestamp.is_none());
    }

    // =========================================================================
    // Tool Call Event Tests
    // =========================================================================

    #[test]
    fn test_tool_call_start_event() {
        use crate::types::ToolCallId;

        let event = ToolCallStartEvent::new(ToolCallId::random(), "get_weather");

        assert_eq!(event.tool_call_name, "get_weather");
        assert!(event.parent_message_id.is_none());

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"toolCallId\""));
        assert!(json.contains("\"toolCallName\":\"get_weather\""));
        assert!(!json.contains("parentMessageId")); // skipped when None
    }

    #[test]
    fn test_tool_call_start_event_with_parent() {
        use crate::types::{MessageId, ToolCallId};

        let event = ToolCallStartEvent::new(ToolCallId::random(), "get_weather")
            .with_parent_message_id(MessageId::random());

        assert!(event.parent_message_id.is_some());

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"parentMessageId\""));
    }

    #[test]
    fn test_tool_call_args_event() {
        use crate::types::ToolCallId;

        let event = ToolCallArgsEvent::new(ToolCallId::random(), r#"{"location":"#);

        assert_eq!(event.delta, r#"{"location":"#);

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"toolCallId\""));
        assert!(json.contains("\"delta\""));
    }

    #[test]
    fn test_tool_call_end_event() {
        use crate::types::ToolCallId;

        let event = ToolCallEndEvent::new(ToolCallId::random());

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"toolCallId\""));
    }

    #[test]
    fn test_tool_call_chunk_event() {
        use crate::types::ToolCallId;

        let event = ToolCallChunkEvent::new()
            .with_tool_call_id(ToolCallId::random())
            .with_tool_call_name("search")
            .with_delta(r#"{"query": "rust"}"#);

        assert!(event.tool_call_id.is_some());
        assert_eq!(event.tool_call_name, Some("search".to_string()));
        assert!(event.delta.is_some());

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"toolCallId\""));
        assert!(json.contains("\"toolCallName\":\"search\""));
        assert!(json.contains("\"delta\""));
    }

    #[test]
    fn test_tool_call_chunk_event_skips_none() {
        let event = ToolCallChunkEvent::new();
        let json = serde_json::to_string(&event).unwrap();

        // Should not contain optional fields when None
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_tool_call_result_event() {
        use crate::types::{MessageId, Role, ToolCallId};

        let event = ToolCallResultEvent::new(
            MessageId::random(),
            ToolCallId::random(),
            r#"{"weather": "sunny", "temp": 72}"#,
        );

        assert_eq!(event.role, Role::Tool);
        assert!(event.content.contains("sunny"));

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"messageId\""));
        assert!(json.contains("\"toolCallId\""));
        assert!(json.contains("\"content\""));
        assert!(json.contains("\"role\":\"tool\""));
    }

    #[test]
    fn test_tool_call_result_event_deserialize_default_role() {
        // Test that role defaults to "tool" when not present in JSON
        let json = r#"{"messageId":"550e8400-e29b-41d4-a716-446655440000","toolCallId":"6ba7b810-9dad-11d1-80b4-00c04fd430c8","content":"result"}"#;
        let event: ToolCallResultEvent = serde_json::from_str(json).unwrap();

        assert_eq!(event.role, Role::Tool);
    }

    // =========================================================================
    // Run Lifecycle Event Tests
    // =========================================================================

    #[test]
    fn test_run_started_event() {
        use crate::types::{RunId, ThreadId};

        let event = RunStartedEvent::new(ThreadId::random(), RunId::random());

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"threadId\""));
        assert!(json.contains("\"runId\""));
    }

    #[test]
    fn test_run_finished_event() {
        use crate::types::{RunId, ThreadId};

        let event = RunFinishedEvent::new(ThreadId::random(), RunId::random());

        assert!(event.result.is_none());

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"threadId\""));
        assert!(json.contains("\"runId\""));
        assert!(!json.contains("\"result\"")); // skipped when None
    }

    #[test]
    fn test_run_finished_event_with_result() {
        use crate::types::{RunId, ThreadId};

        let event = RunFinishedEvent::new(ThreadId::random(), RunId::random())
            .with_result(serde_json::json!({"success": true}));

        assert!(event.result.is_some());

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"result\""));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_run_error_event() {
        let event = RunErrorEvent::new("Connection timeout");

        assert_eq!(event.message, "Connection timeout");
        assert!(event.code.is_none());

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"message\":\"Connection timeout\""));
        assert!(!json.contains("\"code\"")); // skipped when None
    }

    #[test]
    fn test_run_error_event_with_code() {
        let event = RunErrorEvent::new("Rate limit exceeded").with_code("RATE_LIMITED");

        assert_eq!(event.code, Some("RATE_LIMITED".to_string()));

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"code\":\"RATE_LIMITED\""));
    }

    // =========================================================================
    // Interrupt Tests
    // =========================================================================

    #[test]
    fn test_run_finished_outcome_serialization() {
        // Test SCREAMING_SNAKE_CASE serialization
        let success = RunFinishedOutcome::Success;
        let interrupt = RunFinishedOutcome::Interrupt;

        let success_json = serde_json::to_string(&success).unwrap();
        let interrupt_json = serde_json::to_string(&interrupt).unwrap();

        assert_eq!(success_json, "\"SUCCESS\"");
        assert_eq!(interrupt_json, "\"INTERRUPT\"");

        // Test deserialization
        let deserialized: RunFinishedOutcome = serde_json::from_str("\"SUCCESS\"").unwrap();
        assert_eq!(deserialized, RunFinishedOutcome::Success);

        let deserialized: RunFinishedOutcome = serde_json::from_str("\"INTERRUPT\"").unwrap();
        assert_eq!(deserialized, RunFinishedOutcome::Interrupt);
    }

    #[test]
    fn test_run_finished_outcome_default() {
        let outcome = RunFinishedOutcome::default();
        assert_eq!(outcome, RunFinishedOutcome::Success);
    }

    #[test]
    fn test_interrupt_info_empty() {
        let info = InterruptInfo::new();

        assert!(info.id.is_none());
        assert!(info.reason.is_none());
        assert!(info.payload.is_none());

        // Empty struct should serialize to {}
        let json = serde_json::to_string(&info).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_interrupt_info_with_all_fields() {
        let info = InterruptInfo::new()
            .with_id("approval-001")
            .with_reason("human_approval")
            .with_payload(serde_json::json!({"action": "delete", "rows": 42}));

        assert_eq!(info.id, Some("approval-001".to_string()));
        assert_eq!(info.reason, Some("human_approval".to_string()));
        assert!(info.payload.is_some());

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"id\":\"approval-001\""));
        assert!(json.contains("\"reason\":\"human_approval\""));
        assert!(json.contains("\"action\":\"delete\""));
    }

    #[test]
    fn test_run_finished_event_with_interrupt() {
        use crate::types::{RunId, ThreadId};

        let event = RunFinishedEvent::new(ThreadId::random(), RunId::random())
            .with_interrupt(
                InterruptInfo::new()
                    .with_reason("human_approval")
                    .with_payload(serde_json::json!({"proposal": "send email"}))
            );

        // with_interrupt sets outcome to Interrupt
        assert_eq!(event.outcome, Some(RunFinishedOutcome::Interrupt));
        assert!(event.interrupt.is_some());
        assert!(event.result.is_none());

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"outcome\":\"INTERRUPT\""));
        assert!(json.contains("\"interrupt\""));
        assert!(json.contains("\"reason\":\"human_approval\""));
    }

    #[test]
    fn test_run_finished_event_backward_compatibility() {
        use crate::types::{RunId, ThreadId};

        // Old-style event without outcome field
        let event = RunFinishedEvent::new(ThreadId::random(), RunId::random())
            .with_result(serde_json::json!({"done": true}));

        // outcome is None (backward compat)
        assert!(event.outcome.is_none());
        assert!(event.interrupt.is_none());

        // effective_outcome should infer Success
        assert_eq!(event.effective_outcome(), RunFinishedOutcome::Success);

        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains("\"outcome\"")); // skipped when None
    }

    #[test]
    fn test_run_finished_event_effective_outcome() {
        use crate::types::{RunId, ThreadId};

        // No outcome, no interrupt → Success
        let event1 = RunFinishedEvent::new(ThreadId::random(), RunId::random());
        assert_eq!(event1.effective_outcome(), RunFinishedOutcome::Success);

        // No outcome, has interrupt → Interrupt
        let mut event2 = RunFinishedEvent::new(ThreadId::random(), RunId::random());
        event2.interrupt = Some(InterruptInfo::new());
        assert_eq!(event2.effective_outcome(), RunFinishedOutcome::Interrupt);

        // Explicit outcome overrides
        let event3 = RunFinishedEvent::new(ThreadId::random(), RunId::random())
            .with_outcome(RunFinishedOutcome::Interrupt);
        assert_eq!(event3.effective_outcome(), RunFinishedOutcome::Interrupt);
    }

    // =========================================================================
    // Step Event Tests
    // =========================================================================

    #[test]
    fn test_step_started_event() {
        let event = StepStartedEvent::new("process_input");

        assert_eq!(event.step_name, "process_input");

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"stepName\":\"process_input\""));
    }

    #[test]
    fn test_step_finished_event() {
        let event = StepFinishedEvent::new("generate_response");

        assert_eq!(event.step_name, "generate_response");

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"stepName\":\"generate_response\""));
    }

    #[test]
    fn test_step_events_with_timestamp() {
        let start = StepStartedEvent::new("step1").with_timestamp(1234567890.0);
        let end = StepFinishedEvent::new("step1").with_timestamp(1234567891.0);

        assert_eq!(start.base.timestamp, Some(1234567890.0));
        assert_eq!(end.base.timestamp, Some(1234567891.0));
    }

    // =========================================================================
    // State Event Tests
    // =========================================================================

    #[test]
    fn test_state_snapshot_event() {
        let event = StateSnapshotEvent::new(serde_json::json!({"count": 42}));

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"snapshot\""));
        assert!(json.contains("\"count\":42"));
    }

    #[test]
    fn test_state_snapshot_event_default() {
        let event: StateSnapshotEvent<()> = StateSnapshotEvent::default();
        assert!(event.base.timestamp.is_none());
    }

    #[test]
    fn test_state_delta_event() {
        let patches = vec![
            serde_json::json!({"op": "replace", "path": "/count", "value": 43}),
            serde_json::json!({"op": "add", "path": "/new_field", "value": "hello"}),
        ];
        let event = StateDeltaEvent::new(patches);

        assert_eq!(event.delta.len(), 2);

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"delta\""));
        assert!(json.contains("\"op\":\"replace\""));
    }

    #[test]
    fn test_state_delta_event_default() {
        let event = StateDeltaEvent::default();
        assert!(event.delta.is_empty());
    }

    #[test]
    fn test_messages_snapshot_event() {
        use crate::types::{Message, MessageId};

        let messages = vec![
            Message::User {
                id: MessageId::random(),
                content: "Hello".to_string(),
                name: None,
            },
            Message::Assistant {
                id: MessageId::random(),
                content: Some("Hi there!".to_string()),
                name: None,
                tool_calls: None,
            },
        ];
        let event = MessagesSnapshotEvent::new(messages);

        assert_eq!(event.messages.len(), 2);

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"messages\""));
    }

    #[test]
    fn test_messages_snapshot_event_default() {
        let event = MessagesSnapshotEvent::default();
        assert!(event.messages.is_empty());
    }

    // =========================================================================
    // Thinking Step Event Tests
    // =========================================================================

    #[test]
    fn test_thinking_start_event() {
        let event = ThinkingStartEvent::new();

        assert!(event.title.is_none());

        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains("\"title\"")); // skipped when None
    }

    #[test]
    fn test_thinking_start_event_with_title() {
        let event = ThinkingStartEvent::new().with_title("Analyzing query");

        assert_eq!(event.title, Some("Analyzing query".to_string()));

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"title\":\"Analyzing query\""));
    }

    #[test]
    fn test_thinking_end_event() {
        let event = ThinkingEndEvent::new();

        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_thinking_step_events_default() {
        let start = ThinkingStartEvent::default();
        let end = ThinkingEndEvent::default();

        assert!(start.title.is_none());
        assert!(end.base.timestamp.is_none());
    }

    // =========================================================================
    // Special Event Tests
    // =========================================================================

    #[test]
    fn test_raw_event() {
        let event = RawEvent::new(serde_json::json!({"provider_data": "openai"}));

        assert!(event.source.is_none());

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\""));
        assert!(json.contains("\"provider_data\":\"openai\""));
        assert!(!json.contains("\"source\"")); // skipped when None
    }

    #[test]
    fn test_raw_event_with_source() {
        let event = RawEvent::new(serde_json::json!({})).with_source("anthropic");

        assert_eq!(event.source, Some("anthropic".to_string()));

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"source\":\"anthropic\""));
    }

    #[test]
    fn test_custom_event() {
        let event = CustomEvent::new("user_action", serde_json::json!({"clicked": "button"}));

        assert_eq!(event.name, "user_action");

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"name\":\"user_action\""));
        assert!(json.contains("\"value\""));
        assert!(json.contains("\"clicked\":\"button\""));
    }

    // =========================================================================
    // Event Enum Tests
    // =========================================================================

    #[test]
    fn test_event_enum_serialization() {
        use crate::types::MessageId;

        let event: Event = Event::TextMessageStart(TextMessageStartEvent::new(
            MessageId::random(),
        ));

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"TEXT_MESSAGE_START\""));
        assert!(json.contains("\"messageId\""));
        assert!(json.contains("\"role\":\"assistant\""));
    }

    #[test]
    fn test_event_enum_deserialization() {
        let json = r#"{"type":"RUN_ERROR","message":"Test error"}"#;
        let event: Event = serde_json::from_str(json).unwrap();

        match event {
            Event::RunError(e) => assert_eq!(e.message, "Test error"),
            _ => panic!("Expected RunError variant"),
        }
    }

    #[test]
    fn test_event_type_method() {
        use crate::types::MessageId;

        let event: Event = Event::TextMessageEnd(TextMessageEndEvent::new(MessageId::random()));
        assert_eq!(event.event_type(), EventType::TextMessageEnd);

        let event: Event = Event::RunStarted(RunStartedEvent::new(
            crate::types::ThreadId::random(),
            crate::types::RunId::random(),
        ));
        assert_eq!(event.event_type(), EventType::RunStarted);

        let event: Event = Event::Custom(CustomEvent::new("test", serde_json::json!({})));
        assert_eq!(event.event_type(), EventType::Custom);
    }

    #[test]
    fn test_event_timestamp_method() {
        use crate::types::MessageId;

        let event: Event = Event::TextMessageStart(
            TextMessageStartEvent::new(MessageId::random())
                .with_timestamp(1234567890.0),
        );
        assert_eq!(event.timestamp(), Some(1234567890.0));

        let event: Event = Event::ThinkingEnd(ThinkingEndEvent::new());
        assert_eq!(event.timestamp(), None);
    }

    #[test]
    fn test_event_all_variants_serialize() {
        use crate::types::{Message, MessageId, RunId, ThreadId, ToolCallId};

        // Test that all event variants can be serialized
        let events: Vec<Event> = vec![
            Event::TextMessageStart(TextMessageStartEvent::new(MessageId::random())),
            Event::TextMessageContent(TextMessageContentEvent::new_unchecked(MessageId::random(), "Hello")),
            Event::TextMessageEnd(TextMessageEndEvent::new(MessageId::random())),
            Event::TextMessageChunk(TextMessageChunkEvent::new(Role::Assistant).with_delta("Hi")),
            Event::ThinkingTextMessageStart(ThinkingTextMessageStartEvent::new()),
            Event::ThinkingTextMessageContent(ThinkingTextMessageContentEvent::new("thinking...")),
            Event::ThinkingTextMessageEnd(ThinkingTextMessageEndEvent::new()),
            Event::ToolCallStart(ToolCallStartEvent::new(ToolCallId::random(), "test_tool")),
            Event::ToolCallArgs(ToolCallArgsEvent::new(ToolCallId::random(), "{}")),
            Event::ToolCallEnd(ToolCallEndEvent::new(ToolCallId::random())),
            Event::ToolCallChunk(ToolCallChunkEvent::new()),
            Event::ToolCallResult(ToolCallResultEvent::new(MessageId::random(), ToolCallId::random(), "result")),
            Event::ThinkingStart(ThinkingStartEvent::new()),
            Event::ThinkingEnd(ThinkingEndEvent::new()),
            Event::StateSnapshot(StateSnapshotEvent::new(serde_json::json!({}))),
            Event::StateDelta(StateDeltaEvent::new(vec![])),
            Event::MessagesSnapshot(MessagesSnapshotEvent::new(vec![Message::Assistant {
                id: MessageId::random(),
                content: Some("Hi".to_string()),
                name: None,
                tool_calls: None,
            }])),
            Event::ActivitySnapshot(ActivitySnapshotEvent::new(MessageId::random(), "PLAN", serde_json::json!({"steps": []}))),
            Event::ActivityDelta(ActivityDeltaEvent::new(MessageId::random(), "PLAN", vec![serde_json::json!({"op": "add", "path": "/steps/-", "value": "test"})])),
            Event::Raw(RawEvent::new(serde_json::json!({}))),
            Event::Custom(CustomEvent::new("test", serde_json::json!({}))),
            Event::RunStarted(RunStartedEvent::new(ThreadId::random(), RunId::random())),
            Event::RunFinished(RunFinishedEvent::new(ThreadId::random(), RunId::random())),
            Event::RunError(RunErrorEvent::new("error")),
            Event::StepStarted(StepStartedEvent::new("step")),
            Event::StepFinished(StepFinishedEvent::new("step")),
        ];

        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            assert!(json.contains("\"type\":"));

            // Roundtrip test
            let deserialized: Event = serde_json::from_str(&json).unwrap();
            assert_eq!(event.event_type(), deserialized.event_type());
        }
    }
}
