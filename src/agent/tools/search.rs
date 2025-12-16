//! Search tools for agentic code exploration
//!
//! Provides grep-like code search and file finding capabilities.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;
use regex::Regex;

// ============================================================================
// Search Code Tool (grep-like)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SearchCodeArgs {
    /// Search pattern (regex or literal string)
    pub pattern: String,
    /// Optional path to search within (relative to project root)
    pub path: Option<String>,
    /// File extension filter (e.g., "rs", "ts", "py")
    pub extension: Option<String>,
    /// Whether to treat pattern as regex (default: false = literal)
    pub regex: Option<bool>,
    /// Case insensitive search (default: true)
    pub case_insensitive: Option<bool>,
    /// Maximum number of results (default: 50)
    pub max_results: Option<usize>,
}

#[derive(Debug, thiserror::Error)]
#[error("Search error: {0}")]
pub struct SearchCodeError(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCodeTool {
    project_path: PathBuf,
}

impl SearchCodeTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    fn should_skip_dir(name: &str) -> bool {
        matches!(
            name,
            "node_modules"
                | ".git"
                | "target"
                | "__pycache__"
                | ".venv"
                | "dist"
                | "build"
                | ".next"
                | ".nuxt"
                | "vendor"
                | ".cache"
                | "coverage"
        )
    }

    fn is_text_file(path: &PathBuf) -> bool {
        let text_extensions = [
            "rs", "go", "js", "ts", "jsx", "tsx", "py", "java", "kt", "scala",
            "rb", "php", "cs", "cpp", "c", "h", "hpp", "swift", "dart", "elm",
            "clj", "hs", "ml", "r", "sh", "bash", "zsh", "ps1", "bat", "cmd",
            "json", "yaml", "yml", "toml", "xml", "html", "css", "scss", "sass",
            "less", "md", "txt", "sql", "graphql", "prisma", "env", "dockerfile",
            "makefile", "cmake", "gradle", "sbt", "ex", "exs", "erl", "hrl",
        ];

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            return text_extensions.contains(&ext.to_lowercase().as_str());
        }

        // Check for extensionless files like Dockerfile, Makefile
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            return matches!(lower.as_str(), "dockerfile" | "makefile" | "rakefile" | "gemfile" | "procfile" | "justfile");
        }

        false
    }
}

#[derive(Debug, Serialize)]
struct SearchMatch {
    file: String,
    line_number: usize,
    line: String,
    context_before: Vec<String>,
    context_after: Vec<String>,
}

impl Tool for SearchCodeTool {
    const NAME: &'static str = "search_code";

    type Error = SearchCodeError;
    type Args = SearchCodeArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search for code patterns, function names, variables, or any text across the codebase. Returns matching lines with context. Use this to find where something is defined, used, or imported.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern - can be a function name, variable, string literal, or regex pattern"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional subdirectory to search within (e.g., 'src', 'backend/api')"
                    },
                    "extension": {
                        "type": "string",
                        "description": "Filter by file extension (e.g., 'rs', 'ts', 'py'). Omit for all file types."
                    },
                    "regex": {
                        "type": "boolean",
                        "description": "Treat pattern as regex. Default: false (literal string match)"
                    },
                    "case_insensitive": {
                        "type": "boolean",
                        "description": "Case insensitive search. Default: true"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum results to return. Default: 50"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let search_root = if let Some(ref subpath) = args.path {
            self.project_path.join(subpath)
        } else {
            self.project_path.clone()
        };

        if !search_root.exists() {
            return Err(SearchCodeError(format!(
                "Path does not exist: {}",
                args.path.unwrap_or_default()
            )));
        }

        let case_insensitive = args.case_insensitive.unwrap_or(true);
        let is_regex = args.regex.unwrap_or(false);
        let max_results = args.max_results.unwrap_or(50);

        // Build the search pattern
        let pattern_str = if is_regex {
            if case_insensitive {
                format!("(?i){}", args.pattern)
            } else {
                args.pattern.clone()
            }
        } else {
            let escaped = regex::escape(&args.pattern);
            if case_insensitive {
                format!("(?i){}", escaped)
            } else {
                escaped
            }
        };

        let regex = Regex::new(&pattern_str)
            .map_err(|e| SearchCodeError(format!("Invalid pattern: {}", e)))?;

        let mut matches: Vec<SearchMatch> = Vec::new();

        for entry in WalkDir::new(&search_root)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    if let Some(name) = e.file_name().to_str() {
                        return !Self::should_skip_dir(name);
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            if matches.len() >= max_results {
                break;
            }

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Extension filter
            if let Some(ref ext_filter) = args.extension {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.to_lowercase() != ext_filter.to_lowercase() {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Only search text files
            let path_buf = path.to_path_buf();
            if !Self::is_text_file(&path_buf) {
                continue;
            }

            // Read and search file
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue, // Skip binary/unreadable files
            };

            let lines: Vec<&str> = content.lines().collect();
            for (line_idx, line) in lines.iter().enumerate() {
                if matches.len() >= max_results {
                    break;
                }

                if regex.is_match(line) {
                    let relative_path = path
                        .strip_prefix(&self.project_path)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();

                    // Get 1 line of context before/after
                    let context_before = if line_idx > 0 {
                        vec![lines[line_idx - 1].to_string()]
                    } else {
                        vec![]
                    };

                    let context_after = if line_idx + 1 < lines.len() {
                        vec![lines[line_idx + 1].to_string()]
                    } else {
                        vec![]
                    };

                    matches.push(SearchMatch {
                        file: relative_path,
                        line_number: line_idx + 1,
                        line: line.to_string(),
                        context_before,
                        context_after,
                    });
                }
            }
        }

        let result = json!({
            "pattern": args.pattern,
            "total_matches": matches.len(),
            "matches": matches,
            "truncated": matches.len() >= max_results
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| SearchCodeError(format!("Serialization error: {}", e)))
    }
}

