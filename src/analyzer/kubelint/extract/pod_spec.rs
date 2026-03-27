//! PodSpec extraction utilities.

use crate::analyzer::kubelint::context::K8sObject;
use crate::analyzer::kubelint::context::object::*;

/// Extract the PodSpec from a Kubernetes object, if it has one.
pub fn extract_pod_spec(obj: &K8sObject) -> Option<&PodSpec> {
    match obj {
        K8sObject::Deployment(d) => d.pod_spec.as_ref(),
        K8sObject::StatefulSet(d) => d.pod_spec.as_ref(),
        K8sObject::DaemonSet(d) => d.pod_spec.as_ref(),
        K8sObject::ReplicaSet(d) => d.pod_spec.as_ref(),
        K8sObject::Pod(d) => d.spec.as_ref(),
        K8sObject::Job(d) => d.pod_spec.as_ref(),
        K8sObject::CronJob(d) => d.job_spec.as_ref().and_then(|j| j.pod_spec.as_ref()),
        _ => None,
    }
}

/// Check if an object has a PodSpec.
pub fn has_pod_spec(obj: &K8sObject) -> bool {
    extract_pod_spec(obj).is_some()
}
