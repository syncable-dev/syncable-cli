//! Liveness probe detection template.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting containers without liveness probes.
pub struct LivenessProbeTemplate;

impl Template for LivenessProbeTemplate {
    fn key(&self) -> &str {
        "liveness-probe"
    }

    fn human_name(&self) -> &str {
        "Liveness Probe"
    }

    fn description(&self) -> &str {
        "Detects containers without a liveness probe"
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
        Ok(Box::new(LivenessProbeCheck))
    }
}

struct LivenessProbeCheck;

impl CheckFunc for LivenessProbeCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            // Only check regular containers, not init containers
            for container in extract::container::containers(pod_spec) {
                if container.liveness_probe.is_none() {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Container '{}' does not have a liveness probe",
                            container.name
                        ),
                        remediation: Some(
                            "Add a livenessProbe to detect when the container becomes unresponsive."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}
