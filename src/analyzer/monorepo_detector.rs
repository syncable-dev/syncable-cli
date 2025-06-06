use crate::analyzer::{
    AnalysisConfig, ProjectInfo, ProjectCategory, MonorepoAnalysis, TechnologySummary, 
    ArchitecturePattern, analyze_project_with_config, ProjectAnalysis, AnalysisMetadata
};
use crate::error::Result;
use crate::common::file_utils;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use serde_json::Value as JsonValue;
use chrono::Utc;

/// Configuration for monorepo detection
#[derive(Debug, Clone)]
pub struct MonorepoDetectionConfig {
    /// Maximum depth to search for projects
    pub max_depth: usize,
    /// Minimum confidence threshold for considering a directory as a project
    pub min_project_confidence: f32,
    /// Whether to analyze subdirectories that might be projects
    pub deep_scan: bool,
    /// Patterns to exclude from project detection
    pub exclude_patterns: Vec<String>,
}

impl Default for MonorepoDetectionConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            min_project_confidence: 0.6,
            deep_scan: true,
            exclude_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "build".to_string(),
                "dist".to_string(),
                ".next".to_string(),
                "__pycache__".to_string(),
                "vendor".to_string(),
                ".venv".to_string(),
                "venv".to_string(),
                ".env".to_string(),
                "coverage".to_string(),
                "docs".to_string(),
                "tmp".to_string(),
                "temp".to_string(),
            ],
        }
    }
}

/// Detects if a path contains a monorepo and analyzes all projects within it
pub fn analyze_monorepo(path: &Path) -> Result<MonorepoAnalysis> {
    analyze_monorepo_with_config(path, &MonorepoDetectionConfig::default(), &AnalysisConfig::default())
}

/// Analyzes a monorepo with custom configuration
pub fn analyze_monorepo_with_config(
    path: &Path, 
    monorepo_config: &MonorepoDetectionConfig,
    analysis_config: &AnalysisConfig,
) -> Result<MonorepoAnalysis> {
    let start_time = std::time::Instant::now();
    let root_path = file_utils::validate_project_path(path)?;
    
    log::info!("Starting monorepo analysis of: {}", root_path.display());
    
    // Detect potential projects within the path
    let potential_projects = detect_potential_projects(&root_path, monorepo_config)?;
    
    log::debug!("Found {} potential projects", potential_projects.len());
    
    // Determine if this is actually a monorepo or just a single project
    let is_monorepo = determine_if_monorepo(&root_path, &potential_projects, monorepo_config)?;
    
    let mut projects = Vec::new();
    
    if is_monorepo && potential_projects.len() > 1 {
        // Analyze each project separately
        for project_path in potential_projects {
            if let Ok(project_info) = analyze_individual_project(&root_path, &project_path, analysis_config) {
                projects.push(project_info);
            }
        }
        
        // If we didn't find multiple valid projects, treat as single project
        if projects.len() <= 1 {
            log::info!("Detected potential monorepo but only found {} valid project(s), treating as single project", projects.len());
            projects.clear();
            let single_analysis = analyze_project_with_config(&root_path, analysis_config)?;
            projects.push(ProjectInfo {
                path: PathBuf::from("."),
                name: extract_project_name(&root_path, &single_analysis),
                project_category: determine_project_category(&single_analysis, &root_path),
                analysis: single_analysis,
            });
        }
    } else {
        // Single project analysis
        let single_analysis = analyze_project_with_config(&root_path, analysis_config)?;
        projects.push(ProjectInfo {
            path: PathBuf::from("."),
            name: extract_project_name(&root_path, &single_analysis),
            project_category: determine_project_category(&single_analysis, &root_path),
            analysis: single_analysis,
        });
    }
    
    // Generate technology summary
    let technology_summary = generate_technology_summary(&projects);
    
    let duration = start_time.elapsed();
    let metadata = AnalysisMetadata {
        timestamp: Utc::now().to_rfc3339(),
        analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
        analysis_duration_ms: duration.as_millis() as u64,
        files_analyzed: projects.iter().map(|p| p.analysis.analysis_metadata.files_analyzed).sum(),
        confidence_score: calculate_overall_confidence(&projects),
    };
    
    Ok(MonorepoAnalysis {
        root_path,
        is_monorepo: projects.len() > 1,
        projects,
        metadata,
        technology_summary,
    })
}

/// Detects potential project directories within a given path
fn detect_potential_projects(
    root_path: &Path, 
    config: &MonorepoDetectionConfig
) -> Result<Vec<PathBuf>> {
    let mut potential_projects = Vec::new();
    
    // Check if root itself is a project
    if is_project_directory(root_path)? {
        potential_projects.push(root_path.to_path_buf());
    }
    
    if config.deep_scan {
        // Recursively check subdirectories
        scan_for_projects(root_path, root_path, &mut potential_projects, 0, config)?;
    }
    
    // Remove duplicates and sort by path depth (shallower first)
    potential_projects.sort_by_key(|p| p.components().count());
    potential_projects.dedup();
    
    // Filter out nested projects (prefer parent projects)
    filter_nested_projects(potential_projects)
}

