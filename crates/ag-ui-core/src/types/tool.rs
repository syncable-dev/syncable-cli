//! Tool types for the AG-UI protocol.
//!
//! This module defines structures for tool/function calling,
//! including tool definitions and tool call representations.

use crate::types::ids::ToolCallId;
use crate::types::message::FunctionCall;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// A tool call made by an assistant.
///
/// Represents a specific invocation of a tool/function by the model,
/// including the tool call ID, type, and function details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call.
    pub id: ToolCallId,
    /// The type of call (always "function" for now).
    #[serde(rename = "type")]
    pub call_type: String,
    /// The function being called with its arguments.
    pub function: FunctionCall,
}

impl ToolCall {
    /// Creates a new tool call with the given ID and function.
    pub fn new(id: impl Into<ToolCallId>, function: FunctionCall) -> Self {
        Self {
            id: id.into(),
            call_type: "function".to_string(),
            function,
        }
    }
}

/// A tool definition describing a function the model can call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tool {
    /// The name of the tool.
    pub name: String,
    /// A description of what the tool does.
    pub description: String,
    /// JSON Schema describing the tool's parameters.
    pub parameters: JsonValue,
}

impl Tool {
    /// Creates a new tool definition.
    pub fn new(name: String, description: String, parameters: JsonValue) -> Self {
        Self {
            name,
            description,
            parameters,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_call_serialization() {
        let tool_call = ToolCall::new(
            ToolCallId::random(),
            FunctionCall {
                name: "get_weather".to_string(),
                arguments: r#"{"location": "NYC"}"#.to_string(),
            },
        );

        let json = serde_json::to_string(&tool_call).unwrap();
        // Should have "type": "function", not "call_type"
        assert!(json.contains("\"type\":\"function\""));
        assert!(json.contains("\"name\":\"get_weather\""));
    }
}
