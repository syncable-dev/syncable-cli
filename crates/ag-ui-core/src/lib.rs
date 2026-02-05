//! AG-UI Core Types
//!
//! This crate provides the core type definitions for the AG-UI (Agent-User Interaction)
//! protocol. It includes event types, message structures, state management primitives,
//! and error handling for building AG-UI compatible agents.
//!
//! # Overview
//!
//! AG-UI is an event-based protocol that standardizes how AI agents communicate with
//! user-facing applications. This crate provides:
//!
//! - **Event types**: All ~25 AG-UI protocol event types (text messages, tool calls, state, etc.)
//! - **Message types**: Structured message formats for agent-user communication
//! - **State management**: State snapshots and JSON Patch delta operations
//! - **Error handling**: Comprehensive error types for protocol operations
//!
//! # Usage
//!
//! ```rust,ignore
//! use ag_ui_core::{Event, Result};
//! ```

pub mod error;
pub mod event;
pub mod patch;
pub mod state;
pub mod types;

// Re-export key types for convenience
pub use error::{AgUiError, Result};

/// Re-export serde_json::Value for consistent JSON handling across the crate
pub use serde_json::Value as JsonValue;

// Re-export all types at crate root for convenient access
pub use types::*;

// Re-export state traits and helpers
pub use state::{diff_states, AgentState, FwdProps, StateManager, TypedStateManager};

// Re-export event types
pub use event::{
    // Foundation types
    BaseEvent, Event, EventType, EventValidationError,
    // Text message events
    TextMessageChunkEvent, TextMessageContentEvent, TextMessageEndEvent, TextMessageStartEvent,
    // Thinking text message events
    ThinkingTextMessageContentEvent, ThinkingTextMessageEndEvent, ThinkingTextMessageStartEvent,
    // Tool call events
    ToolCallArgsEvent, ToolCallChunkEvent, ToolCallEndEvent, ToolCallResultEvent,
    ToolCallStartEvent,
    // Thinking step events
    ThinkingEndEvent, ThinkingStartEvent,
    // State events
    MessagesSnapshotEvent, StateDeltaEvent, StateSnapshotEvent,
    // Activity events
    ActivityDeltaEvent, ActivitySnapshotEvent,
    // Special events
    CustomEvent, RawEvent,
    // Run lifecycle events
    InterruptInfo, RunErrorEvent, RunFinishedEvent, RunFinishedOutcome, RunStartedEvent,
    // Step events
    StepFinishedEvent, StepStartedEvent,
};
