//! Stylish formatter for helmlint results.
//!
//! Produces human-readable colored output similar to ESLint's stylish formatter.

use crate::analyzer::helmlint::lint::LintResult;
use crate::analyzer::helmlint::types::Severity;
use std::collections::BTreeMap;

/// ANSI color codes.
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const RED: &str = "\x1b[31m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const CYAN: &str = "\x1b[36m";
    pub const DIM: &str = "\x1b[2m";
    pub const BOLD: &str = "\x1b[1m";
    pub const UNDERLINE: &str = "\x1b[4m";
}

/// Format a lint result in stylish format.
pub fn format(result: &LintResult) -> String {
    let mut output = String::new();

    // Group failures by file
    let mut by_file: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for failure in &result.failures {
        let file = failure.file.display().to_string();
        by_file.entry(file).or_default().push(failure);
    }

    // Handle parse errors
    if !result.parse_errors.is_empty() {
        output.push_str(&format!(
            "\n{}{}Parse Errors:{}\n",
            colors::BOLD,
            colors::RED,
            colors::RESET
        ));
        for error in &result.parse_errors {
            output.push_str(&format!(
                "  {}{}{}  {}\n",
                colors::RED,
                "error",
                colors::RESET,
                error
            ));
        }
        output.push('\n');
    }

    if by_file.is_empty() && result.parse_errors.is_empty() {
        output.push_str(&format!(
            "{}{}{}  No issues found\n",
            colors::BOLD,
            result.chart_path,
            colors::RESET
        ));
        return output;
    }

    // Output failures grouped by file
    for (file, failures) in by_file {
        output.push_str(&format!(
            "\n{}{}{}{}",
            colors::UNDERLINE,
            colors::BOLD,
            file,
            colors::RESET
        ));
        output.push('\n');

        for failure in failures {
            let severity_color = match failure.severity {
                Severity::Error => colors::RED,
                Severity::Warning => colors::YELLOW,
                Severity::Info => colors::BLUE,
                Severity::Style => colors::CYAN,
                Severity::Ignore => colors::DIM,
            };

            let severity_text = match failure.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
                Severity::Style => "style",
                Severity::Ignore => "ignore",
            };

            let location = match failure.column {
                Some(col) => format!("{}:{}", failure.line, col),
                None => format!("{}", failure.line),
            };

            output.push_str(&format!(
                "  {}{}:{:>8}{}  {}  {}{}{}",
                colors::DIM,
                location,
                severity_color,
                severity_text,
                colors::RESET,
                failure.message,
                colors::DIM,
                format!("  {}", failure.code),
            ));
            output.push_str(colors::RESET);
            output.push('\n');
        }
    }

    // Summary
    output.push('\n');
    let total = result.failures.len();
    let errors = result.error_count;
    let warnings = result.warning_count;
    let infos = total - errors - warnings;

    if total > 0 {
        output.push_str(&format!(
            "{}{}{}",
            colors::BOLD,
            if errors > 0 { colors::RED } else { colors::YELLOW },
            format!(
                "✖ {} {} ({} {}, {} {}, {} info)\n",
                total,
                if total == 1 { "problem" } else { "problems" },
                errors,
                if errors == 1 { "error" } else { "errors" },
                warnings,
                if warnings == 1 { "warning" } else { "warnings" },
                infos
            )
        ));
        output.push_str(colors::RESET);
    }

    output
}

/// Format without colors (for non-TTY output).
pub fn format_no_color(result: &LintResult) -> String {
    let mut output = String::new();

    // Group failures by file
    let mut by_file: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for failure in &result.failures {
        let file = failure.file.display().to_string();
        by_file.entry(file).or_default().push(failure);
    }

    if !result.parse_errors.is_empty() {
        output.push_str("\nParse Errors:\n");
        for error in &result.parse_errors {
            output.push_str(&format!("  error  {}\n", error));
        }
        output.push('\n');
    }

    if by_file.is_empty() && result.parse_errors.is_empty() {
        output.push_str(&format!("{}  No issues found\n", result.chart_path));
        return output;
    }

    for (file, failures) in by_file {
        output.push_str(&format!("\n{}\n", file));

        for failure in failures {
            let severity_text = match failure.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
                Severity::Style => "style",
                Severity::Ignore => "ignore",
            };

            let location = match failure.column {
                Some(col) => format!("{}:{}", failure.line, col),
                None => format!("{}", failure.line),
            };

            output.push_str(&format!(
                "  {}:  {}  {}  {}\n",
                location, severity_text, failure.message, failure.code
            ));
        }
    }

    // Summary
    output.push('\n');
    let total = result.failures.len();
    let errors = result.error_count;
    let warnings = result.warning_count;
    let infos = total - errors - warnings;

    if total > 0 {
        output.push_str(&format!(
            "✖ {} {} ({} {}, {} {}, {} info)\n",
            total,
            if total == 1 { "problem" } else { "problems" },
            errors,
            if errors == 1 { "error" } else { "errors" },
            warnings,
            if warnings == 1 { "warning" } else { "warnings" },
            infos
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::helmlint::types::{CheckFailure, RuleCategory, Severity};

    #[test]
    fn test_stylish_format_empty() {
        let result = LintResult::new("test-chart");
        let output = format(&result);
        assert!(output.contains("No issues found"));
    }

    #[test]
    fn test_stylish_format_with_failures() {
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

        let output = format_no_color(&result);
        assert!(output.contains("Chart.yaml"));
        assert!(output.contains("error"));
        assert!(output.contains("HL1001"));
    }
}
