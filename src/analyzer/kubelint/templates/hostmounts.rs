//! Host mount detection templates.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting host path mounts.
pub struct HostMountsTemplate;

impl Template for HostMountsTemplate {
    fn key(&self) -> &str {
        "host-mounts"
    }

    fn human_name(&self) -> &str {
        "Host Mounts"
    }

    fn description(&self) -> &str {
        "Detects containers with host path volume mounts"
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
        Ok(Box::new(HostMountsCheck))
    }
}

struct HostMountsCheck;

impl CheckFunc for HostMountsCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for volume in &pod_spec.volumes {
                if let Some(host_path) = &volume.host_path {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Volume '{}' mounts host path '{}'",
                            volume.name, host_path.path
                        ),
                        remediation: Some(
                            "Avoid using hostPath volumes as they provide access to the host filesystem. \
                             Use PersistentVolumeClaims or ConfigMaps instead.".to_string()
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}

/// Template for detecting writable host path mounts.
pub struct WritableHostMountTemplate;

impl Template for WritableHostMountTemplate {
    fn key(&self) -> &str {
        "writable-host-mount"
    }

    fn human_name(&self) -> &str {
        "Writable Host Mount"
    }

    fn description(&self) -> &str {
        "Detects containers with writable host path volume mounts"
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
        Ok(Box::new(WritableHostMountCheck))
    }
}

struct WritableHostMountCheck;

impl CheckFunc for WritableHostMountCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            // Find host path volumes
            let host_volumes: std::collections::HashSet<_> = pod_spec
                .volumes
                .iter()
                .filter(|v| v.host_path.is_some())
                .map(|v| v.name.as_str())
                .collect();

            // Check each container's volume mounts
            for container in extract::container::all_containers(pod_spec) {
                for mount in &container.volume_mounts {
                    if host_volumes.contains(mount.name.as_str()) {
                        // Default is writable (readOnly: false)
                        let is_writable = mount.read_only != Some(true);

                        if is_writable {
                            diagnostics.push(Diagnostic {
                                message: format!(
                                    "Container '{}' has writable host mount at '{}'",
                                    container.name, mount.mount_path
                                ),
                                remediation: Some(
                                    "Set volumeMounts.readOnly to true for host path mounts, \
                                     or avoid using hostPath volumes entirely."
                                        .to_string(),
                                ),
                            });
                        }
                    }
                }
            }
        }

        diagnostics
    }
}
