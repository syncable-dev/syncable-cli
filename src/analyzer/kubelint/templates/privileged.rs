//! Privileged container detection template.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting privileged containers.
pub struct PrivilegedTemplate;

impl Template for PrivilegedTemplate {
    fn key(&self) -> &str {
        "privileged"
    }

    fn human_name(&self) -> &str {
        "Privileged Container"
    }

    fn description(&self) -> &str {
        "Detects containers running in privileged mode"
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
        Ok(Box::new(PrivilegedCheck))
    }
}

struct PrivilegedCheck;

impl CheckFunc for PrivilegedCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                if let Some(sc) = &container.security_context {
                    if sc.privileged == Some(true) {
                        diagnostics.push(Diagnostic {
                            message: format!(
                                "Container '{}' is running in privileged mode",
                                container.name
                            ),
                            remediation: Some(
                                "Do not run containers in privileged mode unless absolutely necessary. \
                                 Set securityContext.privileged to false.".to_string()
                            ),
                        });
                    }
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
    fn test_privileged_container_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: privileged-deploy
spec:
  template:
    spec:
      containers:
      - name: privileged-container
        image: nginx
        securityContext:
          privileged: true
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = PrivilegedCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("privileged mode"));
    }

    #[test]
    fn test_non_privileged_container_ok() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: safe-deploy
spec:
  template:
    spec:
      containers:
      - name: safe-container
        image: nginx
        securityContext:
          privileged: false
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = PrivilegedCheck;
        let diagnostics = check.check(&objects[0]);
        assert!(diagnostics.is_empty());
    }
}
