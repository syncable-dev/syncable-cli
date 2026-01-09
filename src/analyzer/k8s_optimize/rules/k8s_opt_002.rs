//! K8S-OPT-002: No memory request defined.

use super::{OptimizationRule, RuleContext, codes};
use crate::analyzer::k8s_optimize::config::K8sOptimizeConfig;
use crate::analyzer::k8s_optimize::types::{
    OptimizationIssue, ResourceRecommendation, ResourceSpec, RuleCode, Severity,
};

/// Rule: No memory request defined.
pub struct NoMemoryRequestRule;

impl OptimizationRule for NoMemoryRequestRule {
    fn code(&self) -> &'static str {
        codes::NO_MEMORY_REQUEST
    }

    fn description(&self) -> &'static str {
        "No memory request defined"
    }

    fn default_severity(&self) -> Severity {
        Severity::High
    }

    fn check(
        &self,
        ctx: &RuleContext,
        _config: &K8sOptimizeConfig,
    ) -> Option<ResourceRecommendation> {
        // Skip if memory request is defined
        if ctx.current.memory_request.is_some() {
            return None;
        }

        // Skip if no resources at all (handled separately as critical)
        if !ctx.current.has_any() {
            return None;
        }

        let defaults = ctx.workload_type.default_resources();
        let recommended = ResourceSpec {
            cpu_request: None,
            cpu_limit: None,
            memory_request: Some(defaults.memory_request.to_string()),
            memory_limit: Some(defaults.memory_limit.to_string()),
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
            message: "No memory request defined. This can lead to OOM kills and node pressure."
                .to_string(),
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
