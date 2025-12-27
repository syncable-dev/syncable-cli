//! Main linting orchestration for dclint.
//!
//! This module ties together parsing, rules, and pragmas to provide
//! the main linting API.

use std::path::Path;

use crate::analyzer::dclint::config::DclintConfig;
use crate::analyzer::dclint::parser::{ComposeFile, parse_compose};
use crate::analyzer::dclint::pragma::{
    PragmaState, extract_pragmas, starts_with_disable_file_comment,
};
use crate::analyzer::dclint::rules::{LintContext, all_rules};
use crate::analyzer::dclint::types::{CheckFailure, Severity};

/// Result of linting a Docker Compose file.
#[derive(Debug, Clone)]
pub struct LintResult {
    /// The file path that was linted.
    pub file_path: String,
    /// Rule violations found.
    pub failures: Vec<CheckFailure>,
    /// Parse errors (if any).
    pub parse_errors: Vec<String>,
    /// Number of errors.
    pub error_count: usize,
    /// Number of warnings.
    pub warning_count: usize,
    /// Number of fixable errors.
    pub fixable_error_count: usize,
    /// Number of fixable warnings.
    pub fixable_warning_count: usize,
}

impl LintResult {
    /// Create a new empty result.
    pub fn new(file_path: impl Into<String>) -> Self {
        Self {
            file_path: file_path.into(),
            failures: Vec::new(),
            parse_errors: Vec::new(),
            error_count: 0,
            warning_count: 0,
            fixable_error_count: 0,
            fixable_warning_count: 0,
        }
    }

    /// Update counts based on failures.
    fn update_counts(&mut self) {
        self.error_count = self
            .failures
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count();
        self.warning_count = self
            .failures
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count();
        self.fixable_error_count = self
            .failures
            .iter()
            .filter(|f| f.fixable && f.severity == Severity::Error)
            .count();
        self.fixable_warning_count = self
            .failures
            .iter()
            .filter(|f| f.fixable && f.severity == Severity::Warning)
            .count();
    }

    /// Check if there are any failures.
    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    /// Check if there are any errors (failure with Error severity).
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Check if there are any warnings (failure with Warning severity).
    pub fn has_warnings(&self) -> bool {
        self.warning_count > 0
    }

    /// Get the maximum severity in the results.
    pub fn max_severity(&self) -> Option<Severity> {
        self.failures.iter().map(|f| f.severity).max()
    }

    /// Check if the results should cause a non-zero exit.
    pub fn should_fail(&self, threshold: Severity) -> bool {
        if let Some(max) = self.max_severity() {
            max >= threshold
        } else {
            false
        }
    }

    /// Sort failures by line number.
    pub fn sort(&mut self) {
        self.failures.sort();
    }
}

/// Lint a Docker Compose file string.
pub fn lint(content: &str, config: &DclintConfig) -> LintResult {
    lint_with_path(content, "<inline>", config)
}

/// Lint a Docker Compose file string with a path for error messages.
pub fn lint_with_path(content: &str, path: &str, config: &DclintConfig) -> LintResult {
    let mut result = LintResult::new(path);

    // Check for disable-file pragma
    if !config.disable_ignore_pragma && starts_with_disable_file_comment(content) {
        return result; // File is completely disabled
    }

    // Parse the compose file
    let compose = match parse_compose(content) {
        Ok(c) => c,
        Err(err) => {
            result.parse_errors.push(err.to_string());
            return result;
        }
    };

    // Extract pragmas
    let pragmas = if config.disable_ignore_pragma {
        PragmaState::new()
    } else {
        extract_pragmas(content)
    };

    // Run all rules
    let failures = run_rules(&compose, content, path, config, &pragmas);

    // Apply config filters
    result.failures = failures
        .into_iter()
        .filter(|f| {
            // Check severity threshold
            let effective_severity = config.effective_severity(&f.code, f.severity);
            config.should_report(effective_severity)
        })
        .filter(|f| !config.is_rule_ignored(&f.code))
        .filter(|f| !pragmas.is_ignored(&f.code, f.line))
        .filter(|f| {
            // Filter fixable-only if requested
            if config.fixable_only { f.fixable } else { true }
        })
        .map(|mut f| {
            // Apply severity overrides
            f.severity = config.effective_severity(&f.code, f.severity);
            f
        })
        .collect();

    // Sort and update counts
    result.sort();
    result.update_counts();

    result
}

/// Lint a Docker Compose file from a file path.
pub fn lint_file(path: &Path, config: &DclintConfig) -> LintResult {
    let path_str = path.display().to_string();

    // Check if excluded
    if config.is_excluded(&path_str) {
        return LintResult::new(path_str);
    }

    match std::fs::read_to_string(path) {
        Ok(content) => lint_with_path(&content, &path_str, config),
        Err(err) => {
            let mut result = LintResult::new(path_str);
            result
                .parse_errors
                .push(format!("Failed to read file: {}", err));
            result
        }
    }
}

