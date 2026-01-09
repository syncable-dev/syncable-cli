//! K8S-OPT-003: No CPU limit defined.

use super::{OptimizationRule, RuleContext, codes};
use crate::analyzer::k8s_optimize::config::K8sOptimizeConfig;
use crate::analyzer::k8s_optimize::parser::parse_cpu_to_millicores;
use crate::analyzer::k8s_optimize::types::{
    OptimizationIssue, ResourceRecommendation, ResourceSpec, RuleCode, Severity,
};

/// Rule: No CPU limit defined.
pub struct NoCpuLimitRule;

impl OptimizationRule for NoCpuLimitRule {
    fn code(&self) -> &'static str {
        codes::NO_CPU_LIMIT
    }

    fn description(&self) -> &'static str {
        "No CPU limit defined"
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(
        &self,
        ctx: &RuleContext,
        config: &K8sOptimizeConfig,
    ) -> Option<ResourceRecommendation> {
        // Only report if include_info is set (CPU limits are optional)
        if !config.include_info {
            return None;
        }

        // Skip if CPU limit is defined
        if ctx.current.cpu_limit.is_some() {
            return None;
        }

        let defaults = ctx.workload_type.default_resources();

        // Calculate CPU limit based on request if available
        let cpu_limit = if let Some(ref cpu_request) = ctx.current.cpu_request {
            if let Some(millicores) = parse_cpu_to_millicores(cpu_request) {
                let limit_millicores = millicores * defaults.typical_cpu_ratio as u64;
                crate::analyzer::k8s_optimize::parser::millicores_to_cpu_string(limit_millicores)
            } else {
                defaults.cpu_limit.to_string()
            }
        } else {
            defaults.cpu_limit.to_string()
        };

        let recommended = ResourceSpec {
            cpu_request: ctx.current.cpu_request.clone(),
            cpu_limit: Some(cpu_limit),
            memory_request: ctx.current.memory_request.clone(),
            memory_limit: ctx.current.memory_limit.clone(),
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
            message: "No CPU limit defined. Consider adding one if you want to prevent CPU starvation on the node.".to_string(),
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
