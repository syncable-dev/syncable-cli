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
pub mod frameworks;
pub mod language_detector;
pub mod project_context;
pub mod vulnerability_checker;
pub mod security_analyzer;
pub mod security;
pub mod tool_installer;
pub mod monorepo_detector;
pub mod docker_analyzer;
pub mod display;

// Re-export dependency analysis types
pub use dependency_parser::{
    DependencyInfo, DependencyAnalysis, DetailedDependencyMap,
    Vulnerability, VulnerabilitySeverity
};

// Re-export security analysis types
pub use security_analyzer::{
    SecurityAnalyzer, SecurityReport, SecurityFinding, SecuritySeverity,
    SecurityCategory, ComplianceStatus, SecurityAnalysisConfig
};

// Re-export security analysis types
pub use security::{
    SecretPatternManager
};
pub use security::config::SecurityConfigPreset;

// Re-export monorepo analysis types
pub use monorepo_detector::{
    MonorepoDetectionConfig, analyze_monorepo, analyze_monorepo_with_config
};

// Re-export Docker analysis types
pub use docker_analyzer::{
    DockerAnalysis, DockerfileInfo, ComposeFileInfo, DockerService, 
    OrchestrationPattern, NetworkingConfig, DockerEnvironment,
    analyze_docker_infrastructure
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

/// Categories of detected technologies with proper classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TechnologyCategory {
    /// Full-stack meta-frameworks that provide complete application structure
    MetaFramework,
    /// Frontend frameworks that provide application structure (Angular, Svelte)
    FrontendFramework,
    /// Backend frameworks that provide server structure (Express, Django, Spring Boot)
    BackendFramework,
    /// Libraries that provide specific functionality (React, Tanstack Query, Axios)
    Library(LibraryType),
    /// Build and development tools (Vite, Webpack, Rollup)
    BuildTool,
    /// Database and ORM tools (Prisma, TypeORM, SQLAlchemy)
    Database,
    /// Testing frameworks and libraries (Jest, Vitest, Cypress)
    Testing,
    /// JavaScript/Python/etc runtimes (Node.js, Bun, Deno)
    Runtime,
    /// Package managers (npm, yarn, pnpm, pip, cargo)
    PackageManager,
}

/// Specific types of libraries for better classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LibraryType {
    /// UI libraries (React, Vue, Preact)
    UI,
    /// State management (Zustand, Redux, Pinia)
    StateManagement,
    /// Data fetching (Tanstack Query, Apollo, Relay)
    DataFetching,
    /// Routing (React Router, Vue Router - when not meta-framework)
    Routing,
    /// Styling (Styled Components, Emotion, Tailwind)
    Styling,
    /// Utilities (Lodash, Date-fns, Zod)
    Utility,
    /// HTTP clients (Axios, Fetch libraries)
    HttpClient,
    /// Authentication (Auth0, Firebase Auth)
    Authentication,
    /// CLI frameworks (clap, structopt, argh)
    CLI,
    /// Other specific types
    Other(String),
}

/// Represents a detected technology (framework, library, or tool)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectedTechnology {
    pub name: String,
    pub version: Option<String>,
    pub category: TechnologyCategory,
    pub confidence: f32,
    /// Dependencies this technology requires (e.g., Next.js requires React)
    pub requires: Vec<String>,
    /// Technologies that conflict with this one (e.g., Tanstack Start conflicts with React Router v7)
    pub conflicts_with: Vec<String>,
    /// Whether this is the primary technology driving the architecture
    pub is_primary: bool,
}

/// Represents a service within a microservice architecture
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceAnalysis {
    pub name: String,
    pub path: PathBuf,
    pub languages: Vec<DetectedLanguage>,
    pub technologies: Vec<DetectedTechnology>,
    pub entry_points: Vec<EntryPoint>,
    pub ports: Vec<Port>,
    pub environment_variables: Vec<EnvVar>,
    pub build_scripts: Vec<BuildScript>,
    pub service_type: ProjectType,
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

/// Types of project architectures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArchitectureType {
    /// Single application/service
    Monolithic,
    /// Multiple services in one repository
    Microservices,
    /// Mixed approach with both
    Hybrid,
}

