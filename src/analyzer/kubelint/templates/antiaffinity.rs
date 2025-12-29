//! Anti-affinity detection template.

use crate::analyzer::kubelint::context::object::K8sObject;
use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting deployments without pod anti-affinity.
pub struct AntiAffinityTemplate;

impl Template for AntiAffinityTemplate {
    fn key(&self) -> &str {
        "anti-affinity"
    }

    fn human_name(&self) -> &str {
        "Anti-Affinity"
    }

    fn description(&self) -> &str {
        "Detects deployments with multiple replicas but no pod anti-affinity"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::default()
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(AntiAffinityCheck { min_replicas: 2 }))
    }
}

struct AntiAffinityCheck {
    min_replicas: i32,
}

impl CheckFunc for AntiAffinityCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Get replica count (only applicable to certain object types)
        let replicas = match &object.k8s_object {
            K8sObject::Deployment(d) => d.replicas.unwrap_or(1),
            K8sObject::StatefulSet(d) => d.replicas.unwrap_or(1),
            K8sObject::ReplicaSet(d) => d.replicas.unwrap_or(1),
            _ => return diagnostics,
        };

        // Only check if replicas >= min_replicas
        if replicas < self.min_replicas {
            return diagnostics;
        }

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            let has_anti_affinity = pod_spec
                .affinity
                .as_ref()
                .and_then(|a| a.pod_anti_affinity.as_ref())
                .map(|paa| {
                    !paa.required_during_scheduling_ignored_during_execution.is_empty()
                        || !paa.preferred_during_scheduling_ignored_during_execution.is_empty()
                })
                .unwrap_or(false);

            if !has_anti_affinity {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "Object '{}' has {} replicas but no pod anti-affinity rules",
                        object.name(),
                        replicas
                    ),
                    remediation: Some(
                        "Add podAntiAffinity rules to spread replicas across nodes for high availability."
                            .to_string(),
                    ),
                });
            }
        }

        diagnostics
    }
}
