//! # File Discovery Module
//! 
//! Ultra-fast file discovery with git-aware filtering and smart prioritization.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use std::time::SystemTime;

use ahash::AHashSet;
use rayon::prelude::*;
use walkdir::WalkDir;
use log::{debug, trace};

use super::{ScanMode, SecurityError};

/// File metadata for efficient filtering
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: usize,
    pub extension: Option<String>,
    pub is_gitignored: bool,
    pub modified: SystemTime,
    pub priority_hints: PriorityHints,
}

/// Priority hints for file scoring
#[derive(Debug, Clone, Default)]
pub struct PriorityHints {
    pub is_env_file: bool,
    pub is_config_file: bool,
    pub is_secret_file: bool,
    pub is_source_file: bool,
    pub has_secret_keywords: bool,
}

/// Configuration for file discovery
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub use_git: bool,
    pub max_file_size: usize,
    pub priority_extensions: Vec<String>,
    pub scan_mode: ScanMode,
}

/// High-performance file discovery
pub struct FileDiscovery {
    config: DiscoveryConfig,
    ignored_dirs: AHashSet<String>,
    secret_keywords: Vec<&'static str>,
    binary_extensions: AHashSet<&'static str>,
    excluded_filenames: AHashSet<&'static str>,
    asset_extensions: AHashSet<&'static str>,
}

impl FileDiscovery {
    pub fn new(config: DiscoveryConfig) -> Self {
        let ignored_dirs = Self::get_ignored_dirs(&config.scan_mode);
        let secret_keywords = Self::get_secret_keywords();
        let binary_extensions = Self::get_binary_extensions();
        let excluded_filenames = Self::get_excluded_filenames();
        let asset_extensions = Self::get_asset_extensions();
        
        Self {
            config,
            ignored_dirs,
            secret_keywords,
            binary_extensions,
            excluded_filenames,
            asset_extensions,
        }
    }
    
    /// Discover files with ultra-fast git-aware filtering
    pub fn discover_files(&self, project_root: &Path) -> Result<Vec<FileMetadata>, SecurityError> {
        let is_git_repo = project_root.join(".git").exists();
        
        if is_git_repo && self.config.use_git {
            self.git_aware_discovery(project_root)
        } else {
            self.filesystem_discovery(project_root)
        }
    }
    
    /// Git-aware file discovery (fastest method)
    fn git_aware_discovery(&self, project_root: &Path) -> Result<Vec<FileMetadata>, SecurityError> {
        debug!("Using git-aware file discovery");
        
        // Get all tracked files using git ls-files
        let tracked_files = self.get_git_tracked_files(project_root)?;
        
        // Get untracked files that might contain secrets
        let untracked_files = self.get_untracked_secret_files(project_root)?;
        
        // Combine and process in parallel
        let all_paths: Vec<PathBuf> = tracked_files.into_iter()
            .chain(untracked_files)
            .collect();
        
        // Process files in parallel to build metadata
        let files: Vec<FileMetadata> = all_paths
            .par_iter()
            .filter_map(|path| self.build_file_metadata(path, project_root).ok())
            .filter(|meta| self.should_include_file(meta))
            .collect();
        
        Ok(files)
    }
    
    /// Get tracked files from git
    fn get_git_tracked_files(&self, project_root: &Path) -> Result<Vec<PathBuf>, SecurityError> {
        let output = Command::new("git")
            .args(&["ls-files", "-z"]) // -z for null-terminated output
            .current_dir(project_root)
            .output()
            .map_err(|e| SecurityError::FileDiscovery(format!("Git ls-files failed: {}", e)))?;
        
        if !output.status.success() {
            return Err(SecurityError::FileDiscovery("Git ls-files failed".to_string()));
        }
        
        // Parse null-terminated paths
        let paths: Vec<PathBuf> = output.stdout
            .split(|&b| b == 0)
            .filter(|path| !path.is_empty())
            .filter_map(|path| std::str::from_utf8(path).ok())
            .map(|path| project_root.join(path))
            .collect();
        
        Ok(paths)
    }
    
