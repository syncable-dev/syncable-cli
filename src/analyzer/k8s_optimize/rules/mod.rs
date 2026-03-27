//! Individual optimization rules for Kubernetes resources.
//!
//! Each rule is implemented as a separate module with a consistent interface.
//! Rules are identified by codes like K8S-OPT-001, K8S-OPT-002, etc.

mod k8s_opt_001;
mod k8s_opt_002;
mod k8s_opt_003;
mod k8s_opt_004;
mod k8s_opt_005;
mod k8s_opt_006;
mod k8s_opt_007;
mod k8s_opt_008;
mod k8s_opt_009;
mod k8s_opt_010;

use crate::analyzer::k8s_optimize::config::K8sOptimizeConfig;
use crate::analyzer::k8s_optimize::types::{
    ResourceRecommendation, ResourceSpec, Severity, WorkloadType,
};
use std::path::PathBuf;

// ============================================================================
// Rule Trait
// ============================================================================

/// Trait for optimization rules.
pub trait OptimizationRule: Send + Sync {
    /// Get the rule code (e.g., "K8S-OPT-001").
    fn code(&self) -> &'static str;

    /// Get the rule description.
    fn description(&self) -> &'static str;

    /// Get the default severity for this rule.
    fn default_severity(&self) -> Severity;

    /// Check if this rule applies and generate a recommendation if so.
    fn check(
        &self,
        ctx: &RuleContext,
        config: &K8sOptimizeConfig,
    ) -> Option<ResourceRecommendation>;
}

/// Context for rule evaluation.
pub struct RuleContext {
    pub resource_kind: String,
    pub resource_name: String,
    pub namespace: Option<String>,
    pub container_name: String,
    pub file_path: PathBuf,
    pub line: Option<u32>,
    pub current: ResourceSpec,
    pub workload_type: WorkloadType,
}

// ============================================================================
// Rule Codes
// ============================================================================

/// Rule code constants.
pub mod codes {
    pub const NO_CPU_REQUEST: &str = "K8S-OPT-001";
    pub const NO_MEMORY_REQUEST: &str = "K8S-OPT-002";
    pub const NO_CPU_LIMIT: &str = "K8S-OPT-003";
    pub const NO_MEMORY_LIMIT: &str = "K8S-OPT-004";
    pub const HIGH_CPU_REQUEST: &str = "K8S-OPT-005";
    pub const HIGH_MEMORY_REQUEST: &str = "K8S-OPT-006";
    pub const EXCESSIVE_CPU_RATIO: &str = "K8S-OPT-007";
    pub const EXCESSIVE_MEMORY_RATIO: &str = "K8S-OPT-008";
    pub const REQUESTS_EQUAL_LIMITS: &str = "K8S-OPT-009";
    pub const UNBALANCED_RESOURCES: &str = "K8S-OPT-010";
}

// ============================================================================
// Rule Registry
// ============================================================================

/// Get all available optimization rules.
pub fn all_rules() -> Vec<Box<dyn OptimizationRule>> {
    vec![
        Box::new(k8s_opt_001::NoCpuRequestRule),
        Box::new(k8s_opt_002::NoMemoryRequestRule),
        Box::new(k8s_opt_003::NoCpuLimitRule),
        Box::new(k8s_opt_004::NoMemoryLimitRule),
        Box::new(k8s_opt_005::HighCpuRequestRule),
        Box::new(k8s_opt_006::HighMemoryRequestRule),
        Box::new(k8s_opt_007::ExcessiveCpuRatioRule),
        Box::new(k8s_opt_008::ExcessiveMemoryRatioRule),
        Box::new(k8s_opt_009::RequestsEqualLimitsRule),
        Box::new(k8s_opt_010::UnbalancedResourcesRule),
    ]
}

/// Get rule description by code.
pub fn rule_description(code: &str) -> &'static str {
    match code {
        codes::NO_CPU_REQUEST => "No CPU request defined",
        codes::NO_MEMORY_REQUEST => "No memory request defined",
        codes::NO_CPU_LIMIT => "No CPU limit defined",
        codes::NO_MEMORY_LIMIT => "No memory limit defined",
        codes::HIGH_CPU_REQUEST => "CPU request exceeds threshold for workload type",
        codes::HIGH_MEMORY_REQUEST => "Memory request exceeds threshold for workload type",
        codes::EXCESSIVE_CPU_RATIO => "CPU limit to request ratio is excessive",
        codes::EXCESSIVE_MEMORY_RATIO => "Memory limit to request ratio is excessive",
        codes::REQUESTS_EQUAL_LIMITS => "Requests equal limits (no bursting allowed)",
        codes::UNBALANCED_RESOURCES => "Resource allocation is unbalanced for workload type",
        _ => "Unknown rule",
    }
}

// ============================================================================
// Recommendation Generation
// ============================================================================

/// Container context for generating recommendations (backward compatibility).
pub type ContainerContext = RuleContext;

/// Generate recommendations for a container using all applicable rules.
pub fn generate_recommendations(
    ctx: &RuleContext,
    config: &K8sOptimizeConfig,
) -> Vec<ResourceRecommendation> {
    let mut recommendations = Vec::new();

    // Special case: If no resources are defined at all, generate a single critical recommendation
    if !ctx.current.has_requests() && !ctx.current.has_limits() {
        let defaults = ctx.workload_type.default_resources();
        let recommended = ResourceSpec {
            cpu_request: Some(defaults.cpu_request.to_string()),
            cpu_limit: Some(defaults.cpu_limit.to_string()),
            memory_request: Some(defaults.memory_request.to_string()),
            memory_limit: Some(defaults.memory_limit.to_string()),
        };

        recommendations.push(ResourceRecommendation {
            resource_kind: ctx.resource_kind.clone(),
            resource_name: ctx.resource_name.clone(),
            namespace: ctx.namespace.clone(),
            container: ctx.container_name.clone(),
            file_path: ctx.file_path.clone(),
            line: ctx.line,
            issue: crate::analyzer::k8s_optimize::types::OptimizationIssue::NoRequestsDefined,
            severity: Severity::Critical,
            message: "No resource requests defined. This can lead to resource contention, unpredictable scheduling, and OOM kills.".to_string(),
            workload_type: ctx.workload_type,
            current: ctx.current.clone(),
            actual_usage: None,
            recommended: recommended.clone(),
            savings: None,
            fix_yaml: recommended.to_yaml(),
            rule_code: crate::analyzer::k8s_optimize::types::RuleCode::new(codes::NO_CPU_REQUEST),
        });

        return recommendations;
    }

    // Run all rules
    for rule in all_rules() {
        // Skip if rule is ignored
        if config.should_ignore_rule(rule.code()) {
            continue;
        }

        // Check if rule applies
        if let Some(rec) = rule.check(ctx, config) {
            // Filter by severity
            if rec.severity >= config.min_severity {
                recommendations.push(rec);
            }
        }
    }

    recommendations
}

// Re-export rule implementations for direct access
pub use k8s_opt_001::NoCpuRequestRule;
pub use k8s_opt_002::NoMemoryRequestRule;
pub use k8s_opt_003::NoCpuLimitRule;
pub use k8s_opt_004::NoMemoryLimitRule;
pub use k8s_opt_005::HighCpuRequestRule;
pub use k8s_opt_006::HighMemoryRequestRule;
pub use k8s_opt_007::ExcessiveCpuRatioRule;
pub use k8s_opt_008::ExcessiveMemoryRatioRule;
pub use k8s_opt_009::RequestsEqualLimitsRule;
pub use k8s_opt_010::UnbalancedResourcesRule;
