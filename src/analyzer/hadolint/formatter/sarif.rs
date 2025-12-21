//! SARIF formatter for hadolint-rs.
//!
//! Outputs lint results in SARIF (Static Analysis Results Interchange Format)
//! for GitHub Actions Code Scanning integration.
//!
//! SARIF Specification: https://sarifweb.azurewebsites.net/

use crate::analyzer::hadolint::formatter::Formatter;
use crate::analyzer::hadolint::lint::LintResult;
use crate::analyzer::hadolint::types::Severity;
use serde::Serialize;
use std::io::Write;

/// SARIF output formatter for GitHub Actions.
#[derive(Debug, Clone, Default)]
pub struct SarifFormatter;

impl SarifFormatter {
    /// Create a new SARIF formatter.
    pub fn new() -> Self {
        Self
    }
}

/// SARIF 2.1.0 schema structures.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifReport {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<SarifRun>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifDriver {
    name: &'static str,
    information_uri: &'static str,
    version: &'static str,
    rules: Vec<SarifRule>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRule {
    id: String,
    name: String,
    short_description: SarifMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    help_uri: Option<String>,
    default_configuration: SarifRuleConfiguration,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRuleConfiguration {
    level: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifMessage {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult {
    rule_id: String,
    level: &'static str,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifLocation {
    physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifPhysicalLocation {
    artifact_location: SarifArtifactLocation,
    region: SarifRegion,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRegion {
    start_line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_column: Option<u32>,
}

fn severity_to_sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
        Severity::Style => "note",
        Severity::Ignore => "none",
    }
}

fn get_rule_help_uri(code: &str) -> Option<String> {
    if code.starts_with("DL") {
        Some(format!(
            "https://github.com/hadolint/hadolint/wiki/{}",
            code
        ))
    } else if code.starts_with("SC") {
        Some(format!("https://www.shellcheck.net/wiki/{}", code))
    } else {
        None
    }
}

impl Formatter for SarifFormatter {
    fn format<W: Write>(&self, result: &LintResult, filename: &str, writer: &mut W) -> std::io::Result<()> {
        // Collect unique rules for the rules array
        let mut rules: Vec<SarifRule> = Vec::new();
        let mut seen_rules = std::collections::HashSet::new();

        for failure in &result.failures {
            let code = failure.code.to_string();
            if !seen_rules.contains(&code) {
                seen_rules.insert(code.clone());
                rules.push(SarifRule {
                    id: code.clone(),
                    name: code.clone(),
                    short_description: SarifMessage {
                        text: failure.message.clone(),
                    },
                    help_uri: get_rule_help_uri(&code),
                    default_configuration: SarifRuleConfiguration {
                        level: severity_to_sarif_level(failure.severity),
                    },
                });
            }
        }

        // Build results
        let results: Vec<SarifResult> = result
            .failures
            .iter()
            .map(|f| SarifResult {
                rule_id: f.code.to_string(),
                level: severity_to_sarif_level(f.severity),
                message: SarifMessage {
                    text: f.message.clone(),
                },
                locations: vec![SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation {
                            uri: filename.to_string(),
                        },
                        region: SarifRegion {
                            start_line: f.line,
                            start_column: f.column,
                        },
                    },
                }],
            })
            .collect();

        let report = SarifReport {
            schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            version: "2.1.0",
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "hadolint-rs",
                        information_uri: "https://github.com/syncable-dev/syncable-cli",
                        version: env!("CARGO_PKG_VERSION"),
                        rules,
                    },
                },
                results,
            }],
        };

        let json = serde_json::to_string_pretty(&report)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        writeln!(writer, "{}", json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::types::CheckFailure;

    #[test]
    fn test_sarif_output() {
        let mut result = LintResult::new();
        result.failures.push(CheckFailure::new(
            "DL3008",
            Severity::Warning,
            "Pin versions in apt get install",
            5,
        ));

        let formatter = SarifFormatter::new();
        let output = formatter.format_to_string(&result, "Dockerfile");

        assert!(output.contains("\"$schema\""));
        assert!(output.contains("\"version\": \"2.1.0\""));
        assert!(output.contains("hadolint-rs"));
        assert!(output.contains("DL3008"));
        assert!(output.contains("warning"));
    }
}
