//! Checkstyle XML formatter for hadolint-rs.
//!
//! Outputs lint results in Checkstyle XML format for Jenkins and other CI tools.

use crate::analyzer::hadolint::formatter::Formatter;
use crate::analyzer::hadolint::lint::LintResult;
use crate::analyzer::hadolint::types::Severity;
use std::io::Write;

/// Checkstyle XML output formatter for Jenkins.
#[derive(Debug, Clone, Default)]
pub struct CheckstyleFormatter;

impl CheckstyleFormatter {
    /// Create a new Checkstyle formatter.
    pub fn new() -> Self {
        Self
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn severity_to_checkstyle(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
        Severity::Style => "info",
        Severity::Ignore => "info",
    }
}

impl Formatter for CheckstyleFormatter {
    fn format<W: Write>(&self, result: &LintResult, filename: &str, writer: &mut W) -> std::io::Result<()> {
        writeln!(writer, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
        writeln!(writer, r#"<checkstyle version="4.3">"#)?;

        if !result.failures.is_empty() {
            writeln!(writer, r#"  <file name="{}">"#, escape_xml(filename))?;

            for failure in &result.failures {
                let col_attr = failure
                    .column
                    .map(|c| format!(r#" column="{}""#, c))
                    .unwrap_or_default();

                writeln!(
                    writer,
                    r#"    <error line="{}"{}  severity="{}" message="{}" source="{}"/>"#,
                    failure.line,
                    col_attr,
                    severity_to_checkstyle(failure.severity),
                    escape_xml(&failure.message),
                    escape_xml(&failure.code.to_string())
                )?;
            }

            writeln!(writer, "  </file>")?;
        }

        writeln!(writer, "</checkstyle>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::types::CheckFailure;

    #[test]
    fn test_checkstyle_output() {
        let mut result = LintResult::new();
        result.failures.push(CheckFailure::new(
            "DL3008",
            Severity::Warning,
            "Pin versions in apt get install",
            5,
        ));

        let formatter = CheckstyleFormatter::new();
        let output = formatter.format_to_string(&result, "Dockerfile");

        assert!(output.contains("<?xml version"));
        assert!(output.contains("<checkstyle"));
        assert!(output.contains(r#"<file name="Dockerfile">"#));
        assert!(output.contains(r#"line="5""#));
        assert!(output.contains(r#"severity="warning""#));
        assert!(output.contains("DL3008"));
    }

    #[test]
    fn test_xml_escaping() {
        assert_eq!(escape_xml("a < b"), "a &lt; b");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml(r#"a "b""#), "a &quot;b&quot;");
    }
}
