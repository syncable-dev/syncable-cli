//! Output formatters for hadolint-rs lint results.
//!
//! Provides multiple output formats for compatibility with various CI/CD systems:
//! - **TTY**: Colored terminal output for human readability
//! - **JSON**: Machine-readable format for CI/CD pipelines
//! - **SARIF**: Static Analysis Results Interchange Format for GitHub Actions
//! - **Checkstyle**: XML format for Jenkins and other tools
//! - **CodeClimate**: JSON format for GitLab CI
//! - **GNU**: Standard compiler-style output for editors

mod checkstyle;
mod codeclimate;
mod gnu;
mod json;
mod sarif;
mod tty;

pub use checkstyle::CheckstyleFormatter;
pub use codeclimate::CodeClimateFormatter;
pub use gnu::GnuFormatter;
pub use json::JsonFormatter;
pub use sarif::SarifFormatter;
pub use tty::TtyFormatter;

use crate::analyzer::hadolint::lint::LintResult;
use std::io::Write;

/// Output format for lint results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Colored terminal output (default)
    #[default]
    Tty,
    /// JSON format for CI/CD
    Json,
    /// SARIF format for GitHub Actions
    Sarif,
    /// Checkstyle XML for Jenkins
    Checkstyle,
    /// CodeClimate JSON for GitLab
    CodeClimate,
    /// GNU compiler-style output
    Gnu,
}

impl OutputFormat {
    /// Parse format from string (case-insensitive).
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tty" | "terminal" | "color" => Some(Self::Tty),
            "json" => Some(Self::Json),
            "sarif" => Some(Self::Sarif),
            "checkstyle" => Some(Self::Checkstyle),
            "codeclimate" | "gitlab" => Some(Self::CodeClimate),
            "gnu" => Some(Self::Gnu),
            _ => None,
        }
    }

    /// Get all available format names.
    pub fn all_names() -> &'static [&'static str] {
        &["tty", "json", "sarif", "checkstyle", "codeclimate", "gnu"]
    }
}

/// Trait for formatting lint results.
pub trait Formatter {
    /// Format the lint result and write to the given writer.
    fn format<W: Write>(
        &self,
        result: &LintResult,
        filename: &str,
        writer: &mut W,
    ) -> std::io::Result<()>;

    /// Format the lint result to a string.
    fn format_to_string(&self, result: &LintResult, filename: &str) -> String {
        let mut buf = Vec::new();
        self.format(result, filename, &mut buf).unwrap_or_default();
        String::from_utf8(buf).unwrap_or_default()
    }
}

/// Format a lint result using the specified output format.
pub fn format_result<W: Write>(
    result: &LintResult,
    filename: &str,
    format: OutputFormat,
    writer: &mut W,
) -> std::io::Result<()> {
    match format {
        OutputFormat::Tty => TtyFormatter::new().format(result, filename, writer),
        OutputFormat::Json => JsonFormatter::new().format(result, filename, writer),
        OutputFormat::Sarif => SarifFormatter::new().format(result, filename, writer),
        OutputFormat::Checkstyle => CheckstyleFormatter::new().format(result, filename, writer),
        OutputFormat::CodeClimate => CodeClimateFormatter::new().format(result, filename, writer),
        OutputFormat::Gnu => GnuFormatter::new().format(result, filename, writer),
    }
}

/// Format a lint result to a string using the specified output format.
pub fn format_result_to_string(
    result: &LintResult,
    filename: &str,
    format: OutputFormat,
) -> String {
    let mut buf = Vec::new();
    format_result(result, filename, format, &mut buf).unwrap_or_default();
    String::from_utf8(buf).unwrap_or_default()
}
