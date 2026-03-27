//! K8S-OPT-006: Memory request exceeds threshold for workload type.

use super::{OptimizationRule, RuleContext, codes};
use crate::analyzer::k8s_optimize::config::K8sOptimizeConfig;
use crate::analyzer::k8s_optimize::parser::parse_memory_to_bytes;
use crate::analyzer::k8s_optimize::types::{
    OptimizationIssue, ResourceRecommendation, ResourceSpec, RuleCode, Severity, WorkloadType,
};

/// Rule: Memory request exceeds threshold.
pub struct HighMemoryRequestRule;

impl OptimizationRule for HighMemoryRequestRule {
    fn code(&self) -> &'static str {
        codes::HIGH_MEMORY_REQUEST
    }

    fn description(&self) -> &'static str {
        "Memory request exceeds threshold for workload type"
    }

    fn default_severity(&self) -> Severity {
        Severity::High
    }

    fn check(
        &self,
        ctx: &RuleContext,
        config: &K8sOptimizeConfig,
    ) -> Option<ResourceRecommendation> {
        // Exclude database/ML workloads from this check (they legitimately need more memory)
        if matches!(
            ctx.workload_type,
            WorkloadType::Database | WorkloadType::MachineLearning
        ) {
            return None;
        }

        let memory_request = ctx.current.memory_request.as_ref()?;
        let bytes = parse_memory_to_bytes(memory_request)?;
        let mi = bytes / (1024 * 1024);

        // Check if exceeds threshold
        if mi <= config.max_memory_request_mi as u64 {
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
            issue: OptimizationIssue::OverProvisioned,
            severity: self.default_severity(),
            message: format!(
                "Memory request ({}) exceeds {}Mi threshold for {} workload. This is likely over-provisioned.",
                memory_request, config.max_memory_request_mi, ctx.workload_type
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
