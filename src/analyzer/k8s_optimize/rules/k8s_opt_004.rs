//! K8S-OPT-004: No memory limit defined.

use super::{OptimizationRule, RuleContext, codes};
use crate::analyzer::k8s_optimize::config::K8sOptimizeConfig;
use crate::analyzer::k8s_optimize::parser::parse_memory_to_bytes;
use crate::analyzer::k8s_optimize::types::{
    OptimizationIssue, ResourceRecommendation, ResourceSpec, RuleCode, Severity,
};

/// Rule: No memory limit defined.
pub struct NoMemoryLimitRule;

impl OptimizationRule for NoMemoryLimitRule {
    fn code(&self) -> &'static str {
        codes::NO_MEMORY_LIMIT
    }

    fn description(&self) -> &'static str {
        "No memory limit defined"
    }

    fn default_severity(&self) -> Severity {
        Severity::Medium
    }

    fn check(
        &self,
        ctx: &RuleContext,
        _config: &K8sOptimizeConfig,
    ) -> Option<ResourceRecommendation> {
        // Skip if memory limit is defined
        if ctx.current.memory_limit.is_some() {
            return None;
        }

        let defaults = ctx.workload_type.default_resources();

        // Calculate memory limit based on request if available
        let memory_limit = if let Some(ref memory_request) = ctx.current.memory_request {
            if let Some(bytes) = parse_memory_to_bytes(memory_request) {
                let limit_bytes = (bytes as f64 * defaults.typical_memory_ratio) as u64;
                crate::analyzer::k8s_optimize::parser::bytes_to_memory_string(limit_bytes)
            } else {
                defaults.memory_limit.to_string()
            }
        } else {
            defaults.memory_limit.to_string()
        };

        let recommended = ResourceSpec {
            cpu_request: ctx.current.cpu_request.clone(),
            cpu_limit: ctx.current.cpu_limit.clone(),
            memory_request: ctx.current.memory_request.clone(),
            memory_limit: Some(memory_limit),
        };

        Some(ResourceRecommendation {
            resource_kind: ctx.resource_kind.clone(),
            resource_name: ctx.resource_name.clone(),
            namespace: ctx.namespace.clone(),
            container: ctx.container_name.clone(),
            file_path: ctx.file_path.clone(),
            line: ctx.line,
            issue: OptimizationIssue::NoLimitsDefined,
            severity: self.default_severity(),
            message:
                "No memory limit defined. Runaway memory usage can affect other pods on the node."
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
