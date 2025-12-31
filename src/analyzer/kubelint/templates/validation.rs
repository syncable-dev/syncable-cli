//! General validation check templates.

use crate::analyzer::kubelint::context::K8sObject;
use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for checking use of namespace.
pub struct UseNamespaceTemplate;

impl Template for UseNamespaceTemplate {
    fn key(&self) -> &str {
        "use-namespace"
    }

    fn human_name(&self) -> &str {
        "Use Namespace"
    }

    fn description(&self) -> &str {
        "Checks that resources specify a namespace"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["DeploymentLike", "Service", "Ingress", "NetworkPolicy"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(UseNamespaceCheck))
    }
}

struct UseNamespaceCheck;

impl CheckFunc for UseNamespaceCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if object.namespace().is_none() || object.namespace() == Some("default") {
            diagnostics.push(Diagnostic {
                message: format!(
                    "Object '{}' does not specify a namespace or uses the default namespace",
                    object.name()
                ),
                remediation: Some(
                    "Specify an explicit namespace for your resources to improve isolation."
                        .to_string(),
                ),
            });
        }

        diagnostics
    }
}

/// Template for checking restart policy.
pub struct RestartPolicyTemplate;

impl Template for RestartPolicyTemplate {
    fn key(&self) -> &str {
        "restart-policy"
    }

    fn human_name(&self) -> &str {
        "Restart Policy"
    }

    fn description(&self) -> &str {
        "Checks pod restart policy settings"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["DeploymentLike"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(RestartPolicyCheck))
    }
}

struct RestartPolicyCheck;

impl CheckFunc for RestartPolicyCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object)
            && let Some(policy) = &pod_spec.restart_policy {
                // For Deployments, StatefulSets, DaemonSets - must be Always
                match &object.k8s_object {
                    K8sObject::Deployment(_)
                    | K8sObject::StatefulSet(_)
                    | K8sObject::DaemonSet(_)
                    | K8sObject::ReplicaSet(_) => {
                        if policy != "Always" {
                            diagnostics.push(Diagnostic {
                                message: format!(
                                    "Restart policy is '{}' but should be 'Always' for this workload type",
                                    policy
                                ),
                                remediation: Some(
                                    "Deployments, StatefulSets, DaemonSets, and ReplicaSets \
                                     require restartPolicy: Always."
                                        .to_string(),
                                ),
                            });
                        }
                    }
                    _ => {}
                }
            }

        diagnostics
    }
}

/// Template for checking required annotations.
pub struct RequiredAnnotationTemplate;

impl Template for RequiredAnnotationTemplate {
    fn key(&self) -> &str {
        "required-annotation"
    }

    fn human_name(&self) -> &str {
        "Required Annotation"
    }

    fn description(&self) -> &str {
        "Checks for required annotations on resources"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["Any"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        vec![
            ParameterDesc {
                name: "key".to_string(),
                description: "Required annotation key".to_string(),
                param_type: "string".to_string(),
                required: true,
                default: None,
            },
            ParameterDesc {
                name: "value".to_string(),
                description: "Optional required value pattern (regex)".to_string(),
                param_type: "string".to_string(),
                required: false,
                default: None,
            },
        ]
    }

    fn instantiate(&self, params: &serde_yaml::Value) -> Result<Box<dyn CheckFunc>, TemplateError> {
        let key = params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TemplateError::MissingParameter("key".to_string()))?
            .to_string();
        let value_pattern = params
            .get("value")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Ok(Box::new(RequiredAnnotationCheck { key, value_pattern }))
    }
}

struct RequiredAnnotationCheck {
    key: String,
    value_pattern: Option<String>,
}

impl CheckFunc for RequiredAnnotationCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let has_annotation = object
            .annotations()
            .map(|annotations| {
                if let Some(value) = annotations.get(&self.key) {
                    if let Some(pattern) = &self.value_pattern {
                        regex::Regex::new(pattern)
                            .map(|re| re.is_match(value))
                            .unwrap_or(false)
                    } else {
                        true
                    }
                } else {
                    false
                }
            })
            .unwrap_or(false);

        if !has_annotation {
            diagnostics.push(Diagnostic {
                message: format!("Object is missing required annotation '{}'", self.key),
                remediation: Some(format!(
                    "Add the annotation '{}' to your resource metadata.",
                    self.key
                )),
            });
        }

        diagnostics
    }
}

