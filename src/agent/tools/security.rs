//! Security and vulnerability scanning tools using Rig's Tool trait

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use super::compression::{CompressionConfig, compress_tool_output};
use crate::analyzer::security::turbo::{ScanMode, TurboConfig, TurboSecurityAnalyzer};

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

        let scanner = TurboSecurityAnalyzer::new(config)
            .map_err(|e| SecurityScanError(format!("Failed to create scanner: {}", e)))?;

        let report = scanner
            .analyze_project(&path)
            .map_err(|e| SecurityScanError(format!("Scan failed: {}", e)))?;

        // Build full result with all findings (compression will handle size)
        let result = json!({
            "total_findings": report.total_findings,
            "overall_score": report.overall_score,
            "risk_level": format!("{:?}", report.risk_level),
            "files_scanned": report.files_scanned,
            "findings": report.findings.iter().map(|f| {
                json!({
                    "title": f.title,
                    "description": f.description,
                    "severity": format!("{:?}", f.severity),
                    "category": format!("{:?}", f.category),
                    "file": f.file_path.as_ref().map(|p| p.display().to_string()),
                    "line": f.line_number,
                    "evidence": f.evidence.as_ref().map(|e| e.chars().take(100).collect::<String>()),
                })
            }).collect::<Vec<_>>(),
            "recommendations": report.recommendations.clone(),
            "scan_mode": args.mode.as_deref().unwrap_or("balanced"),
        });

        // Use compression - stores full data for RAG retrieval if output is large
        let config = CompressionConfig::default();
        Ok(compress_tool_output(&result, "security_scan", &config))
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
            description:
                "Check the project's dependencies for known security vulnerabilities (CVEs)."
                    .to_string(),
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
            })
            .to_string());
        }

        let checker = crate::analyzer::vulnerability::VulnerabilityChecker::new();
        let report = checker
            .check_all_dependencies(&dependencies, &path)
            .await
            .map_err(|e| VulnerabilitiesError(format!("Vulnerability check failed: {}", e)))?;

        // Build findings array for compression (each vuln as a separate issue)
        let mut findings = Vec::new();
        for dep in &report.vulnerable_dependencies {
            for v in &dep.vulnerabilities {
                findings.push(json!({
                    "code": v.id.clone(),
                    "severity": format!("{:?}", v.severity),
                    "title": v.title.clone(),
                    "message": format!("{} {} has vulnerability: {}", dep.name, dep.version, v.title),
                    "dependency": dep.name.clone(),
                    "version": dep.version.clone(),
                    "language": dep.language.as_str(),
                    "cve": v.cve.clone(),
                    "patched_versions": v.patched_versions.clone(),
                }));
            }
        }

        let result = json!({
            "total_vulnerabilities": report.total_vulnerabilities,
            "critical_count": report.critical_count,
            "high_count": report.high_count,
            "medium_count": report.medium_count,
            "low_count": report.low_count,
            "issues": findings,  // Use "issues" so compression can find it
        });

        // Use compression - stores full data for RAG retrieval if output is large
        let config = CompressionConfig::default();
        Ok(compress_tool_output(
            &result,
            "check_vulnerabilities",
            &config,
        ))
    }
}