/// Backward compatibility type alias
pub type DetectedFramework = DetectedTechnology;

/// Enhanced project analysis with proper technology classification and microservice support
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectAnalysis {
    pub project_root: PathBuf,
    pub languages: Vec<DetectedLanguage>,
    /// All detected technologies (frameworks, libraries, tools) with proper classification
    pub technologies: Vec<DetectedTechnology>,
    /// Legacy field for backward compatibility - will be populated from technologies
    #[deprecated(note = "Use technologies field instead")]
    pub frameworks: Vec<DetectedFramework>,
    pub dependencies: DependencyMap,
    pub entry_points: Vec<EntryPoint>,
    pub ports: Vec<Port>,
    pub environment_variables: Vec<EnvVar>,
    pub project_type: ProjectType,
    pub build_scripts: Vec<BuildScript>,
    /// Individual service analyses for microservice architectures
    pub services: Vec<ServiceAnalysis>,
    /// Whether this is a monolithic project or microservice architecture
    pub architecture_type: ArchitectureType,
    /// Docker infrastructure analysis
    pub docker_analysis: Option<DockerAnalysis>,
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

/// Represents an individual project within a monorepo
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectInfo {
    /// Relative path from the monorepo root
    pub path: PathBuf,
    /// Display name for the project (derived from directory name or package name)
    pub name: String,
    /// Type of project (frontend, backend, service, etc.)
    pub project_category: ProjectCategory,
    /// Full analysis of this specific project
    pub analysis: ProjectAnalysis,
}

/// Category of project within a monorepo
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectCategory {
    Frontend,
    Backend,
    Api,
    Service,
    Library,
    Tool,
    Documentation,
    Infrastructure,
    Unknown,
}

/// Represents the overall analysis of a monorepo or single project
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonorepoAnalysis {
    /// Root path of the analysis
    pub root_path: PathBuf,
    /// Whether this is a monorepo (multiple projects) or single project
    pub is_monorepo: bool,
    /// List of detected projects (will have 1 item for single projects)
    pub projects: Vec<ProjectInfo>,
    /// Overall metadata for the entire analysis
    pub metadata: AnalysisMetadata,
    /// Summary of all technologies found across projects
    pub technology_summary: TechnologySummary,
}

/// Summary of technologies across all projects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TechnologySummary {
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub databases: Vec<String>,
    pub total_projects: usize,
    pub architecture_pattern: ArchitecturePattern,
}

/// Detected architecture patterns
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArchitecturePattern {
    /// Single application
    Monolithic,
    /// Frontend + Backend separation
    Fullstack,
    /// Multiple independent services
    Microservices,
    /// API-first architecture
    ApiFirst,
    /// Event-driven architecture
    EventDriven,
    /// Unknown or mixed pattern
    Mixed,
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
    
    // Analyze Docker infrastructure
    let docker_analysis = analyze_docker_infrastructure(&project_root).ok();
    
    let duration = start_time.elapsed();
    let confidence = calculate_confidence_score(&languages, &frameworks);
    
    #[allow(deprecated)]
    let analysis = ProjectAnalysis {
        project_root,
        languages,
        technologies: frameworks.clone(), // New field with proper technology classification
        frameworks, // Backward compatibility
        dependencies,
        entry_points: context.entry_points,
        ports: context.ports,
        environment_variables: context.environment_variables,
        project_type: context.project_type,
        build_scripts: context.build_scripts,
        services: vec![], // TODO: Implement microservice detection
        architecture_type: ArchitectureType::Monolithic, // TODO: Detect architecture type
        docker_analysis,
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
        
        let technologies = vec![
            DetectedTechnology {
                name: "Actix Web".to_string(),
                version: Some("4.0".to_string()),
                category: TechnologyCategory::BackendFramework,
                confidence: 0.8,
                requires: vec!["serde".to_string(), "tokio".to_string()],
                conflicts_with: vec![],
                is_primary: true,
            }
        ];
        
        let frameworks = technologies.clone(); // For backward compatibility
        
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