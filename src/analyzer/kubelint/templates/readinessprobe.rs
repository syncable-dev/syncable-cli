//! Readiness probe detection template.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting containers without readiness probes.
pub struct ReadinessProbeTemplate;

impl Template for ReadinessProbeTemplate {
    fn key(&self) -> &str {
        "readiness-probe"
    }

    fn human_name(&self) -> &str {
        "Readiness Probe"
    }

    fn description(&self) -> &str {
        "Detects containers without a readiness probe"
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
        Ok(Box::new(ReadinessProbeCheck))
    }
}

struct ReadinessProbeCheck;

impl CheckFunc for ReadinessProbeCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            // Only check regular containers, not init containers
            for container in extract::container::containers(pod_spec) {
                if container.readiness_probe.is_none() {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Container '{}' does not have a readiness probe",
                            container.name
                        ),
                        remediation: Some(
                            "Add a readinessProbe to control when the container receives traffic."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}
