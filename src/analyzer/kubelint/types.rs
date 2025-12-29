//! Core types for the kubelint-rs linter.
//!
//! These types match the Go kube-linter implementation for compatibility:
//! - `Severity` - Check violation severity levels
//! - `RuleCode` - Check identifiers (e.g., "privileged-container")
//! - `CheckFailure` - A single check violation
//! - `Diagnostic` - A diagnostic message from a check

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::path::PathBuf;

/// Severity levels for check violations.
///
/// Ordered from most severe to least severe:
/// `Error > Warning > Info`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Critical issues that must be fixed
    Error,
    /// Important issues that should be addressed
    #[default]
    Warning,
    /// Informational suggestions
    Info,
}

impl Severity {
    /// Parse a severity from a string (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "error" => Some(Self::Error),
            "warning" => Some(Self::Warning),
            "info" => Some(Self::Info),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Ord for Severity {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher severity = lower numeric value for Ord
        let self_val = match self {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Info => 2,
        };
        let other_val = match other {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Info => 2,
        };
        // Reverse so Error > Warning > Info
        other_val.cmp(&self_val)
    }
}

impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A rule/check code identifier (e.g., "privileged-container", "latest-tag").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuleCode(pub String);

impl RuleCode {
    /// Create a new rule code.
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    /// Get the code as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if this is a security-related check.
    pub fn is_security_check(&self) -> bool {
        const SECURITY_CHECKS: &[&str] = &[
            "privileged-container",
            "privilege-escalation",
            "run-as-non-root",
            "read-only-root-fs",
            "drop-net-raw-capability",
            "hostnetwork",
            "hostpid",
            "hostipc",
            "host-mounts",
            "writable-host-mount",
            "docker-sock",
            "unsafe-proc-mount",
            "access-to-secrets",
            "access-to-create-pods",
            "cluster-admin-role-binding",
            "wildcard-in-rules",
        ];
        SECURITY_CHECKS.contains(&self.0.as_str())
    }

    /// Check if this is a best practice check.
    pub fn is_best_practice_check(&self) -> bool {
        const BEST_PRACTICE_CHECKS: &[&str] = &[
            "latest-tag",
            "no-liveness-probe",
            "no-readiness-probe",
            "unset-cpu-requirements",
            "unset-memory-requirements",
            "minimum-replicas",
            "no-anti-affinity",
            "no-rolling-update-strategy",
            "default-service-account",
        ];
        BEST_PRACTICE_CHECKS.contains(&self.0.as_str())
    }
}

impl fmt::Display for RuleCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for RuleCode {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for RuleCode {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for RuleCode {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A diagnostic message produced by a check.
///
/// This is the raw output from a check function before it's
/// enriched with context information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// The diagnostic message describing the issue.
    pub message: String,
    /// Optional remediation advice.
    pub remediation: Option<String>,
}

impl Diagnostic {
    /// Create a new diagnostic with just a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            remediation: None,
        }
    }

    /// Create a diagnostic with message and remediation.
    pub fn with_remediation(message: impl Into<String>, remediation: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            remediation: Some(remediation.into()),
        }
    }
}

impl From<String> for Diagnostic {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for Diagnostic {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

/// A check failure (rule violation) found during linting.
///
/// This is the enriched form of a diagnostic, including context
/// about which object and file triggered the failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckFailure {
    /// The check code that was violated.
    pub code: RuleCode,
    /// The severity of the violation.
    pub severity: Severity,
    /// A human-readable message describing the violation.
    pub message: String,
    /// The file path where the violation occurred.
    pub file_path: PathBuf,
    /// The name of the Kubernetes object.
    pub object_name: String,
    /// The kind of the Kubernetes object (e.g., "Deployment", "Service").
    pub object_kind: String,
    /// The namespace of the object (if applicable).
    pub object_namespace: Option<String>,
    /// Optional line number (1-indexed).
    pub line: Option<u32>,
    /// Optional remediation advice.
    pub remediation: Option<String>,
}

impl CheckFailure {
    /// Create a new check failure.
    pub fn new(
        code: impl Into<RuleCode>,
        severity: Severity,
        message: impl Into<String>,
        file_path: impl Into<PathBuf>,
        object_name: impl Into<String>,
        object_kind: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            file_path: file_path.into(),
            object_name: object_name.into(),
            object_kind: object_kind.into(),
            object_namespace: None,
            line: None,
            remediation: None,
        }
    }

