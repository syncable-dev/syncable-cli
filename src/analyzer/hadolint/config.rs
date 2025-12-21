//! Configuration for the hadolint-rs linter.
//!
//! Supports configuration from:
//! - Programmatic defaults
//! - YAML config files (.hadolint.yaml)
//!
//! Configuration priority (highest to lowest):
//! 1. Programmatic overrides
//! 2. Config file settings
//! 3. Defaults

use crate::analyzer::hadolint::types::{RuleCode, Severity};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Label validation types for DL3049-DL3056 rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelType {
    /// Email address format
    Email,
    /// Git commit hash
    GitHash,
    /// Raw text (no validation)
    RawText,
    /// RFC3339 timestamp
    Rfc3339,
    /// Semantic versioning
    SemVer,
    /// SPDX license identifier
    Spdx,
    /// URL format
    Url,
}

impl LabelType {
    /// Parse a label type from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "email" => Some(Self::Email),
            "hash" => Some(Self::GitHash),
            "text" | "" => Some(Self::RawText),
            "rfc3339" => Some(Self::Rfc3339),
            "semver" => Some(Self::SemVer),
            "spdx" => Some(Self::Spdx),
            "url" => Some(Self::Url),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::GitHash => "hash",
            Self::RawText => "text",
            Self::Rfc3339 => "rfc3339",
            Self::SemVer => "semver",
            Self::Spdx => "spdx",
            Self::Url => "url",
        }
    }
}

/// Configuration for the hadolint linter.
#[derive(Debug, Clone)]
pub struct HadolintConfig {
    /// Rules to ignore entirely.
    pub ignore_rules: HashSet<RuleCode>,
    /// Rules to treat as errors (override default severity).
    pub error_rules: HashSet<RuleCode>,
    /// Rules to treat as warnings (override default severity).
    pub warning_rules: HashSet<RuleCode>,
    /// Rules to treat as info (override default severity).
    pub info_rules: HashSet<RuleCode>,
    /// Rules to treat as style (override default severity).
    pub style_rules: HashSet<RuleCode>,
    /// Allowed Docker registries (for DL3026).
    pub allowed_registries: HashSet<String>,
    /// Label schema requirements (for DL3049-DL3056).
    pub label_schema: HashMap<String, LabelType>,
    /// Fail on labels not in schema.
    pub strict_labels: bool,
    /// Disable inline ignore pragmas.
    pub disable_ignore_pragma: bool,
    /// Minimum severity to report.
    pub failure_threshold: Severity,
    /// Don't fail even if rules are violated.
    pub no_fail: bool,
}

impl Default for HadolintConfig {
    fn default() -> Self {
        Self {
            ignore_rules: HashSet::new(),
            error_rules: HashSet::new(),
            warning_rules: HashSet::new(),
            info_rules: HashSet::new(),
            style_rules: HashSet::new(),
            allowed_registries: HashSet::new(),
            label_schema: HashMap::new(),
            strict_labels: false,
            disable_ignore_pragma: false,
            failure_threshold: Severity::Info,
            no_fail: false,
        }
    }
}

impl HadolintConfig {
    /// Create a new config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load config from a YAML file.
    pub fn from_yaml_file(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
        Self::from_yaml_str(&content)
    }

    /// Load config from a YAML string.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, ConfigError> {
        let value: serde_yaml::Value = serde_yaml::from_str(yaml)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        let mut config = Self::default();

        // Parse ignored rules
        if let Some(ignored) = value.get("ignored").and_then(|v| v.as_sequence()) {
            for item in ignored {
                if let Some(code) = item.as_str() {
                    config.ignore_rules.insert(RuleCode::new(code));
                }
            }
        }

        // Parse override.error
        if let Some(overrides) = value.get("override").and_then(|v| v.as_mapping()) {
            if let Some(errors) = overrides.get("error").and_then(|v| v.as_sequence()) {
                for item in errors {
                    if let Some(code) = item.as_str() {
                        config.error_rules.insert(RuleCode::new(code));
                    }
                }
            }
            if let Some(warnings) = overrides.get("warning").and_then(|v| v.as_sequence()) {
                for item in warnings {
                    if let Some(code) = item.as_str() {
                        config.warning_rules.insert(RuleCode::new(code));
                    }
                }
            }
            if let Some(infos) = overrides.get("info").and_then(|v| v.as_sequence()) {
                for item in infos {
                    if let Some(code) = item.as_str() {
                        config.info_rules.insert(RuleCode::new(code));
                    }
                }
            }
            if let Some(styles) = overrides.get("style").and_then(|v| v.as_sequence()) {
                for item in styles {
                    if let Some(code) = item.as_str() {
                        config.style_rules.insert(RuleCode::new(code));
                    }
                }
            }
        }

        // Parse trusted registries
        if let Some(registries) = value.get("trustedRegistries").and_then(|v| v.as_sequence()) {
            for item in registries {
                if let Some(registry) = item.as_str() {
                    config.allowed_registries.insert(registry.to_string());
                }
            }
        }

        // Parse label schema
        if let Some(schema) = value.get("label-schema").and_then(|v| v.as_mapping()) {
            for (key, val) in schema {
                if let (Some(label), Some(type_str)) = (key.as_str(), val.as_str()) {
                    if let Some(label_type) = LabelType::from_str(type_str) {
                        config.label_schema.insert(label.to_string(), label_type);
                    }
                }
            }
        }

