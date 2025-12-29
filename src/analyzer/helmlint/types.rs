//! Core types for the helmlint linter.
//!
//! These types provide the foundation for rule violations and severity levels:
//! - `Severity` - Rule violation severity levels
//! - `RuleCode` - Rule identifiers (e.g., "HL1001")
//! - `CheckFailure` - A single rule violation
//! - `RuleCategory` - Categories of rules

use std::cmp::Ordering;
use std::fmt;
use std::path::PathBuf;

/// Severity levels for rule violations.
///
/// Ordered from most severe to least severe:
/// `Error > Warning > Info > Style > Ignore`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Severity {
    /// Critical issues that should always be fixed
    Error,
    /// Important issues that should usually be fixed
    #[default]
    Warning,
    /// Informational suggestions for improvement
    Info,
    /// Style recommendations
    Style,
    /// Ignored (rule disabled)
    Ignore,
}

impl Severity {
    /// Parse a severity from a string (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "error" => Some(Self::Error),
            "warning" => Some(Self::Warning),
            "info" => Some(Self::Info),
            "style" => Some(Self::Style),
            "ignore" | "none" | "off" => Some(Self::Ignore),
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
            Self::Ignore => "ignore",
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
            Self::Style => 3,
            Self::Ignore => 4,
        };
        let other_val = match other {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Info => 2,
            Self::Style => 3,
            Self::Ignore => 4,
        };
        // Reverse so Error > Warning > Info > Style > Ignore
        other_val.cmp(&self_val)
    }
}

impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Rule categories for organizing lint rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleCategory {
    /// Chart structure rules (HL1xxx)
    Structure,
    /// Values validation rules (HL2xxx)
    Values,
    /// Template syntax rules (HL3xxx)
    Template,
    /// Security rules (HL4xxx)
    Security,
    /// Best practice rules (HL5xxx)
    BestPractice,
}

impl RuleCategory {
    /// Get the code prefix for this category.
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Structure => "HL1",
            Self::Values => "HL2",
            Self::Template => "HL3",
            Self::Security => "HL4",
            Self::BestPractice => "HL5",
        }
    }

    /// Get the display name for this category.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Structure => "Chart Structure",
            Self::Values => "Values Validation",
            Self::Template => "Template Syntax",
            Self::Security => "Security",
            Self::BestPractice => "Best Practice",
        }
    }

    /// Determine category from rule code.
    pub fn from_code(code: &str) -> Option<Self> {
        if code.starts_with("HL1") {
            Some(Self::Structure)
        } else if code.starts_with("HL2") {
            Some(Self::Values)
        } else if code.starts_with("HL3") {
            Some(Self::Template)
        } else if code.starts_with("HL4") {
            Some(Self::Security)
        } else if code.starts_with("HL5") {
            Some(Self::BestPractice)
        } else {
            None
        }
    }
}

impl fmt::Display for RuleCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// A rule code identifier (e.g., "HL1001", "HL4002").
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

    /// Get the category for this rule.
    pub fn category(&self) -> Option<RuleCategory> {
        RuleCategory::from_code(&self.0)
    }

    /// Check if this is a structure rule (HL1xxx).
    pub fn is_structure_rule(&self) -> bool {
        self.0.starts_with("HL1")
    }

    /// Check if this is a values rule (HL2xxx).
    pub fn is_values_rule(&self) -> bool {
        self.0.starts_with("HL2")
    }

    /// Check if this is a template rule (HL3xxx).
    pub fn is_template_rule(&self) -> bool {
        self.0.starts_with("HL3")
    }

    /// Check if this is a security rule (HL4xxx).
    pub fn is_security_rule(&self) -> bool {
        self.0.starts_with("HL4")
    }

    /// Check if this is a best practice rule (HL5xxx).
    pub fn is_best_practice_rule(&self) -> bool {
        self.0.starts_with("HL5")
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

/// Metadata about a lint rule.
#[derive(Debug, Clone)]
pub struct RuleMeta {
    /// The rule code (e.g., "HL1001").
    pub code: RuleCode,
    /// Short name for the rule.
    pub name: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Default severity level.
    pub severity: Severity,
    /// Rule category.
    pub category: RuleCategory,
    /// Whether this rule can be auto-fixed.
    pub fixable: bool,
}

impl RuleMeta {
    /// Create new rule metadata.
    pub const fn new(
        _code: &'static str,
        name: &'static str,
        description: &'static str,
        severity: Severity,
        category: RuleCategory,
        fixable: bool,
    ) -> Self {
        Self {
            code: RuleCode(String::new()), // Will be set properly at runtime
            name,
            description,
            severity,
            category,
            fixable,
        }
    }
}

/// A check failure (rule violation) found during linting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckFailure {
    /// The rule code that was violated.
    pub code: RuleCode,
    /// The severity of the violation.
    pub severity: Severity,
    /// A human-readable message describing the violation.
    pub message: String,
    /// The file where the violation occurred (relative to chart root).
    pub file: PathBuf,
    /// The line number where the violation occurred (1-indexed).
    pub line: u32,
    /// Optional column number (1-indexed).
    pub column: Option<u32>,
    /// Whether this violation can be auto-fixed.
    pub fixable: bool,
    /// The rule category.
    pub category: RuleCategory,
}

