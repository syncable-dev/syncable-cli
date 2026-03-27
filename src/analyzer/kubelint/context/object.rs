//! Kubernetes object wrappers for linting.

use crate::analyzer::kubelint::types::ObjectKind;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Metadata about a parsed Kubernetes object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    /// The file path where this object was defined.
    pub file_path: PathBuf,
    /// The raw YAML content (for error reporting).
    pub raw: Option<Vec<u8>>,
    /// Line number in the source file (1-indexed).
    pub line_number: Option<u32>,
}

impl ObjectMetadata {
    /// Create new metadata for an object from a file.
    pub fn from_file(path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: path.into(),
            raw: None,
            line_number: None,
        }
    }

    /// Set the raw content.
    pub fn with_raw(mut self, raw: Vec<u8>) -> Self {
        self.raw = Some(raw);
        self
    }

    /// Set the line number.
    pub fn with_line(mut self, line: u32) -> Self {
        self.line_number = Some(line);
        self
    }
}

/// A parsed Kubernetes object ready for linting.
#[derive(Debug, Clone)]
pub struct Object {
    /// Metadata about where this object came from.
    pub metadata: ObjectMetadata,
    /// The Kubernetes object data.
    pub k8s_object: K8sObject,
}

impl Object {
    /// Create a new object.
    pub fn new(metadata: ObjectMetadata, k8s_object: K8sObject) -> Self {
        Self {
            metadata,
            k8s_object,
        }
    }

    /// Get the object's kind.
    pub fn kind(&self) -> ObjectKind {
        self.k8s_object.kind()
    }

    /// Get the object's name.
    pub fn name(&self) -> &str {
        self.k8s_object.name()
    }

    /// Get the object's namespace.
    pub fn namespace(&self) -> Option<&str> {
        self.k8s_object.namespace()
    }

    /// Get annotations from the object.
    pub fn annotations(&self) -> Option<&std::collections::BTreeMap<String, String>> {
        self.k8s_object.annotations()
    }
}

/// An object that failed to parse.
#[derive(Debug, Clone)]
pub struct InvalidObject {
    /// Metadata about where this object came from.
    pub metadata: ObjectMetadata,
    /// The error that occurred during parsing.
    pub load_err: String,
}

impl InvalidObject {
    /// Create a new invalid object record.
    pub fn new(metadata: ObjectMetadata, error: impl Into<String>) -> Self {
        Self {
            metadata,
            load_err: error.into(),
        }
    }
}

/// Enum representing all supported Kubernetes object types.
///
/// This enum provides type-safe access to K8s objects while also
/// supporting unknown/custom types via the Unknown variant.
#[derive(Debug, Clone)]
pub enum K8sObject {
    // Workloads
    Deployment(Box<DeploymentData>),
    StatefulSet(Box<StatefulSetData>),
    DaemonSet(Box<DaemonSetData>),
    ReplicaSet(Box<ReplicaSetData>),
    Pod(Box<PodData>),
    Job(Box<JobData>),
    CronJob(Box<CronJobData>),

    // Services & Networking
    Service(Box<ServiceData>),
    Ingress(Box<IngressData>),
    NetworkPolicy(Box<NetworkPolicyData>),

    // RBAC
    Role(Box<RoleData>),
    ClusterRole(Box<ClusterRoleData>),
    RoleBinding(Box<RoleBindingData>),
    ClusterRoleBinding(Box<ClusterRoleBindingData>),
    ServiceAccount(Box<ServiceAccountData>),

    // Scaling
    HorizontalPodAutoscaler(Box<HpaData>),
    PodDisruptionBudget(Box<PdbData>),

    // Storage
    PersistentVolumeClaim(Box<PvcData>),

    // Unknown/CRD
    Unknown(Box<UnknownObject>),
}

