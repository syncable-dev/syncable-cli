//! Core types for the dclint Docker Compose linter.
//!
//! These types follow the pattern established by hadolint-rs:
//! - `Severity` - Rule violation severity levels
//! - `RuleCode` - Rule identifiers (e.g., "DCL001")
//! - `CheckFailure` - A single rule violation
//! - `RuleCategory` - Category of the rule (style, security, etc.)

use std::cmp::Ordering;
use std::fmt;

/// Severity levels for rule violations.
///
/// Ordered from most severe to least severe:
/// `Error > Warning > Info > Style`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// Critical issues that should always be fixed
    Error,
    /// Important issues that should usually be fixed
    Warning,
    /// Informational suggestions for improvement
    Info,
    /// Style recommendations
    Style,
}

impl Severity {
    /// Parse a severity from a string (case-insensitive).
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "error" | "critical" | "major" => Some(Self::Error),
            "warning" | "minor" => Some(Self::Warning),
            "info" => Some(Self::Info),
            "style" => Some(Self::Style),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Style => "style",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for Severity {
    fn default() -> Self {
        Self::Warning
    }
}

impl Ord for Severity {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher severity = lower numeric value for Ord
        let self_val = match self {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Info => 2,
            Self::Style => 3,
        };
        let other_val = match other {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Info => 2,
            Self::Style => 3,
        };
        // Reverse so Error > Warning > Info > Style
        other_val.cmp(&self_val)
    }
}

impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Category of a lint rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleCategory {
    /// Style and formatting issues
    Style,
    /// Security-related issues
    Security,
    /// Best practice recommendations
    BestPractice,
    /// Performance-related issues
    Performance,
}

impl RuleCategory {
    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Style => "style",
            Self::Security => "security",
            Self::BestPractice => "best-practice",
            Self::Performance => "performance",
        }
    }
}

impl fmt::Display for RuleCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A rule code identifier (e.g., "DCL001").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    /// Check if this is a DCL rule.
    pub fn is_dcl_rule(&self) -> bool {
        self.0.starts_with("DCL")
    }

    /// Get the numeric part of the rule code.
    pub fn number(&self) -> Option<u32> {
        if self.0.starts_with("DCL") {
            self.0[3..].parse().ok()
        } else {
            None
        }
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

/// A check failure (rule violation) found during linting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckFailure {
    /// The rule code that was violated.
    pub code: RuleCode,
    /// The human-readable rule name (e.g., "no-build-and-image").
    pub rule_name: String,
    /// The severity of the violation.
    pub severity: Severity,
    /// The category of the rule.
    pub category: RuleCategory,
    /// A human-readable message describing the violation.
    pub message: String,
    /// The line number where the violation occurred (1-indexed).
    pub line: u32,
    /// The column number where the violation starts (1-indexed).
    pub column: u32,
    /// Optional end line number.
    pub end_line: Option<u32>,
    /// Optional end column number.
    pub end_column: Option<u32>,
    /// Whether this issue can be auto-fixed.
    pub fixable: bool,
    /// Additional context data for the violation.
    pub data: std::collections::HashMap<String, String>,
}

impl CheckFailure {
    /// Create a new check failure.
    pub fn new(
        code: impl Into<RuleCode>,
        rule_name: impl Into<String>,
        severity: Severity,
        category: RuleCategory,
        message: impl Into<String>,
        line: u32,
        column: u32,
    ) -> Self {
        Self {
            code: code.into(),
            rule_name: rule_name.into(),
            severity,
            category,
            message: message.into(),
            line,
            column,
            end_line: None,
            end_column: None,
            fixable: false,
            data: std::collections::HashMap::new(),
        }
    }

    /// Set the end position.
    pub fn with_end(mut self, end_line: u32, end_column: u32) -> Self {
        self.end_line = Some(end_line);
        self.end_column = Some(end_column);
        self
    }

    /// Mark as fixable.
    pub fn with_fixable(mut self, fixable: bool) -> Self {
        self.fixable = fixable;
        self
    }

    /// Add context data.
    pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }
}

impl Ord for CheckFailure {
    fn cmp(&self, other: &Self) -> Ordering {
        // Sort by line number first, then column
        match self.line.cmp(&other.line) {
            Ordering::Equal => self.column.cmp(&other.column),
            other => other,
        }
    }
}

