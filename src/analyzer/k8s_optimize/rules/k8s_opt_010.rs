//! K8S-OPT-010: Unbalanced resource allocation for workload type.

use super::{OptimizationRule, RuleContext, codes};
use crate::analyzer::k8s_optimize::config::K8sOptimizeConfig;
use crate::analyzer::k8s_optimize::parser::{parse_cpu_to_millicores, parse_memory_to_bytes};
use crate::analyzer::k8s_optimize::types::{
    OptimizationIssue, ResourceRecommendation, ResourceSpec, RuleCode, Severity, WorkloadType,
};

/// Rule: Unbalanced resource allocation.
pub struct UnbalancedResourcesRule;

impl OptimizationRule for UnbalancedResourcesRule {
    fn code(&self) -> &'static str {
        codes::UNBALANCED_RESOURCES
    }

    fn description(&self) -> &'static str {
        "Resource allocation is unbalanced for workload type"
    }

    fn default_severity(&self) -> Severity {
        Severity::Low
    }

    fn check(
        &self,
        ctx: &RuleContext,
        config: &K8sOptimizeConfig,
    ) -> Option<ResourceRecommendation> {
        // Only report if include_info is set (this is a low-severity check)
        if !config.include_info {
            return None;
        }

        // Need both CPU and memory requests to check balance
        let cpu_request = ctx.current.cpu_request.as_ref()?;
        let memory_request = ctx.current.memory_request.as_ref()?;

        let cpu_millicores = parse_cpu_to_millicores(cpu_request)?;
        let memory_bytes = parse_memory_to_bytes(memory_request)?;

        // Calculate CPU to memory ratio (millicores per GB)
        let memory_gb = memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        if memory_gb < 0.1 {
            return None; // Too small to calculate meaningful ratio
        }

        let ratio = cpu_millicores as f64 / memory_gb;

        // Expected ratios vary by workload type
        let (expected_min, expected_max) = match ctx.workload_type {
            WorkloadType::Web => (200.0, 2000.0), // Web: 200m-2000m per GB
            WorkloadType::Worker => (500.0, 3000.0), // Worker: higher CPU
            WorkloadType::Database => (100.0, 1000.0), // DB: lower CPU per GB
            WorkloadType::Cache => (100.0, 500.0), // Cache: memory-heavy
            WorkloadType::MessageBroker => (200.0, 1000.0),
            WorkloadType::MachineLearning => (500.0, 4000.0), // ML: high CPU
            WorkloadType::Batch => (500.0, 4000.0),           // Batch: high CPU
            WorkloadType::General => (100.0, 2000.0),         // Wide range
        };

        // Check if ratio is within expected range
        if ratio >= expected_min && ratio <= expected_max {
            return None;
        }

        let defaults = ctx.workload_type.default_resources();
        let recommended = ResourceSpec {
            cpu_request: Some(defaults.cpu_request.to_string()),
            cpu_limit: Some(defaults.cpu_limit.to_string()),
            memory_request: Some(defaults.memory_request.to_string()),
            memory_limit: Some(defaults.memory_limit.to_string()),
        };

        let direction = if ratio < expected_min {
            "CPU-heavy for memory"
        } else {
            "Memory-heavy for CPU"
        };

        Some(ResourceRecommendation {
            resource_kind: ctx.resource_kind.clone(),
            resource_name: ctx.resource_name.clone(),
            namespace: ctx.namespace.clone(),
            container: ctx.container_name.clone(),
            file_path: ctx.file_path.clone(),
            line: ctx.line,
            issue: OptimizationIssue::UnbalancedResources,
            severity: self.default_severity(),
            message: format!(
                "Resource allocation is unbalanced for {} workload: {} (ratio: {:.0} millicores/GB, expected: {:.0}-{:.0}).",
                ctx.workload_type, direction, ratio, expected_min, expected_max
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
