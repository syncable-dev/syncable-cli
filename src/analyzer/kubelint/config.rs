//! Configuration for the kubelint-rs linter.
//!
//! Provides configuration options matching the Go kube-linter:
//! - Check inclusion/exclusion
//! - Path ignoring
//! - Custom check definitions
//! - Failure thresholds

use crate::analyzer::kubelint::types::{ObjectKindsDesc, Severity};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// Configuration for the KubeLint linter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KubelintConfig {
    /// If true, add all built-in checks regardless of defaults.
    #[serde(default, rename = "addAllBuiltIn")]
    pub add_all_builtin: bool,

    /// If true, do not automatically add default checks.
    #[serde(default)]
    pub do_not_auto_add_defaults: bool,

    /// List of check names to include (in addition to defaults).
    #[serde(default)]
    pub include: Vec<String>,

    /// List of check names to exclude.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Glob patterns for paths to ignore.
    #[serde(default)]
    pub ignore_paths: Vec<String>,

    /// Custom check definitions.
    #[serde(default)]
    pub custom_checks: Vec<CheckSpec>,

    /// Minimum severity to report. Checks below this threshold are filtered.
    #[serde(default)]
    pub failure_threshold: Severity,

    /// If true, never return a non-zero exit code.
    #[serde(default)]
    pub no_fail: bool,
}

impl Default for KubelintConfig {
    fn default() -> Self {
        Self {
            add_all_builtin: false,
            do_not_auto_add_defaults: false,
            include: Vec::new(),
            exclude: Vec::new(),
            ignore_paths: Vec::new(),
            custom_checks: Vec::new(),
            failure_threshold: Severity::Warning,
            no_fail: false,
        }
    }
}

impl KubelintConfig {
    /// Create a new default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a check to the include list.
    pub fn include(mut self, check: impl Into<String>) -> Self {
        self.include.push(check.into());
        self
    }

    /// Add a check to the exclude list.
    pub fn exclude(mut self, check: impl Into<String>) -> Self {
        self.exclude.push(check.into());
        self
    }

    /// Add a path pattern to ignore.
    pub fn ignore_path(mut self, pattern: impl Into<String>) -> Self {
        self.ignore_paths.push(pattern.into());
        self
    }

    /// Set the failure threshold.
    pub fn with_threshold(mut self, threshold: Severity) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Enable all built-in checks.
    pub fn with_all_builtin(mut self) -> Self {
        self.add_all_builtin = true;
        self
    }

    /// Disable automatic default checks.
    pub fn without_defaults(mut self) -> Self {
        self.do_not_auto_add_defaults = true;
        self
    }

    /// Check if a check is explicitly excluded.
    pub fn is_check_excluded(&self, check_name: &str) -> bool {
        self.exclude.iter().any(|e| e == check_name)
    }

    /// Check if a check is explicitly included.
    pub fn is_check_included(&self, check_name: &str) -> bool {
        self.include.iter().any(|e| e == check_name)
    }

    /// Get the effective set of check names to run.
    ///
    /// This resolves includes/excludes against the available checks.
    pub fn resolve_checks<'a>(&self, available: &'a [CheckSpec]) -> Vec<&'a CheckSpec> {
        let default_checks: HashSet<&str> = DEFAULT_CHECKS.iter().copied().collect();

        available
            .iter()
            .filter(|check| {
                let name = check.name.as_str();

                // Explicitly excluded checks are always skipped
                if self.is_check_excluded(name) {
                    return false;
                }

                // Explicitly included checks are always included
                if self.is_check_included(name) {
                    return true;
                }

                // If add_all_builtin is set, include all
                if self.add_all_builtin {
                    return true;
                }

                // If not suppressing defaults, include default checks
                if !self.do_not_auto_add_defaults && default_checks.contains(name) {
                    return true;
                }

                false
            })
            .collect()
    }

    /// Check if a file path should be ignored based on ignore_paths patterns.
    pub fn should_ignore_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.ignore_paths {
            if let Ok(glob) = glob::Pattern::new(pattern) {
                if glob.matches(&path_str) {
                    return true;
                }
            }
            // Also check simple prefix/suffix matches
            if path_str.contains(pattern) {
                return true;
            }
        }
        false
    }

    /// Load configuration from a YAML file.
    pub fn load_from_file(path: &Path) -> Result<Self, ConfigError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;

        Self::load_from_str(&content)
    }

    /// Load configuration from a YAML string.
    pub fn load_from_str(content: &str) -> Result<Self, ConfigError> {
        serde_yaml::from_str(content).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Try to load config from default locations (.kube-linter.yaml, .kube-linter.yml).
    pub fn load_from_default() -> Option<Self> {
        for filename in &[".kube-linter.yaml", ".kube-linter.yml"] {
            let path = Path::new(filename);
            if path.exists() {
                if let Ok(config) = Self::load_from_file(path) {
                    return Some(config);
                }
            }
        }
        None
    }
}

