//! Configuration for Kubernetes resource optimization analysis.

use super::types::Severity;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for resource optimization analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sOptimizeConfig {
    /// Minimum severity to report (default: Info)
    pub min_severity: Severity,

    /// Minimum waste percentage to report (default: 10)
    pub waste_threshold_percent: u8,

    /// Safety margin percentage above recommended values (default: 20)
    pub safety_margin_percent: u8,

    /// Include info-level suggestions
    pub include_info: bool,

    /// Rules to ignore (by rule code)
    pub ignore_rules: Vec<String>,

    /// Namespaces to exclude
    pub exclude_namespaces: Vec<String>,

    /// Resource name patterns to exclude
    pub exclude_patterns: Vec<String>,

    /// Include system namespaces (kube-system, etc.)
    pub include_system: bool,

    /// Maximum CPU request before flagging (in millicores, default: 1000)
    pub max_cpu_request_millicores: u32,

    /// Maximum memory request before flagging (in Mi, default: 2048)
    pub max_memory_request_mi: u32,

    /// Maximum CPU limit to request ratio (default: 10)
    pub max_cpu_limit_ratio: f32,

    /// Maximum memory limit to request ratio (default: 4)
    pub max_memory_limit_ratio: f32,

    /// Generate YAML fix snippets
    pub generate_fixes: bool,
}

impl Default for K8sOptimizeConfig {
    fn default() -> Self {
        Self {
            min_severity: Severity::Info,
            waste_threshold_percent: 10,
            safety_margin_percent: 20,
            include_info: false,
            ignore_rules: Vec::new(),
            exclude_namespaces: Vec::new(),
            exclude_patterns: Vec::new(),
            include_system: false,
            max_cpu_request_millicores: 1000, // 1 core
            max_memory_request_mi: 2048,      // 2Gi
            max_cpu_limit_ratio: 10.0,
            max_memory_limit_ratio: 4.0,
            generate_fixes: true,
        }
    }
}

impl K8sOptimizeConfig {
    /// Create a new default config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the minimum severity threshold.
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.min_severity = severity;
        self
    }

    /// Set the waste threshold percentage.
    pub fn with_threshold(mut self, threshold: u8) -> Self {
        self.waste_threshold_percent = threshold;
        self
    }

    /// Set the safety margin percentage.
    pub fn with_safety_margin(mut self, margin: u8) -> Self {
        self.safety_margin_percent = margin;
        self
    }

    /// Include info-level suggestions.
    pub fn with_info(mut self) -> Self {
        self.include_info = true;
        self.min_severity = Severity::Info;
        self
    }

    /// Add a rule to ignore.
    pub fn ignore_rule(mut self, rule: impl Into<String>) -> Self {
        self.ignore_rules.push(rule.into());
        self
    }

    /// Add a namespace to exclude.
    pub fn exclude_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.exclude_namespaces.push(namespace.into());
        self
    }

    /// Include system namespaces.
    pub fn with_system(mut self) -> Self {
        self.include_system = true;
        self
    }

    /// Check if a rule should be ignored.
    pub fn should_ignore_rule(&self, rule: &str) -> bool {
        self.ignore_rules.iter().any(|r| r == rule)
    }

    /// Check if a namespace should be excluded.
    pub fn should_exclude_namespace(&self, namespace: &str) -> bool {
        // Always exclude system namespaces unless include_system is true
        if !self.include_system {
            const SYSTEM_NAMESPACES: &[&str] =
                &["kube-system", "kube-public", "kube-node-lease", "default"];
            if SYSTEM_NAMESPACES.contains(&namespace) {
                return true;
            }
        }

        self.exclude_namespaces.iter().any(|n| n == namespace)
    }

    /// Check if a path should be ignored.
    pub fn should_ignore_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.exclude_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = K8sOptimizeConfig::default();
        assert_eq!(config.waste_threshold_percent, 10);
        assert_eq!(config.safety_margin_percent, 20);
        assert!(!config.include_system);
    }

    #[test]
    fn test_exclude_system_namespaces() {
        let config = K8sOptimizeConfig::default();
        assert!(config.should_exclude_namespace("kube-system"));
        assert!(!config.should_exclude_namespace("production"));

        let config = config.with_system();
        assert!(!config.should_exclude_namespace("kube-system"));
    }

    #[test]
    fn test_builder_pattern() {
        let config = K8sOptimizeConfig::new()
            .with_threshold(20)
            .with_safety_margin(30)
            .ignore_rule("K8S-OPT-001")
            .exclude_namespace("test");

        assert_eq!(config.waste_threshold_percent, 20);
        assert_eq!(config.safety_margin_percent, 30);
        assert!(config.should_ignore_rule("K8S-OPT-001"));
        assert!(config.should_exclude_namespace("test"));
    }
}