impl K8sObject {
    /// Get the object kind.
    pub fn kind(&self) -> ObjectKind {
        match self {
            Self::Deployment(_) => ObjectKind::Deployment,
            Self::StatefulSet(_) => ObjectKind::StatefulSet,
            Self::DaemonSet(_) => ObjectKind::DaemonSet,
            Self::ReplicaSet(_) => ObjectKind::ReplicaSet,
            Self::Pod(_) => ObjectKind::Pod,
            Self::Job(_) => ObjectKind::Job,
            Self::CronJob(_) => ObjectKind::CronJob,
            Self::Service(_) => ObjectKind::Service,
            Self::Ingress(_) => ObjectKind::Ingress,
            Self::NetworkPolicy(_) => ObjectKind::NetworkPolicy,
            Self::Role(_) => ObjectKind::Role,
            Self::ClusterRole(_) => ObjectKind::ClusterRole,
            Self::RoleBinding(_) => ObjectKind::RoleBinding,
            Self::ClusterRoleBinding(_) => ObjectKind::ClusterRoleBinding,
            Self::ServiceAccount(_) => ObjectKind::ServiceAccount,
            Self::HorizontalPodAutoscaler(_) => ObjectKind::HorizontalPodAutoscaler,
            Self::PodDisruptionBudget(_) => ObjectKind::PodDisruptionBudget,
            Self::PersistentVolumeClaim(_) => ObjectKind::PersistentVolumeClaim,
            Self::Unknown(_) => ObjectKind::Any,
        }
    }

    /// Get the object name.
    pub fn name(&self) -> &str {
        match self {
            Self::Deployment(d) => &d.name,
            Self::StatefulSet(d) => &d.name,
            Self::DaemonSet(d) => &d.name,
            Self::ReplicaSet(d) => &d.name,
            Self::Pod(d) => &d.name,
            Self::Job(d) => &d.name,
            Self::CronJob(d) => &d.name,
            Self::Service(d) => &d.name,
            Self::Ingress(d) => &d.name,
            Self::NetworkPolicy(d) => &d.name,
            Self::Role(d) => &d.name,
            Self::ClusterRole(d) => &d.name,
            Self::RoleBinding(d) => &d.name,
            Self::ClusterRoleBinding(d) => &d.name,
            Self::ServiceAccount(d) => &d.name,
            Self::HorizontalPodAutoscaler(d) => &d.name,
            Self::PodDisruptionBudget(d) => &d.name,
            Self::PersistentVolumeClaim(d) => &d.name,
            Self::Unknown(d) => &d.name,
        }
    }

    /// Get the object namespace.
    pub fn namespace(&self) -> Option<&str> {
        match self {
            Self::Deployment(d) => d.namespace.as_deref(),
            Self::StatefulSet(d) => d.namespace.as_deref(),
            Self::DaemonSet(d) => d.namespace.as_deref(),
            Self::ReplicaSet(d) => d.namespace.as_deref(),
            Self::Pod(d) => d.namespace.as_deref(),
            Self::Job(d) => d.namespace.as_deref(),
            Self::CronJob(d) => d.namespace.as_deref(),
            Self::Service(d) => d.namespace.as_deref(),
            Self::Ingress(d) => d.namespace.as_deref(),
            Self::NetworkPolicy(d) => d.namespace.as_deref(),
            Self::Role(d) => d.namespace.as_deref(),
            Self::ClusterRole(_) => None, // Cluster-scoped
            Self::RoleBinding(d) => d.namespace.as_deref(),
            Self::ClusterRoleBinding(_) => None, // Cluster-scoped
            Self::ServiceAccount(d) => d.namespace.as_deref(),
            Self::HorizontalPodAutoscaler(d) => d.namespace.as_deref(),
            Self::PodDisruptionBudget(d) => d.namespace.as_deref(),
            Self::PersistentVolumeClaim(d) => d.namespace.as_deref(),
            Self::Unknown(d) => d.namespace.as_deref(),
        }
    }