/// A check specification defining what to lint and how.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckSpec {
    /// Unique name for this check (e.g., "privileged-container").
    pub name: String,

    /// Human-readable description of what this check does.
    pub description: String,

    /// Remediation advice for fixing violations.
    pub remediation: String,

    /// The template key this check is based on.
    pub template: String,

    /// Parameters to pass to the template.
    #[serde(default)]
    pub params: serde_yaml::Value,

    /// Which object kinds this check applies to.
    #[serde(default)]
    pub scope: CheckScope,
}

impl CheckSpec {
    /// Create a new check specification.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        remediation: impl Into<String>,
        template: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            remediation: remediation.into(),
            template: template.into(),
            params: serde_yaml::Value::Null,
            scope: CheckScope::default(),
        }
    }

    /// Set parameters for this check.
    pub fn with_params(mut self, params: serde_yaml::Value) -> Self {
        self.params = params;
        self
    }

    /// Set the scope for this check.
    pub fn with_scope(mut self, scope: CheckScope) -> Self {
        self.scope = scope;
        self
    }
}

/// Scope configuration for a check.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckScope {
    /// Which object kinds this check applies to.
    #[serde(default, rename = "objectKinds")]
    pub object_kinds: ObjectKindsDesc,
}

impl CheckScope {
    /// Create a new scope with the given object kinds.
    pub fn new(kinds: &[&str]) -> Self {
        Self {
            object_kinds: ObjectKindsDesc::new(kinds),
        }
    }
}

/// Configuration errors.
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// I/O error reading config file.
    IoError(String),
    /// Parse error in config file.
    ParseError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(msg) => write!(f, "I/O error: {}", msg),
            ConfigError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Default checks that are enabled by default (matching kube-linter defaults).
pub const DEFAULT_CHECKS: &[&str] = &[
    "dangling-service",
    "default-service-account",
    "deprecated-service-account",
    "drop-net-raw-capability",
    "env-var-secret",
    "host-mounts",
    "mismatching-selector",
    "no-anti-affinity",
    "no-liveness-probe",
    "no-readiness-probe",
    "no-rolling-update-strategy",
    "privilege-escalation",
    "privileged-container",
    "read-secret-from-env-var",
    "run-as-non-root",
    "ssh-port",
    "unset-cpu-requirements",
    "unset-memory-requirements",
    "writable-host-mount",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KubelintConfig::default();
        assert!(!config.add_all_builtin);
        assert!(!config.do_not_auto_add_defaults);
        assert!(config.include.is_empty());
        assert!(config.exclude.is_empty());
        assert_eq!(config.failure_threshold, Severity::Warning);
    }

    #[test]
    fn test_config_builder() {
        let config = KubelintConfig::new()
            .include("custom-check")
            .exclude("privileged-container")
            .with_threshold(Severity::Error);

        assert!(config.is_check_included("custom-check"));
        assert!(config.is_check_excluded("privileged-container"));
        assert_eq!(config.failure_threshold, Severity::Error);
    }

    #[test]
    fn test_path_ignoring() {
        let config = KubelintConfig::new()
            .ignore_path("**/test/**")
            .ignore_path("vendor/");

        assert!(config.should_ignore_path(Path::new("vendor/k8s/deployment.yaml")));
        // Note: glob matching behavior may vary
    }

    #[test]
    fn test_load_from_str() {
        let yaml = r#"
addAllBuiltIn: true
exclude:
  - latest-tag
  - privileged-container
include:
  - custom-check
failureThreshold: error
"#;
        let config = KubelintConfig::load_from_str(yaml).unwrap();
        assert!(config.add_all_builtin);
        assert!(config.is_check_excluded("latest-tag"));
        assert!(config.is_check_excluded("privileged-container"));
        assert!(config.is_check_included("custom-check"));
        assert_eq!(config.failure_threshold, Severity::Error);
    }

    #[test]
    fn test_check_spec() {
        let check = CheckSpec::new(
            "test-check",
            "A test check",
            "Fix the issue",
            "test-template",
        )
        .with_scope(CheckScope::new(&["Deployment", "StatefulSet"]));

        assert_eq!(check.name, "test-check");
        assert_eq!(check.template, "test-template");
    }
}
