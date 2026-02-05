//! AG-UI Server SDK
//!
//! This crate provides server-side functionality for producing AG-UI protocol events.
//! It enables Rust agents to stream events to frontend applications via various
//! transports (SSE, WebSocket, etc.).
//!
//! # Overview
//!
//! The AG-UI Server SDK includes:
//!
//! - **Event Producer**: High-level API for emitting AG-UI events from agent code
//! - **Transport Layer**: SSE and WebSocket implementations for streaming events
//! - **Error Handling**: Server-specific error types
//!
//! # Usage
//!
//! ```rust,ignore
//! use ag_ui_server::{EventProducer, Result};
//! ```
//!
//! # Integration
//!
//! This crate is designed to integrate with the Syncable CLI agent, enabling
//! any frontend to connect and receive real-time agent events.

pub mod error;
pub mod producer;
pub mod transport;

// Re-export ag-ui-core types for convenience
pub use ag_ui_core::*;

// Re-export server-specific types
pub use error::{Result, ServerError};

// Re-export transport types
pub use transport::{SseHandler, SseSender};

// Re-export producer types
pub use producer::{
    AgentSession, EventProducer, MessageStream, ThinkingMessageStream, ThinkingStep,
    ToolCallStream,
};
