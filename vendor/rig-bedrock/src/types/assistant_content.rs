use aws_sdk_bedrockruntime::types as aws_bedrock;

use rig::{
    completion::CompletionError,
    message::{AssistantContent, Text},
};
use serde::{Deserialize, Serialize};

use crate::types::message::RigMessage;

use super::{converse_output::InternalConverseOutput, json::AwsDocument};
use rig::completion;

#[derive(Clone, Deserialize, Serialize)]
pub struct AwsConverseOutput(pub InternalConverseOutput);

impl TryFrom<AwsConverseOutput> for completion::CompletionResponse<AwsConverseOutput> {
    type Error = CompletionError;

    /// Converts AWS Bedrock Converse API output to a Rig CompletionResponse.
    ///
    /// This preserves ALL content blocks from the assistant response including:
    /// - Text content
    /// - ToolCall/ToolUse blocks
    /// - Reasoning blocks (for extended thinking)
    ///
    /// When extended thinking is enabled, Claude returns content in order:
    /// [Reasoning, ToolCall] or [Reasoning, Text]
    ///
    /// AWS Bedrock requires that when replaying conversation history with thinking enabled,
    /// assistant messages MUST start with thinking/reasoning blocks before any tool_use blocks.
    /// By preserving the full choice, we ensure proper conversation history replay.
    fn try_from(value: AwsConverseOutput) -> Result<Self, Self::Error> {
        let message: RigMessage = value
            .to_owned()
            .0
            .output
            .ok_or(CompletionError::ProviderError(
                "Model didn't return any output".into(),
            ))?
            .as_message()
            .map_err(|_| {
                CompletionError::ProviderError(
                    "Failed to extract message from converse output".into(),
                )
            })?
            .to_owned()
            .try_into()?;

        let choice = match message.0 {
            completion::Message::Assistant { content, .. } => Ok(content),
            _ => Err(CompletionError::ResponseError(
                "Response contained no message or tool call (empty)".to_owned(),
            )),
        }?;

        let usage = value
            .0
            .usage()
            .map(|usage| completion::Usage {
                input_tokens: usage.input_tokens as u64,
                output_tokens: usage.output_tokens as u64,
                total_tokens: usage.total_tokens as u64,
            })
            .unwrap_or_default();

        Ok(completion::CompletionResponse {
            choice,
            usage,
            raw_response: value,
        })
    }
}

pub struct RigAssistantContent(pub AssistantContent);

impl TryFrom<aws_bedrock::ContentBlock> for RigAssistantContent {
    type Error = CompletionError;

    fn try_from(value: aws_bedrock::ContentBlock) -> Result<Self, Self::Error> {
        match value {
            aws_bedrock::ContentBlock::Text(text) => {
                Ok(RigAssistantContent(AssistantContent::Text(Text { text })))
            }
            aws_bedrock::ContentBlock::ToolUse(call) => Ok(RigAssistantContent(
                completion::AssistantContent::tool_call(
                    &call.tool_use_id,
                    &call.name,
                    AwsDocument(call.input).into(),
                ),
            )),
            aws_bedrock::ContentBlock::ReasoningContent(reasoning_block) => match reasoning_block {
                aws_bedrock::ReasoningContentBlock::ReasoningText(reasoning_text) => {
                    Ok(RigAssistantContent(AssistantContent::Reasoning(
                        rig::message::Reasoning::new(&reasoning_text.text)
                            .with_signature(reasoning_text.signature),
                    )))
                }
                _ => Err(CompletionError::ProviderError(
                    "AWS Bedrock returned unsupported ReasoningContentBlock variant".into(),
                )),
            },
            _ => Err(CompletionError::ProviderError(
                "AWS Bedrock returned unsupported ContentBlock".into(),
            )),
        }
    }
}

/// Sanitize tool name to match Bedrock's required pattern: [a-zA-Z0-9_-]+
/// Invalid characters are replaced with underscores.
/// This handles cases where the model hallucinates invalid tool names like "$FUNCTION_NAME".
fn sanitize_tool_name(name: &str) -> String {
    if name.is_empty() {
        return "unknown_tool".to_string();
    }

    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Log warning if name was sanitized
    if sanitized != name {
        tracing::warn!(
            original_name = %name,
            sanitized_name = %sanitized,
            "Tool name contained invalid characters and was sanitized for Bedrock API"
        );
    }

    // Ensure the result isn't empty after sanitization
    if sanitized.is_empty() || sanitized.chars().all(|c| c == '_') {
        return "unknown_tool".to_string();
    }

    sanitized
}

