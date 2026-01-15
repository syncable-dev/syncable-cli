//! Conversation history management with forge-style compaction support
//!
//! This module provides conversation history storage with intelligent compaction
//! inspired by forge's context management approach:
//! - Configurable thresholds (tokens, turns, messages)
//! - Smart eviction strategy (protects tool-call/result adjacency)
//! - Droppable message support for ephemeral content
//! - Summary frame generation for compressed history

use super::compact::{
    CompactConfig, CompactThresholds, CompactionStrategy, ContextSummary, SummaryFrame,
};
use rig::completion::Message;
use serde::{Deserialize, Serialize};

/// Rough token estimate: ~4 characters per token
const CHARS_PER_TOKEN: usize = 4;

/// A conversation turn containing user input and assistant response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub user_message: String,
    pub assistant_response: String,
    /// Tool calls made during this turn (for context preservation)
    pub tool_calls: Vec<ToolCallRecord>,
    /// Estimated token count for this turn
    pub estimated_tokens: usize,
    /// Whether this turn can be dropped entirely (ephemeral content)
    /// Droppable turns are typically file reads or directory listings
    /// that can be re-fetched if needed
    #[serde(default)]
    pub droppable: bool,
}

/// Record of a tool call for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub args_summary: String,
    pub result_summary: String,
    /// Tool call ID for proper message pairing (optional for backwards compat)
    #[serde(default)]
    pub tool_id: Option<String>,
    /// Whether this tool result is droppable (ephemeral content like file reads)
    #[serde(default)]
    pub droppable: bool,
}

/// Conversation history manager with forge-style compaction support
#[derive(Debug, Clone)]
pub struct ConversationHistory {
    /// Full conversation turns
    turns: Vec<ConversationTurn>,
    /// Compressed summary using SummaryFrame (if any)
    summary_frame: Option<SummaryFrame>,
    /// Total estimated tokens in history
    total_tokens: usize,
    /// Compaction configuration
    compact_config: CompactConfig,
    /// Number of user turns (for threshold checking)
    user_turn_count: usize,
    /// Context summary for tracking file operations and decisions
    context_summary: ContextSummary,
}

impl Default for ConversationHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationHistory {
    pub fn new() -> Self {
        Self {
            turns: Vec::new(),
            summary_frame: None,
            total_tokens: 0,
            compact_config: CompactConfig::default(),
            user_turn_count: 0,
            context_summary: ContextSummary::new(),
        }
    }

    /// Create with custom compaction configuration
    pub fn with_config(config: CompactConfig) -> Self {
        Self {
            turns: Vec::new(),
            summary_frame: None,
            total_tokens: 0,
            compact_config: config,
            user_turn_count: 0,
            context_summary: ContextSummary::new(),
        }
    }

    /// Create with aggressive compaction for limited context windows
    pub fn aggressive() -> Self {
        Self::with_config(CompactConfig {
            retention_window: 5,
            eviction_window: 0.7,
            thresholds: CompactThresholds::aggressive(),
        })
    }

    /// Create with relaxed compaction for large context windows
    pub fn relaxed() -> Self {
        Self::with_config(CompactConfig {
            retention_window: 20,
            eviction_window: 0.5,
            thresholds: CompactThresholds::relaxed(),
        })
    }

    /// Estimate tokens in a string (~4 characters per token)
    /// Public so it can be used for pre-request context size estimation
    pub fn estimate_tokens(text: &str) -> usize {
        text.len() / CHARS_PER_TOKEN
    }

