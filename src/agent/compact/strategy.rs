//! Compaction strategy - decides what to evict
//!
//! Implements smart eviction that:
//! - Preserves a retention window of recent messages
//! - Avoids splitting tool call from its result
//! - Handles droppable messages appropriately

use serde::{Deserialize, Serialize};

/// Role of a message in conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Metadata about a message for eviction decisions
#[derive(Debug, Clone)]
pub struct MessageMeta {
    /// Index in the message list
    pub index: usize,
    /// Role of the message
    pub role: MessageRole,
    /// Whether this message can be dropped entirely (ephemeral)
    pub droppable: bool,
    /// Whether this message contains a tool call
    pub has_tool_call: bool,
    /// Whether this is a tool result message
    pub is_tool_result: bool,
    /// Associated tool call ID (for matching call to result)
    pub tool_id: Option<String>,
    /// Estimated token count
    pub token_count: usize,
}

/// Range of messages to evict
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvictionRange {
    /// Start index (inclusive)
    pub start: usize,
    /// End index (exclusive)
    pub end: usize,
}

impl EvictionRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Strategy for choosing what to evict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompactionStrategy {
    /// Evict a percentage of messages
    Evict(f64),
    /// Retain the last N messages
    Retain(usize),
    /// Take minimum of two strategies (more conservative)
    Min(Box<CompactionStrategy>, Box<CompactionStrategy>),
    /// Take maximum of two strategies (more aggressive)
    Max(Box<CompactionStrategy>, Box<CompactionStrategy>),
}

impl Default for CompactionStrategy {
    fn default() -> Self {
        // Default: evict 60% or retain last 10, whichever is more conservative
        Self::Min(Box::new(Self::Evict(0.6)), Box::new(Self::Retain(10)))
    }
}

impl CompactionStrategy {
    /// Calculate eviction range based on strategy
    ///
    /// # Arguments
    /// * `messages` - Metadata about all messages
    /// * `retention_window` - Minimum messages to always keep
    ///
    /// # Returns
    /// The range of messages to evict, adjusted for safety
    pub fn calculate_eviction_range(
        &self,
        messages: &[MessageMeta],
        retention_window: usize,
    ) -> Option<EvictionRange> {
        if messages.len() <= retention_window {
            return None; // Nothing to evict
        }

        let raw_end = self.calculate_raw_end(messages.len(), retention_window);

        // Find safe start: first assistant message (skip initial system/user)
        let start = Self::find_safe_start(messages);

        if start >= raw_end {
            return None; // Nothing to evict
        }

        // Adjust end to avoid splitting tool call/result pairs
        let end = Self::adjust_end_for_tool_safety(messages, raw_end, retention_window);

        if start >= end {
            return None;
        }

        Some(EvictionRange::new(start, end))
    }

    /// Calculate raw end index based on strategy type
    fn calculate_raw_end(&self, total: usize, retention_window: usize) -> usize {
        match self {
            Self::Evict(fraction) => {
                let evict_count = (total as f64 * fraction).floor() as usize;
                total.saturating_sub(retention_window).min(evict_count)
            }
            Self::Retain(keep) => total.saturating_sub(*keep.max(&retention_window)),
            Self::Min(a, b) => {
                let end_a = a.calculate_raw_end(total, retention_window);
                let end_b = b.calculate_raw_end(total, retention_window);
                end_a.min(end_b)
            }
            Self::Max(a, b) => {
                let end_a = a.calculate_raw_end(total, retention_window);
                let end_b = b.calculate_raw_end(total, retention_window);
                end_a.max(end_b)
            }
        }
    }

    /// Find safe start index (first assistant message)
    fn find_safe_start(messages: &[MessageMeta]) -> usize {
        messages
            .iter()
            .position(|m| m.role == MessageRole::Assistant)
            .unwrap_or(0)
    }

    /// Adjust end index to avoid splitting tool call from result
    fn adjust_end_for_tool_safety(
        messages: &[MessageMeta],
        mut end: usize,
        retention_window: usize,
    ) -> usize {
        let min_end = messages.len().saturating_sub(retention_window);

        // Don't go past minimum retention
        if end > min_end {
            end = min_end;
        }

        if end == 0 || end >= messages.len() {
            return end;
        }

        // Check if we're splitting a tool call from its result
        // Look at message at end-1 (last message to evict)
        let last_evicted = &messages[end - 1];

        if last_evicted.has_tool_call {
            // We're evicting a tool call - need to also evict its result
            // Find the tool result with matching ID
            if let Some(tool_id) = &last_evicted.tool_id {
                for i in end..messages.len().min(end + 5) {
                    if messages[i].is_tool_result && messages[i].tool_id.as_ref() == Some(tool_id) {
                        // Found matching result - extend eviction to include it
                        end = i + 1;
                        break;
                    }
                }
            }
        }

        // Check if we're about to evict a tool result without its call
        let msg_at_end = messages.get(end);
        if let Some(msg) = msg_at_end
            && msg.is_tool_result
        {
            // We're keeping a tool result - make sure we also keep its call
            // Move end back to before this tool result group
            while end > 0 {
                let prev = &messages[end - 1];
                if prev.is_tool_result || prev.has_tool_call {
                    end -= 1;
                } else {
                    break;
                }
            }
        }

        // Final safety: don't end in the middle of a tool result sequence
        while end > 0 && end < messages.len() {
            if messages[end].is_tool_result {
                end -= 1;
            } else {
                break;
            }
        }

        end
    }

