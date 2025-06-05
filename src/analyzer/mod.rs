//! # Analyzer Module
//! 
//! This module provides project analysis capabilities for detecting:
//! - Programming languages and their versions
//! - Frameworks and libraries
//! - Dependencies and their versions
//! - Entry points and exposed ports

use crate::error::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub mod dependency_parser;
pub mod framework_detector;
pub mod language_detector;
pub mod project_context;
pub mod vulnerability_checker;
pub mod tool_installer;

// Re-export dependency analysis types
pub use dependency_parser::{
    DependencyInfo, DependencyAnalysis, DetailedDependencyMap,
    Vulnerability, VulnerabilitySeverity
};

/// Represents a detected programming language
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectedLanguage {
    pub name: String,
    pub version: Option<String>,
    pub confidence: f32,
    pub files: Vec<PathBuf>,
    pub main_dependencies: Vec<String>,
    pub dev_dependencies: Vec<String>,
    pub package_manager: Option<String>,
}

/// Represents a detected framework or library
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectedFramework {
    pub name: String,
    pub version: Option<String>,
    pub category: FrameworkCategory,
    pub confidence: f32,
}

/// Categories of frameworks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FrameworkCategory {
    Web,
    Database,
    Testing,
    BuildTool,
    Runtime,
    Other(String),
}

/// Represents application entry points
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntryPoint {
    pub file: PathBuf,
    pub function: Option<String>,
    pub command: Option<String>,
}

/// Represents exposed network ports
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Port {
    pub number: u16,
    pub protocol: Protocol,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Protocol {
    Tcp,
    Udp,
    Http,
    Https,
}

/// Represents environment variables
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnvVar {
    pub name: String,
    pub default_value: Option<String>,
    pub required: bool,
    pub description: Option<String>,
}

/// Represents different project types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectType {
    WebApplication,
    ApiService,
    CliTool,
    Library,
    MobileApp,
    DesktopApp,
    Microservice,
    StaticSite,
    Hybrid, // Multiple types
    Unknown,
}

/// Represents build scripts and commands
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildScript {
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub is_default: bool,
}

/// Type alias for dependency maps
pub type DependencyMap = HashMap<String, String>;

/// Main analysis result containing all detected project information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectAnalysis {
    pub project_root: PathBuf,
    pub languages: Vec<DetectedLanguage>,
    pub frameworks: Vec<DetectedFramework>,
    pub dependencies: DependencyMap,
    pub entry_points: Vec<EntryPoint>,
    pub ports: Vec<Port>,
    pub environment_variables: Vec<EnvVar>,
    pub project_type: ProjectType,
    pub build_scripts: Vec<BuildScript>,
    pub analysis_metadata: AnalysisMetadata,
}

/// Metadata about the analysis process
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisMetadata {
    pub timestamp: String,
    pub analyzer_version: String,
    pub analysis_duration_ms: u64,
    pub files_analyzed: usize,
    pub confidence_score: f32,
}

/// Configuration for project analysis
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    pub include_dev_dependencies: bool,
    pub deep_analysis: bool,
    pub ignore_patterns: Vec<String>,
    pub max_file_size: usize,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            include_dev_dependencies: false,
            deep_analysis: true,
            ignore_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "build".to_string(),
                ".next".to_string(),
                "dist".to_string(),
            ],
            max_file_size: 1024 * 1024, // 1MB
        }
    }
}

/// Analyzes a project directory to detect languages, frameworks, and dependencies.
/// 
/// # Arguments
/// * `path` - The root directory of the project to analyze
/// 
/// # Returns
/// A `ProjectAnalysis` containing detected components or an error
/// 
/// # Examples
/// ```no_run
/// use syncable_cli::analyzer::analyze_project;
/// use std::path::Path;
/// 
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let analysis = analyze_project(Path::new("./my-project"))?;
/// println!("Languages: {:?}", analysis.languages);
/// # Ok(())
/// # }
/// ```
pub fn analyze_project(path: &Path) -> Result<ProjectAnalysis> {
    analyze_project_with_config(path, &AnalysisConfig::default())
}

/// Analyzes a project with custom configuration
pub fn analyze_project_with_config(path: &Path, config: &AnalysisConfig) -> Result<ProjectAnalysis> {
    let start_time = std::time::Instant::now();
    
    // Validate project path
    let project_root = crate::common::file_utils::validate_project_path(path)?;
    
    log::info!("Starting analysis of project: {}", project_root.display());
    
    // Collect project files
    let files = crate::common::file_utils::collect_project_files(&project_root, config)?;
    log::debug!("Found {} files to analyze", files.len());
    
    // Perform parallel analysis
    let languages = language_detector::detect_languages(&files, config)?;
    let frameworks = framework_detector::detect_frameworks(&project_root, &languages, config)?;
    let dependencies = dependency_parser::parse_dependencies(&project_root, &languages, config)?;
    let context = project_context::analyze_context(&project_root, &languages, &frameworks, config)?;
    
    let duration = start_time.elapsed();
    let confidence = calculate_confidence_score(&languages, &frameworks);
    
    let analysis = ProjectAnalysis {
        project_root,
        languages,
        frameworks,
        dependencies,
        entry_points: context.entry_points,
        ports: context.ports,
        environment_variables: context.environment_variables,
        project_type: context.project_type,
        build_scripts: context.build_scripts,
        analysis_metadata: AnalysisMetadata {
            timestamp: Utc::now().to_rfc3339(),
            analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
            analysis_duration_ms: duration.as_millis() as u64,
            files_analyzed: files.len(),
            confidence_score: confidence,
        },
    };
    
    log::info!("Analysis completed in {}ms", duration.as_millis());
    Ok(analysis)
}

/// Calculate overall confidence score based on detection results
fn calculate_confidence_score(
    languages: &[DetectedLanguage],
    frameworks: &[DetectedFramework],
) -> f32 {
    if languages.is_empty() {
        return 0.0;
    }
    
    let lang_confidence: f32 = languages.iter().map(|l| l.confidence).sum::<f32>() / languages.len() as f32;
    let framework_confidence: f32 = if frameworks.is_empty() {
        0.5 // Neutral score if no frameworks detected
    } else {
        frameworks.iter().map(|f| f.confidence).sum::<f32>() / frameworks.len() as f32
    };
    
    (lang_confidence * 0.7 + framework_confidence * 0.3).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_confidence_calculation() {
        let languages = vec![
            DetectedLanguage {
                name: "Rust".to_string(),
                version: Some("1.70.0".to_string()),
                confidence: 0.9,
                files: vec![],
                main_dependencies: vec!["serde".to_string(), "tokio".to_string()],
                dev_dependencies: vec!["assert_cmd".to_string()],
                package_manager: Some("cargo".to_string()),
            }
        ];
        
        let frameworks = vec![
            DetectedFramework {
                name: "Actix Web".to_string(),
                version: Some("4.0".to_string()),
                category: FrameworkCategory::Web,
                confidence: 0.8,
            }
        ];
        
        let score = calculate_confidence_score(&languages, &frameworks);
        assert!(score > 0.8);
        assert!(score <= 1.0);
    }
    
    #[test]
    fn test_empty_analysis() {
        let languages = vec![];
        let frameworks = vec![];
        let score = calculate_confidence_score(&languages, &frameworks);
        assert_eq!(score, 0.0);
    }
} 