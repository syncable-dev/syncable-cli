//! Unsafe proc mount detection template.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting unsafe /proc mount settings.
pub struct UnsafeProcMountTemplate;

impl Template for UnsafeProcMountTemplate {
    fn key(&self) -> &str {
        "unsafe-proc-mount"
    }

    fn human_name(&self) -> &str {
        "Unsafe Proc Mount"
    }

    fn description(&self) -> &str {
        "Detects containers with unsafe /proc mount (procMount: Unmasked)"
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
        Ok(Box::new(UnsafeProcMountCheck))
    }
}

struct UnsafeProcMountCheck;

impl CheckFunc for UnsafeProcMountCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                if let Some(sc) = &container.security_context
                    && let Some(proc_mount) = &sc.proc_mount
                        && proc_mount == "Unmasked" {
                            diagnostics.push(Diagnostic {
                                message: format!(
                                    "Container '{}' has unsafe /proc mount (procMount: Unmasked)",
                                    container.name
                                ),
                                remediation: Some(
                                    "Use the Default procMount type unless Unmasked is absolutely required. \
                                     Unmasked proc mount exposes sensitive kernel information."
                                        .to_string(),
                                ),
                            });
                        }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::kubelint::parser::yaml::parse_yaml;

    #[test]
    fn test_unsafe_proc_mount_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: unsafe-procmount
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
        securityContext:
          procMount: Unmasked
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = UnsafeProcMountCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Unmasked"));
    }

    #[test]
    fn test_default_proc_mount_ok() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: safe-procmount
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
        securityContext:
          procMount: Default
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = UnsafeProcMountCheck;
        let diagnostics = check.check(&objects[0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_no_proc_mount_ok() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: no-procmount
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = UnsafeProcMountCheck;
        let diagnostics = check.check(&objects[0]);
        assert!(diagnostics.is_empty());
    }
}