/// Template for checking required labels.
pub struct RequiredLabelTemplate;

impl Template for RequiredLabelTemplate {
    fn key(&self) -> &str {
        "required-label"
    }

    fn human_name(&self) -> &str {
        "Required Label"
    }

    fn description(&self) -> &str {
        "Checks for required labels on resources"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["Any"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        vec![
            ParameterDesc {
                name: "key".to_string(),
                description: "Required label key".to_string(),
                param_type: "string".to_string(),
                required: true,
                default: None,
            },
            ParameterDesc {
                name: "value".to_string(),
                description: "Optional required value pattern (regex)".to_string(),
                param_type: "string".to_string(),
                required: false,
                default: None,
            },
        ]
    }

    fn instantiate(&self, params: &serde_yaml::Value) -> Result<Box<dyn CheckFunc>, TemplateError> {
        let key = params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TemplateError::MissingParameter("key".to_string()))?
            .to_string();
        let value_pattern = params
            .get("value")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Ok(Box::new(RequiredLabelCheck { key, value_pattern }))
    }
}

struct RequiredLabelCheck {
    key: String,
    value_pattern: Option<String>,
}

impl CheckFunc for RequiredLabelCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let labels = match &object.k8s_object {
            K8sObject::Deployment(d) => d.labels.as_ref(),
            K8sObject::StatefulSet(s) => s.labels.as_ref(),
            K8sObject::DaemonSet(d) => d.labels.as_ref(),
            K8sObject::Pod(p) => p.labels.as_ref(),
            K8sObject::Service(s) => s.labels.as_ref(),
            _ => None,
        };

        let has_label = labels
            .map(|labels| {
                if let Some(value) = labels.get(&self.key) {
                    if let Some(pattern) = &self.value_pattern {
                        regex::Regex::new(pattern)
                            .map(|re| re.is_match(value))
                            .unwrap_or(false)
                    } else {
                        true
                    }
                } else {
                    false
                }
            })
            .unwrap_or(false);

        if !has_label {
            diagnostics.push(Diagnostic {
                message: format!("Object is missing required label '{}'", self.key),
                remediation: Some(format!(
                    "Add the label '{}' to your resource metadata.",
                    self.key
                )),
            });
        }

        diagnostics
    }
}

/// Template for checking deprecated API versions.
pub struct DisallowedGVKTemplate;

impl Template for DisallowedGVKTemplate {
    fn key(&self) -> &str {
        "disallowed-gvk"
    }

    fn human_name(&self) -> &str {
        "Disallowed API Version"
    }

    fn description(&self) -> &str {
        "Checks for deprecated or disallowed API versions"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["Any"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(DisallowedGVKCheck))
    }
}

struct DisallowedGVKCheck;

impl CheckFunc for DisallowedGVKCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let K8sObject::Unknown(unknown) = &object.k8s_object {
            let api_version = &unknown.api_version;

            // Check for deprecated extensions/v1beta1 API
            if api_version == "extensions/v1beta1" {
                diagnostics.push(Diagnostic {
                    message: "Resource uses deprecated API version 'extensions/v1beta1'"
                        .to_string(),
                    remediation: Some(
                        "Migrate to apps/v1 for Deployments, DaemonSets, ReplicaSets; \
                         networking.k8s.io/v1 for Ingress and NetworkPolicy."
                            .to_string(),
                    ),
                });
            }

            // Check for deprecated apps/v1beta1 and apps/v1beta2
            if api_version == "apps/v1beta1" || api_version == "apps/v1beta2" {
                diagnostics.push(Diagnostic {
                    message: format!("Resource uses deprecated API version '{}'", api_version),
                    remediation: Some("Migrate to apps/v1.".to_string()),
                });
            }
        }

        diagnostics
    }
}

/// Template for checking mismatching selectors.
pub struct MismatchingSelectorTemplate;

impl Template for MismatchingSelectorTemplate {
    fn key(&self) -> &str {
        "mismatching-selector"
    }