// ============================================================================
// Find Files Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct FindFilesArgs {
    /// File name pattern (supports * and ? wildcards)
    pub pattern: String,
    /// Optional subdirectory to search in
    pub path: Option<String>,
    /// File extension filter
    pub extension: Option<String>,
    /// Include directories in results (default: false)
    pub include_dirs: Option<bool>,
    /// Maximum results (default: 100)
    pub max_results: Option<usize>,
}

#[derive(Debug, thiserror::Error)]
#[error("Find files error: {0}")]
pub struct FindFilesError(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindFilesTool {
    project_path: PathBuf,
}

impl FindFilesTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    fn matches_pattern(name: &str, pattern: &str) -> bool {
        let pattern_lower = pattern.to_lowercase();
        let name_lower = name.to_lowercase();

        // Handle simple wildcards
        if pattern == "*" {
            return true;
        }

        // Convert simple wildcards to regex-like matching
        if pattern.contains('*') || pattern.contains('?') {
            let regex_pattern = pattern_lower
                .replace('.', r"\.")
                .replace('*', ".*")
                .replace('?', ".");
            
            if let Ok(re) = Regex::new(&format!("^{}$", regex_pattern)) {
                return re.is_match(&name_lower);
            }
        }

        // Plain substring match
        name_lower.contains(&pattern_lower)
    }
}

#[derive(Debug, Serialize)]
struct FileInfo {
    name: String,
    path: String,
    file_type: String,
    size: Option<u64>,
    extension: Option<String>,
}

impl Tool for FindFilesTool {
    const NAME: &'static str = "find_files";

    type Error = FindFilesError;
    type Args = FindFilesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Find files by name pattern. Use wildcards (* for any characters, ? for single character). Great for locating config files, finding all files of a type, or discovering project structure.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "File name pattern with optional wildcards. Examples: 'package.json', '*.config.ts', 'Dockerfile*', 'api*.rs'"
                    },
                    "path": {
                        "type": "string",
                        "description": "Subdirectory to search in (e.g., 'src', 'backend')"
                    },
                    "extension": {
                        "type": "string",
                        "description": "Filter by extension (e.g., 'ts', 'rs', 'yaml')"
                    },
                    "include_dirs": {
                        "type": "boolean",
                        "description": "Include directories in results. Default: false"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum results. Default: 100"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let search_root = if let Some(ref subpath) = args.path {
            self.project_path.join(subpath)
        } else {
            self.project_path.clone()
        };

        if !search_root.exists() {
            return Err(FindFilesError(format!(
                "Path does not exist: {}",
                args.path.unwrap_or_default()
            )));
        }

        let include_dirs = args.include_dirs.unwrap_or(false);
        let max_results = args.max_results.unwrap_or(100);
        let skip_dirs = [
            "node_modules", ".git", "target", "__pycache__", ".venv", 
            "dist", "build", ".next", ".nuxt", "vendor", ".cache", "coverage"
        ];

        let mut results: Vec<FileInfo> = Vec::new();

        for entry in WalkDir::new(&search_root)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    if let Some(name) = e.file_name().to_str() {
                        return !skip_dirs.contains(&name);
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            if results.len() >= max_results {
                break;
            }

            let path = entry.path();
            let is_dir = path.is_dir();

            // Skip dirs if not requested
            if is_dir && !include_dirs {
                continue;
            }

            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };

            // Extension filter
            if let Some(ref ext_filter) = args.extension {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.to_lowercase() != ext_filter.to_lowercase() {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Pattern matching
            if !Self::matches_pattern(file_name, &args.pattern) {
                continue;
            }

            let relative_path = path
                .strip_prefix(&self.project_path)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            let metadata = path.metadata().ok();
            let size = if is_dir { None } else { metadata.as_ref().map(|m| m.len()) };

            results.push(FileInfo {
                name: file_name.to_string(),
                path: relative_path,
                file_type: if is_dir { "directory".to_string() } else { "file".to_string() },
                size,
                extension: path.extension().and_then(|e| e.to_str()).map(|s| s.to_string()),
            });
        }

        let result = json!({
            "pattern": args.pattern,
            "total_found": results.len(),
            "files": results,
            "truncated": results.len() >= max_results
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| FindFilesError(format!("Serialization error: {}", e)))
    }
}
