//! Terraform HCL parser for Kubernetes resources.
//!
//! This module re-exports from `parser::terraform` for backward compatibility.
//! New code should use `crate::analyzer::k8s_optimize::parser::*` directly.

// Re-export everything from the new parser::terraform module
pub use super::parser::terraform::*;