    /// Add a new conversation turn
    pub fn add_turn(
        &mut self,
        user_message: String,
        assistant_response: String,
        tool_calls: Vec<ToolCallRecord>,
    ) {
        // Determine if this turn is droppable based on tool calls
        // Turns that only read files or list directories are droppable
        let droppable = !tool_calls.is_empty()
            && tool_calls.iter().all(|tc| {
                matches!(
                    tc.tool_name.as_str(),
                    "read_file" | "list_directory" | "analyze_project"
                )
            });

        let turn_tokens = Self::estimate_tokens(&user_message)
            + Self::estimate_tokens(&assistant_response)
            + tool_calls
                .iter()
                .map(|tc| {
                    Self::estimate_tokens(&tc.tool_name)
                        + Self::estimate_tokens(&tc.args_summary)
                        + Self::estimate_tokens(&tc.result_summary)
                })
                .sum::<usize>();

        self.turns.push(ConversationTurn {
            user_message,
            assistant_response,
            tool_calls,
            estimated_tokens: turn_tokens,
            droppable,
        });
        self.total_tokens += turn_tokens;
        self.user_turn_count += 1;
    }

    /// Add a turn with explicit droppable flag
    pub fn add_turn_droppable(
        &mut self,
        user_message: String,
        assistant_response: String,
        tool_calls: Vec<ToolCallRecord>,
        droppable: bool,
    ) {
        let turn_tokens = Self::estimate_tokens(&user_message)
            + Self::estimate_tokens(&assistant_response)
            + tool_calls
                .iter()
                .map(|tc| {
                    Self::estimate_tokens(&tc.tool_name)
                        + Self::estimate_tokens(&tc.args_summary)
                        + Self::estimate_tokens(&tc.result_summary)
                })
                .sum::<usize>();

        self.turns.push(ConversationTurn {
            user_message,
            assistant_response,
            tool_calls,
            estimated_tokens: turn_tokens,
            droppable,
        });
        self.total_tokens += turn_tokens;
        self.user_turn_count += 1;
    }

    /// Check if compaction is needed using forge-style thresholds
    pub fn needs_compaction(&self) -> bool {
        let last_is_user = self
            .turns
            .last()
            .map(|t| !t.user_message.is_empty())
            .unwrap_or(false);

        self.compact_config.should_compact(
            self.total_tokens,
            self.user_turn_count,
            self.turns.len(),
            last_is_user,
        )
    }

    /// Get the reason for compaction (for logging)
    pub fn compaction_reason(&self) -> Option<String> {
        self.compact_config.compaction_reason(
            self.total_tokens,
            self.user_turn_count,
            self.turns.len(),
        )
    }

    /// Get current token count
    pub fn token_count(&self) -> usize {
        self.total_tokens
    }

    /// Get number of turns
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Get number of user turns
    pub fn user_turn_count(&self) -> usize {
        self.user_turn_count
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.turns.clear();
        self.summary_frame = None;
        self.total_tokens = 0;
        self.user_turn_count = 0;
        self.context_summary = ContextSummary::new();
    }

    /// Clear turns but preserve the summary frame (for sync with truncated raw_chat_history)
    ///
    /// Use this instead of clear() when raw_chat_history is truncated but we want to
    /// preserve the accumulated context from prior compaction.
    pub fn clear_turns_preserve_context(&mut self) {
        // First compact any remaining turns into the summary
        if self.turns.len() > 1 {
            let _ = self.compact();
        }

        // Now clear turns but keep summary_frame and context_summary
        self.turns.clear();

        // Recalculate tokens (just summary frame now)
        self.total_tokens = self
            .summary_frame
            .as_ref()
            .map(|f| f.token_count)
            .unwrap_or(0);

        // User turn count stays as-is for statistics
    }

