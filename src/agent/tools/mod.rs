//! Agent tools using Rig's Tool trait
//!
//! These tools wrap existing CLI functionality for the agent to use.
//!
//! ## Available Tools
//!
//! ### File Operations
//! - `ReadFileTool` - Read file contents
//! - `WriteFileTool` - Write single files (Dockerfiles, Terraform, etc.)
//! - `WriteFilesTool` - Write multiple files (Terraform modules, Helm charts)
//! - `ListDirectoryTool` - List directory contents
//!
//! ### Analysis
//! - `AnalyzeTool` - Analyze project architecture, dependencies, build commands
//!
//! ### Security
//! - `SecurityScanTool` - Security vulnerability scanning
//! - `VulnerabilitiesTool` - Dependency vulnerability checking
//!
//! ### Shell
//! - `ShellTool` - Execute validation commands (docker build, terraform validate, helm lint)

mod analyze;
mod file_ops;
mod security;
mod shell;

pub use analyze::AnalyzeTool;
pub use file_ops::{ListDirectoryTool, ReadFileTool, WriteFileTool, WriteFilesTool};
pub use security::{SecurityScanTool, VulnerabilitiesTool};
pub use shell::ShellTool;
