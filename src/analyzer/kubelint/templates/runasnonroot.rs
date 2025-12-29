//! Run as non-root detection template.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting containers not running as non-root.
pub struct RunAsNonRootTemplate;

impl Template for RunAsNonRootTemplate {
    fn key(&self) -> &str {
        "run-as-non-root"
    }

    fn human_name(&self) -> &str {
        "Run As Non-Root"
    }

    fn description(&self) -> &str {
        "Detects containers not configured to run as non-root"
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
        Ok(Box::new(RunAsNonRootCheck))
    }
}

struct RunAsNonRootCheck;

impl CheckFunc for RunAsNonRootCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            // Check pod-level security context
            let pod_run_as_non_root = pod_spec
                .security_context
                .as_ref()
                .and_then(|sc| sc.run_as_non_root);

            for container in extract::container::all_containers(pod_spec) {
                // Container-level overrides pod-level
                let container_run_as_non_root = container
                    .security_context
                    .as_ref()
                    .and_then(|sc| sc.run_as_non_root);

                let effective_run_as_non_root = container_run_as_non_root.or(pod_run_as_non_root);

                if effective_run_as_non_root != Some(true) {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Container '{}' is not configured to run as non-root",
                            container.name
                        ),
                        remediation: Some(
                            "Set securityContext.runAsNonRoot to true at pod or container level."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}
