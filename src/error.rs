use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IaCGeneratorError {
    #[error("Project analysis failed: {0}")]
    Analysis(#[from] AnalysisError),

    #[error("IaC generation failed: {0}")]
    Generation(#[from] GeneratorError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Walk directory error: {0}")]
    WalkDir(#[from] walkdir::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Security error: {0}")]
    Security(#[from] SecurityError),
}

#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("Unsupported project type: {0}")]
    UnsupportedProject(String),

    #[error("Failed to detect language in {path}")]
    LanguageDetection { path: PathBuf },

    #[error("Dependency parsing failed for {file}: {reason}")]
    DependencyParsing { file: String, reason: String },

    #[error("Framework detection failed: {0}")]
    FrameworkDetection(String),

    #[error("Invalid project structure: {0}")]
    InvalidStructure(String),
}

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("Template rendering failed: {0}")]
    TemplateRendering(String),

    #[error("Unsupported generator type: {0}")]
    UnsupportedGenerator(String),

    #[error("Output file creation failed: {path}")]
    OutputCreation { path: PathBuf },

    #[error("Invalid generation context: {0}")]
    InvalidContext(String),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid configuration file: {0}")]
    InvalidFile(String),

    #[error("Missing required configuration: {0}")]
    MissingConfig(String),

    #[error("Configuration parsing failed: {0}")]
    ParsingFailed(String),
}

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Invalid path: path traversal detected")]
    PathTraversal,

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),
}

pub type Result<T> = std::result::Result<T, IaCGeneratorError>; 