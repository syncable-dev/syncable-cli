//! RBAC-related check templates.

use crate::analyzer::kubelint::context::K8sObject;
use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting cluster-admin role bindings.
pub struct ClusterAdminRoleBindingTemplate;

impl Template for ClusterAdminRoleBindingTemplate {
    fn key(&self) -> &str {
        "cluster-admin-role-binding"
    }

    fn human_name(&self) -> &str {
        "Cluster Admin Role Binding"
    }

    fn description(&self) -> &str {
        "Detects bindings to the cluster-admin ClusterRole"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["ClusterRoleBinding", "RoleBinding"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(ClusterAdminRoleBindingCheck))
    }
}

struct ClusterAdminRoleBindingCheck;

impl CheckFunc for ClusterAdminRoleBindingCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let role_ref = match &object.k8s_object {
            K8sObject::ClusterRoleBinding(crb) => Some(&crb.role_ref),
            K8sObject::RoleBinding(rb) => Some(&rb.role_ref),
            _ => None,
        };

        if let Some(role_ref) = role_ref {
            if role_ref.kind == "ClusterRole" && role_ref.name == "cluster-admin" {
                diagnostics.push(Diagnostic {
                    message: "Binding grants cluster-admin privileges".to_string(),
                    remediation: Some(
                        "Avoid binding to cluster-admin. Create a more restrictive ClusterRole \
                         with only the required permissions."
                            .to_string(),
                    ),
                });
            }
        }

        diagnostics
    }
}

/// Template for detecting wildcard rules in RBAC.
pub struct WildcardInRulesTemplate;

impl Template for WildcardInRulesTemplate {
    fn key(&self) -> &str {
        "wildcard-in-rules"
    }

    fn human_name(&self) -> &str {
        "Wildcard in RBAC Rules"
    }

    fn description(&self) -> &str {
        "Detects use of wildcards (*) in RBAC rules"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["Role", "ClusterRole"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(WildcardInRulesCheck))
    }
}

struct WildcardInRulesCheck;

impl CheckFunc for WildcardInRulesCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let rules = match &object.k8s_object {
            K8sObject::Role(r) => Some(&r.rules),
            K8sObject::ClusterRole(cr) => Some(&cr.rules),
            _ => None,
        };

        if let Some(rules) = rules {
            for rule in rules {
                // Check for wildcard in verbs
                if rule.verbs.iter().any(|v| v == "*") {
                    diagnostics.push(Diagnostic {
                        message: "Rule uses wildcard (*) in verbs".to_string(),
                        remediation: Some(
                            "Explicitly list the required verbs instead of using wildcard."
                                .to_string(),
                        ),
                    });
                }

                // Check for wildcard in resources
                if rule.resources.iter().any(|r| r == "*") {
                    diagnostics.push(Diagnostic {
                        message: "Rule uses wildcard (*) in resources".to_string(),
                        remediation: Some(
                            "Explicitly list the required resources instead of using wildcard."
                                .to_string(),
                        ),
                    });
                }

                // Check for wildcard in apiGroups
                if rule.api_groups.iter().any(|g| g == "*") {
                    diagnostics.push(Diagnostic {
                        message: "Rule uses wildcard (*) in apiGroups".to_string(),
                        remediation: Some(
                            "Explicitly list the required API groups instead of using wildcard."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}

/// Template for detecting access to secrets.
pub struct AccessToSecretsTemplate;

impl Template for AccessToSecretsTemplate {
    fn key(&self) -> &str {
        "access-to-secrets"
    }

    fn human_name(&self) -> &str {
        "Access to Secrets"
    }

    fn description(&self) -> &str {
        "Detects RBAC rules that grant access to secrets"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["Role", "ClusterRole"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(AccessToSecretsCheck))
    }
}

struct AccessToSecretsCheck;

impl CheckFunc for AccessToSecretsCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let rules = match &object.k8s_object {
            K8sObject::Role(r) => Some(&r.rules),
            K8sObject::ClusterRole(cr) => Some(&cr.rules),
            _ => None,
        };

        if let Some(rules) = rules {
            for rule in rules {
                // Check if rule grants access to secrets
                let grants_secret_access =
                    rule.resources.iter().any(|r| r == "secrets" || r == "*")
                        && rule
                            .api_groups
                            .iter()
                            .any(|g| g == "" || g == "*" || g == "core");

                if grants_secret_access {
                    // Check for sensitive verbs
                    let sensitive_verbs = ["get", "list", "watch", "*"];
                    if rule
                        .verbs
                        .iter()
                        .any(|v| sensitive_verbs.contains(&v.as_str()))
                    {
                        diagnostics.push(Diagnostic {
                            message: "Rule grants read access to secrets".to_string(),
                            remediation: Some(
                                "Avoid granting broad access to secrets. Consider using \
                                 resourceNames to limit access to specific secrets."
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

/// Template for detecting access to create pods.
pub struct AccessToCreatePodsTemplate;

impl Template for AccessToCreatePodsTemplate {
    fn key(&self) -> &str {
        "access-to-create-pods"
    }

    fn human_name(&self) -> &str {
        "Access to Create Pods"
    }

    fn description(&self) -> &str {
        "Detects RBAC rules that grant permission to create pods"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["Role", "ClusterRole"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(AccessToCreatePodsCheck))
    }
}

struct AccessToCreatePodsCheck;

impl CheckFunc for AccessToCreatePodsCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let rules = match &object.k8s_object {
            K8sObject::Role(r) => Some(&r.rules),
            K8sObject::ClusterRole(cr) => Some(&cr.rules),
            _ => None,
        };

        if let Some(rules) = rules {
            for rule in rules {
                // Check if rule grants create access to pods
                let grants_pod_create = rule.resources.iter().any(|r| r == "pods" || r == "*")
                    && rule
                        .api_groups
                        .iter()
                        .any(|g| g == "" || g == "*" || g == "core")
                    && rule.verbs.iter().any(|v| v == "create" || v == "*");

                if grants_pod_create {
                    diagnostics.push(Diagnostic {
                        message: "Rule grants permission to create pods".to_string(),
                        remediation: Some(
                            "Pod creation permission can be used for privilege escalation. \
                             Ensure this is intentional and the scope is limited."
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
    fn test_cluster_admin_binding_detected() {
        let yaml = r#"
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: admin-binding
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: cluster-admin
subjects:
- kind: User
  name: admin
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = ClusterAdminRoleBindingCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("cluster-admin"));
    }

    #[test]
    fn test_non_admin_binding_ok() {
        let yaml = r#"
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: viewer-binding
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: view
subjects:
- kind: User
  name: viewer
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = ClusterAdminRoleBindingCheck;
        let diagnostics = check.check(&objects[0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_wildcard_verbs_detected() {
        let yaml = r#"
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: wildcard-role
rules:
- apiGroups: [""]
  resources: ["pods"]
  verbs: ["*"]
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = WildcardInRulesCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("verbs"));
    }

    #[test]
    fn test_access_to_secrets_detected() {
        let yaml = r#"
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: secret-reader
rules:
- apiGroups: [""]
  resources: ["secrets"]
  verbs: ["get", "list"]
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = AccessToSecretsCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("secrets"));
    }

    #[test]
    fn test_pod_create_detected() {
        let yaml = r#"
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: pod-creator
  namespace: default
rules:
- apiGroups: [""]
  resources: ["pods"]
  verbs: ["create"]
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = AccessToCreatePodsCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("create pods"));
    }
}