    /// Perform forge-style compaction with smart eviction
    /// Returns the summary that was created (for logging/display)
    pub fn compact(&mut self) -> Option<String> {
        use super::compact::strategy::{MessageMeta, MessageRole};
        use super::compact::summary::{
            ToolCallSummary, TurnSummary, extract_assistant_action, extract_user_intent,
        };

        if self.turns.len() < 2 {
            return None; // Nothing to compact
        }

        // Build message metadata for strategy
        let messages: Vec<MessageMeta> = self
            .turns
            .iter()
            .enumerate()
            .flat_map(|(turn_idx, turn)| {
                let mut metas = vec![];

                // User message
                metas.push(MessageMeta {
                    index: turn_idx * 2,
                    role: MessageRole::User,
                    droppable: turn.droppable,
                    has_tool_call: false,
                    is_tool_result: false,
                    tool_id: None,
                    token_count: Self::estimate_tokens(&turn.user_message),
                });

                // Assistant message (may have tool calls)
                let has_tool_call = !turn.tool_calls.is_empty();
                let tool_id = turn.tool_calls.first().and_then(|tc| tc.tool_id.clone());

                metas.push(MessageMeta {
                    index: turn_idx * 2 + 1,
                    role: MessageRole::Assistant,
                    droppable: turn.droppable,
                    has_tool_call,
                    is_tool_result: false,
                    tool_id,
                    token_count: Self::estimate_tokens(&turn.assistant_response),
                });

                metas
            })
            .collect();

        // Use default strategy (evict 60% or retain 10, whichever is more conservative)
        let strategy = CompactionStrategy::default();

        // Calculate eviction range with tool-call safety
        let range =
            strategy.calculate_eviction_range(&messages, self.compact_config.retention_window)?;

        if range.is_empty() {
            return None;
        }

        // Convert message indices to turn indices
        let start_turn = range.start / 2;
        let end_turn = range.end.div_ceil(2);

        if start_turn >= end_turn || end_turn > self.turns.len() {
            return None;
        }

        // Build context summary from turns to evict
        let mut new_context = ContextSummary::new();

        for (i, turn) in self.turns[start_turn..end_turn].iter().enumerate() {
            let turn_summary = TurnSummary {
                turn_number: start_turn + i + 1,
                user_intent: extract_user_intent(&turn.user_message, 80),
                assistant_action: extract_assistant_action(&turn.assistant_response, 100),
                tool_calls: turn
                    .tool_calls
                    .iter()
                    .map(|tc| ToolCallSummary {
                        tool_name: tc.tool_name.clone(),
                        args_summary: tc.args_summary.clone(),
                        result_summary: truncate_text(&tc.result_summary, 100),
                        success: !tc.result_summary.to_lowercase().contains("error"),
                    })
                    .collect(),
                key_decisions: vec![], // Could extract from assistant response
            };
            new_context.add_turn(turn_summary);
        }

        // Merge with existing context summary
        self.context_summary.merge(new_context);

        // Generate summary frame
        let new_frame = SummaryFrame::from_summary(&self.context_summary);

        // Merge with existing frame if present
        if let Some(existing) = &self.summary_frame {
            let merged_content = format!("{}\n\n{}", existing.content, new_frame.content);
            let merged_tokens = existing.token_count + new_frame.token_count;
            self.summary_frame = Some(SummaryFrame {
                content: merged_content,
                token_count: merged_tokens,
            });
        } else {
            self.summary_frame = Some(new_frame);
        }

        // Keep only recent turns (non-evicted)
        let preserved_turns: Vec<_> = self.turns[end_turn..].to_vec();
        let evicted_count = end_turn - start_turn;
        self.turns = preserved_turns;

        // Recalculate token count
        self.total_tokens = self
            .summary_frame
            .as_ref()
            .map(|f| f.token_count)
            .unwrap_or(0)
            + self.turns.iter().map(|t| t.estimated_tokens).sum::<usize>();

        Some(format!(
            "Compacted {} turns ({} → {} tokens)",
            evicted_count,
            self.total_tokens + evicted_count * 500, // rough estimate of evicted tokens
            self.total_tokens
        ))
    }

    /// Emergency compaction - more aggressive than normal
    /// Used when "input too long" error occurs and we need to reduce context urgently.
    /// Temporarily switches to aggressive config, compacts, then restores original.
    pub fn emergency_compact(&mut self) -> Option<String> {
        // Switch to aggressive config temporarily
        let original_config = self.compact_config.clone();
        self.compact_config = CompactConfig {
            retention_window: 3,  // Keep only 3 most recent turns
            eviction_window: 0.9, // Evict 90% of context
            thresholds: CompactThresholds::aggressive(),
        };

        let result = self.compact();

        // Restore original config
        self.compact_config = original_config;
        result
    }

