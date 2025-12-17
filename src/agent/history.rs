//! Conversation history management with compaction support
//!
//! This module provides conversation history storage and automatic compaction
//! when the token count exceeds a configurable threshold, similar to gemini-cli.

use rig::completion::Message;
use serde::{Deserialize, Serialize};

/// Default threshold for compression as a fraction of context window (85%)
pub const DEFAULT_COMPRESSION_THRESHOLD: f32 = 0.85;

/// Fraction of history to preserve after compression (keep last 30%)
pub const COMPRESSION_PRESERVE_FRACTION: f32 = 0.3;

/// Rough token estimate: ~4 characters per token
const CHARS_PER_TOKEN: usize = 4;

/// Maximum context window tokens (conservative estimate for most models)
const DEFAULT_MAX_CONTEXT_TOKENS: usize = 128_000;

/// A conversation turn containing user input and assistant response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub user_message: String,
    pub assistant_response: String,
    /// Tool calls made during this turn (for context preservation)
    pub tool_calls: Vec<ToolCallRecord>,
    /// Estimated token count for this turn
    pub estimated_tokens: usize,
}

/// Record of a tool call for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub args_summary: String,
    pub result_summary: String,
}

/// Conversation history manager with compaction support
#[derive(Debug, Clone)]
pub struct ConversationHistory {
    /// Full conversation turns
    turns: Vec<ConversationTurn>,
    /// Compressed summary of older turns (if any)
    compressed_summary: Option<String>,
    /// Total estimated tokens in history
    total_tokens: usize,
    /// Maximum tokens before triggering compaction
    compression_threshold_tokens: usize,
}

impl Default for ConversationHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationHistory {
    pub fn new() -> Self {
        let max_tokens = DEFAULT_MAX_CONTEXT_TOKENS;
        Self {
            turns: Vec::new(),
            compressed_summary: None,
            total_tokens: 0,
            compression_threshold_tokens: (max_tokens as f32 * DEFAULT_COMPRESSION_THRESHOLD) as usize,
        }
    }

    /// Create with custom compression threshold
    pub fn with_threshold(max_context_tokens: usize, threshold_fraction: f32) -> Self {
        Self {
            turns: Vec::new(),
            compressed_summary: None,
            total_tokens: 0,
            compression_threshold_tokens: (max_context_tokens as f32 * threshold_fraction) as usize,
        }
    }

    /// Estimate tokens in a string
    fn estimate_tokens(text: &str) -> usize {
        text.len() / CHARS_PER_TOKEN
    }

    /// Add a new conversation turn
    pub fn add_turn(&mut self, user_message: String, assistant_response: String, tool_calls: Vec<ToolCallRecord>) {
        let turn_tokens = Self::estimate_tokens(&user_message)
            + Self::estimate_tokens(&assistant_response)
            + tool_calls.iter().map(|tc| {
                Self::estimate_tokens(&tc.tool_name)
                + Self::estimate_tokens(&tc.args_summary)
                + Self::estimate_tokens(&tc.result_summary)
            }).sum::<usize>();

        self.turns.push(ConversationTurn {
            user_message,
            assistant_response,
            tool_calls,
            estimated_tokens: turn_tokens,
        });
        self.total_tokens += turn_tokens;
    }

    /// Check if compaction is needed
    pub fn needs_compaction(&self) -> bool {
        self.total_tokens > self.compression_threshold_tokens
    }

    /// Get current token count
    pub fn token_count(&self) -> usize {
        self.total_tokens
    }

