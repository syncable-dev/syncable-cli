//! Message types for the AG-UI protocol.
//!
//! This module defines message structures for agent-user communication,
//! including role definitions and various message type variants.

use crate::types::ids::{MessageId, ToolCallId};
use crate::types::tool::ToolCall;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// A generated function call from a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function (JSON-encoded string).
    pub arguments: String,
}

/// Message role indicating the sender type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Developer messages, typically for debugging.
    Developer,
    /// System messages, usually containing system prompts.
    System,
    /// Assistant messages from the AI model.
    Assistant,
    /// User messages from the human user.
    User,
    /// Tool messages containing tool/function call results.
    Tool,
    /// Activity messages for tracking agent activities.
    Activity,
}

// Utility methods for serde defaults
impl Role {
    pub(crate) fn developer() -> Self {
        Self::Developer
    }
    pub(crate) fn system() -> Self {
        Self::System
    }
    pub(crate) fn assistant() -> Self {
        Self::Assistant
    }
    pub(crate) fn user() -> Self {
        Self::User
    }
    pub(crate) fn tool() -> Self {
        Self::Tool
    }
    pub(crate) fn activity() -> Self {
        Self::Activity
    }
}

/// A basic message with optional string content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaseMessage {
    /// Unique identifier for this message.
    pub id: MessageId,
    /// The role of the message sender.
    pub role: Role,
    /// The text content of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Optional name for the sender.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A developer message, typically for debugging purposes.
/// Not to be confused with system messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeveloperMessage {
    /// Unique identifier for this message.
    pub id: MessageId,
    /// The role (always Developer).
    #[serde(default = "Role::developer")]
    pub role: Role,
    /// The text content of the message.
    pub content: String,
    /// Optional name for the sender.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl DeveloperMessage {
    /// Creates a new developer message with the given ID and content.
    pub fn new(id: impl Into<MessageId>, content: String) -> Self {
        Self {
            id: id.into(),
            role: Role::Developer,
            content,
            name: None,
        }
    }

    /// Sets the name for this message.
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
}

/// A system message, usually containing the system prompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMessage {
    /// Unique identifier for this message.
    pub id: MessageId,
    /// The role (always System).
    #[serde(default = "Role::system")]
    pub role: Role,
    /// The text content of the message.
    pub content: String,
    /// Optional name for the sender.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl SystemMessage {
    /// Creates a new system message with the given ID and content.
    pub fn new(id: impl Into<MessageId>, content: String) -> Self {
        Self {
            id: id.into(),
            role: Role::System,
            content,
            name: None,
        }
    }

    /// Sets the name for this message.
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
}

/// An assistant message (from the AI model).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// Unique identifier for this message.
    pub id: MessageId,
    /// The role (always Assistant).
    #[serde(default = "Role::assistant")]
    pub role: Role,
    /// The text content of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Optional name for the sender.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool calls made by the assistant.
    #[serde(rename = "toolCalls", skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl AssistantMessage {
    /// Creates a new assistant message with the given ID.
    pub fn new(id: impl Into<MessageId>) -> Self {
        Self {
            id: id.into(),
            role: Role::Assistant,
            content: None,
            name: None,
            tool_calls: None,
        }
    }

    /// Sets the content for this message.
    pub fn with_content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }

    /// Sets the name for this message.
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Sets the tool calls for this message.
    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }
}

/// A user message from the human user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    /// Unique identifier for this message.
    pub id: MessageId,
    /// The role (always User).
    #[serde(default = "Role::user")]
    pub role: Role,
    /// The text content of the message.
    pub content: String,
    /// Optional name for the sender.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl UserMessage {
    /// Creates a new user message with the given ID and content.
    pub fn new(id: impl Into<MessageId>, content: String) -> Self {
        Self {
            id: id.into(),
            role: Role::User,
            content,
            name: None,
        }
    }

    /// Sets the name for this message.
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
}