    fn human_name(&self) -> &str {
        "Mismatching Selector"
    }

    fn description(&self) -> &str {
        "Checks that deployment selector matches pod template labels"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["Deployment", "StatefulSet", "DaemonSet"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(MismatchingSelectorCheck))
    }
}

struct MismatchingSelectorCheck;

impl CheckFunc for MismatchingSelectorCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let (selector, pod_labels) = match &object.k8s_object {
            K8sObject::Deployment(d) => {
                let selector = d.selector.as_ref().and_then(|s| s.match_labels.as_ref());
                let pod_labels = d.pod_spec.as_ref().and(d.labels.as_ref());
                (selector, pod_labels)
            }
            K8sObject::StatefulSet(s) => {
                let selector = s.selector.as_ref().and_then(|s| s.match_labels.as_ref());
                let pod_labels = s.pod_spec.as_ref().and(s.labels.as_ref());
                (selector, pod_labels)
            }
            K8sObject::DaemonSet(d) => {
                let selector = d.selector.as_ref().and_then(|s| s.match_labels.as_ref());
                let pod_labels = d.pod_spec.as_ref().and(d.labels.as_ref());
                (selector, pod_labels)
            }
            _ => (None, None),
        };

        if let (Some(selector_labels), Some(pod_labels)) = (selector, pod_labels) {
            for (key, value) in selector_labels {
                if pod_labels.get(key) != Some(value) {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Selector label '{}={}' does not match pod template labels",
                            key, value
                        ),
                        remediation: Some(
                            "Ensure the selector's matchLabels are present in the pod template's labels."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        diagnostics
    }
}

/// Template for checking node affinity.
pub struct NodeAffinityTemplate;

impl Template for NodeAffinityTemplate {
    fn key(&self) -> &str {
        "node-affinity"
    }

    fn human_name(&self) -> &str {
        "Node Affinity"
    }

    fn description(&self) -> &str {
        "Checks if node affinity is configured"
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
        Ok(Box::new(NodeAffinityCheck))
    }
}

struct NodeAffinityCheck;

impl CheckFunc for NodeAffinityCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            let has_node_affinity = pod_spec
                .affinity
                .as_ref()
                .and_then(|a| a.node_affinity.as_ref())
                .is_some();

            if !has_node_affinity {
                diagnostics.push(Diagnostic {
                    message: "Pod does not have node affinity configured".to_string(),
                    remediation: Some(
                        "Consider adding node affinity rules to control pod placement.".to_string(),
                    ),
                });
            }
        }

        diagnostics
    }
}

/// Template for checking Job TTL after finished.
pub struct JobTtlSecondsAfterFinishedTemplate;

impl Template for JobTtlSecondsAfterFinishedTemplate {
    fn key(&self) -> &str {
        "job-ttl-seconds-after-finished"
    }

    fn human_name(&self) -> &str {
        "Job TTL Seconds After Finished"
    }

    fn description(&self) -> &str {
        "Checks if Job has ttlSecondsAfterFinished set"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["Job"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(JobTtlSecondsAfterFinishedCheck))
    }
}

struct JobTtlSecondsAfterFinishedCheck;

impl CheckFunc for JobTtlSecondsAfterFinishedCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let K8sObject::Job(job) = &object.k8s_object
            && job.ttl_seconds_after_finished.is_none() {
                diagnostics.push(Diagnostic {
                    message: "Job does not have ttlSecondsAfterFinished set".to_string(),
                    remediation: Some(
                        "Set ttlSecondsAfterFinished to automatically clean up finished Jobs."
                            .to_string(),
                    ),
                });
            }

        diagnostics
    }
}

/// Template for checking priority class name.
pub struct PriorityClassNameTemplate;

impl Template for PriorityClassNameTemplate {
    fn key(&self) -> &str {
        "priority-class-name"
    }

    fn human_name(&self) -> &str {
        "Priority Class Name"
    }

    fn description(&self) -> &str {
        "Checks if priorityClassName is set"
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
        Ok(Box::new(PriorityClassNameCheck))
    }
}

struct PriorityClassNameCheck;

