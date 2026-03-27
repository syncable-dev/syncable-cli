//! TTY formatter for hadolint-rs.
//!
//! Outputs lint results with colored terminal output for human readability.
//! Uses ANSI escape codes for colors.

use crate::analyzer::hadolint::formatter::Formatter;
use crate::analyzer::hadolint::lint::LintResult;
use crate::analyzer::hadolint::types::Severity;
use std::io::Write;

/// TTY (terminal) output formatter with colors.
#[derive(Debug, Clone)]
pub struct TtyFormatter {
    /// Use colors in output.
    pub colors: bool,
    /// Show the filename in each line.
    pub show_filename: bool,
}

impl Default for TtyFormatter {
    fn default() -> Self {
        Self {
            colors: true,
            show_filename: true,
        }
    }
}

impl TtyFormatter {
    /// Create a new TTY formatter with colors enabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a TTY formatter without colors.
    pub fn no_color() -> Self {
        Self {
            colors: false,
            show_filename: true,
        }
    }

    fn severity_color(&self, severity: Severity) -> &'static str {
        if !self.colors {
            return "";
        }
        match severity {
            Severity::Error => "\x1b[1;31m",   // Bold red
            Severity::Warning => "\x1b[1;33m", // Bold yellow
            Severity::Info => "\x1b[1;36m",    // Bold cyan
            Severity::Style => "\x1b[1;35m",   // Bold magenta
            Severity::Ignore => "\x1b[2m",     // Dim
        }
    }

    fn reset(&self) -> &'static str {
        if self.colors { "\x1b[0m" } else { "" }
    }

    fn dim(&self) -> &'static str {
        if self.colors { "\x1b[2m" } else { "" }
    }

    fn bold(&self) -> &'static str {
        if self.colors { "\x1b[1m" } else { "" }
    }
}

impl Formatter for TtyFormatter {
    fn format<W: Write>(
        &self,
        result: &LintResult,
        filename: &str,
        writer: &mut W,
    ) -> std::io::Result<()> {
        if result.failures.is_empty() {
            return Ok(());
        }

        for failure in &result.failures {
            let color = self.severity_color(failure.severity);
            let reset = self.reset();
            let dim = self.dim();
            let bold = self.bold();

            // Format: filename:line severity: [code] message
            if self.show_filename {
                write!(writer, "{}{}{}{}:{}", bold, filename, reset, dim, reset)?;
            }

            write!(writer, "{}{}{} ", dim, failure.line, reset)?;

            // Severity badge
            let severity_str = match failure.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
                Severity::Style => "style",
                Severity::Ignore => "ignore",
            };

            write!(writer, "{}{}{}", color, severity_str, reset)?;

            // Rule code
            write!(writer, " {}{}{}: ", dim, failure.code, reset)?;

            // Message
            writeln!(writer, "{}", failure.message)?;
        }

        // Summary line
        let error_count = result
            .failures
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count();
        let warning_count = result
            .failures
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count();
        let info_count = result
            .failures
            .iter()
            .filter(|f| f.severity == Severity::Info)
            .count();
        let style_count = result
            .failures
            .iter()
            .filter(|f| f.severity == Severity::Style)
            .count();

        writeln!(writer)?;

        let mut parts = Vec::new();
        if error_count > 0 {
            parts.push(format!(
                "{}{} error{}{}",
                self.severity_color(Severity::Error),
                error_count,
                if error_count == 1 { "" } else { "s" },
                self.reset()
            ));
        }
        if warning_count > 0 {
            parts.push(format!(
                "{}{} warning{}{}",
                self.severity_color(Severity::Warning),
                warning_count,
                if warning_count == 1 { "" } else { "s" },
                self.reset()
            ));
        }
        if info_count > 0 {
            parts.push(format!(
                "{}{} info{}",
                self.severity_color(Severity::Info),
                info_count,
                self.reset()
            ));
        }
        if style_count > 0 {
            parts.push(format!(
                "{}{} style{}",
                self.severity_color(Severity::Style),
                style_count,
                self.reset()
            ));
        }

        if !parts.is_empty() {
            writeln!(writer, "{}", parts.join(", "))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::types::CheckFailure;

    #[test]
    fn test_tty_output_no_color() {
        let mut result = LintResult::new();
        result.failures.push(CheckFailure::new(
            "DL3008",
            Severity::Warning,
            "Pin versions in apt get install",
            5,
        ));

        let formatter = TtyFormatter::no_color();
        let output = formatter.format_to_string(&result, "Dockerfile");

        assert!(output.contains("Dockerfile"));
        assert!(output.contains("5"));
        assert!(output.contains("warning"));
        assert!(output.contains("DL3008"));
        assert!(output.contains("Pin versions"));
    }
}
