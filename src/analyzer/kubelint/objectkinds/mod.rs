//! Object kind definitions and matching.
//!
//! Defines groups of Kubernetes object kinds that checks can target.

use crate::analyzer::kubelint::types::ObjectKind;

/// Check if an object kind matches a kind specifier.
///
/// Supports both specific kinds (e.g., "Deployment") and group specifiers
/// (e.g., "DeploymentLike").
pub fn matches_kind(specifier: &str, kind: &ObjectKind) -> bool {
    match specifier {
        "DeploymentLike" => kind.is_deployment_like(),
        "JobLike" => kind.is_job_like(),
        "Any" => true,
        _ => specifier == kind.as_str(),
    }
}

/// Get all object kinds that match a specifier.
pub fn expand_kind_specifier(specifier: &str) -> Vec<ObjectKind> {
    match specifier {
        "DeploymentLike" => vec![
            ObjectKind::Deployment,
            ObjectKind::StatefulSet,
            ObjectKind::DaemonSet,
            ObjectKind::ReplicaSet,
            ObjectKind::Pod,
            ObjectKind::Job,
            ObjectKind::CronJob,
            ObjectKind::DeploymentConfig,
        ],
        "JobLike" => vec![ObjectKind::Job, ObjectKind::CronJob],
        "Any" => vec![
            ObjectKind::Deployment,
            ObjectKind::StatefulSet,
            ObjectKind::DaemonSet,
            ObjectKind::ReplicaSet,
            ObjectKind::Pod,
            ObjectKind::Job,
            ObjectKind::CronJob,
            ObjectKind::Service,
            ObjectKind::Ingress,
            ObjectKind::NetworkPolicy,
            ObjectKind::Role,
            ObjectKind::ClusterRole,
            ObjectKind::RoleBinding,
            ObjectKind::ClusterRoleBinding,
            ObjectKind::ServiceAccount,
            ObjectKind::HorizontalPodAutoscaler,
            ObjectKind::PodDisruptionBudget,
        ],
        _ => ObjectKind::from_kind(specifier)
            .map(|k| vec![k])
            .unwrap_or_default(),
    }
}
