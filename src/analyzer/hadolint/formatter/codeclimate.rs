//! CodeClimate formatter for hadolint-rs.
//!
//! Outputs lint results in CodeClimate JSON format for GitLab CI integration.
//!
//! CodeClimate Specification: https://github.com/codeclimate/platform/blob/master/spec/analyzers/SPEC.md

use crate::analyzer::hadolint::formatter::Formatter;
use crate::analyzer::hadolint::lint::LintResult;
use crate::analyzer::hadolint::types::Severity;
use serde::Serialize;
use std::io::Write;

/// CodeClimate JSON output formatter for GitLab CI.
#[derive(Debug, Clone, Default)]
pub struct CodeClimateFormatter;

impl CodeClimateFormatter {
    /// Create a new CodeClimate formatter.
    pub fn new() -> Self {
        Self
    }
}

/// CodeClimate issue structure.
#[derive(Debug, Serialize)]
struct CodeClimateIssue {
    #[serde(rename = "type")]
    issue_type: &'static str,
    check_name: String,
    description: String,
    content: CodeClimateContent,
    categories: Vec<&'static str>,
    location: CodeClimateLocation,
    severity: &'static str,
    fingerprint: String,
}

#[derive(Debug, Serialize)]
struct CodeClimateContent {
    body: String,
}

#[derive(Debug, Serialize)]
struct CodeClimateLocation {
    path: String,
    lines: CodeClimateLines,
}

#[derive(Debug, Serialize)]
struct CodeClimateLines {
    begin: u32,
    end: u32,
}

fn severity_to_codeclimate(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "critical",
        Severity::Warning => "major",
        Severity::Info => "minor",
        Severity::Style => "info",
        Severity::Ignore => "info",
    }
}

fn get_categories(code: &str) -> Vec<&'static str> {
    // Categorize based on rule code prefix
    if code.starts_with("DL") {
        // Dockerfile linting rules
        let rule_num: u32 = code[2..].parse().unwrap_or(0);
        match rule_num {
            // Security-related rules
            3000..=3010 => vec!["Security", "Bug Risk"],
            // Best practices
            3011..=3030 => vec!["Style", "Clarity"],
            // Performance
            3031..=3050 => vec!["Performance"],
            // Deprecated instructions
            4000..=4999 => vec!["Compatibility", "Bug Risk"],
            _ => vec!["Style"],
        }
    } else if code.starts_with("SC") {
        // ShellCheck rules
        vec!["Bug Risk", "Security"]
    } else {
        vec!["Style"]
    }
}

fn generate_fingerprint(filename: &str, code: &str, line: u32) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    filename.hash(&mut hasher);
    code.hash(&mut hasher);
    line.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn get_help_body(code: &str) -> String {
    if code.starts_with("DL") {
        format!(
            "See the hadolint wiki for more information: https://github.com/hadolint/hadolint/wiki/{}",
            code
        )
    } else if code.starts_with("SC") {
        format!(
            "See the ShellCheck wiki for more information: https://www.shellcheck.net/wiki/{}",
            code
        )
    } else {
        "See hadolint documentation for more information.".to_string()
    }
}

impl Formatter for CodeClimateFormatter {
    fn format<W: Write>(&self, result: &LintResult, filename: &str, writer: &mut W) -> std::io::Result<()> {
        let issues: Vec<CodeClimateIssue> = result
            .failures
            .iter()
            .map(|f| {
                let code = f.code.to_string();
                CodeClimateIssue {
                    issue_type: "issue",
                    check_name: code.clone(),
                    description: f.message.clone(),
                    content: CodeClimateContent {
                        body: get_help_body(&code),
                    },
                    categories: get_categories(&code),
                    location: CodeClimateLocation {
                        path: filename.to_string(),
                        lines: CodeClimateLines {
                            begin: f.line,
                            end: f.line,
                        },
                    },
                    severity: severity_to_codeclimate(f.severity),
                    fingerprint: generate_fingerprint(filename, &code, f.line),
                }
            })
            .collect();

        // CodeClimate expects newline-delimited JSON (NDJSON)
        for issue in &issues {
            let json = serde_json::to_string(issue)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            writeln!(writer, "{}", json)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::types::CheckFailure;

    #[test]
    fn test_codeclimate_output() {
        let mut result = LintResult::new();
        result.failures.push(CheckFailure::new(
            "DL3008",
            Severity::Warning,
            "Pin versions in apt get install",
            5,
        ));

        let formatter = CodeClimateFormatter::new();
        let output = formatter.format_to_string(&result, "Dockerfile");

        assert!(output.contains("\"type\":\"issue\""));
        assert!(output.contains("\"check_name\":\"DL3008\""));
        assert!(output.contains("\"severity\":\"major\""));
        assert!(output.contains("\"path\":\"Dockerfile\""));
        assert!(output.contains("\"fingerprint\""));
    }

    #[test]
    fn test_fingerprint_consistency() {
        let fp1 = generate_fingerprint("Dockerfile", "DL3008", 5);
        let fp2 = generate_fingerprint("Dockerfile", "DL3008", 5);
        let fp3 = generate_fingerprint("Dockerfile", "DL3008", 6);

        assert_eq!(fp1, fp2);
        assert_ne!(fp1, fp3);
    }

    #[test]
    fn test_categories() {
        assert!(get_categories("DL3000").contains(&"Security"));
        assert!(get_categories("SC2086").contains(&"Bug Risk"));
        assert!(get_categories("DL4000").contains(&"Compatibility"));
    }
}
