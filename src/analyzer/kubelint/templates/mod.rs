//! Check templates for kube-linter.
//!
//! Templates are reusable check implementations that can be configured
//! with parameters to create specific checks.

pub mod antiaffinity;
pub mod capabilities;
pub mod dangling;
pub mod envvar;
pub mod hostmounts;
pub mod hostnetwork;
pub mod latesttag;
pub mod livenessprobe;
pub mod misc;
pub mod pdb;
pub mod ports;
pub mod privileged;
pub mod privilegeescalation;
pub mod rbac;
pub mod readinessprobe;
pub mod readonlyrootfs;
pub mod replicas;
pub mod requirements;
pub mod runasnonroot;
pub mod serviceaccount;
pub mod unsafeprocmount;
pub mod updateconfig;
pub mod validation;

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};
use std::collections::HashMap;
use std::sync::OnceLock;

/// A check function that analyzes a Kubernetes object.
pub trait CheckFunc: Send + Sync {
    /// Run the check on an object and return any diagnostics.
    fn check(&self, object: &Object) -> Vec<Diagnostic>;
}

/// Parameter description for a template.
#[derive(Debug, Clone)]
pub struct ParameterDesc {
    /// Parameter name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Parameter type (string, bool, int, array, etc.).
    pub param_type: String,
    /// Whether the parameter is required.
    pub required: bool,
    /// Default value (if any).
    pub default: Option<serde_yaml::Value>,
}

/// A template for creating checks.
pub trait Template: Send + Sync {
    /// Get the template key (unique identifier).
    fn key(&self) -> &str;

    /// Get the human-readable name.
    fn human_name(&self) -> &str;

    /// Get the template description.
    fn description(&self) -> &str;

    /// Get the supported object kinds.
    fn supported_object_kinds(&self) -> ObjectKindsDesc;

    /// Get parameter descriptions.
    fn parameters(&self) -> Vec<ParameterDesc>;

    /// Instantiate a check function with the given parameters.
    fn instantiate(&self, params: &serde_yaml::Value) -> Result<Box<dyn CheckFunc>, TemplateError>;
}

/// Template instantiation errors.
#[derive(Debug, Clone)]
pub enum TemplateError {
    /// Missing required parameter.
    MissingParameter(String),
    /// Invalid parameter value.
    InvalidParameter(String),
    /// Unknown template.
    UnknownTemplate(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingParameter(name) => write!(f, "Missing required parameter: {}", name),
            Self::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            Self::UnknownTemplate(key) => write!(f, "Unknown template: {}", key),
        }
    }
}

impl std::error::Error for TemplateError {}

/// Global template registry.
static REGISTRY: OnceLock<HashMap<String, Box<dyn Template>>> = OnceLock::new();