impl TryFrom<RigAssistantContent> for aws_bedrock::ContentBlock {
    type Error = CompletionError;

    fn try_from(value: RigAssistantContent) -> Result<Self, Self::Error> {
        match value.0 {
            AssistantContent::Text(text) => Ok(aws_bedrock::ContentBlock::Text(text.text)),
            AssistantContent::ToolCall(tool_call) => {
                // Sanitize tool name to match Bedrock's pattern: [a-zA-Z0-9_-]+
                let sanitized_name = sanitize_tool_name(&tool_call.function.name);
                let doc: AwsDocument = tool_call.function.arguments.into();
                Ok(aws_bedrock::ContentBlock::ToolUse(
                    aws_bedrock::ToolUseBlock::builder()
                        .tool_use_id(tool_call.id)
                        .name(sanitized_name)
                        .input(doc.0)
                        .build()
                        .map_err(|e| CompletionError::ProviderError(e.to_string()))?,
                ))
            }
            AssistantContent::Reasoning(reasoning) => {
                let mut reasoning_block =
                    aws_bedrock::ReasoningTextBlock::builder().text(reasoning.reasoning.join(""));

                if let Some(sig) = &reasoning.signature {
                    reasoning_block = reasoning_block.signature(sig.clone());
                }

                let reasoning_text_block = reasoning_block.build().map_err(|e| {
                    CompletionError::ProviderError(format!(
                        "Failed to build reasoning block: {}",
                        e
                    ))
                })?;

                Ok(aws_bedrock::ContentBlock::ReasoningContent(
                    aws_bedrock::ReasoningContentBlock::ReasoningText(reasoning_text_block),
                ))
            }
            AssistantContent::Image(_) => Err(CompletionError::ProviderError(
                "AWS Bedrock does not support image content in assistant messages".to_owned(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{
        assistant_content::RigAssistantContent, converse_output::InternalConverseOutput,
        errors::TypeConversionError,
    };

    use super::AwsConverseOutput;
    use aws_sdk_bedrockruntime::types as aws_bedrock;
    use rig::{OneOrMany, completion, message::AssistantContent};

    #[test]
    fn aws_converse_output_to_completion_response() {
        let message = aws_bedrock::Message::builder()
            .role(aws_bedrock::ConversationRole::Assistant)
            .content(aws_bedrock::ContentBlock::Text("txt".into()))
            .build()
            .unwrap();
        let output = aws_bedrock::ConverseOutput::Message(message);
        let converse_output =
            aws_sdk_bedrockruntime::operation::converse::ConverseOutput::builder()
                .output(output)
                .stop_reason(aws_bedrock::StopReason::EndTurn)
                .build()
                .unwrap();
        let converse_output: Result<InternalConverseOutput, TypeConversionError> =
            converse_output.try_into();
        assert!(converse_output.is_ok());
        let converse_output = converse_output.unwrap();
        let completion: Result<completion::CompletionResponse<AwsConverseOutput>, _> =
            AwsConverseOutput(converse_output).try_into();
        assert!(completion.is_ok());
        let completion = completion.unwrap();
        assert_eq!(
            completion.choice,
            OneOrMany::one(AssistantContent::Text("txt".into()))
        );
    }

    #[test]
    fn aws_content_block_to_assistant_content() {
        let content_block = aws_bedrock::ContentBlock::Text("text".into());
        let rig_assistant_content: Result<RigAssistantContent, _> = content_block.try_into();
        assert!(rig_assistant_content.is_ok());
        assert_eq!(
            rig_assistant_content.unwrap().0,
            AssistantContent::Text("text".into())
        );
    }

    #[test]
    fn aws_reasoning_content_to_assistant_content_without_signature() {
        // Test conversion from AWS ReasoningContent to Rig AssistantContent without signature
        let reasoning_text_block = aws_bedrock::ReasoningTextBlock::builder()
            .text("This is my reasoning")
            .build()
            .unwrap();

        let content_block = aws_bedrock::ContentBlock::ReasoningContent(
            aws_bedrock::ReasoningContentBlock::ReasoningText(reasoning_text_block),
        );

        let rig_assistant_content: Result<RigAssistantContent, _> = content_block.try_into();
        assert!(rig_assistant_content.is_ok());

        match rig_assistant_content.unwrap().0 {
            AssistantContent::Reasoning(reasoning) => {
                assert_eq!(reasoning.reasoning, vec!["This is my reasoning"]);
                assert_eq!(reasoning.signature, None);
            }
            _ => panic!("Expected AssistantContent::Reasoning"),
        }
    }

    #[test]
    fn aws_reasoning_content_to_assistant_content_with_signature() {
        // Test conversion from AWS ReasoningContent to Rig AssistantContent with signature
        let reasoning_text_block = aws_bedrock::ReasoningTextBlock::builder()
            .text("This is my reasoning with signature")
            .signature("test_signature_123")
            .build()
            .unwrap();

        let content_block = aws_bedrock::ContentBlock::ReasoningContent(
            aws_bedrock::ReasoningContentBlock::ReasoningText(reasoning_text_block),
        );

        let rig_assistant_content: Result<RigAssistantContent, _> = content_block.try_into();
        assert!(rig_assistant_content.is_ok());

        match rig_assistant_content.unwrap().0 {
            AssistantContent::Reasoning(reasoning) => {
                assert_eq!(
                    reasoning.reasoning,
                    vec!["This is my reasoning with signature"]
                );
                assert_eq!(reasoning.signature, Some("test_signature_123".to_string()));
            }
            _ => panic!("Expected AssistantContent::Reasoning"),
        }
    }

    #[test]
    fn rig_reasoning_to_aws_content_block_without_signature() {
        // Test conversion from Rig Reasoning to AWS ContentBlock without signature
        let reasoning = rig::message::Reasoning::new("My reasoning content");
        let rig_content = RigAssistantContent(AssistantContent::Reasoning(reasoning));

        let aws_content_block: Result<aws_bedrock::ContentBlock, _> = rig_content.try_into();
        assert!(aws_content_block.is_ok());

        match aws_content_block.unwrap() {
            aws_bedrock::ContentBlock::ReasoningContent(
                aws_bedrock::ReasoningContentBlock::ReasoningText(reasoning_text),
            ) => {
                assert_eq!(reasoning_text.text, "My reasoning content");
                assert_eq!(reasoning_text.signature, None);
            }
            _ => panic!("Expected ContentBlock::ReasoningContent"),
        }
    }

    #[test]
    fn rig_reasoning_to_aws_content_block_with_signature() {
        // Test conversion from Rig Reasoning to AWS ContentBlock with signature
        let reasoning = rig::message::Reasoning::new("My reasoning content")
            .with_signature(Some("sig_abc_123".to_string()));
        let rig_content = RigAssistantContent(AssistantContent::Reasoning(reasoning));

        let aws_content_block: Result<aws_bedrock::ContentBlock, _> = rig_content.try_into();
        assert!(aws_content_block.is_ok());

        match aws_content_block.unwrap() {
            aws_bedrock::ContentBlock::ReasoningContent(
                aws_bedrock::ReasoningContentBlock::ReasoningText(reasoning_text),
            ) => {
                assert_eq!(reasoning_text.text, "My reasoning content");
                assert_eq!(reasoning_text.signature, Some("sig_abc_123".to_string()));
            }
            _ => panic!("Expected ContentBlock::ReasoningContent"),
        }
    }

    #[test]
    fn rig_reasoning_with_multiple_strings_to_aws_content_block() {
        // Test that multiple reasoning strings are joined correctly
        let mut reasoning = rig::message::Reasoning::new("First part");
        reasoning.reasoning.push(" Second part".to_string());
        reasoning.reasoning.push(" Third part".to_string());

        let rig_content = RigAssistantContent(AssistantContent::Reasoning(reasoning));

        let aws_content_block: Result<aws_bedrock::ContentBlock, _> = rig_content.try_into();
        assert!(aws_content_block.is_ok());

        match aws_content_block.unwrap() {
            aws_bedrock::ContentBlock::ReasoningContent(
                aws_bedrock::ReasoningContentBlock::ReasoningText(reasoning_text),
            ) => {
                assert_eq!(reasoning_text.text, "First part Second part Third part");
            }
            _ => panic!("Expected ContentBlock::ReasoningContent"),
        }
    }

    #[test]
    fn aws_converse_output_preserves_reasoning_with_tool_call() {
        // Test that when extended thinking is enabled and Claude returns both
        // Reasoning and ToolCall, BOTH are preserved in the response.
        // This is critical for AWS Bedrock's requirement that assistant messages
        // must start with thinking blocks when thinking is enabled.

        // Build a message with both Reasoning and ToolUse content blocks
        let reasoning_block = aws_bedrock::ReasoningTextBlock::builder()
            .text("Let me think about this...")
            .signature("sig_test_123")
            .build()
            .unwrap();

        let tool_use_block = aws_bedrock::ToolUseBlock::builder()
            .tool_use_id("tool_123")
            .name("analyze_project")
            .input(aws_smithy_types::Document::Object(
                std::collections::HashMap::new(),
            ))
            .build()
            .unwrap();

        let message = aws_bedrock::Message::builder()
            .role(aws_bedrock::ConversationRole::Assistant)
            .content(aws_bedrock::ContentBlock::ReasoningContent(
                aws_bedrock::ReasoningContentBlock::ReasoningText(reasoning_block),
            ))
            .content(aws_bedrock::ContentBlock::ToolUse(tool_use_block))
            .build()
            .unwrap();

        let output = aws_bedrock::ConverseOutput::Message(message);
        let converse_output =
            aws_sdk_bedrockruntime::operation::converse::ConverseOutput::builder()
                .output(output)
                .stop_reason(aws_bedrock::StopReason::ToolUse)
                .build()
                .unwrap();

        let converse_output: Result<InternalConverseOutput, TypeConversionError> =
            converse_output.try_into();
        assert!(converse_output.is_ok());

        let completion: Result<completion::CompletionResponse<AwsConverseOutput>, _> =
            AwsConverseOutput(converse_output.unwrap()).try_into();
        assert!(completion.is_ok());

        let completion = completion.unwrap();

        // Verify we have BOTH content blocks preserved
        let contents: Vec<_> = completion.choice.iter().collect();
        assert_eq!(
            contents.len(),
            2,
            "Expected both Reasoning and ToolCall to be preserved"
        );

        // First should be Reasoning
        match &contents[0] {
            AssistantContent::Reasoning(reasoning) => {
                assert_eq!(reasoning.reasoning, vec!["Let me think about this..."]);
                assert_eq!(reasoning.signature, Some("sig_test_123".to_string()));
            }
            _ => panic!(
                "Expected first content to be Reasoning, got {:?}",
                contents[0]
            ),
        }

        // Second should be ToolCall
        match &contents[1] {
            AssistantContent::ToolCall(tool_call) => {
                assert_eq!(tool_call.id, "tool_123");
                assert_eq!(tool_call.function.name, "analyze_project");
            }
            _ => panic!(
                "Expected second content to be ToolCall, got {:?}",
                contents[1]
            ),
        }
    }

    #[test]
    fn test_sanitize_tool_name_valid() {
        use super::sanitize_tool_name;

        // Valid names should pass through unchanged
        assert_eq!(sanitize_tool_name("read_file"), "read_file");
        assert_eq!(sanitize_tool_name("analyze-project"), "analyze-project");
        assert_eq!(sanitize_tool_name("tool123"), "tool123");
        assert_eq!(sanitize_tool_name("My_Tool-Name_123"), "My_Tool-Name_123");
    }

    #[test]
    fn test_sanitize_tool_name_invalid_chars() {
        use super::sanitize_tool_name;

        // Invalid characters should be replaced with underscores
        assert_eq!(sanitize_tool_name("$FUNCTION_NAME"), "_FUNCTION_NAME");
        assert_eq!(sanitize_tool_name("tool.name"), "tool_name");
        assert_eq!(sanitize_tool_name("tool name"), "tool_name");
        assert_eq!(sanitize_tool_name("tool@name#test"), "tool_name_test");
        assert_eq!(sanitize_tool_name("hello/world"), "hello_world");
    }

    #[test]
    fn test_sanitize_tool_name_edge_cases() {
        use super::sanitize_tool_name;

        // Empty string
        assert_eq!(sanitize_tool_name(""), "unknown_tool");

        // All invalid characters
        assert_eq!(sanitize_tool_name("$@#!"), "unknown_tool");

        // Single valid character
        assert_eq!(sanitize_tool_name("a"), "a");

        // Unicode characters get replaced
        assert_eq!(sanitize_tool_name("tøøl"), "t__l");
    }
}
