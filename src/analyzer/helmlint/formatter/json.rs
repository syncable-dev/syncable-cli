//! JSON formatter for helmlint results.
//!
//! Produces machine-readable JSON output.

use crate::analyzer::helmlint::lint::LintResult;
use serde::Serialize;

/// JSON output structure for a lint failure.
#[derive(Serialize)]
struct JsonFailure {
    code: String,
    severity: String,
    message: String,
    file: String,
    line: u32,
    column: Option<u32>,
    category: String,
    fixable: bool,
}

/// JSON output structure for lint results.
#[derive(Serialize)]
struct JsonOutput {
    chart_path: String,
    files_checked: usize,
    error_count: usize,
    warning_count: usize,
    failures: Vec<JsonFailure>,
    parse_errors: Vec<String>,
}

/// Format a lint result as JSON.
pub fn format(result: &LintResult) -> String {
    let output = JsonOutput {
        chart_path: result.chart_path.clone(),
        files_checked: result.files_checked,
        error_count: result.error_count,
        warning_count: result.warning_count,
        failures: result
            .failures
            .iter()
            .map(|f| JsonFailure {
                code: f.code.to_string(),
                severity: format!("{:?}", f.severity).to_lowercase(),
                message: f.message.clone(),
                file: f.file.display().to_string(),
                line: f.line,
                column: f.column,
                category: format!("{:?}", f.category),
                fixable: f.fixable,
            })
            .collect(),
        parse_errors: result.parse_errors.clone(),
    };

    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::helmlint::types::{CheckFailure, RuleCategory, Severity};

    #[test]
    fn test_json_format_empty() {
        let result = LintResult::new("test-chart");
        let json = format(&result);
        assert!(json.contains("\"chart_path\": \"test-chart\""));
        assert!(json.contains("\"failures\": []"));
    }

    #[test]
    fn test_json_format_with_failures() {
        let mut result = LintResult::new("test-chart");
        result.failures.push(CheckFailure::new(
            "HL1001",
            Severity::Error,
            "Missing Chart.yaml",
            ".",
            1,
            RuleCategory::Structure,
        ));
        result.error_count = 1;

        let json = format(&result);
        assert!(json.contains("\"code\": \"HL1001\""));
        assert!(json.contains("\"severity\": \"error\""));
    }
}
