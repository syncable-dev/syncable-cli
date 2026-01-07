//! Resource recommendation generation.
//!
//! This module re-exports from `rules` for backward compatibility.
//! New code should use `crate::analyzer::k8s_optimize::rules::*` directly.

// Re-export everything from the new rules module
pub use super::rules::{
    ContainerContext, OptimizationRule, RuleContext, codes as rules, generate_recommendations,
    rule_description,
};
