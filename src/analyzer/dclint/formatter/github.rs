//! GitHub Actions output formatter for dclint.
//!
//! Produces output in GitHub Actions workflow command format:
//! ::error file={name},line={line},col={col}::{message}

use crate::analyzer::dclint::lint::LintResult;
use crate::analyzer::dclint::types::Severity;

/// Format lint results for GitHub Actions.
pub fn format(results: &[LintResult]) -> String {
    let mut output = String::new();

    for result in results {
        // Parse errors
        for err in &result.parse_errors {
            output.push_str(&format!(
                "::error file={}::Parse error: {}\n",
                result.file_path,
                escape_github(err)
            ));
        }

        // Failures
        for failure in &result.failures {
            let level = match failure.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info | Severity::Style => "notice",
            };

            output.push_str(&format!(
                "::{} file={},line={},col={},title={}::{}\n",
                level,
                result.file_path,
                failure.line,
                failure.column,
                failure.code,
                escape_github(&failure.message)
            ));
        }
    }

    output
}

/// Escape special characters for GitHub Actions.
fn escape_github(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::dclint::types::{CheckFailure, RuleCategory};

    #[test]
    fn test_github_format() {
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

        let output = format(&[result]);
        assert!(output.contains("::error"));
        assert!(output.contains("file=docker-compose.yml"));
        assert!(output.contains("line=5"));
        assert!(output.contains("col=1"));
        assert!(output.contains("title=DCL001"));
    }

    #[test]
    fn test_github_format_warning() {
        let mut result = LintResult::new("docker-compose.yml");
        result.failures.push(CheckFailure::new(
            "DCL006",
            "no-version-field",
            Severity::Warning,
            RuleCategory::Style,
            "Version field is deprecated",
            1,
            1,
        ));

        let output = format(&[result]);
        assert!(output.contains("::warning"));
    }

    #[test]
    fn test_github_format_info() {
        let mut result = LintResult::new("docker-compose.yml");
        result.failures.push(CheckFailure::new(
            "DCL007",
            "require-project-name",
            Severity::Info,
            RuleCategory::BestPractice,
            "Consider adding name field",
            1,
            1,
        ));

        let output = format(&[result]);
        assert!(output.contains("::notice"));
    }

    #[test]
    fn test_escape_github() {
        assert_eq!(escape_github("hello\nworld"), "hello%0Aworld");
        assert_eq!(escape_github("100%"), "100%25");
    }
}
