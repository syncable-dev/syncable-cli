//! JSON formatter.

use crate::analyzer::kubelint::lint::LintResult;
use serde::Serialize;

/// Format a lint result as JSON.
pub fn format(result: &LintResult) -> String {
    let output = JsonOutput::from(result);
    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
}

#[derive(Serialize)]
struct JsonOutput {
    failures: Vec<JsonFailure>,
    summary: JsonSummary,
}

#[derive(Serialize)]
struct JsonFailure {
    check: String,
    severity: String,
    message: String,
    file_path: String,
    object_name: String,
    object_kind: String,
    object_namespace: Option<String>,
    line: Option<u32>,
    remediation: Option<String>,
}

#[derive(Serialize)]
struct JsonSummary {
    objects_analyzed: usize,
    checks_run: usize,
    total_failures: usize,
    passed: bool,
}

impl From<&LintResult> for JsonOutput {
    fn from(result: &LintResult) -> Self {
        Self {
            failures: result.failures.iter().map(JsonFailure::from).collect(),
            summary: JsonSummary {
                objects_analyzed: result.summary.objects_analyzed,
                checks_run: result.summary.checks_run,
                total_failures: result.failures.len(),
                passed: result.summary.passed,
            },
        }
    }
}

impl From<&crate::analyzer::kubelint::types::CheckFailure> for JsonFailure {
    fn from(f: &crate::analyzer::kubelint::types::CheckFailure) -> Self {
        Self {
            check: f.code.to_string(),
            severity: f.severity.to_string(),
            message: f.message.clone(),
            file_path: f.file_path.display().to_string(),
            object_name: f.object_name.clone(),
            object_kind: f.object_kind.clone(),
            object_namespace: f.object_namespace.clone(),
            line: f.line,
            remediation: f.remediation.clone(),
        }
    }
}
