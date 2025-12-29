//! Kubernetes API version tracking and deprecation detection.
//!
//! Tracks deprecated Kubernetes APIs and their replacements.

use std::collections::HashMap;

/// Kubernetes version as (major, minor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct K8sVersion {
    pub major: u32,
    pub minor: u32,
}

impl K8sVersion {
    pub fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Parse from string like "1.25" or "v1.25".
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('v');
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() >= 2 {
            let major = parts[0].parse().ok()?;
            let minor = parts[1].parse().ok()?;
            Some(Self { major, minor })
        } else {
            None
        }
    }
}

impl std::fmt::Display for K8sVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// Information about a deprecated API.
#[derive(Debug, Clone)]
pub struct DeprecatedApi {
    /// The deprecated API version (e.g., "extensions/v1beta1")
    pub api_version: &'static str,
    /// The kind this deprecation applies to (e.g., "Deployment")
    pub kind: Option<&'static str>,
    /// The replacement API version
    pub replacement: &'static str,
    /// Kubernetes version where this was deprecated
    pub deprecated_in: K8sVersion,
    /// Kubernetes version where this was removed
    pub removed_in: K8sVersion,
    /// Additional notes
    pub notes: Option<&'static str>,
}

/// Static list of deprecated Kubernetes APIs.
static DEPRECATED_APIS: &[DeprecatedApi] = &[
    // extensions/v1beta1 deprecations
    DeprecatedApi {
        api_version: "extensions/v1beta1",
        kind: Some("Deployment"),
        replacement: "apps/v1",
        deprecated_in: K8sVersion { major: 1, minor: 9 },
        removed_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "extensions/v1beta1",
        kind: Some("DaemonSet"),
        replacement: "apps/v1",
        deprecated_in: K8sVersion { major: 1, minor: 9 },
        removed_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "extensions/v1beta1",
        kind: Some("ReplicaSet"),
        replacement: "apps/v1",
        deprecated_in: K8sVersion { major: 1, minor: 9 },
        removed_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "extensions/v1beta1",
        kind: Some("Ingress"),
        replacement: "networking.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 14,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "extensions/v1beta1",
        kind: Some("NetworkPolicy"),
        replacement: "networking.k8s.io/v1",
        deprecated_in: K8sVersion { major: 1, minor: 9 },
        removed_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "extensions/v1beta1",
        kind: Some("PodSecurityPolicy"),
        replacement: "policy/v1beta1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 10,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        notes: Some("PodSecurityPolicy is deprecated entirely in 1.21 and removed in 1.25"),
    },
    // apps/v1beta1 deprecations
    DeprecatedApi {
        api_version: "apps/v1beta1",
        kind: Some("Deployment"),
        replacement: "apps/v1",
        deprecated_in: K8sVersion { major: 1, minor: 9 },
        removed_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "apps/v1beta1",
        kind: Some("StatefulSet"),
        replacement: "apps/v1",
        deprecated_in: K8sVersion { major: 1, minor: 9 },
        removed_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        notes: None,
    },
    // apps/v1beta2 deprecations
    DeprecatedApi {
        api_version: "apps/v1beta2",
        kind: Some("Deployment"),
        replacement: "apps/v1",
        deprecated_in: K8sVersion { major: 1, minor: 9 },
        removed_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "apps/v1beta2",
        kind: Some("DaemonSet"),
        replacement: "apps/v1",
        deprecated_in: K8sVersion { major: 1, minor: 9 },
        removed_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "apps/v1beta2",
        kind: Some("ReplicaSet"),
        replacement: "apps/v1",
        deprecated_in: K8sVersion { major: 1, minor: 9 },
        removed_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "apps/v1beta2",
        kind: Some("StatefulSet"),
        replacement: "apps/v1",
        deprecated_in: K8sVersion { major: 1, minor: 9 },
        removed_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        notes: None,
    },
    // networking.k8s.io/v1beta1 deprecations
    DeprecatedApi {
        api_version: "networking.k8s.io/v1beta1",
        kind: Some("Ingress"),
        replacement: "networking.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 19,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "networking.k8s.io/v1beta1",
        kind: Some("IngressClass"),
        replacement: "networking.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 19,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: None,
    },
    // rbac.authorization.k8s.io/v1beta1 deprecations
    DeprecatedApi {
        api_version: "rbac.authorization.k8s.io/v1beta1",
        kind: None,
        replacement: "rbac.authorization.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 17,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: Some("Applies to Role, ClusterRole, RoleBinding, ClusterRoleBinding"),
    },
    // admissionregistration.k8s.io/v1beta1 deprecations
    DeprecatedApi {
        api_version: "admissionregistration.k8s.io/v1beta1",
        kind: None,
        replacement: "admissionregistration.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: Some("Applies to MutatingWebhookConfiguration, ValidatingWebhookConfiguration"),
    },
    // apiextensions.k8s.io/v1beta1 deprecations
    DeprecatedApi {
        api_version: "apiextensions.k8s.io/v1beta1",
        kind: Some("CustomResourceDefinition"),
        replacement: "apiextensions.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 16,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: None,
    },
    // policy/v1beta1 deprecations
    DeprecatedApi {
        api_version: "policy/v1beta1",
        kind: Some("PodDisruptionBudget"),
        replacement: "policy/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 21,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 25,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "policy/v1beta1",
        kind: Some("PodSecurityPolicy"),
        replacement: "None (use Pod Security Admission)",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 21,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 25,
        },
        notes: Some("PodSecurityPolicy is removed. Use Pod Security Admission instead"),
    },
    // batch/v1beta1 deprecations
    DeprecatedApi {
        api_version: "batch/v1beta1",
        kind: Some("CronJob"),
        replacement: "batch/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 21,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 25,
        },
        notes: None,
    },
    // certificates.k8s.io/v1beta1 deprecations
    DeprecatedApi {
        api_version: "certificates.k8s.io/v1beta1",
        kind: Some("CertificateSigningRequest"),
        replacement: "certificates.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 19,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: None,
    },
    // coordination.k8s.io/v1beta1 deprecations
    DeprecatedApi {
        api_version: "coordination.k8s.io/v1beta1",
        kind: Some("Lease"),
        replacement: "coordination.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 14,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: None,
    },
    // storage.k8s.io/v1beta1 deprecations
    DeprecatedApi {
        api_version: "storage.k8s.io/v1beta1",
        kind: Some("CSIDriver"),
        replacement: "storage.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 19,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "storage.k8s.io/v1beta1",
        kind: Some("CSINode"),
        replacement: "storage.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 17,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "storage.k8s.io/v1beta1",
        kind: Some("StorageClass"),
        replacement: "storage.k8s.io/v1",
        deprecated_in: K8sVersion { major: 1, minor: 6 },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: None,
    },
    DeprecatedApi {
        api_version: "storage.k8s.io/v1beta1",
        kind: Some("VolumeAttachment"),
        replacement: "storage.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 13,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: None,
    },
    // scheduling.k8s.io/v1beta1 deprecations
    DeprecatedApi {
        api_version: "scheduling.k8s.io/v1beta1",
        kind: Some("PriorityClass"),
        replacement: "scheduling.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 14,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 22,
        },
        notes: None,
    },
    // discovery.k8s.io/v1beta1 deprecations
    DeprecatedApi {
        api_version: "discovery.k8s.io/v1beta1",
        kind: Some("EndpointSlice"),
        replacement: "discovery.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 21,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 25,
        },
        notes: None,
    },
    // events.k8s.io/v1beta1 deprecations
    DeprecatedApi {
        api_version: "events.k8s.io/v1beta1",
        kind: Some("Event"),
        replacement: "events.k8s.io/v1",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 19,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 25,
        },
        notes: None,
    },
    // autoscaling/v2beta1 deprecations
    DeprecatedApi {
        api_version: "autoscaling/v2beta1",
        kind: Some("HorizontalPodAutoscaler"),
        replacement: "autoscaling/v2",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 23,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 26,
        },
        notes: None,
    },
    // autoscaling/v2beta2 deprecations
    DeprecatedApi {
        api_version: "autoscaling/v2beta2",
        kind: Some("HorizontalPodAutoscaler"),
        replacement: "autoscaling/v2",
        deprecated_in: K8sVersion {
            major: 1,
            minor: 23,
        },
        removed_in: K8sVersion {
            major: 1,
            minor: 26,
        },
        notes: None,
    },
];