    /// Get annotations from the object.
    pub fn annotations(&self) -> Option<&std::collections::BTreeMap<String, String>> {
        match self {
            Self::Deployment(d) => d.annotations.as_ref(),
            Self::StatefulSet(d) => d.annotations.as_ref(),
            Self::DaemonSet(d) => d.annotations.as_ref(),
            Self::ReplicaSet(d) => d.annotations.as_ref(),
            Self::Pod(d) => d.annotations.as_ref(),
            Self::Job(d) => d.annotations.as_ref(),
            Self::CronJob(d) => d.annotations.as_ref(),
            Self::Service(d) => d.annotations.as_ref(),
            Self::Ingress(d) => d.annotations.as_ref(),
            Self::NetworkPolicy(d) => d.annotations.as_ref(),
            Self::Role(d) => d.annotations.as_ref(),
            Self::ClusterRole(d) => d.annotations.as_ref(),
            Self::RoleBinding(d) => d.annotations.as_ref(),
            Self::ClusterRoleBinding(d) => d.annotations.as_ref(),
            Self::ServiceAccount(d) => d.annotations.as_ref(),
            Self::HorizontalPodAutoscaler(d) => d.annotations.as_ref(),
            Self::PodDisruptionBudget(d) => d.annotations.as_ref(),
            Self::PersistentVolumeClaim(d) => d.annotations.as_ref(),
            Self::Unknown(d) => d.annotations.as_ref(),
        }
    }
}

// ============================================================================
// Data structures for each K8s object type
// These are simplified representations; full k8s-openapi types will be used
// in the actual implementation
// ============================================================================

/// Common metadata fields.
#[derive(Debug, Clone, Default)]
pub struct CommonMeta {
    pub name: String,
    pub namespace: Option<String>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
}

/// Simplified container spec.
#[derive(Debug, Clone, Default)]
pub struct ContainerSpec {
    pub name: String,
    pub image: Option<String>,
    pub security_context: Option<SecurityContext>,
    pub resources: Option<ResourceRequirements>,
    pub liveness_probe: Option<Probe>,
    pub readiness_probe: Option<Probe>,
    pub startup_probe: Option<Probe>,
    pub env: Vec<EnvVar>,
    pub volume_mounts: Vec<VolumeMount>,
    pub ports: Vec<ContainerPort>,
}

/// Security context for containers/pods.
#[derive(Debug, Clone, Default)]
pub struct SecurityContext {
    pub privileged: Option<bool>,
    pub allow_privilege_escalation: Option<bool>,
    pub run_as_non_root: Option<bool>,
    pub run_as_user: Option<i64>,
    pub read_only_root_filesystem: Option<bool>,
    pub capabilities: Option<Capabilities>,
    pub proc_mount: Option<String>,
}

/// Linux capabilities.
#[derive(Debug, Clone, Default)]
pub struct Capabilities {
    pub add: Vec<String>,
    pub drop: Vec<String>,
}

/// Resource requirements.
#[derive(Debug, Clone, Default)]
pub struct ResourceRequirements {
    pub limits: Option<std::collections::BTreeMap<String, String>>,
    pub requests: Option<std::collections::BTreeMap<String, String>>,
}

/// Probe configuration.
#[derive(Debug, Clone, Default)]
pub struct Probe {
    pub http_get: Option<HttpGetAction>,
    pub tcp_socket: Option<TcpSocketAction>,
    pub exec: Option<ExecAction>,
}