    /// Set the namespace.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.object_namespace = Some(namespace.into());
        self
    }

    /// Set the line number.
    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    /// Set remediation advice.
    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(remediation.into());
        self
    }

    /// Get a full identifier for the object (namespace/name or just name).
    pub fn object_identifier(&self) -> String {
        match &self.object_namespace {
            Some(ns) => format!("{}/{}", ns, self.object_name),
            None => self.object_name.clone(),
        }
    }
}

impl Ord for CheckFailure {
    fn cmp(&self, other: &Self) -> Ordering {
        // Sort by file path, then by line number, then by severity
        match self.file_path.cmp(&other.file_path) {
            Ordering::Equal => match (self.line, other.line) {
                (Some(a), Some(b)) => match a.cmp(&b) {
                    Ordering::Equal => self.severity.cmp(&other.severity),
                    other => other,
                },
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => self.severity.cmp(&other.severity),
            },
            other => other,
        }
    }
}

impl PartialOrd for CheckFailure {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Object kinds that kube-linter can analyze.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectKind {
    // Core workloads
    Deployment,
    StatefulSet,
    DaemonSet,
    ReplicaSet,
    Pod,
    Job,
    CronJob,

    // Services & Networking
    Service,
    Ingress,
    NetworkPolicy,

    // RBAC
    Role,
    ClusterRole,
    RoleBinding,
    ClusterRoleBinding,
    ServiceAccount,

    // Scaling & Disruption
    HorizontalPodAutoscaler,
    PodDisruptionBudget,

    // Storage
    PersistentVolumeClaim,

    // OpenShift specific
    DeploymentConfig,
    SecurityContextConstraints,

    // Monitoring
    ServiceMonitor,

    // KEDA
    ScaledObject,

    // Any/Unknown
    Any,
}

impl ObjectKind {
    /// Get the string representation matching Kubernetes kind names.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deployment => "Deployment",
            Self::StatefulSet => "StatefulSet",
            Self::DaemonSet => "DaemonSet",
            Self::ReplicaSet => "ReplicaSet",
            Self::Pod => "Pod",
            Self::Job => "Job",
            Self::CronJob => "CronJob",
            Self::Service => "Service",
            Self::Ingress => "Ingress",
            Self::NetworkPolicy => "NetworkPolicy",
            Self::Role => "Role",
            Self::ClusterRole => "ClusterRole",
            Self::RoleBinding => "RoleBinding",
            Self::ClusterRoleBinding => "ClusterRoleBinding",
            Self::ServiceAccount => "ServiceAccount",
            Self::HorizontalPodAutoscaler => "HorizontalPodAutoscaler",
            Self::PodDisruptionBudget => "PodDisruptionBudget",
            Self::PersistentVolumeClaim => "PersistentVolumeClaim",
            Self::DeploymentConfig => "DeploymentConfig",
            Self::SecurityContextConstraints => "SecurityContextConstraints",
            Self::ServiceMonitor => "ServiceMonitor",
            Self::ScaledObject => "ScaledObject",
            Self::Any => "Any",
        }
    }

    /// Parse from a Kubernetes kind string.
    pub fn from_kind(kind: &str) -> Option<Self> {
        match kind {
            "Deployment" => Some(Self::Deployment),
            "StatefulSet" => Some(Self::StatefulSet),
            "DaemonSet" => Some(Self::DaemonSet),
            "ReplicaSet" => Some(Self::ReplicaSet),
            "Pod" => Some(Self::Pod),
            "Job" => Some(Self::Job),
            "CronJob" => Some(Self::CronJob),
            "Service" => Some(Self::Service),
            "Ingress" => Some(Self::Ingress),
            "NetworkPolicy" => Some(Self::NetworkPolicy),
            "Role" => Some(Self::Role),
            "ClusterRole" => Some(Self::ClusterRole),
            "RoleBinding" => Some(Self::RoleBinding),
            "ClusterRoleBinding" => Some(Self::ClusterRoleBinding),
            "ServiceAccount" => Some(Self::ServiceAccount),
            "HorizontalPodAutoscaler" => Some(Self::HorizontalPodAutoscaler),
            "PodDisruptionBudget" => Some(Self::PodDisruptionBudget),
            "PersistentVolumeClaim" => Some(Self::PersistentVolumeClaim),
            "DeploymentConfig" => Some(Self::DeploymentConfig),
            "SecurityContextConstraints" => Some(Self::SecurityContextConstraints),
            "ServiceMonitor" => Some(Self::ServiceMonitor),
            "ScaledObject" => Some(Self::ScaledObject),
            _ => None,
        }
    }

    /// Check if this kind is "DeploymentLike" (has a PodSpec).
    pub fn is_deployment_like(&self) -> bool {
        matches!(
            self,
            Self::Deployment
                | Self::StatefulSet
                | Self::DaemonSet
                | Self::ReplicaSet
                | Self::Pod
                | Self::Job
                | Self::CronJob
                | Self::DeploymentConfig
        )
    }

    /// Check if this kind is "JobLike".
    pub fn is_job_like(&self) -> bool {
        matches!(self, Self::Job | Self::CronJob)
    }
}

