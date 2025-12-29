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
//! - `HelmlintTool` - Native Helm chart structure/template linting
//! - `KubelintTool` - Native Kubernetes manifest security/best practice linting
//!
//! ### Helm vs Kubernetes Linting
//! - **HelmlintTool**: Use for Helm chart development - validates Chart.yaml, values.yaml,
//!   Go template syntax, and Helm-specific best practices. Works on chart directories.
//! - **KubelintTool**: Use for K8s security - checks rendered manifests for privileged containers,
//!   missing probes, RBAC issues, resource limits. Works on YAML files, Helm charts (renders them),
//!   and Kustomize directories.
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
mod helmlint;
mod kubelint;
mod plan;
mod security;
mod shell;
mod terraform;
mod truncation;

pub use truncation::TruncationLimits;

pub use analyze::AnalyzeTool;
pub use dclint::DclintTool;
pub use diagnostics::DiagnosticsTool;
pub use file_ops::{ListDirectoryTool, ReadFileTool, WriteFileTool, WriteFilesTool};
pub use hadolint::HadolintTool;
pub use helmlint::HelmlintTool;
pub use kubelint::KubelintTool;
pub use plan::{PlanCreateTool, PlanListTool, PlanNextTool, PlanUpdateTool};
pub use security::{SecurityScanTool, VulnerabilitiesTool};
pub use shell::ShellTool;
pub use terraform::{TerraformFmtTool, TerraformInstallTool, TerraformValidateTool};
