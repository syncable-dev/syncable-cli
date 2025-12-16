//! Security and vulnerability scanning tools using Rig's Tool trait

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use crate::analyzer::security::turbo::{TurboConfig, TurboSecurityAnalyzer, ScanMode};

// ============================================================================
// Security Scan Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SecurityScanArgs {
    pub mode: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("Security scan error: {0}")]
pub struct SecurityScanError(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanTool {
    project_path: PathBuf,
}

impl SecurityScanTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }
}

impl Tool for SecurityScanTool {
    const NAME: &'static str = "security_scan";

    type Error = SecurityScanError;
    type Args = SecurityScanArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Perform a security scan to detect potential secrets, API keys, passwords, and sensitive data that might be accidentally committed.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["lightning", "fast", "balanced", "thorough", "paranoid"],
                        "description": "Scan mode: lightning (fast), balanced (recommended), thorough, or paranoid"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional subdirectory path to scan"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = match args.path {
            Some(subpath) => self.project_path.join(subpath),
            None => self.project_path.clone(),
        };

        let scan_mode = match args.mode.as_deref() {
            Some("lightning") => ScanMode::Lightning,
            Some("fast") => ScanMode::Fast,
            Some("thorough") => ScanMode::Thorough,
            Some("paranoid") => ScanMode::Paranoid,
            _ => ScanMode::Balanced,
        };

        let config = TurboConfig {
            scan_mode,
            ..TurboConfig::default()
        };
        
        let analyzer = TurboSecurityAnalyzer::new(config)
            .map_err(|e| SecurityScanError(format!("Failed to create analyzer: {}", e)))?;
        
        let report = analyzer.analyze_project(&path)
            .map_err(|e| SecurityScanError(format!("Scan failed: {}", e)))?;
        
        let findings = report.findings;

        let result = json!({
            "total_findings": findings.len(),
            "findings": findings.iter().take(50).map(|f| {
                json!({
                    "file": f.file_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
                    "line": f.line_number,
                    "title": f.title,
                    "severity": format!("{:?}", f.severity),
                    "evidence": f.evidence.as_ref().map(|e| e.chars().take(50).collect::<String>()).unwrap_or_default(),
                })
            }).collect::<Vec<_>>(),
            "scan_mode": args.mode.as_deref().unwrap_or("balanced"),
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| SecurityScanError(format!("Failed to serialize: {}", e)))
    }
}

// ============================================================================
// Vulnerabilities Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct VulnerabilitiesArgs {
    pub path: Option<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("Vulnerability check error: {0}")]
pub struct VulnerabilitiesError(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilitiesTool {
    project_path: PathBuf,
}

impl VulnerabilitiesTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }
}

impl Tool for VulnerabilitiesTool {
    const NAME: &'static str = "check_vulnerabilities";

    type Error = VulnerabilitiesError;
    type Args = VulnerabilitiesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Check the project's dependencies for known security vulnerabilities (CVEs).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Optional subdirectory path to check"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = match args.path {
            Some(subpath) => self.project_path.join(subpath),
            None => self.project_path.clone(),
        };

        let parser = crate::analyzer::dependency_parser::DependencyParser::new();
        let dependencies = parser
            .parse_all_dependencies(&path)
            .map_err(|e| VulnerabilitiesError(format!("Failed to parse dependencies: {}", e)))?;

        if dependencies.is_empty() {
            return Ok(json!({
                "message": "No dependencies found in project",
                "total_vulnerabilities": 0
            }).to_string());
        }

        let checker = crate::analyzer::vulnerability::VulnerabilityChecker::new();
        let report = checker
            .check_all_dependencies(&dependencies, &path)
            .await
            .map_err(|e| VulnerabilitiesError(format!("Vulnerability check failed: {}", e)))?;

        let result = json!({
            "total_vulnerabilities": report.total_vulnerabilities,
            "critical_count": report.critical_count,
            "high_count": report.high_count,
            "medium_count": report.medium_count,
            "low_count": report.low_count,
            "vulnerable_dependencies": report.vulnerable_dependencies.iter().take(20).map(|dep| {
                json!({
                    "name": dep.name,
                    "version": dep.version,
                    "language": dep.language.as_str(),
                    "vulnerabilities": dep.vulnerabilities.iter().map(|v| {
                        json!({
                            "id": v.id,
                            "title": v.title,
                            "severity": format!("{:?}", v.severity),
                            "cve": v.cve,
                            "patched_versions": v.patched_versions,
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| VulnerabilitiesError(format!("Failed to serialize: {}", e)))
    }
}
