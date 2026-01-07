//! K8S-OPT-005: CPU request exceeds threshold for workload type.

use super::{OptimizationRule, RuleContext, codes};
use crate::analyzer::k8s_optimize::config::K8sOptimizeConfig;
use crate::analyzer::k8s_optimize::parser::parse_cpu_to_millicores;
use crate::analyzer::k8s_optimize::types::{
    OptimizationIssue, ResourceRecommendation, ResourceSpec, RuleCode, Severity, WorkloadType,
};

/// Rule: CPU request exceeds threshold.
pub struct HighCpuRequestRule;

impl OptimizationRule for HighCpuRequestRule {
    fn code(&self) -> &'static str {
        codes::HIGH_CPU_REQUEST
    }

    fn description(&self) -> &'static str {
        "CPU request exceeds threshold for workload type"
    }

    fn default_severity(&self) -> Severity {
        Severity::High
    }

    fn check(
        &self,
        ctx: &RuleContext,
        config: &K8sOptimizeConfig,
    ) -> Option<ResourceRecommendation> {
        // Exclude batch/ML workloads from this check (they legitimately need more resources)
        if matches!(
            ctx.workload_type,
            WorkloadType::Batch | WorkloadType::MachineLearning
        ) {
            return None;
        }

        let cpu_request = ctx.current.cpu_request.as_ref()?;
        let millicores = parse_cpu_to_millicores(cpu_request)?;

        // Check if exceeds threshold
        if millicores <= config.max_cpu_request_millicores as u64 {
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
            issue: OptimizationIssue::OverProvisioned,
            severity: self.default_severity(),
            message: format!(
                "CPU request ({}) exceeds {}m threshold for {} workload. This is likely over-provisioned.",
                cpu_request, config.max_cpu_request_millicores, ctx.workload_type
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