/// Recursively scans for project directories
fn scan_for_projects(
    root_path: &Path,
    current_path: &Path,
    projects: &mut Vec<PathBuf>,
    depth: usize,
    config: &MonorepoDetectionConfig,
) -> Result<()> {
    if depth >= config.max_depth {
        return Ok(());
    }
    
    if let Ok(entries) = std::fs::read_dir(current_path) {
        for entry in entries.flatten() {
            if !entry.file_type()?.is_dir() {
                continue;
            }
            
            let dir_name = entry.file_name().to_string_lossy().to_string();
            let dir_path = entry.path();
            
            // Skip excluded patterns
            if should_exclude_directory(&dir_name, config) {
                continue;
            }
            
            // Check if this directory looks like a project
            if is_project_directory(&dir_path)? {
                projects.push(dir_path.clone());
            }
            
            // Continue scanning subdirectories
            scan_for_projects(root_path, &dir_path, projects, depth + 1, config)?;
        }
    }
    
    Ok(())
}

/// Determines if a directory should be excluded from scanning
fn should_exclude_directory(dir_name: &str, config: &MonorepoDetectionConfig) -> bool {
    // Skip hidden directories
    if dir_name.starts_with('.') {
        return true;
    }
    
    // Skip excluded patterns
    config.exclude_patterns.iter().any(|pattern| dir_name == pattern)
}

/// Checks if a directory appears to be a project directory
fn is_project_directory(path: &Path) -> Result<bool> {
    // Common project indicator files
    let project_indicators = [
        // JavaScript/TypeScript
        "package.json",
        // Rust
        "Cargo.toml",
        // Python
        "requirements.txt", "pyproject.toml", "Pipfile", "setup.py",
        // Go
        "go.mod",
        // Java/Kotlin
        "pom.xml", "build.gradle", "build.gradle.kts",
        // .NET
        "*.csproj", "*.fsproj", "*.vbproj",
        // Ruby
        "Gemfile",
        // PHP
        "composer.json",
        // Docker
        "Dockerfile",
    ];
    
    // Check for manifest files
    for indicator in &project_indicators {
        if indicator.contains('*') {
            // Handle glob patterns
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        let pattern = indicator.replace('*', "");
                        if file_name.ends_with(&pattern) {
                            return Ok(true);
                        }
                    }
                }
            }
        } else {
            if path.join(indicator).exists() {
                return Ok(true);
            }
        }
    }
    
    // Check for common source directories with code
    let source_dirs = ["src", "lib", "app", "pages", "components"];
    for src_dir in &source_dirs {
        let src_path = path.join(src_dir);
        if src_path.is_dir() && directory_contains_code(&src_path)? {
            return Ok(true);
        }
    }
    
    Ok(false)
}

/// Checks if a directory contains source code files
fn directory_contains_code(path: &Path) -> Result<bool> {
    let code_extensions = ["js", "ts", "jsx", "tsx", "py", "rs", "go", "java", "kt", "cs", "rb", "php"];
    
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Some(extension) = entry.path().extension() {
                if let Some(ext_str) = extension.to_str() {
                    if code_extensions.contains(&ext_str) {
                        return Ok(true);
                    }
                }
            }
            
            // Recursively check subdirectories (limited depth)
            if entry.file_type()?.is_dir() {
                if directory_contains_code(&entry.path())? {
                    return Ok(true);
                }
            }
        }
    }
    
    Ok(false)
}

/// Filters out nested projects, keeping only top-level ones
fn filter_nested_projects(mut projects: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    projects.sort_by_key(|p| p.components().count());
    
    let mut filtered = Vec::new();
    
    for project in projects {
        let is_nested = filtered.iter().any(|parent: &PathBuf| {
            project.starts_with(parent) && project != *parent
        });
        
        if !is_nested {
            filtered.push(project);
        }
    }
    
    Ok(filtered)
}

