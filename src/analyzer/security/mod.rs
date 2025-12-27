//! # Security Analysis Module
//!
//! Modular security analysis with language-specific analyzers for better threat detection.
//!
//! This module provides a layered approach to security analysis:
//! - Core security patterns (generic)
//! - Language-specific analyzers (JS/TS, Python, etc.)
//! - Framework-specific detection
//! - Context-aware severity assessment

use thiserror::Error;

pub mod config;
pub mod core;
pub mod patterns;
pub mod turbo;

pub use config::SecurityAnalysisConfig;
pub use core::{
    SecurityAnalyzer, SecurityCategory, SecurityFinding, SecurityReport, SecuritySeverity,
};
pub use patterns::SecretPatternManager;
pub use turbo::{ScanMode, TurboConfig, TurboSecurityAnalyzer};

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
