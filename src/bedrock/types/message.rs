use aws_sdk_bedrockruntime::types as aws_bedrock;

use rig::{
    OneOrMany,
    completion::CompletionError,
    message::{AssistantContent, Message, UserContent},
};

use super::{assistant_content::RigAssistantContent, user_content::RigUserContent};

pub struct RigMessage(pub Message);

impl TryFrom<RigMessage> for aws_bedrock::Message {
    type Error = CompletionError;

    fn try_from(value: RigMessage) -> Result<Self, Self::Error> {
        let result = match value.0 {
            Message::User { content } => {
                let message_content = content
                    .into_iter()
                    .map(|user_content| RigUserContent(user_content).try_into())
                    .collect::<Result<Vec<Vec<_>>, _>>()
                    .map_err(|e| CompletionError::RequestError(Box::new(e)))
                    .map(|nested| nested.into_iter().flatten().collect())?;

                aws_bedrock::Message::builder()
                    .role(aws_bedrock::ConversationRole::User)
                    .set_content(Some(message_content))
                    .build()
                    .map_err(|e| CompletionError::RequestError(Box::new(e)))?
            }
            Message::Assistant { content, .. } => {
                // Debug: Log what we're converting from Rig to AWS format
                tracing::debug!(
                    "Converting Assistant message with {} content blocks to AWS format",
                    content.len()
                );
                for (i, c) in content.iter().enumerate() {
                    let type_name = match c {
                        AssistantContent::Reasoning(r) => format!(
                            "Reasoning(len={}, has_sig={})",
                            r.reasoning.len(),
                            r.signature.is_some()
                        ),
                        AssistantContent::ToolCall(t) => {
                            format!("ToolCall(id={}, name={})", t.id, t.function.name)
                        }
                        AssistantContent::Text(t) => format!("Text(len={})", t.text.len()),
                        AssistantContent::Image(_) => "Image".to_string(),
                    };
                    tracing::debug!("  Input content[{}]: {}", i, type_name);
                }

                // Convert all content blocks
                let mut content_blocks: Vec<aws_bedrock::ContentBlock> = content
                    .into_iter()
                    .map(|content| RigAssistantContent(content).try_into())
                    .collect::<Result<Vec<aws_bedrock::ContentBlock>, _>>()?;

                // Debug: Log converted blocks before sorting
                tracing::debug!(
                    "Converted {} content blocks, before sorting:",
                    content_blocks.len()
                );
                for (i, block) in content_blocks.iter().enumerate() {
                    let type_name = match block {
                        aws_bedrock::ContentBlock::ReasoningContent(_) => "ReasoningContent",
                        aws_bedrock::ContentBlock::ToolUse(t) => {
                            tracing::debug!("    ToolUse: id={}, name={}", t.tool_use_id, t.name);
                            "ToolUse"
                        }
                        aws_bedrock::ContentBlock::Text(_) => "Text",
                        _ => "Other",
                    };
                    tracing::debug!("  Block[{}]: {}", i, type_name);
                }

                // CRITICAL: Sort to put Reasoning blocks FIRST
                // AWS Bedrock requires assistant messages to start with thinking blocks
                // when extended thinking is enabled. Without this, multi-turn conversations
                // with tool use fail with: "Expected `thinking` or `redacted_thinking`,
                // but found `tool_use`"
                content_blocks.sort_by_key(|block| match block {
                    aws_bedrock::ContentBlock::ReasoningContent(_) => 0, // First
                    aws_bedrock::ContentBlock::Text(_) => 1,             // Second
                    aws_bedrock::ContentBlock::ToolUse(_) => 2,          // Last
                    _ => 3,
                });

                // Debug: Log after sorting
                tracing::debug!("After sorting, content block order:");
                for (i, block) in content_blocks.iter().enumerate() {
                    let type_name = match block {
                        aws_bedrock::ContentBlock::ReasoningContent(_) => "ReasoningContent",
                        aws_bedrock::ContentBlock::ToolUse(_) => "ToolUse",
                        aws_bedrock::ContentBlock::Text(_) => "Text",
                        _ => "Other",
                    };
                    tracing::debug!("  Block[{}]: {}", i, type_name);
                }

                aws_bedrock::Message::builder()
                    .role(aws_bedrock::ConversationRole::Assistant)
                    .set_content(Some(content_blocks))
                    .build()
                    .map_err(|e| CompletionError::RequestError(Box::new(e)))?
            }
        };
        Ok(result)
    }
}

impl TryFrom<aws_bedrock::Message> for RigMessage {
    type Error = CompletionError;

    fn try_from(message: aws_bedrock::Message) -> Result<Self, Self::Error> {
        match message.role {
            aws_bedrock::ConversationRole::Assistant => {
                let assistant_content = message
                    .content
                    .into_iter()
                    .map(|c| c.try_into())
                    .collect::<Result<Vec<RigAssistantContent>, _>>()?
                    .into_iter()
                    .map(|rig_assistant_content| rig_assistant_content.0)
                    .collect::<Vec<AssistantContent>>();

                let content = OneOrMany::many(assistant_content)
                    .map_err(|e| CompletionError::RequestError(Box::new(e)))?;

                Ok(RigMessage(Message::Assistant { content, id: None }))
            }
            aws_bedrock::ConversationRole::User => {
                let user_content = message
                    .content
                    .into_iter()
                    .map(|c| c.try_into())
                    .collect::<Result<Vec<RigUserContent>, _>>()?
                    .into_iter()
                    .map(|user_content| user_content.0)
                    .collect::<Vec<UserContent>>();

                let content = OneOrMany::many(user_content)
                    .map_err(|e| CompletionError::RequestError(Box::new(e)))?;
                Ok(RigMessage(Message::User { content }))
            }
            _ => Err(CompletionError::ProviderError(
                "AWS Bedrock returned unsupported ConversationRole".into(),
            )),
        }
    }
}

impl TryFrom<super::converse_output::Message> for RigMessage {
    type Error = CompletionError;

    fn try_from(message: super::converse_output::Message) -> Result<Self, Self::Error> {
        let message = aws_bedrock::Message::try_from(message)
            .map_err(|x| CompletionError::ProviderError(format!("Type conversion error: {x}")))?;

        Self::try_from(message)
    }
}

#[cfg(test)]
mod tests {
    use super::RigMessage;
    use aws_sdk_bedrockruntime::types as aws_bedrock;
    use rig::{
        OneOrMany,
        message::{Message, UserContent},
    };

    #[test]
    fn message_to_aws_message() {
        let message = Message::User {
            content: OneOrMany::one(UserContent::Text("text".into())),
        };
        let aws_message: Result<aws_bedrock::Message, _> = RigMessage(message).try_into();
        assert!(aws_message.is_ok());
        let aws_message = aws_message.unwrap();
        assert_eq!(aws_message.role, aws_bedrock::ConversationRole::User);
        assert_eq!(
            aws_message.content,
            vec![aws_bedrock::ContentBlock::Text("text".into())]
        );
    }
}