/// Get the template registry, initializing if needed.
pub fn registry() -> &'static HashMap<String, Box<dyn Template>> {
    REGISTRY.get_or_init(|| {
        let mut map: HashMap<String, Box<dyn Template>> = HashMap::new();

        // Register all built-in templates
        map.insert(
            "privileged".to_string(),
            Box::new(privileged::PrivilegedTemplate),
        );
        map.insert(
            "privilege-escalation".to_string(),
            Box::new(privilegeescalation::PrivilegeEscalationTemplate),
        );
        map.insert(
            "run-as-non-root".to_string(),
            Box::new(runasnonroot::RunAsNonRootTemplate),
        );
        map.insert(
            "read-only-root-fs".to_string(),
            Box::new(readonlyrootfs::ReadOnlyRootFsTemplate),
        );
        map.insert(
            "latest-tag".to_string(),
            Box::new(latesttag::LatestTagTemplate),
        );
        map.insert(
            "liveness-probe".to_string(),
            Box::new(livenessprobe::LivenessProbeTemplate),
        );
        map.insert(
            "readiness-probe".to_string(),
            Box::new(readinessprobe::ReadinessProbeTemplate),
        );
        map.insert(
            "cpu-requirements".to_string(),
            Box::new(requirements::CpuRequirementsTemplate),
        );
        map.insert(
            "memory-requirements".to_string(),
            Box::new(requirements::MemoryRequirementsTemplate),
        );
        map.insert(
            "anti-affinity".to_string(),
            Box::new(antiaffinity::AntiAffinityTemplate),
        );
        map.insert(
            "drop-net-raw-capability".to_string(),
            Box::new(capabilities::DropNetRawCapabilityTemplate),
        );
        map.insert(
            "host-mounts".to_string(),
            Box::new(hostmounts::HostMountsTemplate),
        );
        map.insert(
            "writable-host-mount".to_string(),
            Box::new(hostmounts::WritableHostMountTemplate),
        );
        map.insert(
            "service-account".to_string(),
            Box::new(serviceaccount::ServiceAccountTemplate),
        );
        map.insert(
            "deprecated-service-account-field".to_string(),
            Box::new(serviceaccount::DeprecatedServiceAccountFieldTemplate),
        );
        map.insert(
            "rolling-update-strategy".to_string(),
            Box::new(updateconfig::RollingUpdateStrategyTemplate),
        );

        // Host namespace templates
        map.insert(
            "host-network".to_string(),
            Box::new(hostnetwork::HostNetworkTemplate),
        );
        map.insert(
            "host-pid".to_string(),
            Box::new(hostnetwork::HostPIDTemplate),
        );
        map.insert(
            "host-ipc".to_string(),
            Box::new(hostnetwork::HostIPCTemplate),
        );

        // Replica and scaling templates
        map.insert("replicas".to_string(), Box::new(replicas::ReplicasTemplate));

        // Unsafe proc mount template
        map.insert(
            "unsafe-proc-mount".to_string(),
            Box::new(unsafeprocmount::UnsafeProcMountTemplate),
        );

        // Environment variable templates
        map.insert(
            "env-var-secret".to_string(),
            Box::new(envvar::EnvVarSecretTemplate),
        );
        map.insert(
            "read-secret-from-env-var".to_string(),
            Box::new(envvar::ReadSecretFromEnvVarTemplate),
        );
        map.insert(
            "duplicate-env-var".to_string(),
            Box::new(envvar::DuplicateEnvVarTemplate),
        );

        // Port templates
        map.insert(
            "privileged-ports".to_string(),
            Box::new(ports::PrivilegedPortsTemplate),
        );
        map.insert("ssh-port".to_string(), Box::new(ports::SSHPortTemplate));
        map.insert(
            "liveness-port".to_string(),
            Box::new(ports::LivenessPortTemplate),
        );
        map.insert(
            "readiness-port".to_string(),
            Box::new(ports::ReadinessPortTemplate),
        );

        // RBAC templates
        map.insert(
            "cluster-admin-role-binding".to_string(),
            Box::new(rbac::ClusterAdminRoleBindingTemplate),
        );
        map.insert(
            "wildcard-in-rules".to_string(),
            Box::new(rbac::WildcardInRulesTemplate),
        );
        map.insert(
            "access-to-secrets".to_string(),
            Box::new(rbac::AccessToSecretsTemplate),
        );
        map.insert(
            "access-to-create-pods".to_string(),
            Box::new(rbac::AccessToCreatePodsTemplate),
        );

        // PDB templates
        map.insert(
            "pdb-max-unavailable".to_string(),
            Box::new(pdb::PdbMaxUnavailableTemplate),
        );
        map.insert(
            "pdb-min-available".to_string(),
            Box::new(pdb::PdbMinAvailableTemplate),
        );
        map.insert(
            "pdb-unhealthy-pod-eviction-policy".to_string(),
            Box::new(pdb::PdbUnhealthyPodEvictionPolicyTemplate),
        );

        // Validation templates
        map.insert(
            "use-namespace".to_string(),
            Box::new(validation::UseNamespaceTemplate),
        );
        map.insert(
            "restart-policy".to_string(),
            Box::new(validation::RestartPolicyTemplate),
        );
        map.insert(
            "required-annotation".to_string(),
            Box::new(validation::RequiredAnnotationTemplate),
        );
        map.insert(
            "required-label".to_string(),
            Box::new(validation::RequiredLabelTemplate),
        );
        map.insert(
            "disallowed-gvk".to_string(),
            Box::new(validation::DisallowedGVKTemplate),
        );
        map.insert(
            "mismatching-selector".to_string(),
            Box::new(validation::MismatchingSelectorTemplate),
        );
        map.insert(
            "node-affinity".to_string(),
            Box::new(validation::NodeAffinityTemplate),
        );
        map.insert(
            "job-ttl-seconds-after-finished".to_string(),
            Box::new(validation::JobTtlSecondsAfterFinishedTemplate),
        );
        map.insert(
            "priority-class-name".to_string(),
            Box::new(validation::PriorityClassNameTemplate),
        );
        map.insert(
            "service-type".to_string(),
            Box::new(validation::ServiceTypeTemplate),
        );
        map.insert(
            "hpa-min-replicas".to_string(),
            Box::new(validation::HpaMinReplicasTemplate),
        );

        // Misc templates
        map.insert("sysctls".to_string(), Box::new(misc::SysctlsTemplate));
        map.insert(
            "dnsconfig-options".to_string(),
            Box::new(misc::DnsConfigOptionsTemplate),
        );
        map.insert(
            "startup-port".to_string(),
            Box::new(misc::StartupPortTemplate),
        );
        map.insert(
            "env-var-value-from".to_string(),
            Box::new(misc::EnvVarValueFromTemplate),
        );
        map.insert(
            "target-port".to_string(),
            Box::new(misc::TargetPortTemplate),
        );

        // Dangling resource templates (cross-resource validation)
        map.insert(
            "dangling-service".to_string(),
            Box::new(dangling::DanglingServiceTemplate),
        );
        map.insert(
            "dangling-ingress".to_string(),
            Box::new(dangling::DanglingIngressTemplate),
        );
        map.insert(
            "dangling-hpa".to_string(),
            Box::new(dangling::DanglingHpaTemplate),
        );
        map.insert(
            "dangling-network-policy".to_string(),
            Box::new(dangling::DanglingNetworkPolicyTemplate),
        );
        map.insert(
            "dangling-network-policy-peer".to_string(),
            Box::new(dangling::DanglingNetworkPolicyPeerTemplate),
        );
        map.insert(
            "dangling-service-monitor".to_string(),
            Box::new(dangling::DanglingServiceMonitorTemplate),
        );
        map.insert(
            "non-existent-service-account".to_string(),
            Box::new(dangling::NonExistentServiceAccountTemplate),
        );
        map.insert(
            "non-isolated-pod".to_string(),
            Box::new(dangling::NonIsolatedPodTemplate),
        );
        map.insert(
            "scc-deny-privileged".to_string(),
            Box::new(dangling::SccDenyPrivilegedTemplate),
        );

        map
    })
}

/// Get a template by key.
pub fn get_template(key: &str) -> Option<&'static dyn Template> {
    registry().get(key).map(|t| t.as_ref())
}

/// List all registered templates.
pub fn list_templates() -> Vec<&'static str> {
    registry().keys().map(|s| s.as_str()).collect()
}

/// Initialize all built-in templates.
/// This is called automatically on first access to the registry.
pub fn init_builtin_templates() {
    let _ = registry();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_initialization() {
        let reg = registry();
        assert!(!reg.is_empty());
        assert!(reg.contains_key("privileged"));
        assert!(reg.contains_key("latest-tag"));
    }

    #[test]
    fn test_get_template() {
        let template = get_template("privileged");
        assert!(template.is_some());
        assert_eq!(template.unwrap().key(), "privileged");
    }

    #[test]
    fn test_list_templates() {
        let templates = list_templates();
        assert!(templates.contains(&"privileged"));
        assert!(templates.contains(&"latest-tag"));
    }
}