        // Parse boolean flags
        if let Some(strict) = value.get("strict-labels").and_then(|v| v.as_bool()) {
            config.strict_labels = strict;
        }
        if let Some(disable) = value.get("disable-ignore-pragma").and_then(|v| v.as_bool()) {
            config.disable_ignore_pragma = disable;
        }
        if let Some(no_fail) = value.get("no-fail").and_then(|v| v.as_bool()) {
            config.no_fail = no_fail;
        }

        // Parse failure threshold
        if let Some(threshold) = value.get("failure-threshold").and_then(|v| v.as_str()) {
            if let Some(severity) = Severity::from_str(threshold) {
                config.failure_threshold = severity;
            }
        }

        Ok(config)
    }

    /// Find and load config from standard locations.
    ///
    /// Search order:
    /// 1. .hadolint.yaml in current directory
    /// 2. .hadolint.yml in current directory
    /// 3. XDG config directory
    /// 4. Home directory
    pub fn find_and_load() -> Option<Self> {
        let search_paths = [
            ".hadolint.yaml",
            ".hadolint.yml",
        ];

        for path in &search_paths {
            let path = Path::new(path);
            if path.exists() {
                if let Ok(config) = Self::from_yaml_file(path) {
                    return Some(config);
                }
            }
        }

        // Try XDG config directory
        if let Some(config_dir) = dirs::config_dir() {
            let xdg_path = config_dir.join("hadolint.yaml");
            if xdg_path.exists() {
                if let Ok(config) = Self::from_yaml_file(&xdg_path) {
                    return Some(config);
                }
            }
        }

        // Try home directory
        if let Some(home_dir) = dirs::home_dir() {
            let home_path = home_dir.join(".hadolint.yaml");
            if home_path.exists() {
                if let Ok(config) = Self::from_yaml_file(&home_path) {
                    return Some(config);
                }
            }
        }

        None
    }

    /// Check if a rule should be ignored.
    pub fn is_rule_ignored(&self, code: &RuleCode) -> bool {
        self.ignore_rules.contains(code)
    }

    /// Get the effective severity for a rule.
    pub fn effective_severity(&self, code: &RuleCode, default: Severity) -> Severity {
        if self.error_rules.contains(code) {
            return Severity::Error;
        }
        if self.warning_rules.contains(code) {
            return Severity::Warning;
        }
        if self.info_rules.contains(code) {
            return Severity::Info;
        }
        if self.style_rules.contains(code) {
            return Severity::Style;
        }
        default
    }

    /// Builder method to add an ignored rule.
    pub fn ignore(mut self, code: impl Into<RuleCode>) -> Self {
        self.ignore_rules.insert(code.into());
        self
    }

    /// Builder method to add an allowed registry.
    pub fn allow_registry(mut self, registry: impl Into<String>) -> Self {
        self.allowed_registries.insert(registry.into());
        self
    }

    /// Builder method to set failure threshold.
    pub fn with_threshold(mut self, threshold: Severity) -> Self {
        self.failure_threshold = threshold;
        self
    }
}

/// Errors that can occur when loading configuration.
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// I/O error reading the file.
    IoError(String),
    /// YAML parsing error.
    ParseError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(msg) => write!(f, "I/O error: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HadolintConfig::default();
        assert!(config.ignore_rules.is_empty());
        assert!(!config.strict_labels);
        assert!(!config.disable_ignore_pragma);
        assert_eq!(config.failure_threshold, Severity::Info);
    }

    #[test]
    fn test_yaml_parsing() {
        let yaml = r#"
ignored:
  - DL3008
  - DL3009

override:
  error:
    - DL3001
  warning:
    - DL3002

trustedRegistries:
  - docker.io
  - gcr.io

failure-threshold: warning
strict-labels: true
"#;

        let config = HadolintConfig::from_yaml_str(yaml).unwrap();
        assert!(config.ignore_rules.contains(&RuleCode::new("DL3008")));
        assert!(config.ignore_rules.contains(&RuleCode::new("DL3009")));
        assert!(config.error_rules.contains(&RuleCode::new("DL3001")));
        assert!(config.warning_rules.contains(&RuleCode::new("DL3002")));
        assert!(config.allowed_registries.contains("docker.io"));
        assert!(config.allowed_registries.contains("gcr.io"));
        assert_eq!(config.failure_threshold, Severity::Warning);
        assert!(config.strict_labels);
    }

    #[test]
    fn test_effective_severity() {
        let config = HadolintConfig::default()
            .ignore("DL3008".to_string());

        assert!(config.is_rule_ignored(&RuleCode::new("DL3008")));
        assert!(!config.is_rule_ignored(&RuleCode::new("DL3009")));
    }

    #[test]
    fn test_builder_pattern() {
        let config = HadolintConfig::new()
            .ignore("DL3008")
            .allow_registry("docker.io")
            .with_threshold(Severity::Warning);

        assert!(config.ignore_rules.contains(&RuleCode::new("DL3008")));
        assert!(config.allowed_registries.contains("docker.io"));
        assert_eq!(config.failure_threshold, Severity::Warning);
    }
}