/// Run all enabled rules on the compose file.
fn run_rules(
    compose: &ComposeFile,
    source: &str,
    path: &str,
    config: &DclintConfig,
    _pragmas: &PragmaState,
) -> Vec<CheckFailure> {
    let rules = all_rules();
    let ctx = LintContext::new(compose, source, path);
    let mut all_failures = Vec::new();

    for rule in rules {
        // Skip ignored rules
        if config.is_rule_ignored(rule.code()) {
            continue;
        }

        // Run the rule
        let failures = rule.check(&ctx);
        all_failures.extend(failures);
    }

    all_failures
}

/// Apply auto-fixes to source content.
pub fn fix_content(content: &str, config: &DclintConfig) -> String {
    // Check for disable-file pragma
    if !config.disable_ignore_pragma && starts_with_disable_file_comment(content) {
        return content.to_string();
    }

    let rules = all_rules();
    let mut fixed = content.to_string();

    // Apply fixes from all fixable rules
    for rule in rules {
        if rule.is_fixable() && !config.is_rule_ignored(rule.code()) {
            if let Some(new_content) = rule.fix(&fixed) {
                fixed = new_content;
            }
        }
    }

    fixed
}

/// Apply auto-fixes to a file.
pub fn fix_file(
    path: &Path,
    config: &DclintConfig,
    dry_run: bool,
) -> Result<Option<String>, String> {
    let path_str = path.display().to_string();

    // Check if excluded
    if config.is_excluded(&path_str) {
        return Ok(None);
    }

    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let fixed = fix_content(&content, config);

    if fixed == content {
        return Ok(None); // No changes
    }

    if !dry_run {
        std::fs::write(path, &fixed).map_err(|e| format!("Failed to write file: {}", e))?;
    }

    Ok(Some(fixed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lint_empty() {
        let result = lint("", &DclintConfig::default());
        // Empty content should fail to parse or have no services
        assert!(result.failures.is_empty() || !result.parse_errors.is_empty());
    }

    #[test]
    fn test_lint_valid_compose() {
        let yaml = r#"
name: myproject
services:
  web:
    image: nginx:1.25
    ports:
      - "8080:80"
"#;
        let result = lint(yaml, &DclintConfig::default());
        assert!(result.parse_errors.is_empty());
        // May have some style warnings
    }

    #[test]
    fn test_lint_with_violations() {
        let yaml = r#"
services:
  web:
    build: .
    image: nginx:latest
"#;
        let result = lint(yaml, &DclintConfig::default());
        assert!(result.parse_errors.is_empty());

        // Should catch DCL001 (build+image) and DCL011 (latest tag)
        let codes: Vec<&str> = result.failures.iter().map(|f| f.code.as_str()).collect();
        assert!(
            codes.contains(&"DCL001"),
            "Should detect build+image violation"
        );
    }

    #[test]
    fn test_lint_with_ignore() {
        let yaml = r#"
services:
  web:
    build: .
    image: nginx:latest
"#;
        let config = DclintConfig::default().ignore("DCL001");
        let result = lint(yaml, &config);

        // DCL001 should be ignored
        let codes: Vec<&str> = result.failures.iter().map(|f| f.code.as_str()).collect();
        assert!(!codes.contains(&"DCL001"));
    }

    #[test]
    fn test_lint_with_pragma_ignore() {
        let yaml = r#"
# dclint-disable DCL001
services:
  web:
    build: .
    image: nginx:latest
"#;
        let result = lint(yaml, &DclintConfig::default());

        // DCL001 should be ignored via pragma
        let codes: Vec<&str> = result.failures.iter().map(|f| f.code.as_str()).collect();
        assert!(!codes.contains(&"DCL001"));
    }

    #[test]
    fn test_lint_disable_file() {
        let yaml = r#"
# dclint-disable-file
services:
  web:
    build: .
    image: nginx:latest
"#;
        let result = lint(yaml, &DclintConfig::default());

        // All rules disabled for file
        assert!(result.failures.is_empty());
    }

    #[test]
    fn test_counts() {
        let yaml = r#"
services:
  web:
    build: .
    image: nginx:latest
  db:
    image: postgres
"#;
        let result = lint(yaml, &DclintConfig::default());

        // Should have at least one error (DCL001) and some warnings
        assert!(result.error_count + result.warning_count > 0);
    }

    #[test]
    fn test_fix_content() {
        let yaml = r#"version: "3.8"

services:
  web:
    image: nginx
"#;
        let config = DclintConfig::default();
        let fixed = fix_content(yaml, &config);

        // DCL006 fix should remove version field
        assert!(!fixed.contains("version"));
    }

    #[test]
    fn test_result_sort() {
        let mut result = LintResult::new("test.yml");
        result.failures.push(CheckFailure::new(
            "DCL001",
            "test",
            Severity::Error,
            crate::analyzer::dclint::types::RuleCategory::BestPractice,
            "msg",
            10,
            1,
        ));
        result.failures.push(CheckFailure::new(
            "DCL002",
            "test",
            Severity::Warning,
            crate::analyzer::dclint::types::RuleCategory::Style,
            "msg",
            5,
            1,
        ));
        result.failures.push(CheckFailure::new(
            "DCL003",
            "test",
            Severity::Info,
            crate::analyzer::dclint::types::RuleCategory::Style,
            "msg",
            1,
            1,
        ));

        result.sort();

        assert_eq!(result.failures[0].line, 1);
        assert_eq!(result.failures[1].line, 5);
        assert_eq!(result.failures[2].line, 10);
    }
}
