//! Input types for AG-UI protocol requests.
//!
//! This module defines types for handling client requests to AG-UI agents,
//! including the main `RunAgentInput` request type and supporting types.

use crate::types::ids::{RunId, ThreadId};
use crate::types::message::Message;
use crate::types::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Context information provided to an agent.
///
/// Context items provide additional information to help the agent
/// understand the user's request or environment.
///
/// # Example
///
/// ```
/// use ag_ui_core::Context;
///
/// let ctx = Context::new(
///     "current_page".to_string(),
///     "https://example.com/dashboard".to_string(),
/// );
///
/// assert_eq!(ctx.description, "current_page");
/// assert_eq!(ctx.value, "https://example.com/dashboard");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    /// A description of what this context represents.
    pub description: String,
    /// The value of the context.
    pub value: String,
}

impl Context {
    /// Creates a new context with the given description and value.
    pub fn new(description: String, value: String) -> Self {
        Self { description, value }
    }
}

/// Input for running an agent.
///
/// This is the primary request type sent by clients to start or continue
/// an agent run. It contains the thread and run identifiers, conversation
/// messages, available tools, context, and any custom state.
///
/// # Example
///
/// ```
/// use ag_ui_core::{RunAgentInput, Context, Message, ThreadId, RunId};
///
/// let input = RunAgentInput::new(ThreadId::random(), RunId::random())
///     .with_messages(vec![Message::new_user("Hello!")])
///     .with_context(vec![
///         Context::new("timezone".to_string(), "UTC".to_string()),
///     ]);
///
/// assert!(input.messages.len() == 1);
/// assert!(input.context.len() == 1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunAgentInput {
    /// The thread identifier for this conversation.
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,

    /// The run identifier for this specific agent invocation.
    #[serde(rename = "runId")]
    pub run_id: RunId,

    /// Optional parent run ID for nested or sub-agent runs.
    #[serde(rename = "parentRunId", skip_serializing_if = "Option::is_none")]
    pub parent_run_id: Option<RunId>,

    /// The current state, can be any JSON value.
    pub state: JsonValue,

    /// The conversation messages.
    pub messages: Vec<Message>,

    /// The tools available to the agent.
    pub tools: Vec<Tool>,

    /// Additional context provided to the agent.
    pub context: Vec<Context>,

    /// Forwarded properties from the client.
    #[serde(rename = "forwardedProps")]
    pub forwarded_props: JsonValue,
}

impl RunAgentInput {
    /// Creates a new RunAgentInput with the given thread and run IDs.
    ///
    /// Initializes with empty messages, tools, context, null state,
    /// and null forwarded props.
    pub fn new(thread_id: impl Into<ThreadId>, run_id: impl Into<RunId>) -> Self {
        Self {
            thread_id: thread_id.into(),
            run_id: run_id.into(),
            parent_run_id: None,
            state: JsonValue::Null,
            messages: Vec::new(),
            tools: Vec::new(),
            context: Vec::new(),
            forwarded_props: JsonValue::Null,
        }
    }

    /// Sets the parent run ID for nested runs.
    pub fn with_parent_run_id(mut self, parent_id: impl Into<RunId>) -> Self {
        self.parent_run_id = Some(parent_id.into());
        self
    }

    /// Sets the state.
    pub fn with_state(mut self, state: JsonValue) -> Self {
        self.state = state;
        self
    }

