//! Parsing utilities for Kubernetes resource analysis.
//!
//! This module provides parsers for various input formats:
//! - YAML Kubernetes manifests
//! - Terraform HCL files with kubernetes_* resources
//! - Helm chart rendering

pub mod terraform;
pub mod yaml;

// Re-export from yaml module
pub use yaml::{
    bytes_to_memory_string, cpu_limit_to_request_ratio, detect_workload_type,
    extract_container_image, extract_container_name, extract_resources,
    memory_limit_to_request_ratio, millicores_to_cpu_string, parse_cpu_to_millicores,
    parse_memory_to_bytes,
};

// Re-export from terraform module
pub use terraform::{
    TerraformContainer, TerraformK8sResource, TfResourceSpec, parse_terraform_k8s_resources,
};
