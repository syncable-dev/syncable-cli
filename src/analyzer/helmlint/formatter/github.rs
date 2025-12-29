//! GitHub Actions formatter for helmlint results.
//!
//! Produces GitHub Actions workflow command annotations.
//! See: https://docs.github.com/en/actions/reference/workflow-commands-for-github-actions

use crate::analyzer::helmlint::lint::LintResult;
use crate::analyzer::helmlint::types::Severity;

/// Format a lint result as GitHub Actions annotations.
pub fn format(result: &LintResult) -> String {
    let mut output = String::new();

    // Output parse errors as errors
    for error in &result.parse_errors {
        output.push_str(&format!(
            "::error file={},title=Parse Error::{}\n",
            result.chart_path, error
        ));
    }

    // Output failures as annotations
    for failure in &result.failures {
        let level = match failure.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "notice",
            Severity::Style => "notice",
            Severity::Ignore => continue, // Skip ignored
        };

        let file = failure.file.display().to_string();
        let line = failure.line;
        let title = &failure.code;
        let message = escape_message(&failure.message);

        // Format: ::level file=path,line=N,col=N,title=TITLE::MESSAGE
        let annotation = match failure.column {
            Some(col) => format!(
                "::{}file={},line={},col={},title={}::{}\n",
                level, file, line, col, title, message
            ),
            None => format!(
                "::{}file={},line={},title={}::{}\n",
                level, file, line, title, message
            ),
        };

        output.push_str(&annotation);
    }

    // Summary annotation
    if !result.failures.is_empty() || !result.parse_errors.is_empty() {
        let total = result.failures.len() + result.parse_errors.len();
        let summary = format!(
            "Helmlint found {} {} ({} errors, {} warnings)",
            total,
            if total == 1 { "issue" } else { "issues" },
            result.error_count + result.parse_errors.len(),
            result.warning_count
        );

        if result.error_count > 0 || !result.parse_errors.is_empty() {
            output.push_str(&format!("::error::{}\n", summary));
        } else {
            output.push_str(&format!("::warning::{}\n", summary));
        }
    }

    output
}

/// Escape a message for GitHub Actions annotation format.
/// GitHub Actions uses % encoding for special characters.
fn escape_message(message: &str) -> String {
    message
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace(':', "%3A")
        .replace(',', "%2C")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::helmlint::types::{CheckFailure, RuleCategory, Severity};

    #[test]
    fn test_github_format_empty() {
        let result = LintResult::new("test-chart");
        let output = format(&result);
        assert!(output.is_empty());
    }

    #[test]
    fn test_github_format_error() {
        let mut result = LintResult::new("test-chart");
        result.failures.push(CheckFailure::new(
            "HL1001",
            Severity::Error,
            "Missing Chart.yaml",
            "Chart.yaml",
            1,
            RuleCategory::Structure,
        ));
        result.error_count = 1;

        let output = format(&result);
        assert!(output.contains("::error"));
        assert!(output.contains("file=Chart.yaml"));
        assert!(output.contains("line=1"));
        assert!(output.contains("title=HL1001"));
    }

    #[test]
    fn test_github_format_warning() {
        let mut result = LintResult::new("test-chart");
        result.failures.push(CheckFailure::new(
            "HL1006",
            Severity::Warning,
            "Missing description",
            "Chart.yaml",
            5,
            RuleCategory::Structure,
        ));
        result.warning_count = 1;

        let output = format(&result);
        assert!(output.contains("::warning"));
    }

    #[test]
    fn test_escape_message() {
        assert_eq!(escape_message("hello:world"), "hello%3Aworld");
        assert_eq!(escape_message("a,b"), "a%2Cb");
        assert_eq!(escape_message("line1\nline2"), "line1%0Aline2");
    }
}
