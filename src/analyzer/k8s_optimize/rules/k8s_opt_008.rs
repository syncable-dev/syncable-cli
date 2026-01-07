//! K8S-OPT-008: Memory limit to request ratio is excessive.

use super::{OptimizationRule, RuleContext, codes};
use crate::analyzer::k8s_optimize::config::K8sOptimizeConfig;
use crate::analyzer::k8s_optimize::parser::{
    bytes_to_memory_string, memory_limit_to_request_ratio, parse_memory_to_bytes,
};
use crate::analyzer::k8s_optimize::types::{
    OptimizationIssue, ResourceRecommendation, ResourceSpec, RuleCode, Severity,
};

/// Rule: Excessive memory limit to request ratio.
pub struct ExcessiveMemoryRatioRule;

impl OptimizationRule for ExcessiveMemoryRatioRule {
    fn code(&self) -> &'static str {
        codes::EXCESSIVE_MEMORY_RATIO
    }

    fn description(&self) -> &'static str {
        "Memory limit to request ratio is excessive"
    }

    fn default_severity(&self) -> Severity {
        Severity::Medium
    }

    fn check(
        &self,
        ctx: &RuleContext,
        config: &K8sOptimizeConfig,
    ) -> Option<ResourceRecommendation> {
        let ratio = memory_limit_to_request_ratio(&ctx.current)?;

        // Check if exceeds threshold
        if ratio <= config.max_memory_limit_ratio as f64 {
            return None;
        }

        // Calculate balanced memory limit
        let memory_limit = if let Some(ref memory_request) = ctx.current.memory_request {
            if let Some(bytes) = parse_memory_to_bytes(memory_request) {
                let limit_bytes = (bytes as f64 * config.max_memory_limit_ratio as f64) as u64;
                bytes_to_memory_string(limit_bytes)
            } else {
                ctx.current.memory_limit.clone().unwrap_or_default()
            }
        } else {
            ctx.current.memory_limit.clone().unwrap_or_default()
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
            issue: OptimizationIssue::ExcessiveRatio,
            severity: self.default_severity(),
            message: format!(
                "Memory limit to request ratio is {:.1}x (threshold: {}x). Large ratios can lead to OOM kills under pressure.",
                ratio, config.max_memory_limit_ratio
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
