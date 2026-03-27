//! Linux capabilities detection templates.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting containers that don't drop NET_RAW capability.
pub struct DropNetRawCapabilityTemplate;

impl Template for DropNetRawCapabilityTemplate {
    fn key(&self) -> &str {
        "drop-net-raw-capability"
    }

    fn human_name(&self) -> &str {
        "Drop NET_RAW Capability"
    }

    fn description(&self) -> &str {
        "Detects containers that don't drop the NET_RAW capability"
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
        Ok(Box::new(DropNetRawCheck))
    }
}

struct DropNetRawCheck;

impl CheckFunc for DropNetRawCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                let drops_net_raw = container
                    .security_context
                    .as_ref()
                    .and_then(|sc| sc.capabilities.as_ref())
                    .map(|caps| {
                        caps.drop
                            .iter()
                            .any(|c| c == "NET_RAW" || c == "ALL" || c == "all")
                    })
                    .unwrap_or(false);

                if !drops_net_raw {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Container '{}' does not drop NET_RAW capability",
                            container.name
                        ),
                        remediation: Some(
                            "Add NET_RAW to securityContext.capabilities.drop, or drop ALL capabilities."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}
