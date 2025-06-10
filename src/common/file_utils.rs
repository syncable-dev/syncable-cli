use crate::analyzer::AnalysisConfig;
use crate::error::{SecurityError, IaCGeneratorError};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{WalkDir, DirEntry};

/// Validates a project path and ensures security
pub fn validate_project_path(path: &Path) -> Result<PathBuf, IaCGeneratorError> {
    // Try to canonicalize, but be more forgiving on Windows
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            // On Windows, canonicalize can fail for valid paths due to permissions
            // Fall back to absolute path if the path exists
            if path.exists() {
                path.to_path_buf()
            } else {
                return Err(SecurityError::InvalidPath(
                    format!("Invalid path '{}': {}", path.display(), e)
                ).into());
            }
        }
    };
    
    // Basic validation - path should exist and be a directory
    if !canonical.is_dir() {
        return Err(SecurityError::InvalidPath(
            "Path is not a directory".to_string()
        ).into());
    }
    
    Ok(canonical)
}

/// Collects project files based on configuration
pub fn collect_project_files(
    root: &Path,
    config: &AnalysisConfig,
) -> Result<Vec<PathBuf>, IaCGeneratorError> {
    let mut files = Vec::new();
    
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored(e, &config.ignore_patterns, root))
    {
        let entry = entry?;
        
        if entry.file_type().is_file() {
            let path = entry.path();
            
            // Check file size limit
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.len() > config.max_file_size as u64 {
                    log::debug!("Skipping large file: {}", path.display());
                    continue;
                }
            }
            
            // Only include relevant file types
            if is_relevant_file(path) {
                files.push(path.to_path_buf());
            }
        }
    }
    
    log::debug!("Collected {} relevant files", files.len());
    Ok(files)
}

/// Checks if a directory entry should be ignored
fn is_ignored(entry: &DirEntry, ignore_patterns: &[String], root: &Path) -> bool {
    let path = entry.path();
    
    // Get the relative path from the root
    let relative_path = match path.strip_prefix(root) {
        Ok(rel) => rel,
        Err(_) => return false, // If we can't get relative path, don't ignore
    };
    
    // Check each component of the relative path
    for component in relative_path.components() {
        if let std::path::Component::Normal(name) = component {
            if let Some(name_str) = name.to_str() {
                // Check if this component matches any ignore pattern
                for pattern in ignore_patterns {
                    if name_str == pattern {
                        return true;
                    }
                }
                
                // Ignore hidden files and directories (starting with .)
                if name_str.starts_with('.') && name_str != ".env" {
                    return true;
                }
            }
        }
    }
    
    false
}

/// Determines if a file is relevant for analysis
fn is_relevant_file(path: &Path) -> bool {
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    let filename = path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    
    // Programming language files
    let code_extensions = [
        "rs", "go", "js", "ts", "jsx", "tsx", "py", "java", "kt", "scala",
        "rb", "php", "cs", "fs", "cpp", "cc", "c", "h", "hpp", "swift",
        "dart", "elm", "clj", "cljs", "hs", "ml", "ocaml", "r", "sh", "bash",
        "zsh", "fish", "ps1", "bat", "cmd"
    ];
    
    // Configuration and manifest files
    let config_files = [
        "package.json", "package-lock.json", "yarn.lock", "pnpm-lock.yaml",
        "Cargo.toml", "Cargo.lock", "go.mod", "go.sum", "requirements.txt",
        "Pipfile", "Pipfile.lock", "pyproject.toml", "setup.py", "setup.cfg",
        "pom.xml", "build.gradle", "build.gradle.kts", "sbt", "build.sbt",
        "Gemfile", "Gemfile.lock", "composer.json", "composer.lock",
        "Dockerfile", "docker-compose.yml", "docker-compose.yaml",
        ".dockerignore", "Makefile", "makefile", "CMakeLists.txt",
        ".env", ".env.example", ".env.local", ".env.production",
        "config.yml", "config.yaml", "config.json", "config.toml",
        "app.yml", "app.yaml", "application.yml", "application.yaml",
        "tsconfig.json", "jsconfig.json", ".eslintrc", ".eslintrc.json",
        ".prettierrc", "webpack.config.js", "vite.config.js", "next.config.js",
        "nuxt.config.js", "vue.config.js", "angular.json", ".angular-cli.json"
    ];
    
    // Check by extension
    if code_extensions.contains(&extension) {
        return true;
    }
    
    // Check by filename
    if config_files.contains(&filename) {
        return true;
    }
    
    // Check for common configuration file patterns
    if filename.ends_with(".config.js") ||
       filename.ends_with(".config.ts") ||
       filename.ends_with(".config.json") ||
       filename.ends_with(".yml") ||
       filename.ends_with(".yaml") ||
       filename.ends_with(".toml") {
        return true;
    }
    
    false
}

