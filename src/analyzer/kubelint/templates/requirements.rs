//! CPU and memory requirements detection templates.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting containers without CPU requirements.
pub struct CpuRequirementsTemplate;

impl Template for CpuRequirementsTemplate {
    fn key(&self) -> &str {
        "cpu-requirements"
    }

    fn human_name(&self) -> &str {
        "CPU Requirements"
    }

    fn description(&self) -> &str {
        "Detects containers without CPU requests or limits"
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
        Ok(Box::new(CpuRequirementsCheck {
            require_limits: false,
        }))
    }
}

struct CpuRequirementsCheck {
    require_limits: bool,
}

impl CheckFunc for CpuRequirementsCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                let has_cpu_request = container
                    .resources
                    .as_ref()
                    .and_then(|r| r.requests.as_ref())
                    .map(|r| r.contains_key("cpu"))
                    .unwrap_or(false);

                let has_cpu_limit = container
                    .resources
                    .as_ref()
                    .and_then(|r| r.limits.as_ref())
                    .map(|r| r.contains_key("cpu"))
                    .unwrap_or(false);

                if !has_cpu_request {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Container '{}' does not have a CPU request",
                            container.name
                        ),
                        remediation: Some(
                            "Set resources.requests.cpu for proper scheduling.".to_string(),
                        ),
                    });
                }

                if self.require_limits && !has_cpu_limit {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Container '{}' does not have a CPU limit",
                            container.name
                        ),
                        remediation: Some(
                            "Set resources.limits.cpu to prevent resource exhaustion.".to_string(),
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}

/// Template for detecting containers without memory requirements.
pub struct MemoryRequirementsTemplate;

impl Template for MemoryRequirementsTemplate {
    fn key(&self) -> &str {
        "memory-requirements"
    }

    fn human_name(&self) -> &str {
        "Memory Requirements"
    }

    fn description(&self) -> &str {
        "Detects containers without memory requests or limits"
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
        Ok(Box::new(MemoryRequirementsCheck {
            require_limits: false,
        }))
    }
}

struct MemoryRequirementsCheck {
    require_limits: bool,
}

impl CheckFunc for MemoryRequirementsCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                let has_memory_request = container
                    .resources
                    .as_ref()
                    .and_then(|r| r.requests.as_ref())
                    .map(|r| r.contains_key("memory"))
                    .unwrap_or(false);

                let has_memory_limit = container
                    .resources
                    .as_ref()
                    .and_then(|r| r.limits.as_ref())
                    .map(|r| r.contains_key("memory"))
                    .unwrap_or(false);

                if !has_memory_request {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Container '{}' does not have a memory request",
                            container.name
                        ),
                        remediation: Some(
                            "Set resources.requests.memory for proper scheduling.".to_string(),
                        ),
                    });
                }

                if self.require_limits && !has_memory_limit {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Container '{}' does not have a memory limit",
                            container.name
                        ),
                        remediation: Some(
                            "Set resources.limits.memory to prevent OOM kills.".to_string(),
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}
