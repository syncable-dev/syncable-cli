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
//! ### Linting
//! - `HadolintTool` - Native Dockerfile linting (best practices, security)
//! - `DclintTool` - Native Docker Compose linting (best practices, style, security)
//!
//! ### Diagnostics
//! - `DiagnosticsTool` - Check for code errors via IDE/LSP or language-specific commands
//!
//! ### Terraform
//! - `TerraformFmtTool` - Format Terraform configuration files
//! - `TerraformValidateTool` - Validate Terraform configurations
//! - `TerraformInstallTool` - Install Terraform CLI (auto-detects OS)
//!
//! ### Shell
//! - `ShellTool` - Execute validation commands (docker build, terraform validate, helm lint)
//!
//! ### Planning (Forge-style workflow)
//! - `PlanCreateTool` - Create structured plan files with task checkboxes
//! - `PlanNextTool` - Get next pending task and mark it in-progress
//! - `PlanUpdateTool` - Update task status (done, failed)
//! - `PlanListTool` - List all available plan files
//!
mod analyze;
mod dclint;
mod diagnostics;
mod file_ops;
mod hadolint;
mod plan;
mod security;
mod shell;
mod terraform;
mod truncation;

pub use truncation::TruncationLimits;

pub use analyze::AnalyzeTool;
pub use diagnostics::DiagnosticsTool;
pub use file_ops::{ListDirectoryTool, ReadFileTool, WriteFileTool, WriteFilesTool};
pub use dclint::DclintTool;
pub use hadolint::HadolintTool;
pub use plan::{PlanCreateTool, PlanListTool, PlanNextTool, PlanUpdateTool};
pub use security::{SecurityScanTool, VulnerabilitiesTool};
pub use shell::ShellTool;
pub use terraform::{TerraformFmtTool, TerraformInstallTool, TerraformValidateTool};