#[derive(Debug, Clone, Default)]
pub struct HttpGetAction {
    pub port: i32,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TcpSocketAction {
    pub port: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ExecAction {
    pub command: Vec<String>,
}

/// Environment variable.
#[derive(Debug, Clone, Default)]
pub struct EnvVar {
    pub name: String,
    pub value: Option<String>,
    pub value_from: Option<EnvVarSource>,
}

#[derive(Debug, Clone)]
pub enum EnvVarSource {
    SecretKeyRef { name: String, key: String },
    ConfigMapKeyRef { name: String, key: String },
    FieldRef { field_path: String },
}

/// Volume mount.
#[derive(Debug, Clone, Default)]
pub struct VolumeMount {
    pub name: String,
    pub mount_path: String,
    pub read_only: Option<bool>,
}

/// Container port.
#[derive(Debug, Clone, Default)]
pub struct ContainerPort {
    pub container_port: i32,
    pub protocol: Option<String>,
    pub host_port: Option<i32>,
}

/// Pod spec (simplified).
#[derive(Debug, Clone, Default)]
pub struct PodSpec {
    pub containers: Vec<ContainerSpec>,
    pub init_containers: Vec<ContainerSpec>,
    pub volumes: Vec<Volume>,
    pub service_account_name: Option<String>,
    pub host_network: Option<bool>,
    pub host_pid: Option<bool>,
    pub host_ipc: Option<bool>,
    pub security_context: Option<PodSecurityContext>,
    pub affinity: Option<Affinity>,
    pub dns_config: Option<DnsConfig>,
    pub restart_policy: Option<String>,
    pub priority_class_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PodSecurityContext {
    pub run_as_non_root: Option<bool>,
    pub run_as_user: Option<i64>,
    pub sysctls: Vec<Sysctl>,
}

#[derive(Debug, Clone, Default)]
pub struct Sysctl {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default)]
pub struct Volume {
    pub name: String,
    pub host_path: Option<HostPathVolumeSource>,
    pub secret: Option<SecretVolumeSource>,
}

#[derive(Debug, Clone, Default)]
pub struct HostPathVolumeSource {
    pub path: String,
    pub type_: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SecretVolumeSource {
    pub secret_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Affinity {
    pub pod_anti_affinity: Option<PodAntiAffinity>,
    pub node_affinity: Option<NodeAffinity>,
}

#[derive(Debug, Clone, Default)]
pub struct PodAntiAffinity {
    pub required_during_scheduling_ignored_during_execution: Vec<PodAffinityTerm>,
    pub preferred_during_scheduling_ignored_during_execution: Vec<WeightedPodAffinityTerm>,
}

#[derive(Debug, Clone, Default)]
pub struct PodAffinityTerm {
    pub topology_key: String,
}

#[derive(Debug, Clone, Default)]
pub struct WeightedPodAffinityTerm {
    pub weight: i32,
    pub pod_affinity_term: PodAffinityTerm,
}

#[derive(Debug, Clone, Default)]
pub struct NodeAffinity {
    pub required_during_scheduling_ignored_during_execution: Option<NodeSelector>,
}

#[derive(Debug, Clone, Default)]
pub struct NodeSelector {
    pub node_selector_terms: Vec<NodeSelectorTerm>,
}

#[derive(Debug, Clone, Default)]
pub struct NodeSelectorTerm {
    pub match_expressions: Vec<NodeSelectorRequirement>,
}

#[derive(Debug, Clone, Default)]
pub struct NodeSelectorRequirement {
    pub key: String,
    pub operator: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DnsConfig {
    pub options: Vec<PodDnsConfigOption>,
}

#[derive(Debug, Clone, Default)]
pub struct PodDnsConfigOption {
    pub name: Option<String>,
    pub value: Option<String>,
}

// ============================================================================
// Object data types
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct DeploymentData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub replicas: Option<i32>,
    pub selector: Option<LabelSelector>,
    pub pod_spec: Option<PodSpec>,
    pub strategy: Option<DeploymentStrategy>,
}

#[derive(Debug, Clone, Default)]
pub struct LabelSelector {
    pub match_labels: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Default)]
pub struct DeploymentStrategy {
    pub type_: Option<String>,
    pub rolling_update: Option<RollingUpdateDeployment>,
}

#[derive(Debug, Clone, Default)]
pub struct RollingUpdateDeployment {
    pub max_unavailable: Option<String>,
    pub max_surge: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct StatefulSetData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub replicas: Option<i32>,
    pub selector: Option<LabelSelector>,
    pub pod_spec: Option<PodSpec>,
}

#[derive(Debug, Clone, Default)]
pub struct DaemonSetData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub selector: Option<LabelSelector>,
    pub pod_spec: Option<PodSpec>,
    pub update_strategy: Option<DaemonSetUpdateStrategy>,
}

#[derive(Debug, Clone, Default)]
pub struct DaemonSetUpdateStrategy {
    pub type_: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ReplicaSetData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub replicas: Option<i32>,
    pub selector: Option<LabelSelector>,
    pub pod_spec: Option<PodSpec>,
}

#[derive(Debug, Clone, Default)]
pub struct PodData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub spec: Option<PodSpec>,
}

#[derive(Debug, Clone, Default)]
pub struct JobData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub pod_spec: Option<PodSpec>,
    pub ttl_seconds_after_finished: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct CronJobData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub job_spec: Option<JobData>,
}