    /// Convert history to Rig Message format for the agent
    /// Uses structured summary frames to preserve context
    pub fn to_messages(&self) -> Vec<Message> {
        use rig::OneOrMany;
        use rig::completion::message::{AssistantContent, Text, UserContent};

        let mut messages = Vec::new();

        // Add summary frame as initial context if present
        if let Some(frame) = &self.summary_frame {
            // Add as a user message with the summary, followed by acknowledgment
            messages.push(Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: format!("[Previous conversation context]\n{}", frame.content),
                })),
            });
            messages.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text:
                        "I understand the previous context. I'll continue from where we left off."
                            .to_string(),
                })),
            });
        }

        // Add recent turns with tool call context as text
        for turn in &self.turns {
            // User message
            messages.push(Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: turn.user_message.clone(),
                })),
            });

            // Build assistant response that includes tool call context
            let mut response_text = String::new();

            // If there were tool calls, include them as text context
            if !turn.tool_calls.is_empty() {
                response_text.push_str("[Tools used in this turn:\n");
                for tc in &turn.tool_calls {
                    response_text.push_str(&format!(
                        "  - {}({}) → {}\n",
                        tc.tool_name,
                        truncate_text(&tc.args_summary, 50),
                        truncate_text(&tc.result_summary, 100)
                    ));
                }
                response_text.push_str("]\n\n");
            }

            // Add the actual response
            response_text.push_str(&turn.assistant_response);

            messages.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: response_text,
                })),
            });
        }

        messages
    }

    /// Check if there's any history
    pub fn is_empty(&self) -> bool {
        self.turns.is_empty() && self.summary_frame.is_none()
    }

    /// Get a brief status string for display
    pub fn status(&self) -> String {
        let compressed_info = if self.summary_frame.is_some() {
            format!(" (+{} compacted)", self.context_summary.turns_compacted)
        } else {
            String::new()
        };
        format!(
            "{} turns, ~{} tokens{}",
            self.turns.len(),
            self.total_tokens,
            compressed_info
        )
    }

    /// Get files that have been read during this session
    pub fn files_read(&self) -> impl Iterator<Item = &str> {
        self.context_summary.files_read.iter().map(|s| s.as_str())
    }

    /// Get files that have been written during this session
    pub fn files_written(&self) -> impl Iterator<Item = &str> {
        self.context_summary
            .files_written
            .iter()
            .map(|s| s.as_str())
    }
}

