//! # GitIgnore-Aware Security Analysis
//! 
//! Comprehensive gitignore parsing and pattern matching for security analysis.
//! This module ensures that secret detection is gitignore-aware and can properly
//! assess whether sensitive files are appropriately protected.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::fs;
use log::{info, warn};
use regex::Regex;

/// GitIgnore pattern matcher for security analysis
pub struct GitIgnoreAnalyzer {
    patterns: Vec<GitIgnorePattern>,
    project_root: PathBuf,
    is_git_repo: bool,
}

/// A parsed gitignore pattern with matching logic
#[derive(Debug, Clone)]
pub struct GitIgnorePattern {
    pub original: String,
    pub regex: Regex,
    pub is_negation: bool,
    pub is_directory_only: bool,
    pub is_absolute: bool, // Starts with /
    pub pattern_type: PatternType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    /// Exact filename match (e.g., ".env")
    Exact,
    /// Wildcard pattern (e.g., "*.log")
    Wildcard,
    /// Directory pattern (e.g., "node_modules/")
    Directory,
    /// Path pattern (e.g., "config/*.env")
    Path,
}

/// Result of gitignore analysis for a file
#[derive(Debug, Clone)]
pub struct GitIgnoreStatus {
    pub is_ignored: bool,
    pub matched_pattern: Option<String>,
    pub is_tracked: bool, // Whether file is tracked by git
    pub should_be_ignored: bool, // Whether file contains secrets and should be ignored
    pub risk_level: GitIgnoreRisk,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GitIgnoreRisk {
    /// File is properly ignored and contains no secrets
    Safe,
    /// File contains secrets but is properly ignored
    Protected,
    /// File contains secrets and is NOT ignored (high risk)
    Exposed,
    /// File contains secrets, not ignored, and is tracked by git (critical risk)
    Tracked,
}

impl GitIgnoreAnalyzer {
    pub fn new(project_root: &Path) -> Result<Self, std::io::Error> {
        let project_root = project_root.canonicalize()?;
        let is_git_repo = project_root.join(".git").exists();
        
        let patterns = if is_git_repo {
            Self::parse_gitignore_files(&project_root)?
        } else {
            Self::create_default_patterns()
        };
        
        info!("Initialized GitIgnore analyzer with {} patterns for {}", 
              patterns.len(), project_root.display());
        
        Ok(Self {
            patterns,
            project_root,
            is_git_repo,
        })
    }
    
    /// Parse all relevant .gitignore files
    fn parse_gitignore_files(project_root: &Path) -> Result<Vec<GitIgnorePattern>, std::io::Error> {
        let mut patterns = Vec::new();
        
        // Global gitignore patterns for common secret files
        patterns.extend(Self::create_default_patterns());
        
        // Parse project .gitignore
        let gitignore_path = project_root.join(".gitignore");
        if gitignore_path.exists() {
            let content = fs::read_to_string(&gitignore_path)?;
            patterns.extend(Self::parse_gitignore_content(&content, project_root)?);
            info!("Parsed {} patterns from .gitignore", patterns.len());
        }
        
        // TODO: Parse global gitignore (~/.gitignore_global)
        // TODO: Parse .git/info/exclude
        
        Ok(patterns)
    }
    
    /// Create default patterns for common secret files
    fn create_default_patterns() -> Vec<GitIgnorePattern> {
        let default_patterns = [
            ".env",
            ".env.local",
            ".env.*.local",
            ".env.production",
            ".env.development", 
            ".env.staging",
            ".env.test",
            "*.pem",
            "*.key",
            "*.p12",
            "*.pfx",
            "id_rsa",
            "id_dsa",
            "id_ecdsa",
            "id_ed25519",
            ".aws/credentials",
            ".ssh/",
            "secrets/",
            "private/",
        ];
        
        default_patterns.iter()
            .filter_map(|pattern| Self::parse_pattern(pattern, &PathBuf::from(".")).ok())
            .collect()
    }
    
