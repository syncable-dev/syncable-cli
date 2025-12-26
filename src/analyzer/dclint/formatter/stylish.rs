//! Stylish (colored terminal) output formatter for dclint.

use crate::analyzer::dclint::lint::LintResult;
use crate::analyzer::dclint::types::Severity;

/// Format lint results in stylish format (colored terminal output).
pub fn format(results: &[LintResult]) -> String {
    let mut output = String::new();
    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut total_fixable = 0;

    for result in results {
        if result.failures.is_empty() && result.parse_errors.is_empty() {
            continue;
        }

        // File header
        output.push_str(&format!("\n{}\n", result.file_path));

        // Parse errors
        for err in &result.parse_errors {
            output.push_str(&format!("  error  {}\n", err));
            total_errors += 1;
        }

        // Failures
        for failure in &result.failures {
            let severity_str = match failure.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
                Severity::Style => "style",
            };

            let fixable_str = if failure.fixable { " (fixable)" } else { "" };

            output.push_str(&format!(
                "  {}:{}  {}  {}  {}{}\n",
                failure.line,
                failure.column,
                severity_str,
                failure.message,
                failure.code,
                fixable_str
            ));

            match failure.severity {
                Severity::Error => total_errors += 1,
                Severity::Warning => total_warnings += 1,
                _ => {}
            }

            if failure.fixable {
                total_fixable += 1;
            }
        }
    }

    // Summary
    if total_errors > 0 || total_warnings > 0 {
        output.push('\n');

        let mut parts = Vec::new();
        if total_errors > 0 {
            parts.push(format!(
                "{} {}",
                total_errors,
                if total_errors == 1 { "error" } else { "errors" }
            ));
        }
        if total_warnings > 0 {
            parts.push(format!(
                "{} {}",
                total_warnings,
                if total_warnings == 1 {
                    "warning"
                } else {
                    "warnings"
                }
            ));
        }

        output.push_str(&format!(
            "  {} problem{}\n",
            parts.join(" and "),
            if total_errors + total_warnings == 1 {
                ""
            } else {
                "s"
            }
        ));

        if total_fixable > 0 {
            output.push_str(&format!(
                "  {} {} potentially fixable with --fix\n",
                total_fixable,
                if total_fixable == 1 { "is" } else { "are" }
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::dclint::types::{CheckFailure, RuleCategory};

    #[test]
    fn test_stylish_format() {
        let mut result = LintResult::new("docker-compose.yml");
        result.failures.push(CheckFailure::new(
            "DCL001",
            "no-build-and-image",
            Severity::Error,
            RuleCategory::BestPractice,
            "Service has both build and image",
            5,
            1,
        ));
        result.error_count = 1;

        let output = format(&[result]);
        assert!(output.contains("docker-compose.yml"));
        assert!(output.contains("5:1"));
        assert!(output.contains("error"));
        assert!(output.contains("DCL001"));
        assert!(output.contains("1 error"));
    }

    #[test]
    fn test_stylish_format_multiple() {
        let mut result = LintResult::new("docker-compose.yml");
        result.failures.push(CheckFailure::new(
            "DCL001",
            "test",
            Severity::Error,
            RuleCategory::BestPractice,
            "Error 1",
            5,
            1,
        ));
        result.failures.push(
            CheckFailure::new(
                "DCL006",
                "test",
                Severity::Warning,
                RuleCategory::Style,
                "Warning 1",
                1,
                1,
            )
            .with_fixable(true),
        );
        result.error_count = 1;
        result.warning_count = 1;

        let output = format(&[result]);
        assert!(output.contains("1 error"));
        assert!(output.contains("1 warning"));
        assert!(output.contains("fixable"));
    }

    #[test]
    fn test_stylish_format_empty() {
        let result = LintResult::new("docker-compose.yml");
        let output = format(&[result]);
        assert!(output.is_empty());
    }
}