    /// Get untracked files that might contain secrets
    fn get_untracked_secret_files(&self, project_root: &Path) -> Result<Vec<PathBuf>, SecurityError> {
        // Common secret file patterns that might not be tracked
        let secret_patterns = vec![
            ".env*",
            "*.key",
            "*.pem",
            "*.p12",
            "*credentials*",
            "*secret*",
            "config/*.json",
            "config/*.yml",
        ];
        
        let mut untracked_files = Vec::new();
        
        for pattern in secret_patterns {
            let output = Command::new("git")
                .args(&["ls-files", "--others", "--exclude-standard", pattern])
                .current_dir(project_root)
                .output();
            
            if let Ok(output) = output {
                if output.status.success() {
                    let paths: Vec<PathBuf> = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .map(|line| project_root.join(line))
                        .collect();
                    untracked_files.extend(paths);
                }
            }
        }
        
        Ok(untracked_files)
    }
    
    /// Fallback filesystem discovery
    fn filesystem_discovery(&self, project_root: &Path) -> Result<Vec<FileMetadata>, SecurityError> {
        debug!("Using filesystem discovery");
        
        let walker = WalkDir::new(project_root)
            .follow_links(false)
            .max_depth(20)
            .into_iter()
            .filter_entry(|entry| {
                // Skip ignored directories
                if entry.file_type().is_dir() {
                    let dir_name = entry.file_name().to_string_lossy();
                    return !self.ignored_dirs.contains(dir_name.as_ref());
                }
                true
            });
        
        let files: Vec<FileMetadata> = walker
            .par_bridge()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| self.build_file_metadata(entry.path(), project_root).ok())
            .filter(|meta| self.should_include_file(meta))
            .collect();
        