impl PartialOrd for CheckFailure {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Rule metadata for documentation and display.
#[derive(Debug, Clone)]
pub struct RuleMeta {
    /// Short description of the rule.
    pub description: String,
    /// URL to detailed documentation.
    pub url: String,
}

impl RuleMeta {
    pub fn new(description: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            url: url.into(),
        }
    }
}

/// Configuration level for a rule (matches TypeScript ConfigRuleLevel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigLevel {
    /// Rule is disabled
    Off = 0,
    /// Rule produces warnings
    Warn = 1,
    /// Rule produces errors
    Error = 2,
}

impl ConfigLevel {
    /// Convert from numeric value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Off),
            1 => Some(Self::Warn),
            2 => Some(Self::Error),
            _ => None,
        }
    }

    /// Convert to severity (for non-off levels).
    pub fn to_severity(&self) -> Option<Severity> {
        match self {
            Self::Off => None,
            Self::Warn => Some(Severity::Warning),
            Self::Error => Some(Severity::Error),
        }
    }
}

impl Default for ConfigLevel {
    fn default() -> Self {
        Self::Error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
        assert!(Severity::Info > Severity::Style);
    }

    #[test]
    fn test_severity_from_str() {
        assert_eq!(Severity::from_str("error"), Some(Severity::Error));
        assert_eq!(Severity::from_str("WARNING"), Some(Severity::Warning));
        assert_eq!(Severity::from_str("Info"), Some(Severity::Info));
        assert_eq!(Severity::from_str("style"), Some(Severity::Style));
        assert_eq!(Severity::from_str("critical"), Some(Severity::Error));
        assert_eq!(Severity::from_str("major"), Some(Severity::Error));
        assert_eq!(Severity::from_str("minor"), Some(Severity::Warning));
        assert_eq!(Severity::from_str("invalid"), None);
    }

    #[test]
    fn test_rule_code() {
        let code = RuleCode::new("DCL001");
        assert!(code.is_dcl_rule());
        assert_eq!(code.number(), Some(1));
        assert_eq!(code.as_str(), "DCL001");

        let invalid = RuleCode::new("OTHER");
        assert!(!invalid.is_dcl_rule());
        assert_eq!(invalid.number(), None);
    }

    #[test]
    fn test_check_failure_ordering() {
        let f1 = CheckFailure::new(
            "DCL001",
            "test",
            Severity::Warning,
            RuleCategory::Style,
            "msg1",
            5,
            1,
        );
        let f2 = CheckFailure::new(
            "DCL002",
            "test",
            Severity::Info,
            RuleCategory::Style,
            "msg2",
            10,
            1,
        );
        let f3 = CheckFailure::new(
            "DCL003",
            "test",
            Severity::Error,
            RuleCategory::Style,
            "msg3",
            3,
            1,
        );
        let f4 = CheckFailure::new(
            "DCL004",
            "test",
            Severity::Error,
            RuleCategory::Style,
            "msg4",
            3,
            5,
        );

        let mut failures = vec![f1.clone(), f2.clone(), f3.clone(), f4.clone()];
        failures.sort();

        assert_eq!(failures[0].line, 3);
        assert_eq!(failures[0].column, 1);
        assert_eq!(failures[1].line, 3);
        assert_eq!(failures[1].column, 5);
        assert_eq!(failures[2].line, 5);
        assert_eq!(failures[3].line, 10);
    }

    #[test]
    fn test_config_level() {
        assert_eq!(ConfigLevel::from_u8(0), Some(ConfigLevel::Off));
        assert_eq!(ConfigLevel::from_u8(1), Some(ConfigLevel::Warn));
        assert_eq!(ConfigLevel::from_u8(2), Some(ConfigLevel::Error));
        assert_eq!(ConfigLevel::from_u8(3), None);

        assert_eq!(ConfigLevel::Off.to_severity(), None);
        assert_eq!(ConfigLevel::Warn.to_severity(), Some(Severity::Warning));
        assert_eq!(ConfigLevel::Error.to_severity(), Some(Severity::Error));
    }

    #[test]
    fn test_rule_category() {
        assert_eq!(RuleCategory::Style.as_str(), "style");
        assert_eq!(RuleCategory::Security.as_str(), "security");
        assert_eq!(RuleCategory::BestPractice.as_str(), "best-practice");
        assert_eq!(RuleCategory::Performance.as_str(), "performance");
    }
}
