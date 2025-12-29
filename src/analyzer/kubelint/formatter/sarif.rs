//! SARIF (Static Analysis Results Interchange Format) formatter.
//!
//! SARIF is a standard format for static analysis tool output,
//! supported by GitHub, VS Code, and other tools.

use crate::analyzer::kubelint::lint::LintResult;
use serde::Serialize;

/// Format a lint result as SARIF.
pub fn format(result: &LintResult) -> String {
    let output = SarifOutput::from(result);
    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
}

#[derive(Serialize)]
struct SarifOutput {
    #[serde(rename = "$schema")]
    schema: String,
    version: String,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
struct SarifDriver {
    name: String,
    version: String,
    #[serde(rename = "informationUri")]
    information_uri: String,
    rules: Vec<SarifRule>,
}

#[derive(Serialize)]
struct SarifRule {
    id: String,
    name: String,
    #[serde(rename = "shortDescription")]
    short_description: SarifMessage,
    #[serde(rename = "defaultConfiguration")]
    default_configuration: SarifConfiguration,
}

#[derive(Serialize)]
struct SarifConfiguration {
    level: String,
}

#[derive(Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    level: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize)]
struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: SarifArtifactLocation,
    region: Option<SarifRegion>,
}

#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Serialize)]
struct SarifRegion {
    #[serde(rename = "startLine")]
    start_line: u32,
}

impl From<&LintResult> for SarifOutput {
    fn from(result: &LintResult) -> Self {
        // Collect unique rules
        let mut rules: Vec<SarifRule> = Vec::new();
        let mut seen_rules = std::collections::HashSet::new();

        for failure in &result.failures {
            let rule_id = failure.code.to_string();
            if !seen_rules.contains(&rule_id) {
                seen_rules.insert(rule_id.clone());
                rules.push(SarifRule {
                    id: rule_id.clone(),
                    name: rule_id.clone(),
                    short_description: SarifMessage {
                        text: failure.message.clone(),
                    },
                    default_configuration: SarifConfiguration {
                        level: severity_to_sarif_level(failure.severity),
                    },
                });
            }
        }

        let results: Vec<SarifResult> = result
            .failures
            .iter()
            .map(|f| SarifResult {
                rule_id: f.code.to_string(),
                level: severity_to_sarif_level(f.severity),
                message: SarifMessage {
                    text: format!(
                        "{} ({}/{}): {}",
                        f.code, f.object_kind, f.object_name, f.message
                    ),
                },
                locations: vec![SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation {
                            uri: f.file_path.display().to_string(),
                        },
                        region: f.line.map(|l| SarifRegion { start_line: l }),
                    },
                }],
            })
            .collect();

        Self {
            schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_string(),
            version: "2.1.0".to_string(),
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "kubelint-rs".to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        information_uri: "https://github.com/stackrox/kube-linter".to_string(),
                        rules,
                    },
                },
                results,
            }],
        }
    }
}

fn severity_to_sarif_level(severity: crate::analyzer::kubelint::types::Severity) -> String {
    match severity {
        crate::analyzer::kubelint::types::Severity::Error => "error".to_string(),
        crate::analyzer::kubelint::types::Severity::Warning => "warning".to_string(),
        crate::analyzer::kubelint::types::Severity::Info => "note".to_string(),
    }
}
