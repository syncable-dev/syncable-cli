use super::config::MonorepoDetectionConfig;
use crate::error::Result;
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};

/// Detects potential project directories within a given path
pub(crate) fn detect_potential_projects(
    root_path: &Path,
    config: &MonorepoDetectionConfig,
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
pub(crate) fn determine_if_monorepo(
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