/// A tool message containing the result of a tool/function call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolMessage {
    /// Unique identifier for this message.
    pub id: MessageId,
    /// The text content (tool result).
    pub content: String,
    /// The role (always Tool).
    #[serde(default = "Role::tool")]
    pub role: Role,
    /// The ID of the tool call this result corresponds to.
    #[serde(rename = "toolCallId")]
    pub tool_call_id: ToolCallId,
    /// Optional error message if the tool call failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolMessage {
    /// Creates a new tool message with the given ID, content, and tool call ID.
    pub fn new(
        id: impl Into<MessageId>,
        content: String,
        tool_call_id: impl Into<ToolCallId>,
    ) -> Self {
        Self {
            id: id.into(),
            content,
            role: Role::Tool,
            tool_call_id: tool_call_id.into(),
            error: None,
        }
    }

    /// Sets the error for this message.
    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }
}

/// An activity message for tracking agent activities.
///
/// Activity messages represent structured agent activities like planning,
/// research, or other non-text operations. The content is a flexible JSON
/// object that can hold activity-specific data.
///
/// # Example
///
/// ```
/// use ag_ui_core::{ActivityMessage, MessageId};
/// use serde_json::json;
///
/// let activity = ActivityMessage::new(
///     MessageId::random(),
///     "PLAN".to_string(),
///     json!({"steps": ["research", "implement", "test"]}),
/// );
///
/// assert_eq!(activity.activity_type, "PLAN");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityMessage {
    /// Unique identifier for this message.
    pub id: MessageId,
    /// The role (always Activity).
    #[serde(default = "Role::activity")]
    pub role: Role,
    /// The type of activity (e.g., "PLAN", "RESEARCH").
    #[serde(rename = "activityType")]
    pub activity_type: String,
    /// The activity content as a flexible JSON object.
    pub content: JsonValue,
}

impl ActivityMessage {
    /// Creates a new activity message with the given ID, type, and content.
    pub fn new(
        id: impl Into<MessageId>,
        activity_type: impl Into<String>,
        content: JsonValue,
    ) -> Self {
        Self {
            id: id.into(),
            role: Role::Activity,
            activity_type: activity_type.into(),
            content,
        }
    }

    /// Sets the content for this activity message.
    pub fn with_content(mut self, content: JsonValue) -> Self {
        self.content = content;
        self
    }
}

