//! Output formatters for helmlint results.
//!
//! Provides multiple output formats:
//! - JSON: Machine-readable format
//! - Stylish: Human-readable with colors
//! - GitHub: GitHub Actions annotation format

pub mod github;
pub mod json;
pub mod stylish;

use crate::analyzer::helmlint::lint::LintResult;

/// Output format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// JSON format for machine parsing
    Json,
    /// Human-readable format with colors
    #[default]
    Stylish,
    /// GitHub Actions annotation format
    Github,
    /// Compact single-line format
    Compact,
}

impl OutputFormat {
    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "stylish" | "default" => Some(Self::Stylish),
            "github" | "github-actions" => Some(Self::Github),
            "compact" => Some(Self::Compact),
            _ => None,
        }
    }
}

/// Format a lint result to stdout.
pub fn format_result(result: &LintResult, format: OutputFormat) {
    let output = format_result_to_string(result, format);
    println!("{}", output);
}

/// Format a lint result to a string.
pub fn format_result_to_string(result: &LintResult, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => json::format(result),
        OutputFormat::Stylish => stylish::format(result),
        OutputFormat::Github => github::format(result),
        OutputFormat::Compact => compact_format(result),
    }
}

/// Format multiple results.
pub fn format_results(results: &[LintResult], format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => {
            // Combine into a single JSON array
            let jsons: Vec<String> = results.iter().map(json::format).collect();
            format!("[{}]", jsons.join(","))
        }
        _ => results
            .iter()
            .map(|r| format_result_to_string(r, format))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Compact format: one line per failure.
fn compact_format(result: &LintResult) -> String {
    let mut lines = Vec::new();

    for failure in &result.failures {
        lines.push(format!(
            "{}:{}:{}: {} {}",
            failure.file.display(),
            failure.line,
            failure.column.unwrap_or(1),
            failure.code,
            failure.message
        ));
    }

    if lines.is_empty() {
        format!("{}: No issues found", result.chart_path)
    } else {
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parse() {
        assert_eq!(OutputFormat::parse("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::parse("stylish"), Some(OutputFormat::Stylish));
        assert_eq!(OutputFormat::parse("github"), Some(OutputFormat::Github));
        assert_eq!(OutputFormat::parse("compact"), Some(OutputFormat::Compact));
        assert_eq!(OutputFormat::parse("invalid"), None);
    }
}