    /// Parse gitignore content into patterns
    fn parse_gitignore_content(content: &str, _root: &Path) -> Result<Vec<GitIgnorePattern>, std::io::Error> {
        let mut patterns = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            match Self::parse_pattern(line, &PathBuf::from(".")) {
                Ok(pattern) => patterns.push(pattern),
                Err(e) => {
                    warn!("Failed to parse gitignore pattern on line {}: '{}' - {}", line_num + 1, line, e);
                }
            }
        }
        
        Ok(patterns)
    }
    
    /// Parse a single gitignore pattern
    fn parse_pattern(pattern: &str, _root: &Path) -> Result<GitIgnorePattern, regex::Error> {
        let original = pattern.to_string();
        let mut pattern = pattern.to_string();
        
        // Handle negation
        let is_negation = pattern.starts_with('!');
        if is_negation {
            pattern = pattern[1..].to_string();
        }
        
        // Handle directory-only patterns
        let is_directory_only = pattern.ends_with('/');
        if is_directory_only {
            pattern.pop();
        }
        
        // Handle absolute patterns (starting with /)
        let is_absolute = pattern.starts_with('/');
        if is_absolute {
            pattern = pattern[1..].to_string();
        }
        
        // Determine pattern type
        let pattern_type = if pattern.contains('/') {
            PatternType::Path
        } else if pattern.contains('*') || pattern.contains('?') {
            PatternType::Wildcard
        } else if is_directory_only {
            PatternType::Directory
        } else {
            PatternType::Exact
        };
        
        // Convert to regex
        let regex_pattern = Self::gitignore_to_regex(&pattern, is_absolute, &pattern_type)?;
        let regex = Regex::new(&regex_pattern)?;
        
        Ok(GitIgnorePattern {
            original,
            regex,
            is_negation,
            is_directory_only,
            is_absolute,
            pattern_type,
        })
    }
    
    /// Convert gitignore pattern to regex
    fn gitignore_to_regex(pattern: &str, is_absolute: bool, pattern_type: &PatternType) -> Result<String, regex::Error> {
        let mut regex = String::new();
        
        // Start anchor
        if is_absolute {
            regex.push_str("^");
        } else {
            // Can match anywhere in the path
            regex.push_str("(?:^|/)");
        }
        
        // Process the pattern
        for ch in pattern.chars() {
            match ch {
                '*' => {
                    // Check if this is a double star (**)
                    if pattern.contains("**") {
                        regex.push_str(".*");
                    } else {
                        regex.push_str("[^/]*");
                    }
                }
                '?' => regex.push_str("[^/]"),
                '.' => regex.push_str("\\."),
                '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '+' | '|' | '\\' => {
                    regex.push('\\');
                    regex.push(ch);
                }
                '/' => regex.push_str("/"),
                _ => regex.push(ch),
            }
        }
        
        // Handle directory-only patterns
        match pattern_type {
            PatternType::Directory => {
                regex.push_str("(?:/|$)");
            }
            PatternType::Exact => {
                regex.push_str("(?:/|$)");
            }
            _ => {
                regex.push_str("(?:/.*)?$");
            }
        }
        
