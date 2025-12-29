//! Metadata extraction utilities.

use crate::analyzer::kubelint::context::K8sObject;
use std::collections::BTreeMap;

/// Extract labels from a Kubernetes object.
pub fn extract_labels(obj: &K8sObject) -> Option<&BTreeMap<String, String>> {
    match obj {
        K8sObject::Deployment(d) => d.labels.as_ref(),
        K8sObject::StatefulSet(d) => d.labels.as_ref(),
        K8sObject::DaemonSet(d) => d.labels.as_ref(),
        K8sObject::ReplicaSet(d) => d.labels.as_ref(),
        K8sObject::Pod(d) => d.labels.as_ref(),
        K8sObject::Job(d) => d.labels.as_ref(),
        K8sObject::CronJob(d) => d.labels.as_ref(),
        K8sObject::Service(d) => d.labels.as_ref(),
        K8sObject::Ingress(d) => d.labels.as_ref(),
        K8sObject::NetworkPolicy(d) => d.labels.as_ref(),
        K8sObject::Role(d) => d.labels.as_ref(),
        K8sObject::ClusterRole(d) => d.labels.as_ref(),
        K8sObject::RoleBinding(d) => d.labels.as_ref(),
        K8sObject::ClusterRoleBinding(d) => d.labels.as_ref(),
        K8sObject::ServiceAccount(d) => d.labels.as_ref(),
        K8sObject::HorizontalPodAutoscaler(d) => d.labels.as_ref(),
        K8sObject::PodDisruptionBudget(d) => d.labels.as_ref(),
        K8sObject::PersistentVolumeClaim(d) => d.labels.as_ref(),
        K8sObject::Unknown(d) => d.labels.as_ref(),
    }
}

/// Check if an object has a specific annotation.
pub fn has_annotation(obj: &K8sObject, key: &str) -> bool {
    obj.annotations()
        .map(|a| a.contains_key(key))
        .unwrap_or(false)
}

/// Get an annotation value from an object.
pub fn get_annotation<'a>(obj: &'a K8sObject, key: &str) -> Option<&'a str> {
    obj.annotations()
        .and_then(|a| a.get(key))
        .map(|s| s.as_str())
}