impl CheckFailure {
    /// Create a new check failure.
    pub fn new(
        code: impl Into<RuleCode>,
        severity: Severity,
        message: impl Into<String>,
        file: impl Into<PathBuf>,
        line: u32,
        category: RuleCategory,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            file: file.into(),
            line,
            column: None,
            fixable: false,
            category,
        }
    }

    /// Create a check failure with column information.
    pub fn with_column(
        code: impl Into<RuleCode>,
        severity: Severity,
        message: impl Into<String>,
        file: impl Into<PathBuf>,
        line: u32,
        column: u32,
        category: RuleCategory,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            file: file.into(),
            line,
            column: Some(column),
            fixable: false,
            category,
        }
    }

    /// Set whether this failure is fixable.
    pub fn set_fixable(mut self, fixable: bool) -> Self {
        self.fixable = fixable;
        self
    }
}

impl Ord for CheckFailure {
    fn cmp(&self, other: &Self) -> Ordering {
        // Sort by file first, then line number
        match self.file.cmp(&other.file) {
            Ordering::Equal => self.line.cmp(&other.line),
            other => other,
        }
    }
}

impl PartialOrd for CheckFailure {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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
        assert!(Severity::Style > Severity::Ignore);
    }

    #[test]
    fn test_severity_from_str() {
        assert_eq!(Severity::parse("error"), Some(Severity::Error));
        assert_eq!(Severity::parse("WARNING"), Some(Severity::Warning));
        assert_eq!(Severity::parse("Info"), Some(Severity::Info));
        assert_eq!(Severity::parse("style"), Some(Severity::Style));
        assert_eq!(Severity::parse("ignore"), Some(Severity::Ignore));
        assert_eq!(Severity::parse("off"), Some(Severity::Ignore));
        assert_eq!(Severity::parse("invalid"), None);
    }

    #[test]
    fn test_rule_code_category() {
        assert!(RuleCode::new("HL1001").is_structure_rule());
        assert!(RuleCode::new("HL2001").is_values_rule());
        assert!(RuleCode::new("HL3001").is_template_rule());
        assert!(RuleCode::new("HL4001").is_security_rule());
        assert!(RuleCode::new("HL5001").is_best_practice_rule());
    }

    #[test]
    fn test_rule_category_from_code() {
        assert_eq!(
            RuleCategory::from_code("HL1001"),
            Some(RuleCategory::Structure)
        );
        assert_eq!(
            RuleCategory::from_code("HL2001"),
            Some(RuleCategory::Values)
        );
        assert_eq!(
            RuleCategory::from_code("HL3001"),
            Some(RuleCategory::Template)
        );
        assert_eq!(
            RuleCategory::from_code("HL4001"),
            Some(RuleCategory::Security)
        );
        assert_eq!(
            RuleCategory::from_code("HL5001"),
            Some(RuleCategory::BestPractice)
        );
        assert_eq!(RuleCategory::from_code("XX1001"), None);
    }

    #[test]
    fn test_check_failure_ordering() {
        let f1 = CheckFailure::new(
            "HL1001",
            Severity::Warning,
            "msg1",
            "Chart.yaml",
            5,
            RuleCategory::Structure,
        );
        let f2 = CheckFailure::new(
            "HL1002",
            Severity::Info,
            "msg2",
            "Chart.yaml",
            10,
            RuleCategory::Structure,
        );
        let f3 = CheckFailure::new(
            "HL1003",
            Severity::Error,
            "msg3",
            "Chart.yaml",
            3,
            RuleCategory::Structure,
        );
        let f4 = CheckFailure::new(
            "HL3001",
            Severity::Error,
            "msg4",
            "templates/deployment.yaml",
            1,
            RuleCategory::Template,
        );

        let mut failures = vec![f1.clone(), f2.clone(), f3.clone(), f4.clone()];
        failures.sort();

        assert_eq!(failures[0].line, 3);
        assert_eq!(failures[1].line, 5);
        assert_eq!(failures[2].line, 10);
        assert_eq!(
            failures[3].file.to_str().unwrap(),
            "templates/deployment.yaml"
        );
    }
}
