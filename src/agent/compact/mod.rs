//! Context compaction module (forge-inspired)
//!
//! Provides intelligent compaction of conversation history:
//! - Configurable thresholds (tokens, turns, messages)
//! - Smart eviction strategy (protects tool-call/result adjacency)
//! - Droppable message support for ephemeral content
//! - Summary frame generation for compressed history

mod config;
pub mod strategy;
pub mod summary;

pub use config::{CompactConfig, CompactThresholds};
pub use strategy::{CompactionStrategy, EvictionRange};
pub use summary::{ContextSummary, SummaryFrame};
