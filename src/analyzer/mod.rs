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

pub mod context;
pub mod dclint;
pub mod dependency_parser;
pub mod display;
pub mod docker_analyzer;
pub mod framework_detector;
pub mod frameworks;
pub mod hadolint;
pub mod helmlint;
pub mod k8s_optimize;
pub mod kubelint;
pub mod language_detector;
pub mod monorepo;
pub mod runtime;
pub mod security;
pub mod security_analyzer;
pub mod tool_management;
pub mod vulnerability;

// Re-export dependency analysis types
pub use dependency_parser::{DependencyAnalysis, DependencyInfo, DetailedDependencyMap};

// Re-export security analysis types
pub use security_analyzer::{
    ComplianceStatus, SecurityAnalysisConfig, SecurityAnalyzer, SecurityCategory, SecurityFinding,
    SecurityReport, SecuritySeverity,
};

// Re-export security analysis types
pub use security::SecretPatternManager;
pub use security::config::SecurityConfigPreset;

// Re-export tool management types
pub use tool_management::{InstallationSource, ToolDetector, ToolInstaller, ToolStatus};

// Re-export runtime detection types
pub use runtime::{
    DetectionConfidence, JavaScriptRuntime, PackageManager, RuntimeDetectionResult, RuntimeDetector,
};

// Re-export vulnerability checking types
pub use vulnerability::types::VulnerabilitySeverity as VulnSeverity;
pub use vulnerability::{
    VulnerabilityChecker, VulnerabilityInfo, VulnerabilityReport, VulnerableDependency,
};

// Re-export monorepo analysis types
pub use monorepo::{MonorepoDetectionConfig, analyze_monorepo, analyze_monorepo_with_config};

// Re-export Docker analysis types
pub use docker_analyzer::{
    ComposeFileInfo, DiscoveredDockerfile, DockerAnalysis, DockerEnvironment, DockerService,
    DockerfileInfo, NetworkingConfig, OrchestrationPattern, analyze_docker_infrastructure,
    discover_dockerfiles_for_deployment,
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
    /// File indicators that helped identify this technology
    pub file_indicators: Vec<String>,
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

/// Source of port detection - indicates where the port was discovered
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PortSource {
    /// Detected from Dockerfile EXPOSE directive
    Dockerfile,
    /// Detected from docker-compose.yml ports section
    DockerCompose,
    /// Detected from package.json scripts (Node.js)
    PackageJson,
    /// Inferred from framework defaults (e.g., Express=3000, FastAPI=8000)
    FrameworkDefault,
    /// Detected from environment variable reference (e.g., process.env.PORT)
    EnvVar,
    /// Detected from source code analysis (e.g., .listen(3000))
    SourceCode,
    /// Detected from configuration files (e.g., config.yaml, settings.py)
    ConfigFile,
}

impl PortSource {
    /// Returns a human-readable description of the port source
    pub fn description(&self) -> &'static str {
        match self {
            PortSource::Dockerfile => "Dockerfile EXPOSE",
            PortSource::DockerCompose => "docker-compose.yml",
            PortSource::PackageJson => "package.json scripts",
            PortSource::FrameworkDefault => "framework default",
            PortSource::EnvVar => "environment variable",
            PortSource::SourceCode => "source code",
            PortSource::ConfigFile => "configuration file",
        }
    }
}

/// Represents exposed network ports
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Port {
    pub number: u16,
    pub protocol: Protocol,
    pub description: Option<String>,
    /// Source where this port was detected (optional for backward compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<PortSource>,
}

impl Port {
    /// Create a new port with source information
    pub fn with_source(number: u16, protocol: Protocol, source: PortSource) -> Self {
        Self {
            number,
            protocol,
            description: None,
            source: Some(source),
        }
    }

    /// Create a new port with source and description
    pub fn with_source_and_description(
        number: u16,
        protocol: Protocol,
        source: PortSource,
        description: impl Into<String>,
    ) -> Self {
        Self {
            number,
            protocol,
            description: Some(description.into()),
            source: Some(source),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Protocol {
    Tcp,
    Udp,
    Http,
    Https,
}

/// Source of health endpoint detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthEndpointSource {
    /// Found by analyzing source code patterns
    CodePattern,
    /// Known framework convention (e.g., Spring Actuator)
    FrameworkDefault,
    /// Found in configuration files (e.g., K8s manifests, docker-compose)
    ConfigFile,
}

impl HealthEndpointSource {
    /// Returns a human-readable description of the detection source
    pub fn description(&self) -> &'static str {
        match self {
            HealthEndpointSource::CodePattern => "source code analysis",
            HealthEndpointSource::FrameworkDefault => "framework convention",
            HealthEndpointSource::ConfigFile => "configuration file",
        }
    }
}

/// Represents a detected health check endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthEndpoint {
    /// The HTTP path for the health check (e.g., "/health", "/healthz")
    pub path: String,
    /// Confidence level (0.0-1.0) in this detection
    pub confidence: f32,
    /// Where this endpoint was detected from
    pub source: HealthEndpointSource,
    /// Optional description or context
    pub description: Option<String>,
}