        Ok(regex)
    }
    
    /// Check if a file path matches gitignore patterns
    pub fn analyze_file(&self, file_path: &Path) -> GitIgnoreStatus {
        let relative_path = match file_path.strip_prefix(&self.project_root) {
            Ok(rel) => rel,
            Err(_) => return GitIgnoreStatus {
                is_ignored: false,
                matched_pattern: None,
                is_tracked: false,
                should_be_ignored: false,
                risk_level: GitIgnoreRisk::Safe,
            },
        };
        
        let path_str = relative_path.to_string_lossy();
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        // Check against patterns
        let mut is_ignored = false;
        let mut matched_pattern = None;
        
        for pattern in &self.patterns {
            if pattern.regex.is_match(&path_str) {
                if pattern.is_negation {
                    is_ignored = false;
                    matched_pattern = None;
                } else {
                    is_ignored = true;
                    matched_pattern = Some(pattern.original.clone());
                }
            }
        }
        
        // Check if file is tracked by git
        let is_tracked = if self.is_git_repo {
            self.check_git_tracked(file_path)
        } else {
            false
        };
        
        // Determine if file should be ignored (contains secrets)
        let should_be_ignored = self.should_file_be_ignored(file_path, file_name);
        
        // Assess risk level
        let risk_level = self.assess_risk(is_ignored, is_tracked, should_be_ignored);
        
        GitIgnoreStatus {
            is_ignored,
            matched_pattern,
            is_tracked,
            should_be_ignored,
            risk_level,
        }
    }
    
    /// Check if file is tracked by git
    fn check_git_tracked(&self, file_path: &Path) -> bool {
        use std::process::Command;
        
        Command::new("git")
            .args(&["ls-files", "--error-unmatch"])
            .arg(file_path)
            .current_dir(&self.project_root)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    /// Check if a file should be ignored based on its name/path
    fn should_file_be_ignored(&self, file_path: &Path, file_name: &str) -> bool {
        // Common secret file patterns
        let secret_indicators = [
            ".env", ".key", ".pem", ".p12", ".pfx", 
            "id_rsa", "id_dsa", "id_ecdsa", "id_ed25519",
            "credentials", "secrets", "private"
        ];
        
        let path_str = file_path.to_string_lossy().to_lowercase();
        let file_name_lower = file_name.to_lowercase();
        
        secret_indicators.iter().any(|indicator| {
            file_name_lower.contains(indicator) || path_str.contains(indicator)
        })
    }
    
    /// Assess the risk level for a file
    fn assess_risk(&self, is_ignored: bool, is_tracked: bool, should_be_ignored: bool) -> GitIgnoreRisk {
        match (should_be_ignored, is_ignored, is_tracked) {
            // File contains secrets
            (true, true, _) => GitIgnoreRisk::Protected,      // Ignored (good)
            (true, false, true) => GitIgnoreRisk::Tracked,    // Not ignored AND tracked (critical)
            (true, false, false) => GitIgnoreRisk::Exposed,   // Not ignored but not tracked (high risk)
            // File doesn't contain secrets (or we think it doesn't)
            (false, _, _) => GitIgnoreRisk::Safe,
        }
    }
    
    /// Get all files that should be analyzed for secrets
    pub fn get_files_to_analyze(&self, extensions: &[&str]) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut files = Vec::new();
        self.collect_files_recursive(&self.project_root, extensions, &mut files)?;
        
        // Filter files that are definitely ignored
        let files_to_analyze: Vec<PathBuf> = files.into_iter()
            .filter(|file| {
                let status = self.analyze_file(file);
                // Analyze files that are either:
                // 1. Not ignored (need to check if they should be)
                // 2. Ignored but we want to verify they don't contain secrets anyway
                !status.is_ignored || status.should_be_ignored
            })
            .collect();
        
        info!("Found {} files to analyze for secrets", files_to_analyze.len());
        Ok(files_to_analyze)
    }
    
    /// Recursively collect files with given extensions
    fn collect_files_recursive(
        &self, 
        dir: &Path, 
        extensions: &[&str], 
        files: &mut Vec<PathBuf>
    ) -> Result<(), std::io::Error> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Skip obviously ignored directories
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if matches!(dir_name, ".git" | "node_modules" | "target" | "build" | "dist" | ".next") {
                        continue;
                    }
                }
                
                // Check if directory is ignored
                let status = self.analyze_file(&path);
                if !status.is_ignored {
                    self.collect_files_recursive(&path, extensions, files)?;
                }
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.is_empty() || extensions.contains(&ext) {
                    files.push(path);
                }
            } else {
                // Files without extensions might still be secret files
                files.push(path);
            }
        }
        
        Ok(())
    }
    
    /// Generate recommendations for improving gitignore coverage
    pub fn generate_gitignore_recommendations(&self, secret_files: &[PathBuf]) -> Vec<String> {
        let mut recommendations = Vec::new();
        let mut patterns_to_add = HashSet::new();
        
        for file in secret_files {
            let status = self.analyze_file(file);
            
            if status.risk_level == GitIgnoreRisk::Exposed || status.risk_level == GitIgnoreRisk::Tracked {
                if let Some(file_name) = file.file_name().and_then(|n| n.to_str()) {
                    // Suggest specific patterns
                    if file_name.starts_with(".env") {
                        patterns_to_add.insert(".env*".to_string());
                    } else if file_name.ends_with(".key") || file_name.ends_with(".pem") {
                        patterns_to_add.insert("*.key".to_string());
                        patterns_to_add.insert("*.pem".to_string());
                    } else {
                        patterns_to_add.insert(file_name.to_string());
                    }
                }
                
                if status.risk_level == GitIgnoreRisk::Tracked {
                    recommendations.push(format!(
                        "CRITICAL: '{}' contains secrets and is tracked by git! Remove from git history.",
                        file.display()
                    ));
                }
            }
        }
        
        if !patterns_to_add.is_empty() {
            recommendations.push("Add these patterns to your .gitignore:".to_string());
            for pattern in patterns_to_add {
                recommendations.push(format!("  {}", pattern));
            }
        }
        
        recommendations
    }
}

