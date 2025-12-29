//! Built-in checks for kube-linter.
//!
//! This module registers all 63 built-in checks that come with kube-linter.

use crate::analyzer::kubelint::config::{CheckScope, CheckSpec};

/// Get all built-in check specifications.
pub fn builtin_checks() -> Vec<CheckSpec> {
    vec![
        // Security checks
        CheckSpec::new(
            "privileged-container",
            "Indicates when deployments have containers running in privileged mode.",
            "Do not run your container as privileged unless it is required.",
            "privileged",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "privilege-escalation",
            "Alert on containers of deployments that allow privilege escalation.",
            "Ensure containers do not allow privilege escalation by setting allowPrivilegeEscalation to false.",
            "privilege-escalation",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "run-as-non-root",
            "Indicates when containers are not set to runAsNonRoot.",
            "Set runAsNonRoot to true in your container's securityContext.",
            "run-as-non-root",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "read-only-root-fs",
            "Indicates when containers are running with a read-write root filesystem.",
            "Set readOnlyRootFilesystem to true in your container's securityContext.",
            "read-only-root-fs",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "drop-net-raw-capability",
            "Indicates when containers do not drop NET_RAW capability.",
            "NET_RAW capability allows a container to craft arbitrary network packets. Drop this capability.",
            "drop-net-raw-capability",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "hostnetwork",
            "Indicates when deployments use the host's network namespace.",
            "Ensure deployments do not share the host's network namespace.",
            "host-network",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "hostpid",
            "Indicates when deployments share the host's process namespace.",
            "Ensure deployments do not share the host's process namespace.",
            "host-pid",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "hostipc",
            "Indicates when deployments share the host's IPC namespace.",
            "Ensure deployments do not share the host's IPC namespace.",
            "host-ipc",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "host-mounts",
            "Indicates when deployments mount sensitive host directories.",
            "Do not mount sensitive host paths unless absolutely necessary.",
            "host-mounts",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "writable-host-mount",
            "Indicates when containers mount host directories as writable.",
            "Mount host paths as read-only unless write access is required.",
            "writable-host-mount",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "docker-sock",
            "Indicates when deployments mount the Docker socket.",
            "Do not mount /var/run/docker.sock as it gives full control over Docker.",
            "host-mounts",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "unsafe-proc-mount",
            "Indicates when containers have unsafe /proc mount.",
            "Use the Default procMount type unless Unmasked is absolutely required.",
            "unsafe-proc-mount",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        // Best practice checks
        CheckSpec::new(
            "latest-tag",
            "Indicates when containers use images with the 'latest' tag.",
            "Use specific image tags instead of 'latest' for reproducible deployments.",
            "latest-tag",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "no-liveness-probe",
            "Indicates when containers do not have liveness probes configured.",
            "Add a liveness probe to detect and recover from container failures.",
            "liveness-probe",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "no-readiness-probe",
            "Indicates when containers do not have readiness probes configured.",
            "Add a readiness probe to ensure traffic is only sent to healthy containers.",
            "readiness-probe",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "unset-cpu-requirements",
            "Indicates when containers do not have CPU requirements set.",
            "Set CPU requests and limits for better resource management.",
            "cpu-requirements",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "unset-memory-requirements",
            "Indicates when containers do not have memory requirements set.",
            "Set memory requests and limits for better resource management.",
            "memory-requirements",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "minimum-replicas",
            "Indicates when deployments have fewer than the minimum recommended replicas.",
            "Increase the number of replicas for better availability.",
            "replicas",
        )
        .with_scope(CheckScope::new(&["Deployment", "StatefulSet"])),
        CheckSpec::new(
            "no-anti-affinity",
            "Indicates when deployments do not have pod anti-affinity configured.",
            "Use pod anti-affinity to spread pods across nodes for better availability.",
            "anti-affinity",
        )
        .with_scope(CheckScope::new(&["Deployment", "StatefulSet"])),
        CheckSpec::new(
            "no-rolling-update-strategy",
            "Indicates when deployments do not use a rolling update strategy.",
            "Use RollingUpdate strategy for zero-downtime deployments.",
            "rolling-update-strategy",
        )
        .with_scope(CheckScope::new(&["Deployment"])),
        CheckSpec::new(
            "default-service-account",
            "Indicates when deployments use the default service account.",
            "Create and use a dedicated service account for your workloads.",
            "service-account",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "deprecated-service-account",
            "Indicates when the deprecated serviceAccount field is used.",
            "Use serviceAccountName instead of the deprecated serviceAccount field.",
            "deprecated-service-account-field",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        // RBAC checks
        CheckSpec::new(
            "access-to-secrets",
            "Indicates when RBAC rules grant access to secrets.",
            "Limit access to secrets to only those that need it.",
            "access-to-secrets",
        )
        .with_scope(CheckScope::new(&["Role", "ClusterRole"])),
        CheckSpec::new(
            "access-to-create-pods",
            "Indicates when RBAC rules grant create access to pods.",
            "Limit the ability to create pods as it can be used for privilege escalation.",
            "access-to-create-pods",
        )
        .with_scope(CheckScope::new(&["Role", "ClusterRole"])),
        CheckSpec::new(
            "cluster-admin-role-binding",
            "Indicates when a ClusterRoleBinding grants cluster-admin.",
            "Avoid granting cluster-admin to users or service accounts.",
            "cluster-admin-role-binding",
        )
        .with_scope(CheckScope::new(&["ClusterRoleBinding"])),
        CheckSpec::new(
            "wildcard-in-rules",
            "Indicates when RBAC rules use wildcards.",
            "Avoid wildcards in RBAC rules; be specific about resources and verbs.",
            "wildcard-in-rules",
        )
        .with_scope(CheckScope::new(&["Role", "ClusterRole"])),
        // Validation checks
        CheckSpec::new(
            "dangling-service",
            "Indicates when services have selectors that do not match any pods.",
            "Ensure service selectors match labels on pods.",
            "dangling-service",
        )
        .with_scope(CheckScope::new(&["Service"])),
        CheckSpec::new(
            "dangling-ingress",
            "Indicates when ingresses reference non-existent services.",
            "Ensure ingress backends reference existing services.",
            "dangling-ingress",
        )
        .with_scope(CheckScope::new(&["Ingress"])),
        CheckSpec::new(
            "dangling-horizontalpodautoscaler",
            "Indicates when HPAs target non-existent deployments.",
            "Ensure HPA scaleTargetRef references an existing deployment.",
            "dangling-hpa",
        )
        .with_scope(CheckScope::new(&["HorizontalPodAutoscaler"])),
        CheckSpec::new(
            "dangling-networkpolicy",
            "Indicates when network policies have selectors that do not match any pods.",
            "Ensure network policy pod selectors match labels on pods.",
            "dangling-network-policy",
        )
        .with_scope(CheckScope::new(&["NetworkPolicy"])),
        CheckSpec::new(
            "mismatching-selector",
            "Indicates when deployment selectors do not match pod template labels.",
            "Ensure deployment selector matches pod template labels.",
            "mismatching-selector",
        )
        .with_scope(CheckScope::new(&["Deployment", "StatefulSet", "DaemonSet"])),
        CheckSpec::new(
            "duplicate-env-var",
            "Indicates when containers have duplicate environment variables.",
            "Remove duplicate environment variables.",
            "duplicate-env-var",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "invalid-target-ports",
            "Indicates when services have invalid target ports.",
            "Ensure service target ports reference valid container ports.",
            "target-port",
        )
        .with_scope(CheckScope::new(&["Service"])),
        // Additional checks
        CheckSpec::new(
            "env-var-secret",
            "Indicates when secrets are passed as environment variables.",
            "Mount secrets as volumes instead of environment variables.",
            "env-var-secret",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "read-secret-from-env-var",
            "Indicates when secrets are read from environment variables.",
            "Consider mounting secrets as files instead.",
            "read-secret-from-env-var",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "ssh-port",
            "Indicates when containers expose SSH port (22).",
            "Avoid exposing SSH ports in containers.",
            "ssh-port",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "privileged-ports",
            "Indicates when containers use privileged ports (< 1024).",
            "Use non-privileged ports (>= 1024) when possible.",
            "privileged-ports",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "no-extensions-v1beta",
            "Indicates when deprecated extensions/v1beta1 API is used.",
            "Use apps/v1 API instead of extensions/v1beta1.",
            "disallowed-gvk",
        )
        .with_scope(CheckScope::new(&["Any"])),
        CheckSpec::new(
            "hpa-minimum-replicas",
            "Indicates when HPA minReplicas is set too low.",
            "Set HPA minReplicas to at least 2 for high availability.",
            "hpa-min-replicas",
        )
        .with_scope(CheckScope::new(&["HorizontalPodAutoscaler"])),
        CheckSpec::new(
            "liveness-port",
            "Indicates when liveness probe ports do not match container ports.",
            "Ensure liveness probe ports match defined container ports.",
            "liveness-port",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "readiness-port",
            "Indicates when readiness probe ports do not match container ports.",
            "Ensure readiness probe ports match defined container ports.",
            "readiness-port",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "startup-port",
            "Indicates when startup probe ports do not match container ports.",
            "Ensure startup probe ports match defined container ports.",
            "startup-port",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "non-existent-service-account",
            "Indicates when pods reference non-existent service accounts.",
            "Create the service account or use an existing one.",
            "non-existent-service-account",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "non-isolated-pod",
            "Indicates when pods are not covered by any network policy.",
            "Create network policies to isolate pod traffic.",
            "non-isolated-pod",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "pdb-max-unavailable",
            "Indicates when PDB maxUnavailable is too permissive.",
            "Set appropriate maxUnavailable for PodDisruptionBudgets.",
            "pdb-max-unavailable",
        )
        .with_scope(CheckScope::new(&["PodDisruptionBudget"])),
        CheckSpec::new(
            "pdb-min-available",
            "Indicates when PDB minAvailable is too permissive.",
            "Set appropriate minAvailable for PodDisruptionBudgets.",
            "pdb-min-available",
        )
        .with_scope(CheckScope::new(&["PodDisruptionBudget"])),
        CheckSpec::new(
            "required-annotation-email",
            "Indicates when objects are missing required email annotation.",
            "Add the required annotation to your resource.",
            "required-annotation",
        )
        .with_scope(CheckScope::new(&["Any"])),
        CheckSpec::new(
            "required-label-owner",
            "Indicates when objects are missing required owner label.",
            "Add the required label to your resource.",
            "required-label",
        )
        .with_scope(CheckScope::new(&["Any"])),
        CheckSpec::new(
            "no-node-affinity",
            "Indicates when deployments do not have node affinity configured.",
            "Consider using node affinity to control pod placement.",
            "node-affinity",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "restart-policy",
            "Indicates when pods have inappropriate restart policies.",
            "Use an appropriate restart policy for your workload type.",
            "restart-policy",
        )
        .with_scope(CheckScope::new(&["Pod"])),
        CheckSpec::new(
            "scc-deny-privileged-container",
            "Indicates when SecurityContextConstraints allow privileged containers.",
            "Set allowPrivilegedContainer to false in SCC.",
            "scc-deny-privileged",
        )
        .with_scope(CheckScope::new(&["SecurityContextConstraints"])),
        CheckSpec::new(
            "sysctls",
            "Indicates when pods use unsafe sysctls.",
            "Avoid using unsafe sysctls in pod specifications.",
            "sysctls",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "use-namespace",
            "Indicates when objects are in the default namespace.",
            "Deploy resources to a specific namespace, not default.",
            "use-namespace",
        )
        .with_scope(CheckScope::new(&["Any"])),
        CheckSpec::new(
            "dangling-networkpolicypeer-podselector",
            "Indicates when NetworkPolicy peer pod selectors don't match any pods.",
            "Ensure NetworkPolicy peer selectors match existing pods.",
            "dangling-network-policy-peer",
        )
        .with_scope(CheckScope::new(&["NetworkPolicy"])),
        CheckSpec::new(
            "dangling-servicemonitor",
            "Indicates when ServiceMonitors have selectors that don't match any services.",
            "Ensure ServiceMonitor selectors match existing services.",
            "dangling-service-monitor",
        )
        .with_scope(CheckScope::new(&["ServiceMonitor"])),
        CheckSpec::new(
            "dnsconfig-options",
            "Indicates when pods have missing recommended DNS config options.",
            "Add recommended DNS config options for better reliability.",
            "dnsconfig-options",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "env-var-value-from",
            "Indicates when env vars reference non-existent secrets or configmaps.",
            "Ensure env var references point to existing resources.",
            "env-var-value-from",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "job-ttl-seconds-after-finished",
            "Indicates when jobs don't have ttlSecondsAfterFinished set.",
            "Set ttlSecondsAfterFinished to automatically clean up completed jobs.",
            "job-ttl-seconds-after-finished",
        )
        .with_scope(CheckScope::new(&["Job"])),
        CheckSpec::new(
            "priority-class-name",
            "Indicates when pods don't have a priorityClassName set.",
            "Set a priorityClassName for important workloads.",
            "priority-class-name",
        )
        .with_scope(CheckScope::new(&["DeploymentLike"])),
        CheckSpec::new(
            "service-type",
            "Indicates when services use the LoadBalancer type.",
            "Consider using ClusterIP or NodePort instead of LoadBalancer.",
            "service-type",
        )
        .with_scope(CheckScope::new(&["Service"])),
        CheckSpec::new(
            "pdb-unhealthy-pod-eviction-policy",
            "Indicates when PDB unhealthyPodEvictionPolicy is not configured.",
            "Set unhealthyPodEvictionPolicy to control eviction behavior.",
            "pdb-unhealthy-pod-eviction-policy",
        )
        .with_scope(CheckScope::new(&["PodDisruptionBudget"])),
        // Note: schema-validation requires external schema files
        // Note: sorted-keys is a style check
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_checks_count() {
        let checks = builtin_checks();
        assert!(
            checks.len() >= 60,
            "Expected at least 60 builtin checks, got {}",
            checks.len()
        );
    }

    #[test]
    fn test_builtin_checks_unique_names() {
        let checks = builtin_checks();
        let mut names: Vec<_> = checks.iter().map(|c| &c.name).collect();
        let original_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(
            names.len(),
            original_len,
            "Found duplicate check names"
        );
    }
}