    /// Get number of turns
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.turns.clear();
        self.compressed_summary = None;
        self.total_tokens = 0;
    }

    /// Perform compaction - summarize older turns and keep recent ones
    /// Returns the summary that was created (for logging/display)
    pub fn compact(&mut self) -> Option<String> {
        if self.turns.len() < 2 {
            return None; // Nothing to compact
        }

        // Calculate split point - keep last 30% of turns
        let preserve_count = ((self.turns.len() as f32) * COMPRESSION_PRESERVE_FRACTION).ceil() as usize;
        let preserve_count = preserve_count.max(1); // Keep at least 1 turn
        let split_point = self.turns.len().saturating_sub(preserve_count);

        if split_point == 0 {
            return None; // Nothing to compress
        }

        // Create summary of older turns
        let turns_to_compress = &self.turns[..split_point];
        let summary = self.create_summary(turns_to_compress);

        // Update compressed summary
        let new_summary = if let Some(existing) = &self.compressed_summary {
            format!("{}\n\n{}", existing, summary)
        } else {
            summary.clone()
        };
        self.compressed_summary = Some(new_summary);

        // Keep only recent turns
        let preserved_turns: Vec<_> = self.turns[split_point..].to_vec();
        self.turns = preserved_turns;

        // Recalculate token count
        self.total_tokens = Self::estimate_tokens(self.compressed_summary.as_deref().unwrap_or(""))
            + self.turns.iter().map(|t| t.estimated_tokens).sum::<usize>();

        Some(summary)
    }

    /// Create a text summary of conversation turns
    fn create_summary(&self, turns: &[ConversationTurn]) -> String {
        let mut summary_parts = Vec::new();

        for (i, turn) in turns.iter().enumerate() {
            let mut turn_summary = format!(
                "Turn {}: User asked about: {}",
                i + 1,
                Self::truncate_text(&turn.user_message, 100)
            );

            if !turn.tool_calls.is_empty() {
                let tool_names: Vec<_> = turn.tool_calls.iter()
                    .map(|tc| tc.tool_name.as_str())
                    .collect();
                turn_summary.push_str(&format!(". Tools used: {}", tool_names.join(", ")));
            }

            turn_summary.push_str(&format!(
                ". Response summary: {}",
                Self::truncate_text(&turn.assistant_response, 200)
            ));

            summary_parts.push(turn_summary);
        }

        format!(
            "Previous conversation summary ({} turns compressed):\n{}",
            turns.len(),
            summary_parts.join("\n")
        )
    }

    /// Truncate text with ellipsis
    fn truncate_text(text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}...", &text[..max_len.saturating_sub(3)])
        }
    }

    /// Convert history to Rig Message format for the agent
    pub fn to_messages(&self) -> Vec<Message> {
        use rig::completion::message::{Text, UserContent, AssistantContent};
        use rig::OneOrMany;

        let mut messages = Vec::new();

        // Add compressed summary as initial context if present
        if let Some(summary) = &self.compressed_summary {
            // Add as a user message with the summary, followed by acknowledgment
            messages.push(Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: format!("[Previous conversation context]\n{}", summary),
                })),
            });
            messages.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: "I understand the previous context. How can I help you continue?".to_string(),
                })),
            });
        }

        // Add recent turns
        for turn in &self.turns {
            // User message
            messages.push(Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: turn.user_message.clone(),
                })),
            });

            // Assistant response (simplified - just the text response)
            // Note: Tool calls are implicitly part of the response context
            messages.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: turn.assistant_response.clone(),
                })),
            });
        }

        messages
    }

    /// Check if there's any history
    pub fn is_empty(&self) -> bool {
        self.turns.is_empty() && self.compressed_summary.is_none()
    }

    /// Get a brief status string for display
    pub fn status(&self) -> String {
        let compressed_info = if self.compressed_summary.is_some() {
            " (with compressed history)"
        } else {
            ""
        };
        format!(
            "{} turns, ~{} tokens{}",
            self.turns.len(),
            self.total_tokens,
            compressed_info
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_turn() {
        let mut history = ConversationHistory::new();
        history.add_turn(
            "Hello".to_string(),
            "Hi there!".to_string(),
            vec![],
        );
        assert_eq!(history.turn_count(), 1);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_compaction() {
        let mut history = ConversationHistory::with_threshold(1000, 0.1); // Low threshold

        // Add many turns to trigger compaction
        for i in 0..10 {
            history.add_turn(
                format!("Question {}", i),
                format!("Answer {} with lots of detail to increase token count", i),
                vec![ToolCallRecord {
                    tool_name: "analyze".to_string(),
                    args_summary: "path: .".to_string(),
                    result_summary: "Found rust project".to_string(),
                }],
            );
        }

        if history.needs_compaction() {
            let summary = history.compact();
            assert!(summary.is_some());
            assert!(history.turn_count() < 10);
        }
    }

    #[test]
    fn test_to_messages() {
        let mut history = ConversationHistory::new();
        history.add_turn(
            "What is this project?".to_string(),
            "This is a Rust CLI tool.".to_string(),
            vec![],
        );

        let messages = history.to_messages();
        assert_eq!(messages.len(), 2); // 1 user + 1 assistant
    }

    #[test]
    fn test_clear() {
        let mut history = ConversationHistory::new();
        history.add_turn("Test".to_string(), "Response".to_string(), vec![]);
        history.clear();
        assert!(history.is_empty());
        assert_eq!(history.token_count(), 0);
    }
}
