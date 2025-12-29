//! Output formatters for lint results.

pub mod json;
pub mod plain;
pub mod sarif;

use crate::analyzer::kubelint::lint::LintResult;

/// Output format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Plain text output.
    #[default]
    Plain,
    /// JSON output.
    Json,
    /// SARIF format for IDE integration.
    Sarif,
    /// GitHub Actions annotations.
    GitHub,
}

impl OutputFormat {
    /// Parse from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "plain" | "text" => Some(Self::Plain),
            "json" => Some(Self::Json),
            "sarif" => Some(Self::Sarif),
            "github" | "github-actions" => Some(Self::GitHub),
            _ => None,
        }
    }
}

/// Format a lint result to a string.
pub fn format_result_to_string(result: &LintResult, format: OutputFormat) -> String {
    match format {
        OutputFormat::Plain => plain::format(result),
        OutputFormat::Json => json::format(result),
        OutputFormat::Sarif => sarif::format(result),
        OutputFormat::GitHub => plain::format_github(result),
    }
}

/// Format and print a lint result.
pub fn format_result(result: &LintResult, format: OutputFormat) {
    print!("{}", format_result_to_string(result, format));
}
