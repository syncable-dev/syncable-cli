//! # Core Security Analysis Types
//! 
//! Base types and functionality shared across all security analyzers.

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Security finding severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecuritySeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Categories of security findings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SecurityCategory {
    /// Exposed secrets, API keys, passwords
    SecretsExposure,
    /// Insecure configuration settings
    InsecureConfiguration,
    /// Language/framework-specific security patterns
    CodeSecurityPattern,
    /// Infrastructure and deployment security
    InfrastructureSecurity,
    /// Authentication and authorization issues
    AuthenticationSecurity,
    /// Data protection and privacy concerns
    DataProtection,
    /// Network and communication security
    NetworkSecurity,
    /// Compliance and regulatory requirements
    Compliance,
    /// Code injection vulnerabilities (eval, exec, etc.)
    CodeInjection,
    /// Command injection vulnerabilities (subprocess, os.system, etc.)
    CommandInjection,
}

/// A security finding with details and remediation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: SecuritySeverity,
    pub category: SecurityCategory,
    pub file_path: Option<PathBuf>,
    pub line_number: Option<usize>,
    pub column_number: Option<usize>,
    pub evidence: Option<String>,
    pub remediation: Vec<String>,
    pub references: Vec<String>,
    pub cwe_id: Option<String>,
    pub compliance_frameworks: Vec<String>,
}

/// Comprehensive security analysis report
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityReport {
    pub analyzed_at: chrono::DateTime<chrono::Utc>,
    pub overall_score: f32, // 0-100, higher is better
    pub risk_level: SecuritySeverity,
    pub total_findings: usize,
    pub findings_by_severity: HashMap<SecuritySeverity, usize>,
    pub findings_by_category: HashMap<SecurityCategory, usize>,
    pub findings: Vec<SecurityFinding>,
    pub recommendations: Vec<String>,
    pub compliance_status: HashMap<String, ComplianceStatus>,
}

/// Compliance framework status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub framework: String,
    pub coverage: f32, // 0-100%
    pub missing_controls: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Base security analyzer trait
pub trait SecurityAnalyzer {
    type Config;
    type Error: std::error::Error;
    
    /// Analyze a project for security issues
    fn analyze_project(&self, project_root: &std::path::Path) -> Result<SecurityReport, Self::Error>;
    
    /// Get the analyzer's configuration
    fn config(&self) -> &Self::Config;
    
    /// Get supported file extensions for this analyzer
    fn supported_extensions(&self) -> Vec<&'static str>;
} 