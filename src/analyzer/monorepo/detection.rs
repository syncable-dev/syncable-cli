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

            // Skip placeholder/template directories like `${{ values.name }}`
            if is_placeholder_dir(&dir_path) {
                continue;
            }

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
    config
        .exclude_patterns
        .iter()
        .any(|pattern| dir_name == pattern)
}

/// Checks if a directory appears to be a project directory
fn is_project_directory(path: &Path) -> Result<bool> {
    // If package.json exists but has a template placeholder name, treat as non-project
    let pkg = path.join("package.json");
    if pkg.exists() {
        if let Ok(content) = std::fs::read_to_string(&pkg) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if json
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.contains("${") || s.contains("}}"))
                    == Some(true)
                {
                    return Ok(false);
                }
            }
        }
    }

    // Common project indicator files
    let project_indicators = [
        // JavaScript/TypeScript
        "package.json",
        // Rust
        "Cargo.toml",
        // Python
        "requirements.txt",
        "pyproject.toml",
        "Pipfile",
        "setup.py",
        // Go
        "go.mod",
        // Java/Kotlin
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        // .NET
        "*.csproj",
        "*.fsproj",
        "*.vbproj",
        // Ruby
        "Gemfile",
        // PHP
        "composer.json",
        // Docker
        "Dockerfile",
    ];

    let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Skip obvious template placeholders and generic buckets when no manifest exists
    let generic_buckets = [
        "src", "packages", "apps", "app", "libs", "services", "packages",
    ];
    let is_template_placeholder = is_placeholder_dir(path);

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

    // If we reach here there is no manifest. Avoid promoting plain source buckets to projects.
    if is_template_placeholder || generic_buckets.contains(&dir_name) {
        return Ok(false);
    }

    Ok(false)
}

/// Returns true for directory names that are template placeholders (e.g. `${{ values.name }}`)
fn is_placeholder_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.contains("${") || s.contains("}}"))
        .unwrap_or(false)
}

/// Checks if a directory contains source code files
fn directory_contains_code(path: &Path) -> Result<bool> {
    let code_extensions = [
        "js", "ts", "jsx", "tsx", "py", "rs", "go", "java", "kt", "cs", "rb", "php",
    ];

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
    // Keep all distinct projects, including nested ones (workspace roots often co-exist with member crates/apps)
    projects.sort();
    projects.dedup();
    Ok(projects)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_nested_projects_for_workspaces() {
        let projects = vec![
            PathBuf::from("."),
            PathBuf::from("apps/api"),
            PathBuf::from("apps/web"),
            PathBuf::from("libs/common"),
        ];

        let filtered = filter_nested_projects(projects).unwrap();

        assert!(filtered.iter().any(|p| p == &PathBuf::from(".")));
        assert!(filtered.iter().any(|p| p == &PathBuf::from("apps/api")));
        assert!(filtered.iter().any(|p| p == &PathBuf::from("apps/web")));
        assert!(filtered.iter().any(|p| p == &PathBuf::from("libs/common")));
    }

    #[test]
    fn skips_placeholder_dirs() {
        assert!(is_placeholder_dir(Path::new("${{ values.name }}")));
        assert!(is_placeholder_dir(Path::new("templates/${{ service }}")));
        assert!(!is_placeholder_dir(Path::new("apps/api")));
    }

    #[test]
    fn skips_placeholder_package_json_name() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_path = tmp.path().join("package.json");
        std::fs::write(
            &pkg_path,
            r#"{ "name": "${{ values.name }}", "version": "1.0.0" }"#,
        )
        .unwrap();

        assert!(!is_project_directory(tmp.path()).unwrap());
    }
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
        "lerna.json",          // Lerna
        "nx.json",             // Nx
        "rush.json",           // Rush
        "pnpm-workspace.yaml", // pnpm workspaces
        "yarn.lock",           // Yarn workspaces (need to check package.json)
        "packages",            // Common packages directory
        "apps",                // Common apps directory
        "services",            // Common services directory
        "libs",                // Common libs directory
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