    /// Sets the messages.
    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    /// Sets the available tools.
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = tools;
        self
    }

    /// Sets the context items.
    pub fn with_context(mut self, context: Vec<Context>) -> Self {
        self.context = context;
        self
    }

    /// Sets the forwarded props.
    pub fn with_forwarded_props(mut self, props: JsonValue) -> Self {
        self.forwarded_props = props;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_context_serialization() {
        let ctx = Context::new("current_page".to_string(), "/dashboard".to_string());
        let json = serde_json::to_string(&ctx).unwrap();

        assert!(json.contains("\"description\":\"current_page\""));
        assert!(json.contains("\"value\":\"/dashboard\""));
    }

    #[test]
    fn test_context_deserialization() {
        let json = r#"{"description":"timezone","value":"UTC"}"#;
        let ctx: Context = serde_json::from_str(json).unwrap();

        assert_eq!(ctx.description, "timezone");
        assert_eq!(ctx.value, "UTC");
    }

    #[test]
    fn test_run_agent_input_minimal() {
        let thread_id = ThreadId::random();
        let run_id = RunId::random();
        let input = RunAgentInput::new(thread_id.clone(), run_id.clone());

        assert_eq!(input.thread_id, thread_id);
        assert_eq!(input.run_id, run_id);
        assert!(input.parent_run_id.is_none());
        assert_eq!(input.state, JsonValue::Null);
        assert!(input.messages.is_empty());
        assert!(input.tools.is_empty());
        assert!(input.context.is_empty());
        assert_eq!(input.forwarded_props, JsonValue::Null);
    }

    #[test]
    fn test_run_agent_input_full() {
        let thread_id = ThreadId::random();
        let run_id = RunId::random();
        let parent_id = RunId::random();

        let input = RunAgentInput::new(thread_id.clone(), run_id.clone())
            .with_parent_run_id(parent_id.clone())
            .with_state(json!({"count": 42}))
            .with_messages(vec![Message::new_user("Hello")])
            .with_tools(vec![Tool::new(
                "get_weather".to_string(),
                "Get weather".to_string(),
                json!({"type": "object"}),
            )])
            .with_context(vec![Context::new("tz".to_string(), "UTC".to_string())])
            .with_forwarded_props(json!({"custom": true}));

        assert_eq!(input.thread_id, thread_id);
        assert_eq!(input.run_id, run_id);
        assert_eq!(input.parent_run_id, Some(parent_id));
        assert_eq!(input.state, json!({"count": 42}));
        assert_eq!(input.messages.len(), 1);
        assert_eq!(input.tools.len(), 1);
        assert_eq!(input.context.len(), 1);
        assert_eq!(input.forwarded_props, json!({"custom": true}));
    }

    #[test]
    fn test_run_agent_input_builder() {
        let input = RunAgentInput::new(ThreadId::random(), RunId::random())
            .with_state(json!(null))
            .with_messages(vec![])
            .with_tools(vec![])
            .with_context(vec![])
            .with_forwarded_props(json!({}));

        assert_eq!(input.state, JsonValue::Null);
        assert!(input.messages.is_empty());
        assert_eq!(input.forwarded_props, json!({}));
    }

    #[test]
    fn test_run_agent_input_serialization() {
        let thread_id = ThreadId::random();
        let run_id = RunId::random();
        let input = RunAgentInput::new(thread_id, run_id);

        let json = serde_json::to_string(&input).unwrap();

        // Check camelCase field names
        assert!(json.contains("\"threadId\""));
        assert!(json.contains("\"runId\""));
        assert!(json.contains("\"forwardedProps\""));
        // parentRunId should be skipped when None
        assert!(!json.contains("\"parentRunId\""));
    }

    #[test]
    fn test_run_agent_input_serialization_with_parent() {
        let input = RunAgentInput::new(ThreadId::random(), RunId::random())
            .with_parent_run_id(RunId::random());

        let json = serde_json::to_string(&input).unwrap();

        // parentRunId should be present when Some
        assert!(json.contains("\"parentRunId\""));
    }

    #[test]
    fn test_run_agent_input_roundtrip() {
        let thread_id = ThreadId::random();
        let run_id = RunId::random();
        let parent_id = RunId::random();

        let original = RunAgentInput::new(thread_id, run_id)
            .with_parent_run_id(parent_id)
            .with_state(json!({"nested": {"value": 123}}))
            .with_messages(vec![
                Message::new_user("Hello"),
                Message::new_assistant("Hi there!"),
            ])
            .with_context(vec![
                Context::new("key1".to_string(), "value1".to_string()),
                Context::new("key2".to_string(), "value2".to_string()),
            ])
            .with_forwarded_props(json!({"prop": "value"}));

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: RunAgentInput = serde_json::from_str(&json).unwrap();

        assert_eq!(original.thread_id, deserialized.thread_id);
        assert_eq!(original.run_id, deserialized.run_id);
        assert_eq!(original.parent_run_id, deserialized.parent_run_id);
        assert_eq!(original.state, deserialized.state);
        assert_eq!(original.messages.len(), deserialized.messages.len());
        assert_eq!(original.context.len(), deserialized.context.len());
        assert_eq!(original.forwarded_props, deserialized.forwarded_props);
    }
}
