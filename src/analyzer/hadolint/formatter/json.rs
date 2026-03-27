//! JSON formatter for hadolint-rs.
//!
//! Outputs lint results in JSON format for CI/CD pipeline integration.
//! Compatible with the original hadolint JSON output.

use crate::analyzer::hadolint::formatter::Formatter;
use crate::analyzer::hadolint::lint::LintResult;
use crate::analyzer::hadolint::types::Severity;
use serde::Serialize;
use std::io::Write;

/// JSON output formatter.
#[derive(Debug, Clone, Default)]
pub struct JsonFormatter {
    /// Pretty-print the JSON output.
    pub pretty: bool,
}

impl JsonFormatter {
    /// Create a new JSON formatter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a JSON formatter with pretty-printing enabled.
    pub fn pretty() -> Self {
        Self { pretty: true }
    }
}

/// JSON representation of a lint failure.
#[derive(Debug, Serialize)]
struct JsonFailure {
    line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<u32>,
    code: String,
    message: String,
    level: String,
    file: String,
}

impl Formatter for JsonFormatter {
    fn format<W: Write>(
        &self,
        result: &LintResult,
        filename: &str,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let failures: Vec<JsonFailure> = result
            .failures
            .iter()
            .map(|f| JsonFailure {
                line: f.line,
                column: f.column,
                code: f.code.to_string(),
                message: f.message.clone(),
                level: match f.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "info",
                    Severity::Style => "style",
                    Severity::Ignore => "ignore",
                }
                .to_string(),
                file: filename.to_string(),
            })
            .collect();

        let json = if self.pretty {
            serde_json::to_string_pretty(&failures)
        } else {
            serde_json::to_string(&failures)
        }
        .map_err(std::io::Error::other)?;

        writeln!(writer, "{}", json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::types::CheckFailure;

    #[test]
    fn test_json_output() {
        let mut result = LintResult::new();
        result.failures.push(CheckFailure::new(
            "DL3008",
            Severity::Warning,
            "Pin versions in apt get install",
            5,
        ));

        let formatter = JsonFormatter::new();
        let output = formatter.format_to_string(&result, "Dockerfile");

        assert!(output.contains("DL3008"));
        assert!(output.contains("warning"));
        assert!(output.contains("Pin versions"));
    }
}