    /// Filter out droppable messages from a range
    /// Returns indices of non-droppable messages to summarize
    pub fn filter_droppable(messages: &[MessageMeta], range: &EvictionRange) -> Vec<usize> {
        (range.start..range.end)
            .filter(|&i| !messages[i].droppable)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_messages(roles: &[(MessageRole, bool, bool)]) -> Vec<MessageMeta> {
        roles
            .iter()
            .enumerate()
            .map(|(i, (role, has_tool_call, is_tool_result))| MessageMeta {
                index: i,
                role: *role,
                droppable: false,
                has_tool_call: *has_tool_call,
                is_tool_result: *is_tool_result,
                tool_id: if *has_tool_call || *is_tool_result {
                    Some(format!("tool_{}", i))
                } else {
                    None
                },
                token_count: 100,
            })
            .collect()
    }

    #[test]
    fn test_eviction_range_empty() {
        let strategy = CompactionStrategy::Retain(10);
        let messages = make_messages(&[
            (MessageRole::System, false, false),
            (MessageRole::User, false, false),
            (MessageRole::Assistant, false, false),
        ]);

        let range = strategy.calculate_eviction_range(&messages, 5);
        assert!(range.is_none());
    }

    #[test]
    fn test_eviction_starts_at_assistant() {
        let strategy = CompactionStrategy::Evict(0.5);
        let messages = make_messages(&[
            (MessageRole::System, false, false),
            (MessageRole::User, false, false),
            (MessageRole::Assistant, false, false),
            (MessageRole::User, false, false),
            (MessageRole::Assistant, false, false),
            (MessageRole::User, false, false),
            (MessageRole::Assistant, false, false),
        ]);

        let range = strategy.calculate_eviction_range(&messages, 2);
        assert!(range.is_some());
        let range = range.unwrap();
        // Should start at index 2 (first assistant)
        assert_eq!(range.start, 2);
    }

    #[test]
    fn test_tool_call_result_adjacency() {
        let mut messages = make_messages(&[
            (MessageRole::System, false, false),
            (MessageRole::User, false, false),
            (MessageRole::Assistant, true, false), // has tool call
            (MessageRole::Tool, false, true),      // tool result
            (MessageRole::Assistant, false, false),
            (MessageRole::User, false, false),
            (MessageRole::Assistant, false, false),
        ]);

        // Set matching tool IDs
        messages[2].tool_id = Some("call_1".to_string());
        messages[3].tool_id = Some("call_1".to_string());

        let strategy = CompactionStrategy::Retain(2);
        let range = strategy.calculate_eviction_range(&messages, 2);

        // Should either evict both tool call and result, or neither
        if let Some(range) = range {
            // If evicting, should include both call and result
            if range.end > 2 && range.end <= 3 {
                panic!("Eviction split tool call from result!");
            }
        }
    }

    #[test]
    fn test_filter_droppable() {
        let mut messages = make_messages(&[
            (MessageRole::System, false, false),
            (MessageRole::User, false, false),
            (MessageRole::Assistant, false, false),
            (MessageRole::User, false, false), // droppable
            (MessageRole::Assistant, false, false),
        ]);
        messages[3].droppable = true;

        let range = EvictionRange::new(0, 5);
        let non_droppable = CompactionStrategy::filter_droppable(&messages, &range);

        assert_eq!(non_droppable.len(), 4);
        assert!(!non_droppable.contains(&3));
    }

    #[test]
    fn test_min_strategy() {
        let strategy = CompactionStrategy::Min(
            Box::new(CompactionStrategy::Evict(0.8)),
            Box::new(CompactionStrategy::Retain(5)),
        );

        // With 10 messages:
        // Evict(0.8) would evict 8, keeping 2
        // Retain(5) would evict 5, keeping 5
        // Min should be more conservative = evict less = end at 5

        let messages = make_messages(&vec![(MessageRole::Assistant, false, false); 10]);

        let range = strategy.calculate_eviction_range(&messages, 3);
        assert!(range.is_some());
        // Min strategy should be more conservative
    }
}
