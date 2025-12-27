//! Core types for the hadolint-rs linter.
//!
//! These types match the Haskell hadolint implementation for compatibility:
//! - `Severity` - Rule violation severity levels
//! - `RuleCode` - Rule identifiers (e.g., "DL3008")
//! - `CheckFailure` - A single rule violation
//! - `State` - Stateful rule accumulator

use std::cmp::Ordering;
use std::fmt;

/// Severity levels for rule violations.
///
/// Ordered from most severe to least severe:
/// `Error > Warning > Info > Style > Ignore`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Severity {
    /// Critical issues that should always be fixed
    Error,
    /// Important issues that should usually be fixed
    Warning,
    /// Informational suggestions for improvement
    #[default]
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
            "ignore" | "none" => Some(Self::Ignore),
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

/// A rule code identifier (e.g., "DL3008", "SC2086").
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

    /// Check if this is a Dockerfile rule (DL prefix).
    pub fn is_dockerfile_rule(&self) -> bool {
        self.0.starts_with("DL")
    }

    /// Check if this is a ShellCheck rule (SC prefix).
    pub fn is_shellcheck_rule(&self) -> bool {
        self.0.starts_with("SC")
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
    /// The severity of the violation.
    pub severity: Severity,
    /// A human-readable message describing the violation.
    pub message: String,
    /// The line number where the violation occurred (1-indexed).
    pub line: u32,
    /// Optional column number (1-indexed).
    pub column: Option<u32>,
}

impl CheckFailure {
    /// Create a new check failure.
    pub fn new(
        code: impl Into<RuleCode>,
        severity: Severity,
        message: impl Into<String>,
        line: u32,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            line,
            column: None,
        }
    }

    /// Create a check failure with column information.
    pub fn with_column(
        code: impl Into<RuleCode>,
        severity: Severity,
        message: impl Into<String>,
        line: u32,
        column: u32,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            line,
            column: Some(column),
        }
    }
}

impl Ord for CheckFailure {
    fn cmp(&self, other: &Self) -> Ordering {
        // Sort by line number first
        self.line.cmp(&other.line)
    }
}

impl PartialOrd for CheckFailure {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// State accumulator for stateful rules.
///
/// Used by `custom_rule` and `very_custom_rule` to track state across
/// multiple instructions during the analysis pass.
#[derive(Debug, Clone)]
pub struct State<T> {
    /// Accumulated failures found during analysis.
    pub failures: Vec<CheckFailure>,
    /// Custom state for the rule.
    pub state: T,
}

impl<T: Default> Default for State<T> {
    fn default() -> Self {
        Self {
            failures: Vec::new(),
            state: T::default(),
        }
    }
}

impl<T> State<T> {
    /// Create a new state with the given initial state.
    pub fn new(state: T) -> Self {
        Self {
            failures: Vec::new(),
            state,
        }
    }

    /// Add a failure to the state.
    pub fn add_failure(&mut self, failure: CheckFailure) {
        self.failures.push(failure);
    }

    /// Modify the state with a function.
    pub fn modify<F>(&mut self, f: F)
    where
        F: FnOnce(&mut T),
    {
        f(&mut self.state);
    }

    /// Replace the state entirely.
    pub fn replace_state(&mut self, new_state: T) {
        self.state = new_state;
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
        assert_eq!(Severity::parse("none"), Some(Severity::Ignore));
        assert_eq!(Severity::parse("invalid"), None);
    }

    #[test]
    fn test_rule_code() {
        let dl_code = RuleCode::new("DL3008");
        assert!(dl_code.is_dockerfile_rule());
        assert!(!dl_code.is_shellcheck_rule());

        let sc_code = RuleCode::new("SC2086");
        assert!(!sc_code.is_dockerfile_rule());
        assert!(sc_code.is_shellcheck_rule());
    }

    #[test]
    fn test_check_failure_ordering() {
        let f1 = CheckFailure::new("DL3008", Severity::Warning, "msg1", 5);
        let f2 = CheckFailure::new("DL3009", Severity::Info, "msg2", 10);
        let f3 = CheckFailure::new("DL3010", Severity::Error, "msg3", 3);

        let mut failures = vec![f1.clone(), f2.clone(), f3.clone()];
        failures.sort();

        assert_eq!(failures[0].line, 3);
        assert_eq!(failures[1].line, 5);
        assert_eq!(failures[2].line, 10);
    }

    #[test]
    fn test_state() {
        let mut state: State<i32> = State::new(0);
        assert_eq!(state.state, 0);
        assert!(state.failures.is_empty());

        state.modify(|s| *s += 10);
        assert_eq!(state.state, 10);

        state.add_failure(CheckFailure::new("DL3008", Severity::Warning, "test", 1));
        assert_eq!(state.failures.len(), 1);
    }
}
