//! Terminal UI module for agent interactions
//!
//! Provides a rich terminal UI experience with Syncable's brand colors:
//! - Beautiful response formatting with markdown rendering
//! - Real-time streaming response display
//! - Visible tool call execution with status indicators
//! - Animated spinners with witty phrases during processing
//! - Thinking/reasoning indicators
//! - Elapsed time tracking

pub mod colors;
pub mod hooks;
pub mod response;
pub mod spinner;
pub mod streaming;
pub mod tool_display;

pub use colors::*;
pub use hooks::*;
pub use response::*;
pub use spinner::*;
pub use streaming::*;
pub use tool_display::*;