impl CheckFunc for PriorityClassNameCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object)
            && pod_spec.priority_class_name.is_none() {
                diagnostics.push(Diagnostic {
                    message: "Pod does not have priorityClassName set".to_string(),
                    remediation: Some(
                        "Set priorityClassName to control pod scheduling priority.".to_string(),
                    ),
                });
            }

        diagnostics
    }
}

/// Template for checking Service type.
pub struct ServiceTypeTemplate;

impl Template for ServiceTypeTemplate {
    fn key(&self) -> &str {
        "service-type"
    }

    fn human_name(&self) -> &str {
        "Service Type"
    }

    fn description(&self) -> &str {
        "Checks Service type configuration"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["Service"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        vec![ParameterDesc {
            name: "disallowedTypes".to_string(),
            description: "List of disallowed service types".to_string(),
            param_type: "array".to_string(),
            required: false,
            default: Some(serde_yaml::Value::Sequence(vec![
                serde_yaml::Value::String("NodePort".to_string()),
                serde_yaml::Value::String("LoadBalancer".to_string()),
            ])),
        }]
    }

    fn instantiate(&self, params: &serde_yaml::Value) -> Result<Box<dyn CheckFunc>, TemplateError> {
        let disallowed = params
            .get("disallowedTypes")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(|| vec!["NodePort".to_string(), "LoadBalancer".to_string()]);
        Ok(Box::new(ServiceTypeCheck { disallowed }))
    }
}

struct ServiceTypeCheck {
    disallowed: Vec<String>,
}

impl CheckFunc for ServiceTypeCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let K8sObject::Service(svc) = &object.k8s_object
            && let Some(svc_type) = &svc.type_
                && self.disallowed.contains(svc_type) {
                    diagnostics.push(Diagnostic {
                        message: format!("Service uses disallowed type '{}'", svc_type),
                        remediation: Some(format!(
                            "Consider using ClusterIP instead of {}.",
                            svc_type
                        )),
                    });
                }

        diagnostics
    }
}

/// Template for checking HPA minimum replicas.
pub struct HpaMinReplicasTemplate;

impl Template for HpaMinReplicasTemplate {
    fn key(&self) -> &str {
        "hpa-min-replicas"
    }

    fn human_name(&self) -> &str {
        "HPA Minimum Replicas"
    }

    fn description(&self) -> &str {
        "Checks HorizontalPodAutoscaler minReplicas setting"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["HorizontalPodAutoscaler"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        vec![ParameterDesc {
            name: "minReplicas".to_string(),
            description: "Minimum recommended minReplicas value".to_string(),
            param_type: "integer".to_string(),
            required: false,
            default: Some(serde_yaml::Value::Number(2.into())),
        }]
    }

    fn instantiate(&self, params: &serde_yaml::Value) -> Result<Box<dyn CheckFunc>, TemplateError> {
        let min_replicas = params
            .get("minReplicas")
            .and_then(|v| v.as_i64())
            .unwrap_or(2) as i32;
        Ok(Box::new(HpaMinReplicasCheck { min_replicas }))
    }
}

struct HpaMinReplicasCheck {
    min_replicas: i32,
}

impl CheckFunc for HpaMinReplicasCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let K8sObject::HorizontalPodAutoscaler(hpa) = &object.k8s_object
            && let Some(min) = hpa.min_replicas
                && min < self.min_replicas {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "HPA minReplicas is {} but should be at least {}",
                            min, self.min_replicas
                        ),
                        remediation: Some(format!(
                            "Set minReplicas to at least {} for better availability.",
                            self.min_replicas
                        )),
                    });
                }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::kubelint::parser::yaml::parse_yaml;

    #[test]
    fn test_use_namespace_default() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: test-deploy
  namespace: default
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = UseNamespaceCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_use_namespace_ok() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: test-deploy
  namespace: production
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = UseNamespaceCheck;
        let diagnostics = check.check(&objects[0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_hpa_min_replicas() {
        let yaml = r#"
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: test-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: test-deploy
  minReplicas: 1
  maxReplicas: 10
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = HpaMinReplicasCheck { min_replicas: 2 };
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
    }
}
