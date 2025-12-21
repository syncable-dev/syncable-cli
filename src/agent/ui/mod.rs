//! Terminal UI module for agent interactions
//!
//! Provides a rich terminal UI experience with Syncable's brand colors:
//! - Beautiful response formatting with markdown rendering
//! - Real-time streaming response display
//! - Visible tool call execution with status indicators
//! - Animated spinners with witty phrases during processing
//! - Thinking/reasoning indicators
//! - Elapsed time tracking
//! - Interactive tool confirmation prompts
//! - Diff rendering for file changes

pub mod autocomplete;
pub mod colors;
pub mod confirmation;
pub mod diff;
pub mod hadolint_display;
pub mod hooks;
pub mod input;
pub mod response;
pub mod shell_output;
pub mod spinner;
pub mod streaming;
pub mod tool_display;

pub use autocomplete::*;
pub use colors::*;
pub use confirmation::*;
pub use diff::*;
pub use hadolint_display::*;
pub use hooks::*;
pub use input::*;
pub use response::*;
pub use shell_output::*;
pub use spinner::*;
pub use streaming::*;
pub use tool_display::*;
