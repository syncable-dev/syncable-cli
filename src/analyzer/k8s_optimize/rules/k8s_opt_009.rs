//! K8S-OPT-009: Requests equal limits (no bursting allowed).

use super::{OptimizationRule, RuleContext, codes};
use crate::analyzer::k8s_optimize::config::K8sOptimizeConfig;
use crate::analyzer::k8s_optimize::types::{
    OptimizationIssue, ResourceRecommendation, RuleCode, Severity,
};

/// Rule: Requests equal limits (Guaranteed QoS).
pub struct RequestsEqualLimitsRule;

impl OptimizationRule for RequestsEqualLimitsRule {
    fn code(&self) -> &'static str {
        codes::REQUESTS_EQUAL_LIMITS
    }

    fn description(&self) -> &'static str {
        "Requests equal limits (no bursting allowed)"
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(
        &self,
        ctx: &RuleContext,
        config: &K8sOptimizeConfig,
    ) -> Option<ResourceRecommendation> {
        // Only report if include_info is set
        if !config.include_info {
            return None;
        }

        // Must have both requests and limits
        if !ctx.current.has_requests() || !ctx.current.has_limits() {
            return None;
        }

        // Check if requests equal limits
        let cpu_equal = ctx.current.cpu_request == ctx.current.cpu_limit;
        let memory_equal = ctx.current.memory_request == ctx.current.memory_limit;

        if !cpu_equal || !memory_equal {
            return None;
        }

        // Keep as-is - this is just informational
        Some(ResourceRecommendation {
            resource_kind: ctx.resource_kind.clone(),
            resource_name: ctx.resource_name.clone(),
            namespace: ctx.namespace.clone(),
            container: ctx.container_name.clone(),
            file_path: ctx.file_path.clone(),
            line: ctx.line,
            issue: OptimizationIssue::UnbalancedResources,
            severity: self.default_severity(),
            message: "Requests equal limits. This creates a Guaranteed QoS class, which is good for stability but prevents bursting.".to_string(),
            workload_type: ctx.workload_type,
            current: ctx.current.clone(),
            actual_usage: None,
            recommended: ctx.current.clone(), // Keep as-is
            savings: None,
            fix_yaml: ctx.current.to_yaml(),
            rule_code: RuleCode::new(self.code()),
        })
    }
}
