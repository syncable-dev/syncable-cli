//! Kubernetes schema validation and API version tracking.
//!
//! This module provides:
//! - Deprecated Kubernetes API detection
//! - Basic resource kind validation
//! - Kubernetes version compatibility checking

pub mod api_versions;

pub use api_versions::{DeprecatedApi, K8sVersion, get_replacement_api, is_api_deprecated};
