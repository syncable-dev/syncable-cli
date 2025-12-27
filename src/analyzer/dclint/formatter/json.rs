//! JSON output formatter for dclint.

use serde_json::json;

use crate::analyzer::dclint::lint::LintResult;

/// Format lint results as JSON.
pub fn format(results: &[LintResult]) -> String {
    let output: Vec<serde_json::Value> = results
        .iter()
        .map(|result| {
            let messages: Vec<serde_json::Value> = result
                .failures
                .iter()
                .map(|f| {
                    json!({
                        "ruleId": f.code.as_str(),
                        "ruleName": f.rule_name,
                        "severity": match f.severity {
                            crate::analyzer::dclint::types::Severity::Error => 2,
                            crate::analyzer::dclint::types::Severity::Warning => 1,
                            crate::analyzer::dclint::types::Severity::Info => 0,
                            crate::analyzer::dclint::types::Severity::Style => 0,
                        },
                        "severityName": f.severity.as_str(),
                        "category": f.category.as_str(),
                        "message": f.message,
                        "line": f.line,
                        "column": f.column,
                        "endLine": f.end_line,
                        "endColumn": f.end_column,
                        "fixable": f.fixable,
                        "data": f.data
                    })
                })
                .collect();

            json!({
                "filePath": result.file_path,
                "messages": messages,
                "errorCount": result.error_count,
                "warningCount": result.warning_count,
                "fixableErrorCount": result.fixable_error_count,
                "fixableWarningCount": result.fixable_warning_count,
                "parseErrors": result.parse_errors
            })
        })
        .collect();

    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

    #[test]
    fn test_json_format() {
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
        result.error_count = 1;

        let output = format(&[result]);
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert!(parsed.is_array());
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 1);

        let file_result = &arr[0];
        assert_eq!(file_result["filePath"], "docker-compose.yml");
        assert_eq!(file_result["errorCount"], 1);

        let messages = file_result["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["ruleId"], "DCL001");
        assert_eq!(messages[0]["line"], 5);
    }

    #[test]
    fn test_json_format_empty() {
        let result = LintResult::new("docker-compose.yml");
        let output = format(&[result]);
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let arr = parsed.as_array().unwrap();
        let messages = arr[0]["messages"].as_array().unwrap();
        assert!(messages.is_empty());
    }
}