        Ok(files)
    }
    
    /// Build file metadata with priority hints
    fn build_file_metadata(&self, path: &Path, project_root: &Path) -> Result<FileMetadata, std::io::Error> {
        let metadata = fs::metadata(path)?;
        let size = metadata.len() as usize;
        let modified = metadata.modified()?;
        
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase());
        
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        let file_name_lower = file_name.to_lowercase();
        
        // Check gitignore status efficiently
        let is_gitignored = if project_root.join(".git").exists() {
            self.check_gitignore_batch(path, project_root)
        } else {
            false
        };
        
        // Build priority hints
        let priority_hints = PriorityHints {
            is_env_file: file_name_lower.starts_with(".env") || file_name_lower.ends_with(".env"),
            is_config_file: self.is_config_file(&file_name_lower, &extension),
            is_secret_file: self.is_secret_file(&file_name_lower, path),
            is_source_file: self.is_source_file(&extension),
            has_secret_keywords: self.has_secret_keywords(&file_name_lower),
        };
        
        Ok(FileMetadata {
            path: path.to_path_buf(),
            size,
            extension,
            is_gitignored,
            modified,
            priority_hints,
        })
    }
    
    /// Batch check gitignore status
    fn check_gitignore_batch(&self, path: &Path, project_root: &Path) -> bool {
        // Quick check using git check-ignore
        let output = Command::new("git")
            .args(&["check-ignore", path.to_str().unwrap_or("")])
            .current_dir(project_root)
            .output();
        
        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
    
    /// Check if file should be included based on filters
    fn should_include_file(&self, meta: &FileMetadata) -> bool {
        // Size filter
        if meta.size > self.config.max_file_size {
            trace!("Skipping large file: {} ({} bytes)", meta.path.display(), meta.size);
            return false;
        }
        
        // Enhanced binary file detection
        if self.is_binary_file(meta) {
            trace!("Skipping binary file: {}", meta.path.display());
            return false;
        }
        
        // Asset file detection (images, fonts, media)
        if self.is_asset_file(meta) {
            trace!("Skipping asset file: {}", meta.path.display());
            return false;
        }
        
        // Exclude files that are unlikely to contain real secrets
        if self.should_exclude_from_security_scan(meta) {
            trace!("Excluding from security scan: {}", meta.path.display());
            return false;
        }
        
        // Critical files always included
        if meta.is_critical() {
            return true;
        }
        
        // Scan mode specific filtering
        match self.config.scan_mode {
            ScanMode::Lightning => {
                // Only critical files (already handled above)
                false
            }
            ScanMode::Fast => {
                // Priority files or small source files
                meta.is_priority() || (meta.priority_hints.is_source_file && meta.size < 50_000)
            }
            _ => true, // Include all for other modes
        }
    }
    
    /// Enhanced binary file detection
    fn is_binary_file(&self, meta: &FileMetadata) -> bool {
        if let Some(ext) = &meta.extension {
            if self.binary_extensions.contains(ext.as_str()) {
                return true;
            }
        }
        
        // Check filename patterns
        let filename = meta.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        if self.excluded_filenames.contains(filename.as_str()) {
            return true;
        }
        
        false
    }
    
    /// Check if file is an asset (images, fonts, media)
    fn is_asset_file(&self, meta: &FileMetadata) -> bool {
        if let Some(ext) = &meta.extension {
            if self.asset_extensions.contains(ext.as_str()) {
                return true;
            }
        }
        
        // Check for asset directories
        let path_str = meta.path.to_string_lossy().to_lowercase();
        let asset_dirs = [
            "/assets/", "/static/", "/public/", "/images/", "/img/", 
            "/media/", "/fonts/", "/icons/", "/graphics/", "/pictures/"
        ];
        
        asset_dirs.iter().any(|&dir| path_str.contains(dir))
    }
    
    /// Check if file should be excluded from security scanning
    fn should_exclude_from_security_scan(&self, meta: &FileMetadata) -> bool {
        let path_str = meta.path.to_string_lossy().to_lowercase();
        
        // DEPENDENCY LOCK FILES - These contain package hashes/metadata, not secrets
        if self.is_dependency_lock_file(meta) {
            return true;
        }
        
        // SVG files often contain base64 encoded graphics that trigger false positives
        if meta.extension.as_deref() == Some("svg") {
            return true;
        }
        
        // Minified and bundled files
        if self.is_minified_or_bundled_file(meta) {
            return true;
        }
        
        // Documentation and non-code files that rarely contain real secrets
        let exclude_patterns = [
            ".md", ".txt", ".rst", ".adoc", ".asciidoc",
            "readme", "changelog", "license", "todo",
            "roadmap", "contributing", "authors",
            // Test files (often contain fake/example data)
            "/test/", "/tests/", "/spec/", "/specs/",
            "__test__", "__spec__", ".test.", ".spec.",
            "_test.", "_spec.", "fixtures", "mocks", "examples",
            // Documentation directories
            "/docs/", "/doc/", "/documentation/",
            // Framework/library detection files (they contain patterns but not secrets)
            "frameworks/", "detector", "rules", "patterns",
            // Build artifacts and generated files
            "target/", "build/", "dist/", ".next/", "coverage/",
            ".nuxt/", ".output/", ".vercel/", ".netlify/",
            // IDE and editor files
            ".vscode/", ".idea/", ".vs/", "*.swp", "*.swo",
            // OS files
            ".ds_store", "thumbs.db", "desktop.ini",
        ];
        
        // Check patterns
        if exclude_patterns.iter().any(|&pattern| path_str.contains(pattern)) {
            return true;
        }
        
        // Documentation file extensions
        if let Some(ext) = &meta.extension {
            let doc_extensions = ["md", "txt", "rst", "adoc", "asciidoc", "rtf"];
            if doc_extensions.contains(&ext.as_str()) {
                return true;
            }
        }
        
        // Check if filename suggests it's documentation, examples, or code generation
        let filename = meta.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        let doc_filenames = [
            "readme", "changelog", "license", "authors", "contributing",
            "roadmap", "todo", "examples", "demo", "sample", "fixture",
            // Code generation and API example files
            "apicodedialog", "codedialog", "codeexample", "apiexample",
            "codesnippet", "snippets", "templates", "codegenerator",
            "apitool", "playground", "sandbox",
        ];
        
        if doc_filenames.iter().any(|&name| filename.contains(name)) {
            return true;
        }
        
        false
    }
    
    /// Check if file is minified or bundled
    fn is_minified_or_bundled_file(&self, meta: &FileMetadata) -> bool {
        let filename = meta.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Minified file patterns
        let minified_patterns = [
            ".min.", ".bundle.", ".chunk.", ".vendor.",
            "-min.", "-bundle.", "-chunk.", "-vendor.",
            "_min.", "_bundle.", "_chunk.", "_vendor.",
        ];
        
        minified_patterns.iter().any(|&pattern| filename.contains(pattern))
    }
    
    /// Get ignored directories based on scan mode
    fn get_ignored_dirs(scan_mode: &ScanMode) -> AHashSet<String> {
        let mut dirs = AHashSet::new();
        
        // Always ignore these
        let always_ignore = vec![
            ".git", "node_modules", "target", "build", "dist", ".next",
            "coverage", "__pycache__", ".pytest_cache", ".mypy_cache",
            "vendor", "packages", ".bundle", "bower_components",
            ".nuxt", ".output", ".vercel", ".netlify", ".vscode", ".idea",
        ];
        
        for dir in always_ignore {
            dirs.insert(dir.to_string());
        }
        
        // Additional ignores for faster modes
        if matches!(scan_mode, ScanMode::Lightning | ScanMode::Fast) {
            let fast_ignore = vec!["test", "tests", "spec", "specs", "docs", "documentation"];
            for dir in fast_ignore {
                dirs.insert(dir.to_string());
            }
        }
        
        dirs
    }
    
    /// Get comprehensive binary file extensions
    fn get_binary_extensions() -> AHashSet<&'static str> {
        let mut extensions = AHashSet::new();
        
        // Executables and libraries
        let binary_exts = [
            "exe", "dll", "so", "dylib", "lib", "a", "o", "obj",
            "bin", "com", "scr", "msi", "deb", "rpm", "pkg",
            // Archives
            "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "ace",
            "cab", "dmg", "iso", "img",
            // Media files
            "mp3", "mp4", "avi", "mov", "wmv", "flv", "mkv", "webm",
            "wav", "flac", "ogg", "aac", "m4a", "wma",
            // Images (will be handled separately as assets)
            "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tga", "webp",
            "ico", "cur", "psd", "ai", "eps", "raw", "cr2", "nef",
            // Fonts
            "ttf", "otf", "woff", "woff2", "eot",
            // Documents
            "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
            "odt", "ods", "odp", "rtf",
            // Databases
            "db", "sqlite", "sqlite3", "mdb", "accdb",
            // Other binary formats
            "pyc", "pyo", "class", "jar", "war", "ear",
        ];
        
        for ext in binary_exts {
            extensions.insert(ext);
        }
        
        extensions
    }
    
    /// Get asset file extensions (images, media, fonts)
    fn get_asset_extensions() -> AHashSet<&'static str> {
        let mut extensions = AHashSet::new();
        
        let asset_exts = [
            // Images
            "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tga", "webp",
            "ico", "cur", "psd", "ai", "eps", "raw", "cr2", "nef", "svg",
            // Fonts
            "ttf", "otf", "woff", "woff2", "eot",
            // Media
            "mp3", "mp4", "avi", "mov", "wmv", "flv", "mkv", "webm",
            "wav", "flac", "ogg", "aac", "m4a", "wma",
        ];
        
        for ext in asset_exts {
            extensions.insert(ext);
        }
        
        extensions
    }
    
    /// Get filenames that should be excluded
    fn get_excluded_filenames() -> AHashSet<&'static str> {
        let mut filenames = AHashSet::new();
        
        let excluded = [
            // OS files
            ".ds_store", "thumbs.db", "desktop.ini", "folder.ico",
            // Editor files
            ".gitkeep", ".keep", ".placeholder",
            // Temporary files
            ".tmp", ".temp", ".swp", ".swo", ".bak", ".backup",
        ];
        
        for filename in excluded {
            filenames.insert(filename);
        }
        
        filenames
    }
    
    /// Get secret keywords for detection
    fn get_secret_keywords() -> Vec<&'static str> {
        vec![
            "secret", "key", "token", "password", "credential",
            "auth", "api", "private", "access", "bearer",
        ]
    }
    
    fn is_config_file(&self, name: &str, extension: &Option<String>) -> bool {
        let config_extensions = ["json", "yml", "yaml", "toml", "ini", "conf", "config", "xml"];
        let config_names = ["config", "settings", "configuration", ".env"];
        
        if let Some(ext) = extension {
            if config_extensions.contains(&ext.as_str()) {
                return true;
            }
        }
        
        config_names.iter().any(|&n| name.contains(n))
    }
    
    fn is_secret_file(&self, name: &str, path: &Path) -> bool {
        let secret_patterns = [
            ".env", ".key", ".pem", ".p12", ".pfx",
            "credentials", "secret", "private", "cert",
        ];
        
        // Check filename
        if secret_patterns.iter().any(|&p| name.contains(p)) {
            return true;
        }
        
        // Check path components
        let path_str = path.to_string_lossy().to_lowercase();
        secret_patterns.iter().any(|&p| path_str.contains(p))
    }
    
    fn is_source_file(&self, extension: &Option<String>) -> bool {
        if let Some(ext) = extension {
            let source_extensions = [
                "js", "jsx", "ts", "tsx", "py", "java", "kt", "go",
                "rs", "rb", "php", "cs", "cpp", "c", "h", "swift",
                "scala", "clj", "ex", "exs",
            ];
            source_extensions.contains(&ext.as_str())
        } else {
            false
        }
    }
    
    fn has_secret_keywords(&self, name: &str) -> bool {
        self.secret_keywords.iter().any(|&keyword| name.contains(keyword))
    }
    
    /// Enhanced dependency lock file detection
    fn is_dependency_lock_file(&self, meta: &FileMetadata) -> bool {
        let filename = meta.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Common dependency lock files that contain package hashes and metadata
        let lock_files = [
            // JavaScript/Node.js
            "package-lock.json",
            "yarn.lock", 
            "pnpm-lock.yaml",
            "bun.lockb",  // Bun lock file (binary format)
            // Python
            "poetry.lock",
            "pipfile.lock",
            "pip-lock.txt",
            "pdm.lock",
            // Rust
            "cargo.lock",
            // Go
            "go.sum",
            "go.mod",
            // Java
            "gradle.lockfile",
            "maven-dependency-plugin.log",
            // Ruby
            "gemfile.lock",
            // PHP
            "composer.lock",
            // .NET
            "packages.lock.json",
            "paket.lock",
            // Others
            "mix.lock",  // Elixir
            "pubspec.lock",  // Dart
            "swift.resolved", // Swift
            "flake.lock", // Nix
        ];
        
        // Check if filename matches any lock file pattern
        lock_files.iter().any(|&pattern| filename == pattern) ||
        // Also check for common lock file patterns
        filename.ends_with(".lock") ||
        filename.ends_with("-lock.json") ||
        filename.ends_with("-lock.yaml") ||
        filename.ends_with("-lock.yml") ||
        filename.ends_with(".lockb") ||  // Binary lock files
        filename.contains("shrinkwrap") ||
        filename.contains("lockfile")
    }
}