/// Determines if the detected projects constitute a monorepo
fn determine_if_monorepo(
    root_path: &Path,
    potential_projects: &[PathBuf],
    _config: &MonorepoDetectionConfig,
) -> Result<bool> {
    // If we have multiple project directories, likely a monorepo
    if potential_projects.len() > 1 {
        return Ok(true);
    }
    
    // Check for common monorepo indicators
    let monorepo_indicators = [
        "lerna.json",           // Lerna
        "nx.json",              // Nx
        "rush.json",            // Rush
        "pnpm-workspace.yaml",  // pnpm workspaces
        "yarn.lock",            // Yarn workspaces (need to check package.json)
        "packages",             // Common packages directory
        "apps",                 // Common apps directory
        "services",             // Common services directory
        "libs",                 // Common libs directory
    ];
    
    for indicator in &monorepo_indicators {
        if root_path.join(indicator).exists() {
            return Ok(true);
        }
    }
    
    // Check package.json for workspace configuration
    let package_json_path = root_path.join("package.json");
    if package_json_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&package_json_path) {
            if let Ok(package_json) = serde_json::from_str::<JsonValue>(&content) {
                // Check for workspaces
                if package_json.get("workspaces").is_some() {
                    return Ok(true);
                }
            }
        }
    }
    
    Ok(false)
}

/// Analyzes an individual project within a monorepo
fn analyze_individual_project(
    root_path: &Path,
    project_path: &Path,
    config: &AnalysisConfig,
) -> Result<ProjectInfo> {
    log::debug!("Analyzing individual project: {}", project_path.display());
    
    let analysis = analyze_project_with_config(project_path, config)?;
    let relative_path = project_path.strip_prefix(root_path)
        .unwrap_or(project_path)
        .to_path_buf();
    
    let name = extract_project_name(project_path, &analysis);
    let category = determine_project_category(&analysis, project_path);
    
    Ok(ProjectInfo {
        path: relative_path,
        name,
        project_category: category,
        analysis,
    })
}

/// Extracts a meaningful project name from path and analysis
fn extract_project_name(project_path: &Path, _analysis: &ProjectAnalysis) -> String {
    // Try to get name from package.json
    let package_json_path = project_path.join("package.json");
    if package_json_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&package_json_path) {
            if let Ok(package_json) = serde_json::from_str::<JsonValue>(&content) {
                if let Some(name) = package_json.get("name").and_then(|n| n.as_str()) {
                    return name.to_string();
                }
            }
        }
    }
    
    // Try to get name from Cargo.toml
    let cargo_toml_path = project_path.join("Cargo.toml");
    if cargo_toml_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_toml_path) {
            if let Ok(cargo_toml) = toml::from_str::<toml::Value>(&content) {
                if let Some(name) = cargo_toml.get("package")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str()) {
                    return name.to_string();
                }
            }
        }
    }
    
    // Try to get name from pyproject.toml
    let pyproject_toml_path = project_path.join("pyproject.toml");
    if pyproject_toml_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&pyproject_toml_path) {
            if let Ok(pyproject) = toml::from_str::<toml::Value>(&content) {
                if let Some(name) = pyproject.get("project")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str()) {
                    return name.to_string();
                } else if let Some(name) = pyproject.get("tool")
                    .and_then(|t| t.get("poetry"))
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str()) {
                    return name.to_string();
                }
            }
        }
    }
    
    // Fall back to directory name
    project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Determines the category of a project based on its analysis
fn determine_project_category(analysis: &ProjectAnalysis, project_path: &Path) -> ProjectCategory {
    let dir_name = project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    // Check directory name patterns first
    let category_from_name = match dir_name.as_str() {
        name if name.contains("frontend") || name.contains("client") || name.contains("web") => Some(ProjectCategory::Frontend),
        name if name.contains("backend") || name.contains("server") => Some(ProjectCategory::Backend),
        name if name.contains("api") => Some(ProjectCategory::Api),
        name if name.contains("service") => Some(ProjectCategory::Service),
        name if name.contains("lib") || name.contains("library") => Some(ProjectCategory::Library),
        name if name.contains("tool") || name.contains("cli") => Some(ProjectCategory::Tool),
        name if name.contains("docs") || name.contains("doc") => Some(ProjectCategory::Documentation),
        name if name.contains("infra") || name.contains("deploy") => Some(ProjectCategory::Infrastructure),
        _ => None,
    };
    
    // If we found a category from the directory name, return it
    if let Some(category) = category_from_name {
        return category;
    }
    
    // Analyze technologies to determine category
    let has_frontend_tech = analysis.technologies.iter().any(|t| {
        matches!(t.name.as_str(), 
            "React" | "Vue.js" | "Angular" | "Next.js" | "Nuxt.js" | "Svelte" | 
            "Astro" | "Gatsby" | "Vite" | "Webpack" | "Parcel"
        )
    });
    
    let has_backend_tech = analysis.technologies.iter().any(|t| {
        matches!(t.name.as_str(),
            "Express.js" | "FastAPI" | "Django" | "Flask" | "Actix Web" | "Rocket" |
            "Spring Boot" | "Gin" | "Echo" | "Fiber" | "ASP.NET"
        )
    });
    
    let has_api_tech = analysis.technologies.iter().any(|t| {
        matches!(t.name.as_str(),
            "REST API" | "GraphQL" | "gRPC" | "FastAPI" | "Express.js"
        )
    });
    
    let has_database = analysis.technologies.iter().any(|t| {
        matches!(t.category, crate::analyzer::TechnologyCategory::Database)
    });
    
    if has_frontend_tech && !has_backend_tech {
        ProjectCategory::Frontend
    } else if has_backend_tech && !has_frontend_tech {
        ProjectCategory::Backend
    } else if has_api_tech || (has_backend_tech && has_database) {
        ProjectCategory::Api
    } else if matches!(analysis.project_type, crate::analyzer::ProjectType::Library) {
        ProjectCategory::Library
    } else if matches!(analysis.project_type, crate::analyzer::ProjectType::CliTool) {
        ProjectCategory::Tool
    } else {
        ProjectCategory::Unknown
    }
}

