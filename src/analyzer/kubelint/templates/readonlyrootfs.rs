//! Read-only root filesystem detection template.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting containers without read-only root filesystem.
pub struct ReadOnlyRootFsTemplate;

impl Template for ReadOnlyRootFsTemplate {
    fn key(&self) -> &str {
        "read-only-root-fs"
    }

    fn human_name(&self) -> &str {
        "Read-Only Root Filesystem"
    }

    fn description(&self) -> &str {
        "Detects containers without a read-only root filesystem"
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
        Ok(Box::new(ReadOnlyRootFsCheck))
    }
}

struct ReadOnlyRootFsCheck;

impl CheckFunc for ReadOnlyRootFsCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                let read_only = container
                    .security_context
                    .as_ref()
                    .and_then(|sc| sc.read_only_root_filesystem);

                if read_only != Some(true) {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Container '{}' does not have a read-only root filesystem",
                            container.name
                        ),
                        remediation: Some(
                            "Set securityContext.readOnlyRootFilesystem to true.".to_string()
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}
