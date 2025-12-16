//! Agent tools using Rig's Tool trait
//!
//! These tools wrap existing CLI functionality for the agent to use.

mod analyze;
mod file_ops;
mod security;

pub use analyze::AnalyzeTool;
pub use file_ops::{ListDirectoryTool, ReadFileTool};
pub use security::{SecurityScanTool, VulnerabilitiesTool};