/// Generates a summary of technologies across all projects
fn generate_technology_summary(projects: &[ProjectInfo]) -> TechnologySummary {
    let mut all_languages = HashSet::new();
    let mut all_frameworks = HashSet::new();
    let mut all_databases = HashSet::new();
    
    for project in projects {
        // Collect languages
        for lang in &project.analysis.languages {
            all_languages.insert(lang.name.clone());
        }
        
        // Collect technologies
        for tech in &project.analysis.technologies {
            match tech.category {
                crate::analyzer::TechnologyCategory::FrontendFramework |
                crate::analyzer::TechnologyCategory::BackendFramework |
                crate::analyzer::TechnologyCategory::MetaFramework => {
                    all_frameworks.insert(tech.name.clone());
                }
                crate::analyzer::TechnologyCategory::Database => {
                    all_databases.insert(tech.name.clone());
                }
                _ => {}
            }
        }
    }
    
    let architecture_pattern = determine_architecture_pattern(projects);
    
    TechnologySummary {
        languages: all_languages.into_iter().collect(),
        frameworks: all_frameworks.into_iter().collect(),
        databases: all_databases.into_iter().collect(),
        total_projects: projects.len(),
        architecture_pattern,
    }
}

/// Determines the overall architecture pattern
fn determine_architecture_pattern(projects: &[ProjectInfo]) -> ArchitecturePattern {
    if projects.len() == 1 {
        return ArchitecturePattern::Monolithic;
    }
    
    let has_frontend = projects.iter().any(|p| p.project_category == ProjectCategory::Frontend);
    let has_backend = projects.iter().any(|p| matches!(p.project_category, ProjectCategory::Backend | ProjectCategory::Api));
    let service_count = projects.iter().filter(|p| p.project_category == ProjectCategory::Service).count();
    
    if service_count >= 2 {
        ArchitecturePattern::Microservices
    } else if has_frontend && has_backend {
        ArchitecturePattern::Fullstack
    } else if projects.iter().all(|p| p.project_category == ProjectCategory::Api) {
        ArchitecturePattern::ApiFirst
    } else {
        ArchitecturePattern::Mixed
    }
}

/// Calculates overall confidence score across all projects
fn calculate_overall_confidence(projects: &[ProjectInfo]) -> f32 {
    if projects.is_empty() {
        return 0.0;
    }
    
    let total_confidence: f32 = projects.iter()
        .map(|p| p.analysis.analysis_metadata.confidence_score)
        .sum();
    
    total_confidence / projects.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_single_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create a simple Node.js project
        fs::write(root.join("package.json"), r#"{"name": "test-app"}"#).unwrap();
        fs::write(root.join("index.js"), "console.log('hello');").unwrap();
        
        let analysis = analyze_monorepo(root).unwrap();
        
        assert!(!analysis.is_monorepo);
        assert_eq!(analysis.projects.len(), 1);
        assert_eq!(analysis.projects[0].name, "test-app");
    }
    
    #[test]
    fn test_monorepo_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create frontend project
        let frontend_dir = root.join("frontend");
        fs::create_dir_all(&frontend_dir).unwrap();
        fs::write(frontend_dir.join("package.json"), r#"{"name": "frontend-app", "dependencies": {"react": "^18.0.0"}}"#).unwrap();
        
        // Create backend project
        let backend_dir = root.join("backend");
        fs::create_dir_all(&backend_dir).unwrap();
        fs::write(backend_dir.join("package.json"), r#"{"name": "backend-api", "dependencies": {"express": "^4.18.0"}}"#).unwrap();
        
        // Create root package.json with workspaces
        fs::write(root.join("package.json"), r#"{"name": "monorepo", "workspaces": ["frontend", "backend"]}"#).unwrap();
        
        let analysis = analyze_monorepo(root).unwrap();
        
        assert!(analysis.is_monorepo);
        assert_eq!(analysis.projects.len(), 2);
        assert_eq!(analysis.technology_summary.architecture_pattern, ArchitecturePattern::Fullstack);
    }
} 