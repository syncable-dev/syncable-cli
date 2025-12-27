//! Rule system framework for dclint.
//!
//! Provides the infrastructure for defining and running Docker Compose linting rules.
//! Follows the hadolint-rs pattern with:
//! - `Rule` trait for all rules
//! - `SimpleRule` for stateless checks
//! - `FixableRule` for rules that can auto-fix issues

use crate::analyzer::dclint::parser::ComposeFile;
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, RuleCode, RuleMeta, Severity};

// Rule modules
pub mod dcl001;
pub mod dcl002;
pub mod dcl003;
pub mod dcl004;
pub mod dcl005;
pub mod dcl006;
pub mod dcl007;
pub mod dcl008;
pub mod dcl009;
pub mod dcl010;
pub mod dcl011;
pub mod dcl012;
pub mod dcl013;
pub mod dcl014;
pub mod dcl015;

/// Context for linting a compose file.
#[derive(Debug, Clone)]
pub struct LintContext<'a> {
    /// The parsed compose file.
    pub compose: &'a ComposeFile,
    /// The raw source content.
    pub source: &'a str,
    /// The file path (for error messages).
    pub path: &'a str,
}

impl<'a> LintContext<'a> {
    pub fn new(compose: &'a ComposeFile, source: &'a str, path: &'a str) -> Self {
        Self {
            compose,
            source,
            path,
        }
    }
}

/// A rule that can check Docker Compose files.
pub trait Rule: Send + Sync {
    /// Get the rule code (e.g., "DCL001").
    fn code(&self) -> &RuleCode;

    /// Get the human-readable rule name (e.g., "no-build-and-image").
    fn name(&self) -> &str;

    /// Get the default severity.
    fn severity(&self) -> Severity;

    /// Get the rule category.
    fn category(&self) -> RuleCategory;

    /// Get the rule metadata (description, URL).
    fn meta(&self) -> &RuleMeta;

    /// Whether this rule can auto-fix issues.
    fn is_fixable(&self) -> bool {
        false
    }

    /// Check the compose file and return any failures.
    fn check(&self, context: &LintContext) -> Vec<CheckFailure>;

    /// Auto-fix the source content (if fixable).
    /// Returns the fixed content, or None if no fix was applied.
    fn fix(&self, _source: &str) -> Option<String> {
        None
    }

    /// Get a message for this rule violation.
    fn get_message(&self, _details: &std::collections::HashMap<String, String>) -> String {
        self.meta().description.clone()
    }
}

/// Base implementation for a simple (non-fixable) rule.
pub struct SimpleRule<F>
where
    F: Fn(&LintContext) -> Vec<CheckFailure> + Send + Sync,
{
    code: RuleCode,
    name: String,
    severity: Severity,
    category: RuleCategory,
    meta: RuleMeta,
    check_fn: F,
}

impl<F> SimpleRule<F>
where
    F: Fn(&LintContext) -> Vec<CheckFailure> + Send + Sync,
{
    pub fn new(
        code: impl Into<RuleCode>,
        name: impl Into<String>,
        severity: Severity,
        category: RuleCategory,
        description: impl Into<String>,
        url: impl Into<String>,
        check_fn: F,
    ) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            severity,
            category,
            meta: RuleMeta::new(description, url),
            check_fn,
        }
    }
}

