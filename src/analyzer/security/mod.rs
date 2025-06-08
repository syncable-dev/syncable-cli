//! # Security Analysis Module
//! 
//! Modular security analysis with language-specific analyzers for better threat detection.
//! 
//! This module provides a layered approach to security analysis:
//! - Core security patterns (generic)
//! - Language-specific analyzers (JS/TS, Python, etc.)
//! - Framework-specific detection
//! - Context-aware severity assessment

use std::path::Path;
use thiserror::Error;

pub mod core;
pub mod javascript;
pub mod patterns;
pub mod config;
pub mod gitignore;

pub use core::{SecurityAnalyzer, SecurityReport, SecurityFinding, SecuritySeverity, SecurityCategory};
pub use javascript::JavaScriptSecurityAnalyzer;
pub use patterns::SecretPatternManager;
pub use config::SecurityAnalysisConfig;
pub use gitignore::{GitIgnoreAnalyzer, GitIgnoreStatus, GitIgnoreRisk};

/// Modular security analyzer that delegates to language-specific analyzers
pub struct ModularSecurityAnalyzer {
    javascript_analyzer: JavaScriptSecurityAnalyzer,
    // TODO: Add other language analyzers
    // python_analyzer: PythonSecurityAnalyzer,
    // rust_analyzer: RustSecurityAnalyzer,
}

impl ModularSecurityAnalyzer {
    pub fn new() -> Result<Self, SecurityError> {
        Ok(Self {
            javascript_analyzer: JavaScriptSecurityAnalyzer::new()?,
        })
    }
    
    pub fn with_config(config: SecurityAnalysisConfig) -> Result<Self, SecurityError> {
        Ok(Self {
            javascript_analyzer: JavaScriptSecurityAnalyzer::with_config(config.clone())?,
        })
    }
    
    /// Analyze a project with appropriate language-specific analyzers
    pub fn analyze_project(&mut self, project_root: &Path, languages: &[crate::analyzer::DetectedLanguage]) -> Result<SecurityReport, SecurityError> {
        let mut all_findings = Vec::new();
        
        // Analyze JavaScript/TypeScript files
        if languages.iter().any(|lang| matches!(lang.name.as_str(), "JavaScript" | "TypeScript" | "JSX" | "TSX")) {
            let js_report = self.javascript_analyzer.analyze_project(project_root)?;
            all_findings.extend(js_report.findings);
        }
        
        // TODO: Add other language analyzers based on detected languages
        
        // Combine results into a comprehensive report
        Ok(SecurityReport::from_findings(all_findings))
    }
}

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Security analysis failed: {0}")]
    AnalysisFailed(String),
    
    #[error("Pattern compilation error: {0}")]
    PatternError(#[from] regex::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JavaScript security analysis error: {0}")]
    JavaScriptError(String),
} 