impl HealthEndpoint {
    /// Create a new health endpoint with high confidence from code analysis
    pub fn from_code(path: impl Into<String>, confidence: f32) -> Self {
        Self {
            path: path.into(),
            confidence,
            source: HealthEndpointSource::CodePattern,
            description: None,
        }
    }

    /// Create a health endpoint from a framework default
    pub fn from_framework(path: impl Into<String>, framework: &str) -> Self {
        Self {
            path: path.into(),
            confidence: 0.7, // Framework defaults have moderate confidence
            source: HealthEndpointSource::FrameworkDefault,
            description: Some(format!("{} default health endpoint", framework)),
        }
    }
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

/// Detected infrastructure files and configurations in the project
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct InfrastructurePresence {
    /// Whether Kubernetes manifests were detected
    pub has_kubernetes: bool,
    /// Paths to directories or files containing K8s manifests
    pub kubernetes_paths: Vec<PathBuf>,
    /// Whether Helm charts were detected
    pub has_helm: bool,
    /// Paths to Helm chart directories (containing Chart.yaml)
    pub helm_chart_paths: Vec<PathBuf>,
    /// Whether docker-compose files were detected
    pub has_docker_compose: bool,
    /// Whether Terraform files were detected
    pub has_terraform: bool,
    /// Paths to directories containing .tf files
    pub terraform_paths: Vec<PathBuf>,
    /// Whether Syncable deployment config exists
    pub has_deployment_config: bool,
    /// Summary of what was detected for display purposes
    pub summary: Option<String>,
}

impl InfrastructurePresence {
    /// Returns true if any infrastructure was detected
    pub fn has_any(&self) -> bool {
        self.has_kubernetes
            || self.has_helm
            || self.has_docker_compose
            || self.has_terraform
            || self.has_deployment_config
    }

    /// Returns a list of detected infrastructure types
    pub fn detected_types(&self) -> Vec<&'static str> {
        let mut types = Vec::new();
        if self.has_kubernetes {
            types.push("Kubernetes");
        }
        if self.has_helm {
            types.push("Helm");
        }
        if self.has_docker_compose {
            types.push("Docker Compose");
        }
        if self.has_terraform {
            types.push("Terraform");
        }
        if self.has_deployment_config {
            types.push("Syncable Config");
        }
        types
    }
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
    /// Detected health check endpoints
    #[serde(default)]
    pub health_endpoints: Vec<HealthEndpoint>,
    pub environment_variables: Vec<EnvVar>,
    pub project_type: ProjectType,
    pub build_scripts: Vec<BuildScript>,
    /// Individual service analyses for microservice architectures
    pub services: Vec<ServiceAnalysis>,
    /// Whether this is a monolithic project or microservice architecture
    pub architecture_type: ArchitectureType,
    /// Docker infrastructure analysis
    pub docker_analysis: Option<DockerAnalysis>,
    /// Detected infrastructure (K8s, Helm, Terraform, etc.)
    #[serde(default)]
    pub infrastructure: Option<InfrastructurePresence>,
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
pub fn analyze_project_with_config(
    path: &Path,
    config: &AnalysisConfig,
) -> Result<ProjectAnalysis> {
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
    let context = context::analyze_context(&project_root, &languages, &frameworks, config)?;

    // Detect health check endpoints
    let health_endpoints =
        context::detect_health_endpoints(&project_root, &frameworks, config.max_file_size);

    // Detect infrastructure presence (K8s, Helm, Terraform, etc.)
    let infrastructure = context::detect_infrastructure(&project_root);

    // Analyze Docker infrastructure
    let docker_analysis = analyze_docker_infrastructure(&project_root).ok();

    let duration = start_time.elapsed();
    let confidence = calculate_confidence_score(&languages, &frameworks);

    #[allow(deprecated)]
    let analysis = ProjectAnalysis {
        project_root,
        languages,
        technologies: frameworks.clone(), // New field with proper technology classification
        frameworks,                       // Backward compatibility
        dependencies,
        entry_points: context.entry_points,
        ports: context.ports,
        health_endpoints,
        environment_variables: context.environment_variables,
        project_type: context.project_type,
        build_scripts: context.build_scripts,
        services: vec![], // TODO: Implement microservice detection
        architecture_type: ArchitectureType::Monolithic, // TODO: Detect architecture type
        docker_analysis,
        infrastructure: Some(infrastructure),
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

    let lang_confidence: f32 =
        languages.iter().map(|l| l.confidence).sum::<f32>() / languages.len() as f32;
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
        let languages = vec![DetectedLanguage {
            name: "Rust".to_string(),
            version: Some("1.70.0".to_string()),
            confidence: 0.9,
            files: vec![],
            main_dependencies: vec!["serde".to_string(), "tokio".to_string()],
            dev_dependencies: vec!["assert_cmd".to_string()],
            package_manager: Some("cargo".to_string()),
        }];

        let technologies = vec![DetectedTechnology {
            name: "Actix Web".to_string(),
            version: Some("4.0".to_string()),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.8,
            requires: vec!["serde".to_string(), "tokio".to_string()],
            conflicts_with: vec![],
            is_primary: true,
            file_indicators: vec![],
        }];

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
