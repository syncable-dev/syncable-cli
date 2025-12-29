//! Extractors for Kubernetes object data.
//!
//! Helper functions to extract specific data from K8s objects
//! for use in checks.

pub mod container;
pub mod metadata;
pub mod pod_spec;

pub use container::*;
pub use metadata::*;
pub use pod_spec::*;
