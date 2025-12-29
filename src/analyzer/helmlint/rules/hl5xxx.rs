//! HL5xxx - Best Practice Rules
//!
//! Rules for validating Kubernetes best practices in Helm charts.

use crate::analyzer::helmlint::parser::template::TemplateToken;
use crate::analyzer::helmlint::rules::{LintContext, Rule};
use crate::analyzer::helmlint::types::{CheckFailure, RuleCategory, Severity};

/// Get all HL5xxx rules.
pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(HL5001),
        Box::new(HL5002),
        Box::new(HL5003),
        Box::new(HL5004),
        Box::new(HL5005),
        Box::new(HL5006),
    ]
}

/// HL5001: Missing resource limits
pub struct HL5001;

impl Rule for HL5001 {
    fn code(&self) -> &'static str {
        "HL5001"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "missing-resource-limits"
    }

    fn description(&self) -> &'static str {
        "Container should have resource limits defined"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        // Check values.yaml for resource limits
        if let Some(values) = ctx.values {
            let has_limits = values
                .defined_paths
                .iter()
                .any(|p| p.contains("resources.limits") || p.ends_with(".limits"));

            if !has_limits {
                failures.push(CheckFailure::new(
                    "HL5001",
                    Severity::Warning,
                    "No resource limits found in values.yaml. Define resources.limits for predictable resource usage",
                    "values.yaml",
                    1,
                    RuleCategory::BestPractice,
                ));
            }
        }

        failures
    }
}

/// HL5002: Missing resource requests
pub struct HL5002;

impl Rule for HL5002 {
    fn code(&self) -> &'static str {
        "HL5002"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "missing-resource-requests"
    }

    fn description(&self) -> &'static str {
        "Container should have resource requests defined"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        if let Some(values) = ctx.values {
            let has_requests = values
                .defined_paths
                .iter()
                .any(|p| p.contains("resources.requests") || p.ends_with(".requests"));

            if !has_requests {
                failures.push(CheckFailure::new(
                    "HL5002",
                    Severity::Warning,
                    "No resource requests found in values.yaml. Define resources.requests for proper scheduling",
                    "values.yaml",
                    1,
                    RuleCategory::BestPractice,
                ));
            }
        }

        failures
    }
}

/// HL5003: Missing liveness probe
pub struct HL5003;

impl Rule for HL5003 {
    fn code(&self) -> &'static str {
        "HL5003"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn name(&self) -> &'static str {
        "missing-liveness-probe"
    }

    fn description(&self) -> &'static str {
        "Container should have a liveness probe for health checking"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        // Check if any template has livenessProbe
        let has_liveness_in_template = ctx.templates.iter().any(|t| {
            t.tokens.iter().any(|token| match token {
                TemplateToken::Text { content, .. } => content.contains("livenessProbe"),
                TemplateToken::Action { content, .. } => content.contains("livenessProbe"),
                _ => false,
            })
        });

        // Check values.yaml
        let has_liveness_in_values = ctx
            .values
            .map(|v| {
                v.defined_paths
                    .iter()
                    .any(|p| p.to_lowercase().contains("livenessprobe"))
            })
            .unwrap_or(false);

        if !has_liveness_in_template && !has_liveness_in_values {
            failures.push(CheckFailure::new(
                "HL5003",
                Severity::Info,
                "No livenessProbe found. Consider adding a liveness probe for container health monitoring",
                "templates/",
                1,
                RuleCategory::BestPractice,
            ));
        }

        failures
    }
}

/// HL5004: Missing readiness probe
pub struct HL5004;

impl Rule for HL5004 {
    fn code(&self) -> &'static str {
        "HL5004"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn name(&self) -> &'static str {
        "missing-readiness-probe"
    }

    fn description(&self) -> &'static str {
        "Container should have a readiness probe for traffic management"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        let has_readiness_in_template = ctx.templates.iter().any(|t| {
            t.tokens.iter().any(|token| match token {
                TemplateToken::Text { content, .. } => content.contains("readinessProbe"),
                TemplateToken::Action { content, .. } => content.contains("readinessProbe"),
                _ => false,
            })
        });

        let has_readiness_in_values = ctx
            .values
            .map(|v| {
                v.defined_paths
                    .iter()
                    .any(|p| p.to_lowercase().contains("readinessprobe"))
            })
            .unwrap_or(false);

        if !has_readiness_in_template && !has_readiness_in_values {
            failures.push(CheckFailure::new(
                "HL5004",
                Severity::Info,
                "No readinessProbe found. Consider adding a readiness probe for proper load balancing",
                "templates/",
                1,
                RuleCategory::BestPractice,
            ));
        }

        failures
    }
}