/// Check if an API version is deprecated for a given kind.
pub fn is_api_deprecated(api_version: &str, kind: Option<&str>) -> Option<&'static DeprecatedApi> {
    DEPRECATED_APIS
        .iter()
        .find(|api| api.api_version == api_version && (api.kind.is_none() || api.kind == kind))
}

/// Get the replacement API for a deprecated API.
pub fn get_replacement_api(api_version: &str, kind: Option<&str>) -> Option<&'static str> {
    is_api_deprecated(api_version, kind).map(|api| api.replacement)
}

/// Check if an API is deprecated in a specific Kubernetes version.
pub fn is_api_deprecated_in_version(
    api_version: &str,
    kind: Option<&str>,
    k8s_version: K8sVersion,
) -> Option<&'static DeprecatedApi> {
    DEPRECATED_APIS.iter().find(|api| {
        api.api_version == api_version
            && (api.kind.is_none() || api.kind == kind)
            && k8s_version >= api.deprecated_in
    })
}

/// Check if an API is removed in a specific Kubernetes version.
pub fn is_api_removed_in_version(
    api_version: &str,
    kind: Option<&str>,
    k8s_version: K8sVersion,
) -> Option<&'static DeprecatedApi> {
    DEPRECATED_APIS.iter().find(|api| {
        api.api_version == api_version
            && (api.kind.is_none() || api.kind == kind)
            && k8s_version >= api.removed_in
    })
}