impl fmt::Display for ObjectKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Describes which object kinds a check applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectKindsDesc {
    /// List of object kind identifiers.
    /// Can include specific kinds or group names like "DeploymentLike".
    pub object_kinds: Vec<String>,
}

impl ObjectKindsDesc {
    /// Create a new object kinds description.
    pub fn new(kinds: &[&str]) -> Self {
        Self {
            object_kinds: kinds.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Check if the given kind matches this description.
    pub fn matches(&self, kind: &ObjectKind) -> bool {
        // Empty list means "DeploymentLike" (for DEPLOYMENT_LIKE const)
        if self.object_kinds.is_empty() {
            return kind.is_deployment_like();
        }

        for k in &self.object_kinds {
            match k.as_str() {
                "DeploymentLike" if kind.is_deployment_like() => return true,
                "JobLike" if kind.is_job_like() => return true,
                "Any" => return true,
                _ if k == kind.as_str() => return true,
                _ => continue,
            }
        }
        false
    }
}

impl Default for ObjectKindsDesc {
    fn default() -> Self {
        Self {
            object_kinds: vec!["DeploymentLike".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
    }

    #[test]
    fn test_severity_from_str() {
        assert_eq!(Severity::parse("error"), Some(Severity::Error));
        assert_eq!(Severity::parse("WARNING"), Some(Severity::Warning));
        assert_eq!(Severity::parse("Info"), Some(Severity::Info));
        assert_eq!(Severity::parse("invalid"), None);
    }

    #[test]
    fn test_rule_code() {
        let code = RuleCode::new("privileged-container");
        assert!(code.is_security_check());
        assert!(!code.is_best_practice_check());

        let code = RuleCode::new("latest-tag");
        assert!(!code.is_security_check());
        assert!(code.is_best_practice_check());
    }

    #[test]
    fn test_check_failure_ordering() {
        let f1 = CheckFailure::new(
            "check1",
            Severity::Warning,
            "msg1",
            "a.yaml",
            "obj1",
            "Deployment",
        )
        .with_line(10);
        let f2 = CheckFailure::new(
            "check2",
            Severity::Error,
            "msg2",
            "a.yaml",
            "obj2",
            "Service",
        )
        .with_line(5);
        let f3 = CheckFailure::new(
            "check3",
            Severity::Info,
            "msg3",
            "b.yaml",
            "obj3",
            "Pod",
        );

        let mut failures = vec![f1.clone(), f2.clone(), f3.clone()];
        failures.sort();

        // Should be sorted by file, then line
        assert_eq!(failures[0].file_path.to_str(), Some("a.yaml"));
        assert_eq!(failures[0].line, Some(5));
        assert_eq!(failures[1].file_path.to_str(), Some("a.yaml"));
        assert_eq!(failures[1].line, Some(10));
        assert_eq!(failures[2].file_path.to_str(), Some("b.yaml"));
    }

    #[test]
    fn test_object_kind_matching() {
        let desc = ObjectKindsDesc::new(&["DeploymentLike"]);
        assert!(desc.matches(&ObjectKind::Deployment));
        assert!(desc.matches(&ObjectKind::StatefulSet));
        assert!(desc.matches(&ObjectKind::DaemonSet));
        assert!(desc.matches(&ObjectKind::Job));
        assert!(!desc.matches(&ObjectKind::Service));

        let desc = ObjectKindsDesc::new(&["Service", "Ingress"]);
        assert!(desc.matches(&ObjectKind::Service));
        assert!(desc.matches(&ObjectKind::Ingress));
        assert!(!desc.matches(&ObjectKind::Deployment));
    }

    #[test]
    fn test_diagnostic() {
        let d = Diagnostic::new("container is privileged");
        assert_eq!(d.message, "container is privileged");
        assert!(d.remediation.is_none());

        let d = Diagnostic::with_remediation("issue", "fix it");
        assert_eq!(d.message, "issue");
        assert_eq!(d.remediation, Some("fix it".to_string()));
    }
}