impl GitIgnoreStatus {
    /// Get a human-readable description of the status
    pub fn description(&self) -> String {
        match self.risk_level {
            GitIgnoreRisk::Safe => "File appears safe".to_string(),
            GitIgnoreRisk::Protected => format!(
                "File contains secrets but is protected (ignored by: {})", 
                self.matched_pattern.as_deref().unwrap_or("default pattern")
            ),
            GitIgnoreRisk::Exposed => "File contains secrets but is NOT in .gitignore!".to_string(),
            GitIgnoreRisk::Tracked => "CRITICAL: File contains secrets and is tracked by git!".to_string(),
        }
    }
    
    /// Get recommended action for this file
    pub fn recommended_action(&self) -> String {
        match self.risk_level {
            GitIgnoreRisk::Safe => "No action needed".to_string(),
            GitIgnoreRisk::Protected => "Verify secrets are still necessary".to_string(),
            GitIgnoreRisk::Exposed => "Add to .gitignore immediately".to_string(),
            GitIgnoreRisk::Tracked => "Remove from git history and add to .gitignore".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_gitignore_pattern_parsing() {
        let patterns = vec![
            ".env",
            "*.log",
            "/config.json",
            "secrets/",
            "!important.env",
        ];
        
        for pattern_str in patterns {
            let pattern = GitIgnoreAnalyzer::parse_pattern(pattern_str, &PathBuf::from("."));
            assert!(pattern.is_ok(), "Failed to parse pattern: {}", pattern_str);
        }
    }
    
    #[test]
    fn test_pattern_matching() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = GitIgnoreAnalyzer::new(temp_dir.path()).unwrap();
        
        // Test exact pattern matching
        let env_pattern = GitIgnoreAnalyzer::parse_pattern(".env", &PathBuf::from(".")).unwrap();
        assert!(env_pattern.regex.is_match(".env"));
        assert!(env_pattern.regex.is_match("subdir/.env"));
        assert!(!env_pattern.regex.is_match("not-env"));
    }
    
    #[test]
    fn test_nested_directory_matching() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = GitIgnoreAnalyzer::new(temp_dir.path()).unwrap();
        
        // Create a pattern for .env files
        let env_pattern = GitIgnoreAnalyzer::parse_pattern(".env*", &PathBuf::from(".")).unwrap();
        
        // Test various nested scenarios
        let test_paths = [
            ".env",
            "secrets/.env",
            "config/production/.env.local",
            "deeply/nested/folder/.env.production",
        ];
        
        for path in &test_paths {
            assert!(env_pattern.regex.is_match(path), "Pattern should match: {}", path);
        }
    }
} 