//! Output formatters for dclint results.
//!
//! Provides various output formats for lint results:
//! - JSON - Machine-readable JSON output
//! - Stylish - Colored terminal output (default)
//! - Compact - Single line per issue
//! - GitHub - GitHub Actions annotations
//! - CodeClimate - CodeClimate format
//! - JUnit - JUnit XML format

pub mod github;
pub mod json;
pub mod stylish;

use crate::analyzer::dclint::lint::LintResult;

/// Output format for lint results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// JSON format for machine processing
    Json,
    /// Stylish colored terminal output (default)
    #[default]
    Stylish,
    /// Single line per issue
    Compact,
    /// GitHub Actions annotations
    GitHub,
    /// CodeClimate format
    CodeClimate,
    /// JUnit XML format
    JUnit,
}

impl OutputFormat {
    /// Parse from string (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "stylish" => Some(Self::Stylish),
            "compact" => Some(Self::Compact),
            "github" | "github-actions" => Some(Self::GitHub),
            "codeclimate" | "code-climate" => Some(Self::CodeClimate),
            "junit" => Some(Self::JUnit),
            _ => None,
        }
    }
}

/// Format lint results according to the specified format.
pub fn format_results(results: &[LintResult], format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => json::format(results),
        OutputFormat::Stylish => stylish::format(results),
        OutputFormat::Compact => format_compact(results),
        OutputFormat::GitHub => github::format(results),
        OutputFormat::CodeClimate => format_codeclimate(results),
        OutputFormat::JUnit => format_junit(results),
    }
}

/// Format a single result.
pub fn format_result(result: &LintResult, format: OutputFormat) -> String {
    format_results(std::slice::from_ref(result), format)
}

/// Format results as a string.
pub fn format_result_to_string(result: &LintResult, format: OutputFormat) -> String {
    format_result(result, format)
}

/// Compact format (one line per issue).
fn format_compact(results: &[LintResult]) -> String {
    let mut output = String::new();

    for result in results {
        for failure in &result.failures {
            output.push_str(&format!(
                "{}:{}:{}: {} [{}] {}\n",
                result.file_path,
                failure.line,
                failure.column,
                failure.severity,
                failure.code,
                failure.message
            ));
        }
    }

    output
}

/// CodeClimate format.
fn format_codeclimate(results: &[LintResult]) -> String {
    let mut issues = Vec::new();

    for result in results {
        for failure in &result.failures {
            issues.push(serde_json::json!({
                "type": "issue",
                "check_name": failure.code.as_str(),
                "description": failure.message,
                "content": {
                    "body": failure.message
                },
                "categories": [failure.category.as_str()],
                "location": {
                    "path": result.file_path,
                    "lines": {
                        "begin": failure.line,
                        "end": failure.end_line.unwrap_or(failure.line)
                    }
                },
                "severity": match failure.severity {
                    crate::analyzer::dclint::types::Severity::Error => "critical",
                    crate::analyzer::dclint::types::Severity::Warning => "major",
                    crate::analyzer::dclint::types::Severity::Info => "minor",
                    crate::analyzer::dclint::types::Severity::Style => "info",
                },
                "fingerprint": format!("{}-{}-{}", failure.code, result.file_path, failure.line)
            }));
        }
    }

    serde_json::to_string_pretty(&issues).unwrap_or_else(|_| "[]".to_string())
}

/// JUnit XML format.
fn format_junit(results: &[LintResult]) -> String {
    let mut output = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    output.push('\n');

    let total_tests: usize = results.iter().map(|r| r.failures.len().max(1)).sum();
    let total_failures: usize = results.iter().map(|r| r.failures.len()).sum();

    output.push_str(&format!(
        r#"<testsuite name="dclint" tests="{}" failures="{}">"#,
        total_tests, total_failures
    ));
    output.push('\n');

    for result in results {
        if result.failures.is_empty() {
            output.push_str(&format!(
                r#"  <testcase name="{}" classname="dclint"/>"#,
                escape_xml(&result.file_path)
            ));
            output.push('\n');
        } else {
            for failure in &result.failures {
                output.push_str(&format!(
                    r#"  <testcase name="{}:{}" classname="dclint.{}">"#,
                    escape_xml(&result.file_path),
                    failure.line,
                    failure.code
                ));
                output.push('\n');
                output.push_str(&format!(
                    r#"    <failure message="{}" type="{}"/>"#,
                    escape_xml(&failure.message),
                    failure.severity
                ));
                output.push('\n');
                output.push_str("  </testcase>\n");
            }
        }
    }

    output.push_str("</testsuite>\n");
    output
}

/// Escape XML special characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

    fn make_result() -> LintResult {
        let mut result = LintResult::new("docker-compose.yml");
        result.failures.push(CheckFailure::new(
            "DCL001",
            "no-build-and-image",
            Severity::Error,
            RuleCategory::BestPractice,
            "Test message",
            5,
            1,
        ));
        result
    }

    #[test]
    fn test_compact_format() {
        let result = make_result();
        let output = format_compact(&[result]);
        assert!(output.contains("docker-compose.yml"));
        assert!(output.contains("DCL001"));
        assert!(output.contains("5:1"));
    }

    #[test]
    fn test_junit_format() {
        let result = make_result();
        let output = format_junit(&[result]);
        assert!(output.contains("<?xml"));
        assert!(output.contains("testsuite"));
        assert!(output.contains("DCL001"));
    }

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::parse("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::parse("JSON"), Some(OutputFormat::Json));
        assert_eq!(
            OutputFormat::parse("stylish"),
            Some(OutputFormat::Stylish)
        );
        assert_eq!(OutputFormat::parse("github"), Some(OutputFormat::GitHub));
        assert_eq!(OutputFormat::parse("invalid"), None);
    }
}
