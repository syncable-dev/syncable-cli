//! PodDisruptionBudget check templates.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::context::K8sObject;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for checking PDB maxUnavailable settings.
pub struct PdbMaxUnavailableTemplate;

impl Template for PdbMaxUnavailableTemplate {
    fn key(&self) -> &str {
        "pdb-max-unavailable"
    }

    fn human_name(&self) -> &str {
        "PDB Max Unavailable"
    }

    fn description(&self) -> &str {
        "Checks PodDisruptionBudget maxUnavailable settings"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["PodDisruptionBudget"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(PdbMaxUnavailableCheck))
    }
}

struct PdbMaxUnavailableCheck;

impl CheckFunc for PdbMaxUnavailableCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let K8sObject::PodDisruptionBudget(pdb) = &object.k8s_object {
            if let Some(max_unavailable) = &pdb.max_unavailable {
                // Check if it's set to 0 or 0%
                if max_unavailable == "0" || max_unavailable == "0%" {
                    diagnostics.push(Diagnostic {
                        message: "PDB maxUnavailable is set to 0, which blocks all voluntary disruptions".to_string(),
                        remediation: Some(
                            "Set maxUnavailable to at least 1 or a non-zero percentage to allow \
                             voluntary disruptions during cluster maintenance."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}

/// Template for checking PDB minAvailable settings.
pub struct PdbMinAvailableTemplate;

impl Template for PdbMinAvailableTemplate {
    fn key(&self) -> &str {
        "pdb-min-available"
    }

    fn human_name(&self) -> &str {
        "PDB Min Available"
    }

    fn description(&self) -> &str {
        "Checks PodDisruptionBudget minAvailable settings"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["PodDisruptionBudget"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(PdbMinAvailableCheck))
    }
}

struct PdbMinAvailableCheck;

impl CheckFunc for PdbMinAvailableCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let K8sObject::PodDisruptionBudget(pdb) = &object.k8s_object {
            if let Some(min_available) = &pdb.min_available {
                // Check if it's set to 100%
                if min_available == "100%" {
                    diagnostics.push(Diagnostic {
                        message: "PDB minAvailable is set to 100%, which blocks all voluntary disruptions".to_string(),
                        remediation: Some(
                            "Set minAvailable to less than 100% to allow voluntary disruptions \
                             during cluster maintenance."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}

/// Template for checking PDB unhealthyPodEvictionPolicy.
pub struct PdbUnhealthyPodEvictionPolicyTemplate;

impl Template for PdbUnhealthyPodEvictionPolicyTemplate {
    fn key(&self) -> &str {
        "pdb-unhealthy-pod-eviction-policy"
    }

    fn human_name(&self) -> &str {
        "PDB Unhealthy Pod Eviction Policy"
    }

    fn description(&self) -> &str {
        "Checks PodDisruptionBudget unhealthyPodEvictionPolicy settings"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["PodDisruptionBudget"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(PdbUnhealthyPodEvictionPolicyCheck))
    }
}

struct PdbUnhealthyPodEvictionPolicyCheck;

impl CheckFunc for PdbUnhealthyPodEvictionPolicyCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let K8sObject::PodDisruptionBudget(pdb) = &object.k8s_object {
            // Check if unhealthyPodEvictionPolicy is not set (defaults to IfHealthyBudget)
            if pdb.unhealthy_pod_eviction_policy.is_none() {
                diagnostics.push(Diagnostic {
                    message: "PDB does not specify unhealthyPodEvictionPolicy".to_string(),
                    remediation: Some(
                        "Consider setting unhealthyPodEvictionPolicy to 'AlwaysAllow' to allow \
                         eviction of unhealthy pods even when budget is violated."
                            .to_string(),
                    ),
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
    fn test_pdb_max_unavailable_zero() {
        let yaml = r#"
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: strict-pdb
spec:
  maxUnavailable: 0
  selector:
    matchLabels:
      app: test
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = PdbMaxUnavailableCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("maxUnavailable"));
    }

    #[test]
    fn test_pdb_min_available_100_percent() {
        let yaml = r#"
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: strict-pdb
spec:
  minAvailable: "100%"
  selector:
    matchLabels:
      app: test
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = PdbMinAvailableCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("minAvailable"));
    }
}