impl<F> Rule for SimpleRule<F>
where
    F: Fn(&LintContext) -> Vec<CheckFailure> + Send + Sync,
{
    fn code(&self) -> &RuleCode {
        &self.code
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn severity(&self) -> Severity {
        self.severity
    }

    fn category(&self) -> RuleCategory {
        self.category
    }

    fn meta(&self) -> &RuleMeta {
        &self.meta
    }

    fn check(&self, context: &LintContext) -> Vec<CheckFailure> {
        (self.check_fn)(context)
    }
}

/// Base implementation for a fixable rule.
pub struct FixableRule<C, X>
where
    C: Fn(&LintContext) -> Vec<CheckFailure> + Send + Sync,
    X: Fn(&str) -> Option<String> + Send + Sync,
{
    code: RuleCode,
    name: String,
    severity: Severity,
    category: RuleCategory,
    meta: RuleMeta,
    check_fn: C,
    fix_fn: X,
}

impl<C, X> FixableRule<C, X>
where
    C: Fn(&LintContext) -> Vec<CheckFailure> + Send + Sync,
    X: Fn(&str) -> Option<String> + Send + Sync,
{
    pub fn new(
        code: impl Into<RuleCode>,
        name: impl Into<String>,
        severity: Severity,
        category: RuleCategory,
        description: impl Into<String>,
        url: impl Into<String>,
        check_fn: C,
        fix_fn: X,
    ) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            severity,
            category,
            meta: RuleMeta::new(description, url),
            check_fn,
            fix_fn,
        }
    }
}

impl<C, X> Rule for FixableRule<C, X>
where
    C: Fn(&LintContext) -> Vec<CheckFailure> + Send + Sync,
    X: Fn(&str) -> Option<String> + Send + Sync,
{
    fn code(&self) -> &RuleCode {
        &self.code
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn severity(&self) -> Severity {
        self.severity
    }

    fn category(&self) -> RuleCategory {
        self.category
    }

    fn meta(&self) -> &RuleMeta {
        &self.meta
    }

    fn is_fixable(&self) -> bool {
        true
    }

    fn check(&self, context: &LintContext) -> Vec<CheckFailure> {
        (self.check_fn)(context)
    }

    fn fix(&self, source: &str) -> Option<String> {
        (self.fix_fn)(source)
    }
}

/// Helper to create a check failure for a rule.
pub fn make_failure(
    code: &RuleCode,
    name: &str,
    severity: Severity,
    category: RuleCategory,
    message: impl Into<String>,
    line: u32,
    column: u32,
    fixable: bool,
) -> CheckFailure {
    CheckFailure::new(
        code.clone(),
        name,
        severity,
        category,
        message,
        line,
        column,
    )
    .with_fixable(fixable)
}

/// Get all enabled rules.
pub fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(dcl001::rule()),
        Box::new(dcl002::rule()),
        Box::new(dcl003::rule()),
        Box::new(dcl004::rule()),
        Box::new(dcl005::rule()),
        Box::new(dcl006::rule()),
        Box::new(dcl007::rule()),
        Box::new(dcl008::rule()),
        Box::new(dcl009::rule()),
        Box::new(dcl010::rule()),
        Box::new(dcl011::rule()),
        Box::new(dcl012::rule()),
        Box::new(dcl013::rule()),
        Box::new(dcl014::rule()),
        Box::new(dcl015::rule()),
    ]
}

/// Get rule definitions for documentation.
pub fn rule_definitions() -> Vec<RuleDefinition> {
    all_rules()
        .iter()
        .map(|r| RuleDefinition {
            code: r.code().clone(),
            name: r.name().to_string(),
            severity: r.severity(),
            category: r.category(),
            description: r.meta().description.clone(),
            url: r.meta().url.clone(),
            fixable: r.is_fixable(),
        })
        .collect()
}

/// Rule definition for documentation/introspection.
#[derive(Debug, Clone)]
pub struct RuleDefinition {
    pub code: RuleCode,
    pub name: String,
    pub severity: Severity,
    pub category: RuleCategory,
    pub description: String,
    pub url: String,
    pub fixable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_rules_count() {
        let rules = all_rules();
        assert_eq!(rules.len(), 15, "Expected 15 rules");
    }

    #[test]
    fn test_rule_codes_unique() {
        let rules = all_rules();
        let mut codes: Vec<String> = rules.iter().map(|r| r.code().to_string()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), 15, "Rule codes should be unique");
    }

    #[test]
    fn test_rule_names_unique() {
        let rules = all_rules();
        let mut names: Vec<String> = rules.iter().map(|r| r.name().to_string()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 15, "Rule names should be unique");
    }
}