#[derive(Debug, Clone, Default)]
pub struct ServiceData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub selector: Option<std::collections::BTreeMap<String, String>>,
    pub ports: Vec<ServicePort>,
    pub type_: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ServicePort {
    pub port: i32,
    pub target_port: Option<String>,
    pub protocol: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct IngressData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub rules: Vec<IngressRule>,
}

#[derive(Debug, Clone, Default)]
pub struct IngressRule {
    pub host: Option<String>,
    pub http: Option<HttpIngressRuleValue>,
}

#[derive(Debug, Clone, Default)]
pub struct HttpIngressRuleValue {
    pub paths: Vec<HttpIngressPath>,
}

#[derive(Debug, Clone, Default)]
pub struct HttpIngressPath {
    pub path: Option<String>,
    pub backend: IngressBackend,
}

#[derive(Debug, Clone, Default)]
pub struct IngressBackend {
    pub service: Option<IngressServiceBackend>,
}

#[derive(Debug, Clone, Default)]
pub struct IngressServiceBackend {
    pub name: String,
    pub port: Option<ServiceBackendPort>,
}

#[derive(Debug, Clone, Default)]
pub struct ServiceBackendPort {
    pub number: Option<i32>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkPolicyData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub pod_selector: Option<LabelSelector>,
}

#[derive(Debug, Clone, Default)]
pub struct RoleData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Default)]
pub struct ClusterRoleData {
    pub name: String,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Default)]
pub struct PolicyRule {
    pub api_groups: Vec<String>,
    pub resources: Vec<String>,
    pub verbs: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RoleBindingData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub role_ref: RoleRef,
    pub subjects: Vec<Subject>,
}

#[derive(Debug, Clone, Default)]
pub struct ClusterRoleBindingData {
    pub name: String,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub role_ref: RoleRef,
    pub subjects: Vec<Subject>,
}

#[derive(Debug, Clone, Default)]
pub struct RoleRef {
    pub api_group: String,
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct Subject {
    pub kind: String,
    pub name: String,
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ServiceAccountData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Default)]
pub struct HpaData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub min_replicas: Option<i32>,
    pub max_replicas: i32,
    pub scale_target_ref: CrossVersionObjectReference,
}

#[derive(Debug, Clone, Default)]
pub struct CrossVersionObjectReference {
    pub api_version: Option<String>,
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct PdbData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub min_available: Option<String>,
    pub max_unavailable: Option<String>,
    pub selector: Option<LabelSelector>,
    pub unhealthy_pod_eviction_policy: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PvcData {
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Default)]
pub struct UnknownObject {
    pub api_version: String,
    pub kind: String,
    pub name: String,
    pub namespace: Option<String>,
    pub annotations: Option<std::collections::BTreeMap<String, String>>,
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    pub raw: serde_yaml::Value,
}
