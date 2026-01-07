//! K8S-OPT-007: CPU limit to request ratio is excessive.

use super::{OptimizationRule, RuleContext, codes};
use crate::analyzer::k8s_optimize::config::K8sOptimizeConfig;
use crate::analyzer::k8s_optimize::parser::{
    cpu_limit_to_request_ratio, millicores_to_cpu_string, parse_cpu_to_millicores,
};
use crate::analyzer::k8s_optimize::types::{
    OptimizationIssue, ResourceRecommendation, ResourceSpec, RuleCode, Severity,
};

/// Rule: Excessive CPU limit to request ratio.
pub struct ExcessiveCpuRatioRule;

impl OptimizationRule for ExcessiveCpuRatioRule {
    fn code(&self) -> &'static str {
        codes::EXCESSIVE_CPU_RATIO
    }

    fn description(&self) -> &'static str {
        "CPU limit to request ratio is excessive"
    }

    fn default_severity(&self) -> Severity {
        Severity::Medium
    }

    fn check(
        &self,
        ctx: &RuleContext,
        config: &K8sOptimizeConfig,
    ) -> Option<ResourceRecommendation> {
        let ratio = cpu_limit_to_request_ratio(&ctx.current)?;

        // Check if exceeds threshold
        if ratio <= config.max_cpu_limit_ratio as f64 {
            return None;
        }

        // Calculate balanced CPU limit
        let cpu_limit = if let Some(ref cpu_request) = ctx.current.cpu_request {
            if let Some(millicores) = parse_cpu_to_millicores(cpu_request) {
                let limit_millicores = millicores * config.max_cpu_limit_ratio as u64;
                millicores_to_cpu_string(limit_millicores)
            } else {
                ctx.current.cpu_limit.clone().unwrap_or_default()
            }
        } else {
            ctx.current.cpu_limit.clone().unwrap_or_default()
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
            issue: OptimizationIssue::ExcessiveRatio,
            severity: self.default_severity(),
            message: format!(
                "CPU limit to request ratio is {:.1}x (threshold: {}x). Large ratios can indicate over-provisioned limits.",
                ratio, config.max_cpu_limit_ratio
            ),
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
