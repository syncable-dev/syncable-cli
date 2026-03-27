//! Plain text formatter.

use crate::analyzer::kubelint::lint::LintResult;

/// Format a lint result as plain text.
pub fn format(result: &LintResult) -> String {
    let mut output = String::new();

    for failure in &result.failures {
        let location = match failure.line {
            Some(line) => format!("{}:{}", failure.file_path.display(), line),
            None => failure.file_path.display().to_string(),
        };

        output.push_str(&format!(
            "{}: [{}] {} ({}/{}) - {}\n",
            location,
            failure.severity,
            failure.code,
            failure.object_kind,
            failure.object_name,
            failure.message,
        ));

        if let Some(ref remediation) = failure.remediation {
            output.push_str(&format!("  Remediation: {}\n", remediation));
        }
    }

    if result.failures.is_empty() {
        output.push_str("No lint errors found.\n");
    } else {
        output.push_str(&format!("\nFound {} issue(s).\n", result.failures.len()));
    }

    output
}

/// Format for GitHub Actions annotations.
pub fn format_github(result: &LintResult) -> String {
    let mut output = String::new();

    for failure in &result.failures {
        let level = match failure.severity {
            crate::analyzer::kubelint::types::Severity::Error => "error",
            crate::analyzer::kubelint::types::Severity::Warning => "warning",
            crate::analyzer::kubelint::types::Severity::Info => "notice",
        };

        let file = failure.file_path.display();
        let line = failure.line.unwrap_or(1);

        output.push_str(&format!(
            "::{}file={},line={}::[{}] {} - {}\n",
            level, file, line, failure.code, failure.object_name, failure.message,
        ));
    }

    output
}