/// Reads file content safely with size limits
pub fn read_file_safe(path: &Path, max_size: usize) -> Result<String, IaCGeneratorError> {
    let metadata = fs::metadata(path)?;
    
    if metadata.len() > max_size as u64 {
        return Err(SecurityError::InvalidPath(
            format!("File too large: {}", path.display())
        ).into());
    }
    
    Ok(fs::read_to_string(path)?)
}

/// Checks if a file exists and is readable
pub fn is_readable_file(path: &Path) -> bool {
    path.is_file() && fs::metadata(path).is_ok()
}

/// Gets the relative path from root to target
pub fn get_relative_path(root: &Path, target: &Path) -> PathBuf {
    target.strip_prefix(root)
        .unwrap_or(target)
        .to_path_buf()
}

/// Find files matching specific patterns using glob
pub fn find_files_by_patterns(root: &Path, patterns: &[&str]) -> Result<Vec<PathBuf>, std::io::Error> {
    use glob::glob;
    let mut files = Vec::new();
    
    for pattern in patterns {
        // Use cross-platform path joining
        let full_pattern = root.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();
        
        // Use glob to find matching files
        if let Ok(entries) = glob(&pattern_str) {
            for entry in entries {
                if let Ok(path) = entry {
                    if path.is_file() {
                        files.push(path);
                    }
                }
            }
        }
    }
    
    // Also try recursive patterns - use cross-platform glob patterns
    for pattern in patterns {
        // Use proper cross-platform recursive pattern
        let recursive_pattern = if cfg!(windows) {
            // Windows uses backslashes but glob understands forward slashes
            root.join("**").join(pattern)
        } else {
            root.join("**").join(pattern)
        };
        let pattern_str = recursive_pattern.to_string_lossy().replace('\\', "/");
        
        if let Ok(entries) = glob(&pattern_str) {
            for entry in entries {
                if let Ok(path) = entry {
                    if path.is_file() && !files.contains(&path) {
                        files.push(path);
                    }
                }
            }
        }
    }
    
    files.sort();
    files.dedup();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_is_relevant_file() {
        assert!(is_relevant_file(Path::new("src/main.rs")));
        assert!(is_relevant_file(Path::new("package.json")));
        assert!(is_relevant_file(Path::new("Dockerfile")));
        assert!(!is_relevant_file(Path::new("README.md")));
        assert!(!is_relevant_file(Path::new("image.png")));
    }
    
    #[test]
    fn test_validate_project_path() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        
        let result = validate_project_path(path);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_collect_project_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create test files
        fs::write(root.join("main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("package.json"), "{}").unwrap();
        fs::write(root.join("README.md"), "# Test").unwrap();
        
        let config = AnalysisConfig::default();
        let files = collect_project_files(root, &config).unwrap();
        
        assert_eq!(files.len(), 2); // main.rs and package.json
        assert!(files.iter().any(|f| f.file_name().unwrap() == "main.rs"));
        assert!(files.iter().any(|f| f.file_name().unwrap() == "package.json"));
    }
} 