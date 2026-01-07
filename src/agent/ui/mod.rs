//! Terminal UI module for agent interactions
//!
//! Provides a rich terminal UI experience with Syncable's brand colors:
//! - Beautiful response formatting with markdown rendering
//! - Real-time streaming response display
//! - Visible tool call execution with status indicators
//! - Animated progress bar with token counter during generation
//! - Thinking/reasoning indicators
//! - Elapsed time tracking
//! - Interactive tool confirmation prompts
//! - Diff rendering for file changes
//! - ANSI scroll regions for split layout (output + fixed input)

pub mod autocomplete;
pub mod colors;
pub mod confirmation;
pub mod diff;
pub mod hadolint_display;
pub mod helmlint_display;
pub mod hooks;
pub mod input;
pub mod kubelint_display;
pub mod layout;
pub mod plan_menu;
pub mod progress;
pub mod prometheus_display;
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
pub use helmlint_display::*;
pub use hooks::*;
pub use input::*;
pub use kubelint_display::*;
pub use layout::*;
pub use plan_menu::*;
pub use progress::*;
pub use prometheus_display::*;
pub use response::*;
pub use shell_output::*;
pub use spinner::*;
pub use streaming::*;
pub use tool_display::*;
