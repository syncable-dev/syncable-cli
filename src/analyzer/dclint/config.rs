//! Configuration for the dclint Docker Compose linter.
//!
//! Provides configuration options matching the TypeScript docker-compose-linter:
//! - Rule-level configuration (off/warn/error)
//! - Per-rule options
//! - Global settings (quiet, debug, exclude patterns)

use std::collections::HashMap;

use crate::analyzer::dclint::types::{ConfigLevel, RuleCode, Severity};

/// Configuration for a single rule.
#[derive(Debug, Clone)]
pub struct RuleConfig {
    /// The configuration level (off, warn, error).
    pub level: ConfigLevel,
    /// Optional rule-specific options.
    pub options: HashMap<String, serde_json::Value>,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            level: ConfigLevel::Error,
            options: HashMap::new(),
        }
    }
}

impl RuleConfig {
    /// Create a new rule config with the given level.
    pub fn with_level(level: ConfigLevel) -> Self {
        Self {
            level,
            options: HashMap::new(),
        }
    }

    /// Create a rule config that's disabled.
    pub fn off() -> Self {
        Self::with_level(ConfigLevel::Off)
    }

    /// Create a rule config that produces warnings.
    pub fn warn() -> Self {
        Self::with_level(ConfigLevel::Warn)
    }

    /// Create a rule config that produces errors.
    pub fn error() -> Self {
        Self::with_level(ConfigLevel::Error)
    }

    /// Add an option to the rule config.
    pub fn with_option(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.options.insert(key.into(), value);
        self
    }

    /// Get an option value.
    pub fn get_option(&self, key: &str) -> Option<&serde_json::Value> {
        self.options.get(key)
    }

    /// Get a boolean option with a default value.
    pub fn get_bool_option(&self, key: &str, default: bool) -> bool {
        self.options
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(default)
    }

    /// Get a string option.
    pub fn get_string_option(&self, key: &str) -> Option<&str> {
        self.options.get(key).and_then(|v| v.as_str())
    }

    /// Get an array option as a vector of strings.
    pub fn get_string_array_option(&self, key: &str) -> Vec<String> {
        self.options
            .get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Main configuration for dclint.
#[derive(Debug, Clone)]
pub struct DclintConfig {
    /// Per-rule configuration.
    pub rules: HashMap<String, RuleConfig>,
    /// Suppress non-error output.
    pub quiet: bool,
    /// Enable debug output.
    pub debug: bool,
    /// File patterns to exclude from linting.
    pub exclude: Vec<String>,
    /// Minimum severity threshold for reporting.
    pub threshold: Severity,
    /// Whether to disable pragma (comment-based) ignores.
    pub disable_ignore_pragma: bool,
    /// Whether to report fixable issues only.
    pub fixable_only: bool,
}

impl Default for DclintConfig {
    fn default() -> Self {
        Self {
            rules: HashMap::new(),
            quiet: false,
            debug: false,
            exclude: Vec::new(),
            threshold: Severity::Style,
            disable_ignore_pragma: false,
            fixable_only: false,
        }
    }
}

impl DclintConfig {
    /// Create a new default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set quiet mode.
    pub fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// Set debug mode.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Add an exclude pattern.
    pub fn with_exclude(mut self, pattern: impl Into<String>) -> Self {
        self.exclude.push(pattern.into());
        self
    }

    /// Set multiple exclude patterns.
    pub fn with_excludes(mut self, patterns: Vec<String>) -> Self {
        self.exclude = patterns;
        self
    }

    /// Set the severity threshold.
    pub fn with_threshold(mut self, threshold: Severity) -> Self {
        self.threshold = threshold;
        self
    }

    /// Configure a specific rule.
    pub fn with_rule(mut self, rule: impl Into<String>, config: RuleConfig) -> Self {
        self.rules.insert(rule.into(), config);
        self
    }

    /// Disable a rule.
    pub fn ignore(mut self, rule: impl Into<String>) -> Self {
        self.rules.insert(rule.into(), RuleConfig::off());
        self
    }

    /// Set a rule to warn level.
    pub fn warn(mut self, rule: impl Into<String>) -> Self {
        self.rules.insert(rule.into(), RuleConfig::warn());
        self
    }

    /// Set a rule to error level.
    pub fn error(mut self, rule: impl Into<String>) -> Self {
        self.rules.insert(rule.into(), RuleConfig::error());
        self
    }

    /// Disable pragma (comment-based) ignores.
    pub fn with_disable_ignore_pragma(mut self, disable: bool) -> Self {
        self.disable_ignore_pragma = disable;
        self
    }

    /// Check if a rule is ignored (disabled).
    pub fn is_rule_ignored(&self, code: &RuleCode) -> bool {
        self.rules
            .get(code.as_str())
            .map(|c| c.level == ConfigLevel::Off)
            .unwrap_or(false)
    }

    /// Get the configuration for a specific rule.
    pub fn get_rule_config(&self, code: &str) -> Option<&RuleConfig> {
        self.rules.get(code)
    }

    /// Get the effective severity for a rule, applying any overrides.
    pub fn effective_severity(&self, code: &RuleCode, default: Severity) -> Severity {
        self.rules
            .get(code.as_str())
            .and_then(|c| c.level.to_severity())
            .unwrap_or(default)
    }

    /// Check if an issue should be reported based on threshold.
    pub fn should_report(&self, severity: Severity) -> bool {
        severity >= self.threshold
    }

    /// Check if a file path should be excluded.
    pub fn is_excluded(&self, path: &str) -> bool {
        for pattern in &self.exclude {
            // Simple glob matching
            if pattern.contains('*') {
                let pattern_regex = pattern.replace('.', "\\.").replace('*', ".*");
                if let Ok(re) = regex::Regex::new(&format!("^{}$", pattern_regex))
                    && re.is_match(path)
                {
                    return true;
                }
            } else if path.contains(pattern) {
                return true;
            }
        }
        false
    }
}

/// Builder for creating DclintConfig from various sources.
pub struct DclintConfigBuilder {
    config: DclintConfig,
}

impl DclintConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: DclintConfig::default(),
        }
    }

    /// Load configuration from a JSON value (matching TypeScript config format).
    pub fn from_json(mut self, json: &serde_json::Value) -> Self {
        if let Some(rules) = json.get("rules").and_then(|v| v.as_object()) {
            for (name, value) in rules {
                let rule_config = match value {
                    // Simple numeric level: 0, 1, or 2
                    serde_json::Value::Number(n) => {
                        if let Some(level) = n.as_u64().and_then(|n| ConfigLevel::from_u8(n as u8))
                        {
                            RuleConfig::with_level(level)
                        } else {
                            continue;
                        }
                    }
                    // Array format: [level, options]
                    serde_json::Value::Array(arr) => {
                        let level = arr
                            .first()
                            .and_then(|v| v.as_u64())
                            .and_then(|n| ConfigLevel::from_u8(n as u8))
                            .unwrap_or(ConfigLevel::Error);

                        let mut config = RuleConfig::with_level(level);

                        if let Some(opts) = arr.get(1).and_then(|v| v.as_object()) {
                            for (k, v) in opts {
                                config.options.insert(k.clone(), v.clone());
                            }
                        }

                        config
                    }
                    _ => continue,
                };

                self.config.rules.insert(name.clone(), rule_config);
            }
        }

        if let Some(quiet) = json.get("quiet").and_then(|v| v.as_bool()) {
            self.config.quiet = quiet;
        }

        if let Some(debug) = json.get("debug").and_then(|v| v.as_bool()) {
            self.config.debug = debug;
        }

        if let Some(exclude) = json.get("exclude").and_then(|v| v.as_array()) {
            self.config.exclude = exclude
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }

        self
    }

    /// Build the final configuration.
    pub fn build(self) -> DclintConfig {
        self.config
    }
}

