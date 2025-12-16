//! Agent tools using Rig's Tool trait
//!
//! These tools wrap existing CLI functionality for the agent to use.
//! 
//! ## Available Tools
//! 
//! ### Analysis & Understanding
//! - `AnalyzeTool` - Comprehensive project analysis (languages, frameworks, dependencies)
//! - `SearchCodeTool` - Grep-like code search with regex support
//! - `FindFilesTool` - Find files by name pattern/extension
//! - `ReadFileTool` - Read file contents with line range support
//! - `ListDirectoryTool` - List directory contents recursively
//! 
//! ### Security
//! - `SecurityScanTool` - Scan for secrets and security issues
//! - `VulnerabilitiesTool` - Check dependencies for known vulnerabilities
//! 
//! ### Generation
//! - `GenerateIaCTool` - Generate Dockerfile, Docker Compose, Terraform

mod analyze;
mod discover;
mod file_ops;
mod generate;
mod search;
mod security;

pub use analyze::AnalyzeTool;
pub use discover::DiscoverServicesTool;
pub use file_ops::{ListDirectoryTool, ReadFileTool};
pub use generate::GenerateIaCTool;
pub use search::{FindFilesTool, SearchCodeTool};
pub use security::{SecurityScanTool, VulnerabilitiesTool};
