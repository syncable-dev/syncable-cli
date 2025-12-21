//! GNU formatter for hadolint-rs.
//!
//! Outputs lint results in GNU compiler-style format for editor integration.
//! Format: filename:line:column: severity: message [code]

use crate::analyzer::hadolint::formatter::Formatter;
use crate::analyzer::hadolint::lint::LintResult;
use crate::analyzer::hadolint::types::Severity;
use std::io::Write;

/// GNU compiler-style output formatter.
#[derive(Debug, Clone, Default)]
pub struct GnuFormatter;

impl GnuFormatter {
    /// Create a new GNU formatter.
    pub fn new() -> Self {
        Self
    }
}

impl Formatter for GnuFormatter {
    fn format<W: Write>(&self, result: &LintResult, filename: &str, writer: &mut W) -> std::io::Result<()> {
        for failure in &result.failures {
            let severity_str = match failure.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
                Severity::Style => "style",
                Severity::Ignore => "note",
            };

            // GNU format: file:line:column: severity: message [code]
            if let Some(col) = failure.column {
                writeln!(
                    writer,
                    "{}:{}:{}: {}: {} [{}]",
                    filename,
                    failure.line,
                    col,
                    severity_str,
                    failure.message,
                    failure.code
                )?;
            } else {
                writeln!(
                    writer,
                    "{}:{}: {}: {} [{}]",
                    filename,
                    failure.line,
                    severity_str,
                    failure.message,
                    failure.code
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::types::CheckFailure;

    #[test]
    fn test_gnu_output() {
        let mut result = LintResult::new();
        result.failures.push(CheckFailure::new(
            "DL3008",
            Severity::Warning,
            "Pin versions in apt get install",
            5,
        ));

        let formatter = GnuFormatter::new();
        let output = formatter.format_to_string(&result, "Dockerfile");

        assert_eq!(
            output.trim(),
            "Dockerfile:5: warning: Pin versions in apt get install [DL3008]"
        );
    }

    #[test]
    fn test_gnu_output_with_column() {
        let mut result = LintResult::new();
        result.failures.push(CheckFailure::with_column(
            "DL3008",
            Severity::Warning,
            "Pin versions",
            5,
            10,
        ));

        let formatter = GnuFormatter::new();
        let output = formatter.format_to_string(&result, "Dockerfile");

        assert!(output.contains("Dockerfile:5:10:"));
    }
}
