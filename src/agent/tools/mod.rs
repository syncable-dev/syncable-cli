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
//! ### Resource Optimization
//! - `K8sOptimizeTool` - Kubernetes resource right-sizing and cost optimization
//! - `K8sCostsTool` - Kubernetes workload cost attribution and analysis
//! - `K8sDriftTool` - Detect configuration drift between manifests and cluster
//!
//! ### Prometheus Integration (for live K8s analysis)
//! - `PrometheusDiscoverTool` - Discover Prometheus services in Kubernetes cluster
//! - `PrometheusConnectTool` - Establish connection to Prometheus (port-forward or URL)
//! - `BackgroundProcessManager` - Manage long-running background processes
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
//! ### Web
//! - `WebFetchTool` - Fetch content from URLs (converts HTML to markdown)
//!
//! ## Error Handling Pattern
//!
//! Tools use the shared error utilities in `error.rs`:
//!
//! 1. Each tool keeps its own error type (e.g., `ReadFileError`, `ShellError`)
//! 2. Use `ToolErrorContext` trait to add context when propagating errors
//! 3. Use `format_error_for_llm` for structured JSON error responses to the agent
//! 4. Error categories help the agent understand and recover from errors
//!
//! See `error.rs` for the complete error handling infrastructure.

mod analyze;
pub mod background;
pub mod compression;
mod dclint;
mod diagnostics;
pub mod error;
mod fetch;
mod file_ops;
mod hadolint;
mod helmlint;
mod k8s_costs;
mod k8s_drift;
mod k8s_optimize;
mod kubelint;
pub mod output_store;
mod plan;
mod prometheus_connect;
mod prometheus_discover;
mod retrieve;
mod security;
mod shell;
mod terraform;
mod truncation;

pub use truncation::{TruncationLimits, truncate_json_output};

// Smart compression exports
pub use compression::{CompressionConfig, compress_analysis_output, compress_tool_output};
pub use retrieve::{ListOutputsTool, RetrieveOutputTool};

// Error handling utilities for tools
pub use error::{
    ErrorCategory, ToolErrorContext, detect_error_category, format_error_for_llm,
    format_error_with_context,
};

pub use analyze::AnalyzeTool;
pub use background::BackgroundProcessManager;
pub use dclint::DclintTool;
pub use diagnostics::DiagnosticsTool;
pub use fetch::WebFetchTool;
pub use file_ops::{ListDirectoryTool, ReadFileTool, WriteFileTool, WriteFilesTool};
pub use hadolint::HadolintTool;
pub use helmlint::HelmlintTool;
pub use k8s_costs::K8sCostsTool;
pub use k8s_drift::K8sDriftTool;
pub use k8s_optimize::K8sOptimizeTool;
pub use kubelint::KubelintTool;
pub use plan::{PlanCreateTool, PlanListTool, PlanNextTool, PlanUpdateTool};
pub use prometheus_connect::PrometheusConnectTool;
pub use prometheus_discover::PrometheusDiscoverTool;
pub use security::{SecurityScanTool, VulnerabilitiesTool};
pub use shell::ShellTool;
pub use terraform::{TerraformFmtTool, TerraformInstallTool, TerraformValidateTool};
