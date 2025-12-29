//! Replica count check templates.

use crate::analyzer::kubelint::context::{Object, K8sObject};
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for checking minimum replica count.
pub struct ReplicasTemplate;

impl Template for ReplicasTemplate {
    fn key(&self) -> &str {
        "replicas"
    }

    fn human_name(&self) -> &str {
        "Minimum Replicas"
    }

    fn description(&self) -> &str {
        "Checks that deployments have at least a minimum number of replicas"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["Deployment", "StatefulSet"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        vec![ParameterDesc {
            name: "minReplicas".to_string(),
            description: "Minimum required replicas".to_string(),
            param_type: "integer".to_string(),
            required: false,
            default: Some(serde_yaml::Value::Number(2.into())),
        }]
    }

    fn instantiate(
        &self,
        params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        let min_replicas = params
            .get("minReplicas")
            .and_then(|v| v.as_i64())
            .unwrap_or(2) as i32;
        Ok(Box::new(ReplicasCheck { min_replicas }))
    }
}

struct ReplicasCheck {
    min_replicas: i32,
}

impl CheckFunc for ReplicasCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let replicas = match &object.k8s_object {
            K8sObject::Deployment(d) => d.replicas,
            K8sObject::StatefulSet(s) => s.replicas,
            _ => None,
        };

        if let Some(replica_count) = replicas {
            if replica_count < self.min_replicas {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "Object has only {} replicas, but minimum recommended is {}",
                        replica_count, self.min_replicas
                    ),
                    remediation: Some(format!(
                        "Increase replicas to at least {} for better availability.",
                        self.min_replicas
                    )),
                });
            }
        } else {
            // No replicas specified - defaults to 1
            if self.min_replicas > 1 {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "No replica count specified (defaults to 1), but minimum recommended is {}",
                        self.min_replicas
                    ),
                    remediation: Some(format!(
                        "Explicitly set replicas to at least {}.",
                        self.min_replicas
                    )),
                });
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
    fn test_low_replicas_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: single-replica
spec:
  replicas: 1
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = ReplicasCheck { min_replicas: 2 };
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("only 1 replicas"));
    }

    #[test]
    fn test_adequate_replicas_ok() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: multi-replica
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = ReplicasCheck { min_replicas: 2 };
        let diagnostics = check.check(&objects[0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_no_replicas_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: no-replica-spec
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = ReplicasCheck { min_replicas: 2 };
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("defaults to 1"));
    }
}