/// Build a map of deprecated APIs for quick lookup.
pub fn build_deprecation_map() -> HashMap<String, Vec<&'static DeprecatedApi>> {
    let mut map: HashMap<String, Vec<&'static DeprecatedApi>> = HashMap::new();
    for api in DEPRECATED_APIS {
        map.entry(api.api_version.to_string())
            .or_default()
            .push(api);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k8s_version_parse() {
        assert_eq!(K8sVersion::parse("1.25"), Some(K8sVersion::new(1, 25)));
        assert_eq!(K8sVersion::parse("v1.28"), Some(K8sVersion::new(1, 28)));
        assert_eq!(K8sVersion::parse("invalid"), None);
    }

    #[test]
    fn test_k8s_version_ordering() {
        assert!(K8sVersion::new(1, 25) > K8sVersion::new(1, 20));
        assert!(K8sVersion::new(1, 25) < K8sVersion::new(1, 26));
        assert!(K8sVersion::new(1, 25) == K8sVersion::new(1, 25));
    }

    #[test]
    fn test_is_api_deprecated() {
        // Test known deprecated API
        let result = is_api_deprecated("extensions/v1beta1", Some("Deployment"));
        assert!(result.is_some());
        let api = result.unwrap();
        assert_eq!(api.replacement, "apps/v1");

        // Test non-deprecated API
        let result = is_api_deprecated("apps/v1", Some("Deployment"));
        assert!(result.is_none());
    }

    #[test]
    fn test_get_replacement_api() {
        assert_eq!(
            get_replacement_api("extensions/v1beta1", Some("Deployment")),
            Some("apps/v1")
        );
        assert_eq!(
            get_replacement_api("networking.k8s.io/v1beta1", Some("Ingress")),
            Some("networking.k8s.io/v1")
        );
        assert_eq!(get_replacement_api("apps/v1", Some("Deployment")), None);
    }

    #[test]
    fn test_deprecated_in_version() {
        // extensions/v1beta1 Deployment deprecated in 1.9
        let result = is_api_deprecated_in_version(
            "extensions/v1beta1",
            Some("Deployment"),
            K8sVersion::new(1, 10),
        );
        assert!(result.is_some());

        let result = is_api_deprecated_in_version(
            "extensions/v1beta1",
            Some("Deployment"),
            K8sVersion::new(1, 8),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_removed_in_version() {
        // extensions/v1beta1 Deployment removed in 1.16
        let result = is_api_removed_in_version(
            "extensions/v1beta1",
            Some("Deployment"),
            K8sVersion::new(1, 16),
        );
        assert!(result.is_some());

        let result = is_api_removed_in_version(
            "extensions/v1beta1",
            Some("Deployment"),
            K8sVersion::new(1, 15),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_build_deprecation_map() {
        let map = build_deprecation_map();
        assert!(map.contains_key("extensions/v1beta1"));
        assert!(map.contains_key("apps/v1beta1"));
        assert!(!map.contains_key("apps/v1"));
    }
}
