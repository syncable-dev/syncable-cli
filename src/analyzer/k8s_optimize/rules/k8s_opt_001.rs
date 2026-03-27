//! K8S-OPT-001: No CPU request defined.

use super::{OptimizationRule, RuleContext, codes};
use crate::analyzer::k8s_optimize::config::K8sOptimizeConfig;
use crate::analyzer::k8s_optimize::types::{
    OptimizationIssue, ResourceRecommendation, ResourceSpec, RuleCode, Severity,
};

/// Rule: No CPU request defined.
pub struct NoCpuRequestRule;

impl OptimizationRule for NoCpuRequestRule {
    fn code(&self) -> &'static str {
        codes::NO_CPU_REQUEST
    }

    fn description(&self) -> &'static str {
        "No CPU request defined"
    }

    fn default_severity(&self) -> Severity {
        Severity::High
    }

    fn check(
        &self,
        ctx: &RuleContext,
        _config: &K8sOptimizeConfig,
    ) -> Option<ResourceRecommendation> {
        // Skip if CPU request is defined
        if ctx.current.cpu_request.is_some() {
            return None;
        }

        // Skip if no resources at all (handled separately as critical)
        if !ctx.current.has_any() {
            return None;
        }

        let defaults = ctx.workload_type.default_resources();
        let recommended = ResourceSpec {
            cpu_request: Some(defaults.cpu_request.to_string()),
            cpu_limit: Some(defaults.cpu_limit.to_string()),
            memory_request: None,
            memory_limit: None,
        };

        Some(ResourceRecommendation {
            resource_kind: ctx.resource_kind.clone(),
            resource_name: ctx.resource_name.clone(),
            namespace: ctx.namespace.clone(),
            container: ctx.container_name.clone(),
            file_path: ctx.file_path.clone(),
            line: ctx.line,
            issue: OptimizationIssue::NoRequestsDefined,
            severity: self.default_severity(),
            message: "No CPU request defined. This can lead to resource contention and unpredictable scheduling.".to_string(),
            workload_type: ctx.workload_type,
            current: ctx.current.clone(),
            actual_usage: None,
            recommended: recommended.clone(),
            savings: None,
            fix_yaml: recommended.to_yaml(),
            rule_code: RuleCode::new(self.code()),
        })
    }
}
