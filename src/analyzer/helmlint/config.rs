//! Configuration for the helmlint linter.
//!
//! Provides configuration options for:
//! - Enabling/disabling rules
//! - Severity overrides
//! - Kubernetes version targeting
//! - Values schema validation

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::analyzer::helmlint::types::Severity;

/// Configuration for the helmlint linter.
#[derive(Debug, Clone)]
pub struct HelmlintConfig {
    /// Rules to ignore (by code, e.g., "HL1001").
    pub ignored_rules: HashSet<String>,

    /// Severity overrides for specific rules.
    pub severity_overrides: HashMap<String, Severity>,

    /// Minimum severity threshold for reporting.
    pub failure_threshold: Severity,

    /// If true, ignore inline pragma comments.
    pub disable_ignore_pragma: bool,

    /// If true, don't fail even if errors are found.
    pub no_fail: bool,

    /// Target Kubernetes version for API deprecation checks.
    pub k8s_version: Option<String>,

    /// Path to a JSON schema for values.yaml validation.
    pub values_schema_path: Option<PathBuf>,

    /// Strict mode - treat warnings as errors.
    pub strict: bool,

    /// Only report fixable issues.
    pub fixable_only: bool,

    /// Files or patterns to exclude.
    pub exclude_patterns: Vec<String>,
}

impl Default for HelmlintConfig {
    fn default() -> Self {
        Self {
            ignored_rules: HashSet::new(),
            severity_overrides: HashMap::new(),
            failure_threshold: Severity::Warning,
            disable_ignore_pragma: false,
            no_fail: false,
            k8s_version: None,
            values_schema_path: None,
            strict: false,
            fixable_only: false,
            exclude_patterns: Vec::new(),
        }
    }
}

impl HelmlintConfig {
    /// Create a new default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a rule to ignore.
    pub fn ignore(mut self, rule: impl Into<String>) -> Self {
        self.ignored_rules.insert(rule.into());
        self
    }

    /// Add multiple rules to ignore.
    pub fn ignore_all(mut self, rules: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for rule in rules {
            self.ignored_rules.insert(rule.into());
        }
        self
    }

    /// Override severity for a specific rule.
    pub fn with_severity(mut self, rule: impl Into<String>, severity: Severity) -> Self {
        self.severity_overrides.insert(rule.into(), severity);
        self
    }

    /// Set the failure threshold.
    pub fn with_threshold(mut self, threshold: Severity) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Set the target Kubernetes version.
    pub fn with_k8s_version(mut self, version: impl Into<String>) -> Self {
        self.k8s_version = Some(version.into());
        self
    }

    /// Set the values schema path.
    pub fn with_values_schema(mut self, path: impl Into<PathBuf>) -> Self {
        self.values_schema_path = Some(path.into());
        self
    }

    /// Enable strict mode.
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Check if a rule is ignored.
    pub fn is_rule_ignored(&self, code: &str) -> bool {
        self.ignored_rules.contains(code)
    }

    /// Get the effective severity for a rule.
    pub fn effective_severity(&self, code: &str, default: Severity) -> Severity {
        if let Some(&override_severity) = self.severity_overrides.get(code) {
            override_severity
        } else if self.strict && default == Severity::Warning {
            Severity::Error
        } else {
            default
        }
    }

    /// Check if a severity should be reported based on threshold.
    pub fn should_report(&self, severity: Severity) -> bool {
        severity >= self.failure_threshold
    }

    /// Check if a file is excluded.
    pub fn is_excluded(&self, path: &str) -> bool {
        for pattern in &self.exclude_patterns {
            if path.contains(pattern) {
                return true;
            }
            // Simple glob matching
            if pattern.contains('*') {
                let parts: Vec<&str> = pattern.split('*').collect();
                let mut remaining = path;
                let mut matched = true;
                for (i, part) in parts.iter().enumerate() {
                    if part.is_empty() {
                        continue;
                    }
                    if i == 0 {
                        if !remaining.starts_with(part) {
                            matched = false;
                            break;
                        }
                        remaining = &remaining[part.len()..];
                    } else if i == parts.len() - 1 {
                        if !remaining.ends_with(part) {
                            matched = false;
                            break;
                        }
                    } else if let Some(pos) = remaining.find(part) {
                        remaining = &remaining[pos + part.len()..];
                    } else {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    return true;
                }
            }
        }
        false
    }

    /// Parse Kubernetes version string to (major, minor).
    pub fn parse_k8s_version(&self) -> Option<(u32, u32)> {
        self.k8s_version.as_ref().and_then(|v| {
            let v = v.trim_start_matches('v');
            let parts: Vec<&str> = v.split('.').collect();
            if parts.len() >= 2 {
                let major = parts[0].parse().ok()?;
                let minor = parts[1].parse().ok()?;
                Some((major, minor))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HelmlintConfig::default();
        assert!(config.ignored_rules.is_empty());
        assert!(config.severity_overrides.is_empty());
        assert_eq!(config.failure_threshold, Severity::Warning);
        assert!(!config.strict);
    }

    #[test]
    fn test_ignore_rule() {
        let config = HelmlintConfig::default().ignore("HL1001");
        assert!(config.is_rule_ignored("HL1001"));
        assert!(!config.is_rule_ignored("HL1002"));
    }

    #[test]
    fn test_severity_override() {
        let config = HelmlintConfig::default().with_severity("HL1001", Severity::Error);
        assert_eq!(
            config.effective_severity("HL1001", Severity::Warning),
            Severity::Error
        );
        assert_eq!(
            config.effective_severity("HL1002", Severity::Warning),
            Severity::Warning
        );
    }

    #[test]
    fn test_strict_mode() {
        let config = HelmlintConfig::default().with_strict(true);
        assert_eq!(
            config.effective_severity("HL1001", Severity::Warning),
            Severity::Error
        );
        assert_eq!(
            config.effective_severity("HL1001", Severity::Info),
            Severity::Info
        );
    }

    #[test]
    fn test_k8s_version_parsing() {
        let config = HelmlintConfig::default().with_k8s_version("v1.28");
        assert_eq!(config.parse_k8s_version(), Some((1, 28)));

        let config = HelmlintConfig::default().with_k8s_version("1.25.0");
        assert_eq!(config.parse_k8s_version(), Some((1, 25)));
    }

    #[test]
    fn test_exclusion() {
        let mut config = HelmlintConfig::default();
        config.exclude_patterns = vec!["test".to_string(), "*.bak".to_string()];

        assert!(config.is_excluded("templates/test.yaml"));
        assert!(config.is_excluded("backup.bak"));
        assert!(!config.is_excluded("templates/deployment.yaml"));
    }
}