impl Default for DclintConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DclintConfig::default();
        assert!(!config.quiet);
        assert!(!config.debug);
        assert!(config.exclude.is_empty());
        assert!(config.rules.is_empty());
    }

    #[test]
    fn test_rule_config() {
        let config = DclintConfig::default()
            .ignore("DCL001")
            .warn("DCL002")
            .error("DCL003");

        assert!(config.is_rule_ignored(&RuleCode::new("DCL001")));
        assert!(!config.is_rule_ignored(&RuleCode::new("DCL002")));
        assert!(!config.is_rule_ignored(&RuleCode::new("DCL003")));
        assert!(!config.is_rule_ignored(&RuleCode::new("DCL004"))); // Not configured
    }

    #[test]
    fn test_effective_severity() {
        let config = DclintConfig::default().warn("DCL001").error("DCL002");

        assert_eq!(
            config.effective_severity(&RuleCode::new("DCL001"), Severity::Error),
            Severity::Warning
        );
        assert_eq!(
            config.effective_severity(&RuleCode::new("DCL002"), Severity::Warning),
            Severity::Error
        );
        // Non-configured rule uses default
        assert_eq!(
            config.effective_severity(&RuleCode::new("DCL003"), Severity::Info),
            Severity::Info
        );
    }

    #[test]
    fn test_threshold() {
        let config = DclintConfig::default().with_threshold(Severity::Warning);

        assert!(config.should_report(Severity::Error));
        assert!(config.should_report(Severity::Warning));
        assert!(!config.should_report(Severity::Info));
        assert!(!config.should_report(Severity::Style));
    }

    #[test]
    fn test_exclude_patterns() {
        let config = DclintConfig::default()
            .with_exclude("node_modules")
            .with_exclude("*.test.yml");

        assert!(config.is_excluded("path/to/node_modules/file.yml"));
        assert!(config.is_excluded("docker-compose.test.yml"));
        assert!(!config.is_excluded("docker-compose.yml"));
    }

    #[test]
    fn test_rule_options() {
        let rule_config = RuleConfig::default()
            .with_option("checkPullPolicy", serde_json::json!(true))
            .with_option("pattern", serde_json::json!("^[a-z]+$"));

        assert!(rule_config.get_bool_option("checkPullPolicy", false));
        assert_eq!(rule_config.get_string_option("pattern"), Some("^[a-z]+$"));
        assert!(rule_config.get_bool_option("nonexistent", false) == false);
    }

    #[test]
    fn test_config_from_json() {
        let json = serde_json::json!({
            "rules": {
                "no-build-and-image": 2,
                "no-version-field": [1, { "allowEmpty": true }],
                "services-alphabetical-order": 0
            },
            "quiet": true,
            "exclude": ["*.test.yml"]
        });

        let config = DclintConfigBuilder::new().from_json(&json).build();

        assert!(config.quiet);
        assert_eq!(config.exclude, vec!["*.test.yml"]);

        let rule1 = config.get_rule_config("no-build-and-image").unwrap();
        assert_eq!(rule1.level, ConfigLevel::Error);

        let rule2 = config.get_rule_config("no-version-field").unwrap();
        assert_eq!(rule2.level, ConfigLevel::Warn);
        assert!(rule2.get_bool_option("allowEmpty", false));

        let rule3 = config
            .get_rule_config("services-alphabetical-order")
            .unwrap();
        assert_eq!(rule3.level, ConfigLevel::Off);
    }
}
