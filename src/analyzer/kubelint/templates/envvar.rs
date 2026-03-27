//! Environment variable check templates.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::context::object::EnvVarSource;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};
use regex::Regex;

/// Template for detecting secrets in environment variable values.
pub struct EnvVarSecretTemplate;

impl Template for EnvVarSecretTemplate {
    fn key(&self) -> &str {
        "env-var-secret"
    }

    fn human_name(&self) -> &str {
        "Environment Variable Secret"
    }

    fn description(&self) -> &str {
        "Detects environment variables that may contain secrets"
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
        Ok(Box::new(EnvVarSecretCheck))
    }
}

struct EnvVarSecretCheck;

impl CheckFunc for EnvVarSecretCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Patterns for secret-looking env var names
        let secret_name_pattern =
            Regex::new(r"(?i)(password|secret|key|token|credential|api_key|apikey|auth)").unwrap();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                for env_var in &container.env {
                    // Check if the env var name suggests it contains a secret
                    if secret_name_pattern.is_match(&env_var.name) {
                        // Check if it has a hardcoded value (not from secret or configmap)
                        if env_var.value.is_some() && env_var.value_from.is_none() {
                            diagnostics.push(Diagnostic {
                                message: format!(
                                    "Container '{}' has environment variable '{}' that appears to \
                                     contain a secret as a plain value",
                                    container.name, env_var.name
                                ),
                                remediation: Some(
                                    "Use a Kubernetes Secret with secretKeyRef instead of \
                                     hardcoding sensitive values in environment variables."
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

/// Template for detecting reading secrets directly from environment variables.
pub struct ReadSecretFromEnvVarTemplate;

impl Template for ReadSecretFromEnvVarTemplate {
    fn key(&self) -> &str {
        "read-secret-from-env-var"
    }

    fn human_name(&self) -> &str {
        "Read Secret From Env Var"
    }

    fn description(&self) -> &str {
        "Detects when secrets are exposed through environment variables"
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
        Ok(Box::new(ReadSecretFromEnvVarCheck))
    }
}

struct ReadSecretFromEnvVarCheck;

impl CheckFunc for ReadSecretFromEnvVarCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                for env_var in &container.env {
                    // Check if the env var references a secret
                    if let Some(EnvVarSource::SecretKeyRef { .. }) = &env_var.value_from {
                        diagnostics.push(Diagnostic {
                            message: format!(
                                "Container '{}' reads secret into environment variable '{}'",
                                container.name, env_var.name
                            ),
                            remediation: Some(
                                "Consider mounting secrets as files instead of exposing \
                                 them as environment variables. Environment variables can \
                                 be logged or exposed through /proc."
                                    .to_string(),
                            ),
                        });
                    }
                }
            }
        }

        diagnostics
    }
}

/// Template for detecting duplicate environment variables.
pub struct DuplicateEnvVarTemplate;

impl Template for DuplicateEnvVarTemplate {
    fn key(&self) -> &str {
        "duplicate-env-var"
    }

    fn human_name(&self) -> &str {
        "Duplicate Environment Variable"
    }

    fn description(&self) -> &str {
        "Detects duplicate environment variable definitions"
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
        Ok(Box::new(DuplicateEnvVarCheck))
    }
}

struct DuplicateEnvVarCheck;

impl CheckFunc for DuplicateEnvVarCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        use std::collections::HashSet;
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                let mut seen: HashSet<&str> = HashSet::new();
                for env_var in &container.env {
                    if !seen.insert(&env_var.name) {
                        diagnostics.push(Diagnostic {
                            message: format!(
                                "Container '{}' has duplicate environment variable '{}'",
                                container.name, env_var.name
                            ),
                            remediation: Some(
                                "Remove duplicate environment variable definitions. \
                                 Only the last definition will be used."
                                    .to_string(),
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
    fn test_env_var_secret_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: secret-in-env
spec:
  template:
    spec:
      containers:
      - name: app
        image: myapp:1.0
        env:
        - name: DB_PASSWORD
          value: "supersecret123"
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = EnvVarSecretCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("DB_PASSWORD"));
    }

    #[test]
    fn test_env_var_secret_ref_ok() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: secret-ref
spec:
  template:
    spec:
      containers:
      - name: app
        image: myapp:1.0
        env:
        - name: DB_PASSWORD
          valueFrom:
            secretKeyRef:
              name: db-secret
              key: password
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = EnvVarSecretCheck;
        let diagnostics = check.check(&objects[0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_duplicate_env_var_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: dup-env
spec:
  template:
    spec:
      containers:
      - name: app
        image: myapp:1.0
        env:
        - name: FOO
          value: "bar"
        - name: FOO
          value: "baz"
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = DuplicateEnvVarCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("duplicate"));
    }

    #[test]
    fn test_read_secret_from_env_var_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: secret-env
spec:
  template:
    spec:
      containers:
      - name: app
        image: myapp:1.0
        env:
        - name: DB_PASSWORD
          valueFrom:
            secretKeyRef:
              name: db-secret
              key: password
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = ReadSecretFromEnvVarCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("reads secret"));
    }
}
