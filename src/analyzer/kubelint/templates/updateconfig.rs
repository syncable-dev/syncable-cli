//! Update strategy detection templates.

use crate::analyzer::kubelint::context::object::K8sObject;
use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting deployments without rolling update strategy.
pub struct RollingUpdateStrategyTemplate;

impl Template for RollingUpdateStrategyTemplate {
    fn key(&self) -> &str {
        "rolling-update-strategy"
    }

    fn human_name(&self) -> &str {
        "Rolling Update Strategy"
    }

    fn description(&self) -> &str {
        "Detects deployments without a rolling update strategy configured"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["Deployment", "DaemonSet"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(RollingUpdateStrategyCheck))
    }
}

struct RollingUpdateStrategyCheck;

impl CheckFunc for RollingUpdateStrategyCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        match &object.k8s_object {
            K8sObject::Deployment(dep) => {
                let strategy = dep.strategy.as_ref();
                let strategy_type = strategy.and_then(|s| s.type_.as_deref());

                // Check if strategy is not RollingUpdate (or unset - defaults to RollingUpdate)
                if strategy_type == Some("Recreate") {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Deployment '{}' uses Recreate strategy instead of RollingUpdate",
                            object.name()
                        ),
                        remediation: Some(
                            "Consider using RollingUpdate strategy for zero-downtime deployments."
                                .to_string(),
                        ),
                    });
                }

                // Check if RollingUpdate but no parameters configured
                if strategy_type.is_none() || strategy_type == Some("RollingUpdate") {
                    let rolling_update = strategy.and_then(|s| s.rolling_update.as_ref());
                    if rolling_update.is_none() {
                        diagnostics.push(Diagnostic {
                            message: format!(
                                "Deployment '{}' has no explicit rolling update configuration",
                                object.name()
                            ),
                            remediation: Some(
                                "Configure strategy.rollingUpdate.maxSurge and maxUnavailable \
                                 for controlled rollouts.".to_string()
                            ),
                        });
                    }
                }
            }
            K8sObject::DaemonSet(ds) => {
                let strategy_type = ds.update_strategy.as_ref().and_then(|s| s.type_.as_deref());

                if strategy_type == Some("OnDelete") {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "DaemonSet '{}' uses OnDelete strategy instead of RollingUpdate",
                            object.name()
                        ),
                        remediation: Some(
                            "Consider using RollingUpdate strategy for automatic updates."
                                .to_string(),
                        ),
                    });
                }
            }
            _ => {}
        }

        diagnostics
    }
}