/// Helper to truncate text with ellipsis
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_turn() {
        let mut history = ConversationHistory::new();
        history.add_turn("Hello".to_string(), "Hi there!".to_string(), vec![]);
        assert_eq!(history.turn_count(), 1);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_droppable_detection() {
        let mut history = ConversationHistory::new();

        // Turn with only read_file should be droppable
        history.add_turn(
            "Read the file".to_string(),
            "Here's the content".to_string(),
            vec![ToolCallRecord {
                tool_name: "read_file".to_string(),
                args_summary: "src/main.rs".to_string(),
                result_summary: "file content...".to_string(),
                tool_id: Some("tool_1".to_string()),
                droppable: true,
            }],
        );
        assert!(history.turns[0].droppable);

        // Turn with write_file should NOT be droppable
        history.add_turn(
            "Write the file".to_string(),
            "Done".to_string(),
            vec![ToolCallRecord {
                tool_name: "write_file".to_string(),
                args_summary: "src/new.rs".to_string(),
                result_summary: "success".to_string(),
                tool_id: Some("tool_2".to_string()),
                droppable: false,
            }],
        );
        assert!(!history.turns[1].droppable);
    }

    #[test]
    fn test_compaction() {
        // Use aggressive config for easier testing
        let mut history = ConversationHistory::with_config(CompactConfig {
            retention_window: 2,
            eviction_window: 0.6,
            thresholds: CompactThresholds {
                token_threshold: Some(500),
                turn_threshold: Some(5),
                message_threshold: Some(10),
                on_turn_end: None,
            },
        });

        // Add many turns to trigger compaction
        for i in 0..10 {
            history.add_turn(
                format!("Question {} with lots of text to increase token count", i),
                format!(
                    "Answer {} with lots of detail to increase token count even more",
                    i
                ),
                vec![ToolCallRecord {
                    tool_name: "analyze".to_string(),
                    args_summary: "path: .".to_string(),
                    result_summary: "Found rust project with many files".to_string(),
                    tool_id: Some(format!("tool_{}", i)),
                    droppable: false,
                }],
            );
        }

        if history.needs_compaction() {
            let summary = history.compact();
            assert!(summary.is_some());
            assert!(history.turn_count() < 10);
            assert!(history.summary_frame.is_some());
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

    #[test]
    fn test_compaction_reason() {
        let mut history = ConversationHistory::with_config(CompactConfig {
            retention_window: 2,
            eviction_window: 0.6,
            thresholds: CompactThresholds {
                token_threshold: Some(100),
                turn_threshold: Some(3),
                message_threshold: Some(5),
                on_turn_end: None,
            },
        });

        // Add turns to exceed threshold
        for i in 0..5 {
            history.add_turn(format!("Question {}", i), format!("Answer {}", i), vec![]);
        }

        assert!(history.needs_compaction());
        let reason = history.compaction_reason();
        assert!(reason.is_some());
    }

    #[test]
    fn test_clear_turns_preserve_context() {
        // Create history with aggressive compaction to trigger summary
        let mut history = ConversationHistory::with_config(CompactConfig {
            retention_window: 2,
            eviction_window: 0.6,
            thresholds: CompactThresholds {
                token_threshold: Some(200),
                turn_threshold: Some(3),
                message_threshold: Some(5),
                on_turn_end: None,
            },
        });

        // Add turns to trigger compaction
        for i in 0..6 {
            history.add_turn(
                format!("Question {} with extra text", i),
                format!("Answer {} with more detail", i),
                vec![],
            );
        }

        // Trigger compaction to build summary
        if history.needs_compaction() {
            let _ = history.compact();
        }

        // Verify we have a summary frame now
        let had_summary_before = history.summary_frame.is_some();

        // Now clear turns while preserving context
        history.clear_turns_preserve_context();

        // Verify turns are cleared but summary is preserved
        assert_eq!(history.turn_count(), 0, "Turns should be cleared");
        assert!(
            history.summary_frame.is_some() == had_summary_before,
            "Summary frame should be preserved"
        );

        // Token count should only include summary frame
        if history.summary_frame.is_some() {
            assert!(history.token_count() > 0, "Should have tokens from summary");
        }

        // to_messages should still work and include summary
        let messages = history.to_messages();
        if history.summary_frame.is_some() {
            assert!(!messages.is_empty(), "Should still have summary in messages");
        }
    }

    #[test]
    fn test_clear_vs_clear_preserve_context() {
        let mut history = ConversationHistory::new();

        // Add some turns
        for i in 0..5 {
            history.add_turn(format!("Q{}", i), format!("A{}", i), vec![]);
        }

        // Force compaction
        let _ = history.compact();
        let had_summary = history.summary_frame.is_some();

        // Test clear_turns_preserve_context
        let mut history_preserve = history.clone();
        history_preserve.clear_turns_preserve_context();

        // Test regular clear
        let mut history_clear = history.clone();
        history_clear.clear();

        // Verify difference
        if had_summary {
            assert!(
                history_preserve.summary_frame.is_some(),
                "preserve should keep summary"
            );
            assert!(history_clear.summary_frame.is_none(), "clear removes summary");
        }

        // Both should have no turns
        assert_eq!(history_preserve.turn_count(), 0);
        assert_eq!(history_clear.turn_count(), 0);
    }
}
