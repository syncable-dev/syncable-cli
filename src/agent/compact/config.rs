//! Compaction configuration
//!
//! Defines when and how compaction should be triggered.

use serde::{Deserialize, Serialize};

/// Default values for compaction
pub mod defaults {
    /// Default retention window - messages to keep after compaction
    pub const RETENTION_WINDOW: usize = 10;

    /// Default eviction window - percentage of context to summarize
    pub const EVICTION_WINDOW: f64 = 0.6;

    /// Default token threshold - trigger compaction at this token count
    pub const TOKEN_THRESHOLD: usize = 80_000;

    /// Default turn threshold - trigger compaction after this many turns
    pub const TURN_THRESHOLD: usize = 20;

    /// Default message threshold - trigger compaction after this many messages
    pub const MESSAGE_THRESHOLD: usize = 50;
}

/// Thresholds that trigger compaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactThresholds {
    /// Token count threshold (triggers when exceeded)
    pub token_threshold: Option<usize>,

    /// User turn count threshold (triggers when exceeded)
    pub turn_threshold: Option<usize>,

    /// Total message count threshold (triggers when exceeded)
    pub message_threshold: Option<usize>,

    /// Trigger compaction when last message is from user
    /// (useful for compacting before sending new request)
    pub on_turn_end: Option<bool>,
}

impl Default for CompactThresholds {
    fn default() -> Self {
        Self {
            token_threshold: Some(defaults::TOKEN_THRESHOLD),
            turn_threshold: Some(defaults::TURN_THRESHOLD),
            message_threshold: Some(defaults::MESSAGE_THRESHOLD),
            on_turn_end: None,
        }
    }
}

impl CompactThresholds {
    /// Create minimal thresholds (for aggressive compaction)
    pub fn aggressive() -> Self {
        Self {
            token_threshold: Some(40_000),
            turn_threshold: Some(10),
            message_threshold: Some(25),
            on_turn_end: Some(true),
        }
    }

    /// Create relaxed thresholds (for large context windows)
    pub fn relaxed() -> Self {
        Self {
            token_threshold: Some(150_000),
            turn_threshold: Some(50),
            message_threshold: Some(100),
            on_turn_end: None,
        }
    }

    /// Disable all thresholds (manual compaction only)
    pub fn disabled() -> Self {
        Self {
            token_threshold: None,
            turn_threshold: None,
            message_threshold: None,
            on_turn_end: None,
        }
    }
}

/// Complete compaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactConfig {
    /// Number of most recent messages to always preserve
    pub retention_window: usize,

    /// Percentage of context eligible for summarization (0.0-1.0)
    /// Higher = more aggressive compaction
    pub eviction_window: f64,

    /// Thresholds that trigger automatic compaction
    pub thresholds: CompactThresholds,
}

impl Default for CompactConfig {
    fn default() -> Self {
        Self {
            retention_window: defaults::RETENTION_WINDOW,
            eviction_window: defaults::EVICTION_WINDOW,
            thresholds: CompactThresholds::default(),
        }
    }
}

impl CompactConfig {
    /// Create with custom retention window
    pub fn with_retention(retention: usize) -> Self {
        Self {
            retention_window: retention,
            ..Default::default()
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(thresholds: CompactThresholds) -> Self {
        Self {
            thresholds,
            ..Default::default()
        }
    }

    /// Check if compaction should be triggered based on current state
    ///
    /// # Arguments
    /// * `token_count` - Current estimated token count
    /// * `turn_count` - Number of user turns
    /// * `message_count` - Total number of messages
    /// * `last_is_user` - Whether the last message is from user
    pub fn should_compact(
        &self,
        token_count: usize,
        turn_count: usize,
        message_count: usize,
        last_is_user: bool,
    ) -> bool {
        // Check token threshold
        if let Some(threshold) = self.thresholds.token_threshold {
            if token_count >= threshold {
                return true;
            }
        }

        // Check turn threshold
        if let Some(threshold) = self.thresholds.turn_threshold {
            if turn_count >= threshold {
                return true;
            }
        }

        // Check message threshold
        if let Some(threshold) = self.thresholds.message_threshold {
            if message_count >= threshold {
                return true;
            }
        }

        // Check turn end trigger
        if let Some(true) = self.thresholds.on_turn_end {
            if last_is_user {
                // Only trigger if we're also close to other thresholds
                let near_token = self.thresholds.token_threshold
                    .map(|t| token_count >= t / 2)
                    .unwrap_or(false);
                let near_turn = self.thresholds.turn_threshold
                    .map(|t| turn_count >= t / 2)
                    .unwrap_or(false);

                if near_token || near_turn {
                    return true;
                }
            }
        }

        false
    }

    /// Get the reason why compaction was triggered
    pub fn compaction_reason(
        &self,
        token_count: usize,
        turn_count: usize,
        message_count: usize,
    ) -> Option<String> {
        if let Some(threshold) = self.thresholds.token_threshold {
            if token_count >= threshold {
                return Some(format!("token count ({}) >= threshold ({})", token_count, threshold));
            }
        }

        if let Some(threshold) = self.thresholds.turn_threshold {
            if turn_count >= threshold {
                return Some(format!("turn count ({}) >= threshold ({})", turn_count, threshold));
            }
        }

        if let Some(threshold) = self.thresholds.message_threshold {
            if message_count >= threshold {
                return Some(format!("message count ({}) >= threshold ({})", message_count, threshold));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CompactConfig::default();
        assert_eq!(config.retention_window, defaults::RETENTION_WINDOW);
        assert!((config.eviction_window - defaults::EVICTION_WINDOW).abs() < f64::EPSILON);
    }

    #[test]
    fn test_should_compact_tokens() {
        let config = CompactConfig::default();
        assert!(!config.should_compact(50_000, 5, 10, false));
        assert!(config.should_compact(100_000, 5, 10, false));
    }

    #[test]
    fn test_should_compact_turns() {
        let config = CompactConfig::default();
        assert!(!config.should_compact(10_000, 10, 20, false));
        assert!(config.should_compact(10_000, 25, 50, false));
    }

    #[test]
    fn test_should_compact_messages() {
        let config = CompactConfig::default();
        assert!(!config.should_compact(10_000, 10, 30, false));
        assert!(config.should_compact(10_000, 10, 60, false));
    }

    #[test]
    fn test_aggressive_thresholds() {
        let thresholds = CompactThresholds::aggressive();
        assert_eq!(thresholds.token_threshold, Some(40_000));
        assert_eq!(thresholds.turn_threshold, Some(10));
    }

    #[test]
    fn test_disabled_thresholds() {
        let config = CompactConfig::with_thresholds(CompactThresholds::disabled());
        assert!(!config.should_compact(1_000_000, 1000, 10000, true));
    }
}
