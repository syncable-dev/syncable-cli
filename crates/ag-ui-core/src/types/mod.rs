//! AG-UI Protocol Types
//!
//! This module defines core protocol types including:
//! - Message types (user, assistant, system, tool)
//! - Role definitions
//! - ID types (MessageId, RunId, ThreadId, ToolCallId)
//! - Context and input types
//! - Content types (text, binary) for multimodal messages

mod content;
mod ids;
mod input;
mod message;
mod tool;

pub use content::*;
pub use ids::*;
pub use input::*;
pub use message::*;
pub use tool::*;