/// HL5005: Using deprecated Kubernetes API
pub struct HL5005;

impl Rule for HL5005 {
    fn code(&self) -> &'static str {
        "HL5005"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "deprecated-api"
    }

    fn description(&self) -> &'static str {
        "Template uses deprecated Kubernetes API version"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        // Deprecated APIs and their replacements
        let deprecated_apis = [
            (
                "extensions/v1beta1",
                "apps/v1",
                "Deployment, DaemonSet, ReplicaSet",
            ),
            ("apps/v1beta1", "apps/v1", "Deployment, StatefulSet"),
            (
                "apps/v1beta2",
                "apps/v1",
                "Deployment, StatefulSet, DaemonSet, ReplicaSet",
            ),
            (
                "networking.k8s.io/v1beta1",
                "networking.k8s.io/v1",
                "Ingress",
            ),
            (
                "rbac.authorization.k8s.io/v1beta1",
                "rbac.authorization.k8s.io/v1",
                "Role, ClusterRole, RoleBinding",
            ),
            (
                "admissionregistration.k8s.io/v1beta1",
                "admissionregistration.k8s.io/v1",
                "MutatingWebhookConfiguration, ValidatingWebhookConfiguration",
            ),
            (
                "apiextensions.k8s.io/v1beta1",
                "apiextensions.k8s.io/v1",
                "CustomResourceDefinition",
            ),
            ("policy/v1beta1", "policy/v1", "PodDisruptionBudget"),
            ("batch/v1beta1", "batch/v1", "CronJob"),
        ];

        for template in ctx.templates {
            for token in &template.tokens {
                if let TemplateToken::Text { content, line } = token {
                    for (deprecated, replacement, resources) in &deprecated_apis {
                        if content.contains(&format!("apiVersion: {}", deprecated)) {
                            failures.push(CheckFailure::new(
                                "HL5005",
                                Severity::Error,
                                format!(
                                    "Deprecated API '{}' for {}. Use '{}' instead",
                                    deprecated, resources, replacement
                                ),
                                &template.path,
                                *line,
                                RuleCategory::BestPractice,
                            ));
                        }
                    }
                }
            }
        }

        failures
    }
}

/// HL5006: Labels missing recommended keys
pub struct HL5006;

impl Rule for HL5006 {
    fn code(&self) -> &'static str {
        "HL5006"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn name(&self) -> &'static str {
        "missing-recommended-labels"
    }

    fn description(&self) -> &'static str {
        "Resources should have recommended Kubernetes labels"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        // Recommended labels per Kubernetes best practices
        let recommended_labels = [
            "app.kubernetes.io/name",
            "app.kubernetes.io/instance",
            "app.kubernetes.io/version",
            "app.kubernetes.io/component",
            "app.kubernetes.io/part-of",
            "app.kubernetes.io/managed-by",
        ];

        // Check if helpers define standard labels
        let has_labels_helper = ctx
            .helpers
            .map(|h| {
                h.helpers.iter().any(|helper| {
                    helper.name.contains("labels") || helper.name.contains("selectorLabels")
                })
            })
            .unwrap_or(false);

        if !has_labels_helper {
            // Check templates for any recommended labels
            let has_recommended_labels = ctx.templates.iter().any(|t| {
                t.tokens.iter().any(|token| match token {
                    TemplateToken::Text { content, .. } => {
                        recommended_labels.iter().any(|l| content.contains(l))
                    }
                    _ => false,
                })
            });

            if !has_recommended_labels {
                failures.push(CheckFailure::new(
                    "HL5006",
                    Severity::Info,
                    "No recommended Kubernetes labels found. Consider adding app.kubernetes.io/* labels",
                    "templates/_helpers.tpl",
                    1,
                    RuleCategory::BestPractice,
                ));
            }
        }

        failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rules_exist() {
        let all_rules = rules();
        assert!(!all_rules.is_empty());
    }

    #[test]
    fn test_deprecated_api_list() {
        // Verify our deprecated API list is reasonable
        let deprecated_apis = [
            "extensions/v1beta1",
            "apps/v1beta1",
            "apps/v1beta2",
            "networking.k8s.io/v1beta1",
        ];

        for api in &deprecated_apis {
            assert!(api.contains("beta") || api.contains("v1beta"));
        }
    }
}