impl FileMetadata {
    /// Check if file is critical (must scan)
    pub fn is_critical(&self) -> bool {
        self.priority_hints.is_env_file || 
        self.priority_hints.is_secret_file ||
        self.extension.as_deref() == Some("pem") ||
        self.extension.as_deref() == Some("key")
    }
    
    /// Check if file is high priority
    pub fn is_priority(&self) -> bool {
        self.is_critical() ||
        self.priority_hints.is_config_file ||
        self.priority_hints.has_secret_keywords
    }
    
    /// Calculate priority score (higher = more important)
    pub fn priority_score(&self) -> u32 {
        let mut score: u32 = 0;
        
        if self.priority_hints.is_env_file { score += 1000; }
        if self.priority_hints.is_secret_file { score += 900; }
        if self.priority_hints.is_config_file { score += 500; }
        if self.priority_hints.has_secret_keywords { score += 300; }
        if !self.is_gitignored { score += 200; }
        if self.priority_hints.is_source_file { score += 100; }
        
        // Penalize large files
        if self.size > 1_000_000 { score = score.saturating_sub(100); }
        
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_file_priority_scoring() {
        let meta = FileMetadata {
            path: PathBuf::from(".env"),
            size: 100,
            extension: Some("env".to_string()),
            is_gitignored: false,
            modified: SystemTime::now(),
            priority_hints: PriorityHints {
                is_env_file: true,
                is_config_file: true,
                is_secret_file: true,
                is_source_file: false,
                has_secret_keywords: true,
            },
        };
        
        assert!(meta.is_critical());
        assert!(meta.is_priority());
        assert!(meta.priority_score() > 2000);
    }
    
    #[test]
    fn test_file_discovery() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".env"), "SECRET=123").unwrap();
        fs::write(temp_dir.path().join("config.json"), "{}").unwrap();
        fs::create_dir(temp_dir.path().join("node_modules")).unwrap();
        fs::write(temp_dir.path().join("node_modules/test.js"), "code").unwrap();
        
        let config = DiscoveryConfig {
            use_git: false,
            max_file_size: 1024 * 1024,
            priority_extensions: vec!["env".to_string()],
            scan_mode: ScanMode::Fast,
        };
        
        let discovery = FileDiscovery::new(config);
        let files = discovery.discover_files(temp_dir.path()).unwrap();
        
        // Should find .env and config.json but not node_modules/test.js
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.path.ends_with(".env")));
        assert!(files.iter().any(|f| f.path.ends_with("config.json")));
    }
    
    #[test]
    fn test_binary_file_detection() {
        let config = DiscoveryConfig {
            use_git: false,
            max_file_size: 1024 * 1024,
            priority_extensions: vec![],
            scan_mode: ScanMode::Fast,
        };
        let discovery = FileDiscovery::new(config);
        
        let binary_meta = FileMetadata {
            path: PathBuf::from("test.jpg"),
            size: 100,
            extension: Some("jpg".to_string()),
            is_gitignored: false,
            modified: SystemTime::now(),
            priority_hints: PriorityHints::default(),
        };
        
        assert!(discovery.is_binary_file(&binary_meta));
    }
    
    #[test]
    fn test_lock_file_detection() {
        let config = DiscoveryConfig {
            use_git: false,
            max_file_size: 1024 * 1024,
            priority_extensions: vec![],
            scan_mode: ScanMode::Fast,
        };
        let discovery = FileDiscovery::new(config);
        
        let lock_files = [
            "package-lock.json",
            "yarn.lock",
            "pnpm-lock.yaml",
            "bun.lockb",
            "cargo.lock",
            "go.sum",
        ];
        
        for lock_file in lock_files {
            let meta = FileMetadata {
                path: PathBuf::from(lock_file),
                size: 100,
                extension: None,
                is_gitignored: false,
                modified: SystemTime::now(),
                priority_hints: PriorityHints::default(),
            };
            
            assert!(discovery.is_dependency_lock_file(&meta), "Failed to detect {}", lock_file);
        }
    }
} 