/// Represents the different types of messages in a conversation.
///
/// This enum provides a unified type for all message variants, using the
/// role field as the discriminant for JSON serialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    /// A developer message for debugging.
    Developer {
        id: MessageId,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// A system message (usually the system prompt).
    System {
        id: MessageId,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// An assistant message from the AI model.
    Assistant {
        id: MessageId,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(rename = "toolCalls", skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
    },
    /// A user message from the human user.
    User {
        id: MessageId,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// A tool message containing tool call results.
    Tool {
        id: MessageId,
        content: String,
        #[serde(rename = "toolCallId")]
        tool_call_id: ToolCallId,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// An activity message for tracking agent activities.
    Activity {
        id: MessageId,
        #[serde(rename = "activityType")]
        activity_type: String,
        content: JsonValue,
    },
}

impl Message {
    /// Creates a new message with the given role, ID, and content.
    pub fn new<S: AsRef<str>>(role: Role, id: impl Into<MessageId>, content: S) -> Self {
        match role {
            Role::Developer => Self::Developer {
                id: id.into(),
                content: content.as_ref().to_string(),
                name: None,
            },
            Role::System => Self::System {
                id: id.into(),
                content: content.as_ref().to_string(),
                name: None,
            },
            Role::Assistant => Self::Assistant {
                id: id.into(),
                content: Some(content.as_ref().to_string()),
                name: None,
                tool_calls: None,
            },
            Role::User => Self::User {
                id: id.into(),
                content: content.as_ref().to_string(),
                name: None,
            },
            Role::Tool => Self::Tool {
                id: id.into(),
                content: content.as_ref().to_string(),
                tool_call_id: ToolCallId::random(),
                error: None,
            },
            Role::Activity => Self::Activity {
                id: id.into(),
                activity_type: "custom".to_string(),
                content: JsonValue::String(content.as_ref().to_string()),
            },
        }
    }

    /// Creates a new user message with a random ID.
    pub fn new_user<S: AsRef<str>>(content: S) -> Self {
        Self::new(Role::User, MessageId::random(), content)
    }

    /// Creates a new tool message with a random ID.
    pub fn new_tool<S: AsRef<str>>(content: S) -> Self {
        Self::new(Role::Tool, MessageId::random(), content)
    }

    /// Creates a new system message with a random ID.
    pub fn new_system<S: AsRef<str>>(content: S) -> Self {
        Self::new(Role::System, MessageId::random(), content)
    }

    /// Creates a new assistant message with a random ID.
    pub fn new_assistant<S: AsRef<str>>(content: S) -> Self {
        Self::new(Role::Assistant, MessageId::random(), content)
    }

    /// Creates a new developer message with a random ID.
    pub fn new_developer<S: AsRef<str>>(content: S) -> Self {
        Self::new(Role::Developer, MessageId::random(), content)
    }

    /// Creates a new activity message with a random ID.
    pub fn new_activity(activity_type: impl Into<String>, content: JsonValue) -> Self {
        Self::Activity {
            id: MessageId::random(),
            activity_type: activity_type.into(),
            content,
        }
    }

    /// Returns a reference to the message ID.
    pub fn id(&self) -> &MessageId {
        match self {
            Message::Developer { id, .. } => id,
            Message::System { id, .. } => id,
            Message::Assistant { id, .. } => id,
            Message::User { id, .. } => id,
            Message::Tool { id, .. } => id,
            Message::Activity { id, .. } => id,
        }
    }

    /// Returns a mutable reference to the message ID.
    pub fn id_mut(&mut self) -> &mut MessageId {
        match self {
            Message::Developer { id, .. } => id,
            Message::System { id, .. } => id,
            Message::Assistant { id, .. } => id,
            Message::User { id, .. } => id,
            Message::Tool { id, .. } => id,
            Message::Activity { id, .. } => id,
        }
    }

    /// Returns the role of this message.
    pub fn role(&self) -> Role {
        match self {
            Message::Developer { .. } => Role::Developer,
            Message::System { .. } => Role::System,
            Message::Assistant { .. } => Role::Assistant,
            Message::User { .. } => Role::User,
            Message::Tool { .. } => Role::Tool,
            Message::Activity { .. } => Role::Activity,
        }
    }

    /// Returns the content of this message, if any.
    ///
    /// Note: Activity messages have JSON content, not string content.
    /// Use `activity_content()` to access their content.
    pub fn content(&self) -> Option<&str> {
        match self {
            Message::Developer { content, .. } => Some(content),
            Message::System { content, .. } => Some(content),
            Message::User { content, .. } => Some(content),
            Message::Tool { content, .. } => Some(content),
            Message::Assistant { content, .. } => content.as_deref(),
            Message::Activity { .. } => None,
        }
    }

    /// Returns a mutable reference to the content of this message.
    ///
    /// Note: Activity messages have JSON content, not string content.
    /// Use `activity_content_mut()` to modify their content.
    pub fn content_mut(&mut self) -> Option<&mut String> {
        match self {
            Message::Developer { content, .. }
            | Message::System { content, .. }
            | Message::User { content, .. }
            | Message::Tool { content, .. } => Some(content),
            Message::Assistant { content, .. } => {
                if content.is_none() {
                    *content = Some(String::new());
                }
                content.as_mut()
            }
            Message::Activity { .. } => None,
        }
    }

    /// Returns the activity content of this message, if it's an activity message.
    pub fn activity_content(&self) -> Option<&JsonValue> {
        match self {
            Message::Activity { content, .. } => Some(content),
            _ => None,
        }
    }

    /// Returns a mutable reference to the activity content, if it's an activity message.
    pub fn activity_content_mut(&mut self) -> Option<&mut JsonValue> {
        match self {
            Message::Activity { content, .. } => Some(content),
            _ => None,
        }
    }

    /// Returns the activity type, if this is an activity message.
    pub fn activity_type(&self) -> Option<&str> {
        match self {
            Message::Activity { activity_type, .. } => Some(activity_type),
            _ => None,
        }
    }

    /// Returns the tool calls for this message, if any.
    pub fn tool_calls(&self) -> Option<&[ToolCall]> {
        match self {
            Message::Assistant { tool_calls, .. } => tool_calls.as_deref(),
            _ => None,
        }
    }

    /// Returns a mutable reference to the tool calls for this message.
    pub fn tool_calls_mut(&mut self) -> Option<&mut Vec<ToolCall>> {
        match self {
            Message::Assistant { tool_calls, .. } => {
                if tool_calls.is_none() {
                    *tool_calls = Some(Vec::new());
                }
                tool_calls.as_mut()
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_serialization() {
        let role = Role::Assistant;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"assistant\"");

        let role = Role::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");
    }

    #[test]
    fn test_developer_message_builder() {
        let msg = DeveloperMessage::new(MessageId::random(), "debug info".to_string())
            .with_name("debugger".to_string());

        assert_eq!(msg.role, Role::Developer);
        assert_eq!(msg.content, "debug info");
        assert_eq!(msg.name, Some("debugger".to_string()));
    }

    #[test]
    fn test_assistant_message_builder() {
        let msg = AssistantMessage::new(MessageId::random())
            .with_content("Hello!".to_string())
            .with_name("Claude".to_string());

        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, Some("Hello!".to_string()));
        assert_eq!(msg.name, Some("Claude".to_string()));
    }

    #[test]
    fn test_message_enum_serialization() {
        let msg = Message::new_user("Hello, world!");
        let json = serde_json::to_string(&msg).unwrap();

        // Should contain "role": "user"
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello, world!\""));
    }

    #[test]
    fn test_message_accessors() {
        let msg = Message::new_assistant("I can help with that.");

        assert_eq!(msg.role(), Role::Assistant);
        assert_eq!(msg.content(), Some("I can help with that."));
        assert!(msg.tool_calls().is_none());
    }

    #[test]
    fn test_activity_role_serialization() {
        let role = Role::Activity;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"activity\"");

        let parsed: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Role::Activity);
    }

    #[test]
    fn test_activity_message_struct() {
        use serde_json::json;

        let activity = ActivityMessage::new(
            MessageId::random(),
            "PLAN",
            json!({"steps": ["research", "implement"]}),
        );

        assert_eq!(activity.role, Role::Activity);
        assert_eq!(activity.activity_type, "PLAN");
        assert_eq!(activity.content["steps"][0], "research");
    }

    #[test]
    fn test_activity_message_serialization() {
        use serde_json::json;

        let activity = ActivityMessage::new(
            MessageId::random(),
            "RESEARCH",
            json!({"query": "rust async"}),
        );

        let json_str = serde_json::to_string(&activity).unwrap();
        assert!(json_str.contains("\"role\":\"activity\""));
        assert!(json_str.contains("\"activityType\":\"RESEARCH\""));
        assert!(json_str.contains("\"query\":\"rust async\""));
    }

    #[test]
    fn test_activity_message_enum() {
        use serde_json::json;

        let msg = Message::new_activity("PLAN", json!({"steps": ["a", "b"]}));

        assert_eq!(msg.role(), Role::Activity);
        assert!(msg.content().is_none()); // Activity has JSON content, not string
        assert!(msg.activity_content().is_some());
        assert_eq!(msg.activity_type(), Some("PLAN"));
    }

    #[test]
    fn test_activity_message_enum_serialization() {
        use serde_json::json;

        let msg = Message::new_activity("DEPLOY", json!({"target": "production"}));
        let json_str = serde_json::to_string(&msg).unwrap();

        assert!(json_str.contains("\"role\":\"activity\""));
        assert!(json_str.contains("\"activityType\":\"DEPLOY\""));
        assert!(json_str.contains("\"target\":\"production\""));

        // Roundtrip
        let parsed: Message = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.role(), Role::Activity);
        assert_eq!(parsed.activity_type(), Some("DEPLOY"));
    }

    #[test]
    fn test_activity_content_accessors() {
        use serde_json::json;

        let mut msg = Message::new_activity("TEST", json!({"status": "pending"}));

        // Test immutable accessor
        assert!(msg.activity_content().is_some());
        assert_eq!(msg.activity_content().unwrap()["status"], "pending");

        // Test mutable accessor
        if let Some(content) = msg.activity_content_mut() {
            content["status"] = json!("complete");
        }
        assert_eq!(msg.activity_content().unwrap()["status"], "complete");

        // Non-activity messages should return None
        let user_msg = Message::new_user("hello");
        assert!(user_msg.activity_content().is_none());
        assert!(user_msg.activity_type().is_none());
    }
}
