//! Privilege escalation detection template.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting privilege escalation.
pub struct PrivilegeEscalationTemplate;

impl Template for PrivilegeEscalationTemplate {
    fn key(&self) -> &str {
        "privilege-escalation"
    }

    fn human_name(&self) -> &str {
        "Privilege Escalation"
    }

    fn description(&self) -> &str {
        "Detects containers that allow privilege escalation"
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
        Ok(Box::new(PrivilegeEscalationCheck))
    }
}

struct PrivilegeEscalationCheck;

impl CheckFunc for PrivilegeEscalationCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                // Check if allowPrivilegeEscalation is explicitly set to true
                // or if it's not set at all (default is true in Kubernetes)
                let allows_escalation = container
                    .security_context
                    .as_ref()
                    .map(|sc| sc.allow_privilege_escalation != Some(false))
                    .unwrap_or(true);

                if allows_escalation {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Container '{}' allows privilege escalation",
                            container.name
                        ),
                        remediation: Some(
                            "Set securityContext.allowPrivilegeEscalation to false.".to_string()
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}
