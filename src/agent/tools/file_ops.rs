//! File operation tools for reading, writing, and exploring the project using Rig's Tool trait
//!
//! Provides tools for:
//! - Reading files (ReadFileTool)
//! - Writing single files (WriteFileTool) - for Dockerfiles, terraform files, etc.
//! - Writing multiple files (WriteFilesTool) - for Terraform modules, Helm charts
//! - Listing directories (ListDirectoryTool)
//!
//! File write operations include interactive diff confirmation before applying changes.
//!
//! ## Truncation Limits
//!
//! Tool outputs are truncated to prevent context overflow:
//! - File reads: Max 2000 lines (use start_line/end_line for specific sections)
//! - Directory listings: Max 500 entries
//! - Long lines: Truncated at 2000 characters

use super::error::{ErrorCategory, format_error_for_llm};
use super::response::{
    format_cancelled, format_file_content, format_file_content_range, format_list,
};
use super::truncation::{TruncationLimits, truncate_dir_listing, truncate_file_content};
use crate::agent::ide::IdeClient;
use crate::agent::ui::confirmation::ConfirmationResult;
use crate::agent::ui::diff::{confirm_file_write, confirm_file_write_with_ide};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

// ============================================================================
// Read File Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ReadFileArgs {
    pub path: String,
    pub start_line: Option<u64>,
    pub end_line: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
#[error("Read file error: {0}")]
pub struct ReadFileError(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileTool {
    project_path: PathBuf,
}

impl ReadFileTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    /// Check if file content appears to be binary (contains null bytes in first 1KB)
    fn is_likely_binary(content: &[u8]) -> bool {
        let check_len = content.len().min(1024);
        content[..check_len].contains(&0)
    }

    /// Check if a symlink target is within the project boundary
    fn validate_symlink_target(&self, path: &PathBuf) -> Result<PathBuf, String> {
        let canonical_project = self.project_path.canonicalize().map_err(|e| {
            format_error_for_llm(
                "read_file",
                ErrorCategory::InternalError,
                &format!("Invalid project path: {}", e),
                Some(vec!["This is an internal configuration error"]),
            )
        })?;

        // Read the symlink target and resolve it
        let target = fs::read_link(path).map_err(|e| {
            format_error_for_llm(
                "read_file",
                ErrorCategory::FileNotFound,
                &format!("Cannot read symlink '{}': {}", path.display(), e),
                Some(vec!["The symlink may be broken or inaccessible"]),
            )
        })?;

        // Resolve the target path (make it absolute if relative)
        let resolved = if target.is_absolute() {
            target.clone()
        } else {
            path.parent().unwrap_or(path).join(&target)
        };

        // Canonicalize the resolved target
        let canonical_target = match resolved.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                let hint1 = format!(
                    "Symlink '{}' points to '{}'",
                    path.display(),
                    target.display()
                );
                let hint2 = format!("Error: {}", e);
                return Err(format_error_for_llm(
                    "read_file",
                    ErrorCategory::FileNotFound,
                    &format!("Symlink target does not exist: {}", resolved.display()),
                    Some(vec![&hint1, &hint2]),
                ));
            }
        };

        // Verify the target is within project boundary
        if !canonical_target.starts_with(&canonical_project) {
            let hint_symlink = format!("Symlink: {}", path.display());
            let hint_target = format!("Target: {}", target.display());
            let hint_project = format!("Project root: {}", self.project_path.display());
            return Err(format_error_for_llm(
                "read_file",
                ErrorCategory::PathOutsideBoundary,
                &format!(
                    "Symlink target '{}' is outside project boundary",
                    target.display()
                ),
                Some(vec![
                    "The symlink points to a location outside the project directory",
                    &hint_symlink,
                    &hint_target,
                    &hint_project,
                ]),
            ));
        }

        Ok(canonical_target)
    }

    /// Validates a path is within the project boundary.
    /// Returns Ok(Some(path)) if valid, Ok(None) with formatted error string if invalid.
    fn validate_path(&self, requested: &PathBuf) -> Result<PathBuf, String> {
        let canonical_project = self.project_path.canonicalize().map_err(|e| {
            format_error_for_llm(
                "read_file",
                ErrorCategory::InternalError,
                &format!("Invalid project path: {}", e),
                Some(vec!["This is an internal configuration error"]),
            )
        })?;

        let target = if requested.is_absolute() {
            requested.clone()
        } else {
            self.project_path.join(requested)
        };

        let canonical_target = target.canonicalize().map_err(|e| {
            let kind = e.kind();
            match kind {
                std::io::ErrorKind::NotFound => format_error_for_llm(
                    "read_file",
                    ErrorCategory::FileNotFound,
                    &format!("File not found: {}", requested.display()),
                    Some(vec![
                        "Check if the file path is spelled correctly",
                        "Use list_directory to explore available files",
                        &format!("Project root: {}", self.project_path.display()),
                    ]),
                ),
                std::io::ErrorKind::PermissionDenied => format_error_for_llm(
                    "read_file",
                    ErrorCategory::PermissionDenied,
                    &format!("Permission denied: {}", requested.display()),
                    Some(vec![
                        "The file exists but cannot be read due to permissions",
                    ]),
                ),
                _ => format_error_for_llm(
                    "read_file",
                    ErrorCategory::FileNotFound,
                    &format!("Cannot access file '{}': {}", requested.display(), e),
                    Some(vec!["Verify the path exists and is accessible"]),
                ),
            }
        })?;

        if !canonical_target.starts_with(&canonical_project) {
            return Err(format_error_for_llm(
                "read_file",
                ErrorCategory::PathOutsideBoundary,
                &format!("Path '{}' is outside project boundary", requested.display()),
                Some(vec![
                    "Paths must be within the project directory",
                    "Use relative paths from project root",
                    &format!("Project root: {}", self.project_path.display()),
                ]),
            ));
        }

        Ok(canonical_target)
    }
}

impl Tool for ReadFileTool {
    const NAME: &'static str = "read_file";

    type Error = ReadFileError;
    type Args = ReadFileArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Read the contents of a file in the project.

**Truncation Limits:**
- Maximum 2000 lines returned by default
- Lines longer than 2000 characters are truncated
- Use start_line/end_line to read specific sections of large files

**Path Restrictions:**
- Paths must be within the project directory (security boundary)
- Both relative and absolute paths are supported
- Relative paths are resolved from project root

**Line Range Usage:**
- start_line: 1-based line number to start reading from
- end_line: 1-based line number to stop at (inclusive)
- If only start_line is provided, reads from that line to end of file
- If start_line exceeds file length, returns an error with file size info"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read (relative to project root or absolute within project)"
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "Starting line number (1-based). Use with end_line to read specific sections of large files."
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "Ending line number (1-based, inclusive). If omitted with start_line, reads to end of file."
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let requested_path = PathBuf::from(&args.path);
        let file_path = match self.validate_path(&requested_path) {
            Ok(path) => path,
            Err(error_msg) => return Ok(error_msg), // Return formatted error as success for LLM
        };

        // Check if file is a symlink and validate target is within project
        let symlink_metadata = fs::symlink_metadata(&file_path)
            .map_err(|e| ReadFileError(format!("Cannot access file: {}", e)))?;

        if symlink_metadata.file_type().is_symlink() {
            // Validate symlink target is within project boundary
            if let Err(error_msg) = self.validate_symlink_target(&file_path) {
                return Ok(error_msg);
            }
        }

        let metadata = fs::metadata(&file_path)
            .map_err(|e| ReadFileError(format!("Cannot read file: {}", e)))?;

        // Handle empty files gracefully
        if metadata.len() == 0 {
            return Ok(format_file_content(&args.path, "(empty file)", 0, 0, false));
        }

        const MAX_SIZE: u64 = 1024 * 1024;
        if metadata.len() > MAX_SIZE {
            return Ok(format_error_for_llm(
                "read_file",
                ErrorCategory::ValidationFailed,
                &format!(
                    "File too large ({} bytes). Maximum size is {} bytes.",
                    metadata.len(),
                    MAX_SIZE
                ),
                Some(vec![
                    "Use start_line/end_line to read specific sections",
                    "Consider if you need the entire file",
                ]),
            ));
        }

        // Read as bytes first to check for binary content
        let raw_content = fs::read(&file_path)
            .map_err(|e| ReadFileError(format!("Failed to read file: {}", e)))?;

        // Check for binary content
        if Self::is_likely_binary(&raw_content) {
            return Ok(format_error_for_llm(
                "read_file",
                ErrorCategory::ValidationFailed,
                &format!(
                    "File '{}' appears to be binary (contains null bytes)",
                    args.path
                ),
                Some(vec![
                    "This tool is designed for text files only",
                    "Binary files cannot be displayed as text",
                    "Consider using a hex viewer or specialized tool for binary files",
                ]),
            ));
        }

        // Convert to string (now safe since we checked for binary)
        let content = String::from_utf8_lossy(&raw_content).into_owned();

        // Use response utilities for consistent formatting
        if let Some(start) = args.start_line {
            // User requested specific line range - respect it exactly
            let lines: Vec<&str> = content.lines().collect();
            let start_idx = (start as usize).saturating_sub(1);
            let end_idx = args
                .end_line
                .map(|e| (e as usize).min(lines.len()))
                .unwrap_or(lines.len());

            if start_idx >= lines.len() {
                return Ok(format_error_for_llm(
                    "read_file",
                    ErrorCategory::ValidationFailed,
                    &format!(
                        "Start line {} exceeds file length ({} lines)",
                        start,
                        lines.len()
                    ),
                    Some(vec![
                        &format!("File has {} lines total", lines.len()),
                        "Use start_line within valid range",
                    ]),
                ));
            }

            // Ensure end_idx >= start_idx to avoid slice panic when end_line < start_line
            let end_idx = end_idx.max(start_idx);

            let selected: Vec<String> = lines[start_idx..end_idx]
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>4} | {}", start_idx + i + 1, line))
                .collect();

            Ok(format_file_content_range(
                &args.path,
                &selected.join("\n"),
                start as usize,
                end_idx,
                lines.len(),
            ))
        } else {
            // Full file read - apply truncation to prevent context overflow
            let limits = TruncationLimits::default();
            let truncated = truncate_file_content(&content, &limits);

            Ok(format_file_content(
                &args.path,
                &truncated.content,
                truncated.total_lines,
                truncated.returned_lines,
                truncated.was_truncated,
            ))
        }
    }
}

// ============================================================================
// List Directory Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListDirectoryArgs {
    pub path: Option<String>,
    pub recursive: Option<bool>,
}

#[derive(Debug, thiserror::Error)]
#[error("List directory error: {0}")]
pub struct ListDirectoryError(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDirectoryTool {
    project_path: PathBuf,
}

impl ListDirectoryTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    /// Validates a path is within the project boundary.
    /// Returns Ok(path) if valid, Err(formatted_error_string) if invalid.
    fn validate_path(&self, requested: &PathBuf) -> Result<PathBuf, String> {
        let canonical_project = self.project_path.canonicalize().map_err(|e| {
            format_error_for_llm(
                "list_directory",
                ErrorCategory::InternalError,
                &format!("Invalid project path: {}", e),
                Some(vec!["This is an internal configuration error"]),
            )
        })?;

        let target = if requested.is_absolute() {
            requested.clone()
        } else {
            self.project_path.join(requested)
        };

        let canonical_target = target.canonicalize().map_err(|e| {
            let kind = e.kind();
            match kind {
                std::io::ErrorKind::NotFound => format_error_for_llm(
                    "list_directory",
                    ErrorCategory::FileNotFound,
                    &format!("Directory not found: {}", requested.display()),
                    Some(vec![
                        "Check if the directory path is spelled correctly",
                        "Use '.' to list the project root",
                        &format!("Project root: {}", self.project_path.display()),
                    ]),
                ),
                std::io::ErrorKind::PermissionDenied => format_error_for_llm(
                    "list_directory",
                    ErrorCategory::PermissionDenied,
                    &format!("Permission denied: {}", requested.display()),
                    Some(vec![
                        "The directory exists but cannot be read due to permissions",
                    ]),
                ),
                _ => format_error_for_llm(
                    "list_directory",
                    ErrorCategory::FileNotFound,
                    &format!("Cannot access directory '{}': {}", requested.display(), e),
                    Some(vec!["Verify the path exists and is accessible"]),
                ),
            }
        })?;

        if !canonical_target.starts_with(&canonical_project) {
            return Err(format_error_for_llm(
                "list_directory",
                ErrorCategory::PathOutsideBoundary,
                &format!("Path '{}' is outside project boundary", requested.display()),
                Some(vec![
                    "Paths must be within the project directory",
                    "Use '.' for project root",
                    &format!("Project root: {}", self.project_path.display()),
                ]),
            ));
        }

        Ok(canonical_target)
    }

    fn list_entries(
        &self,
        base_path: &PathBuf,
        current_path: &PathBuf,
        recursive: bool,
        depth: usize,
        max_depth: usize,
        entries: &mut Vec<serde_json::Value>,
    ) -> Result<(), ListDirectoryError> {
        let skip_dirs = [
            "node_modules",
            ".git",
            "target",
            "__pycache__",
            ".venv",
            "venv",
            "dist",
            "build",
        ];

        let dir_name = current_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if depth > 0 && skip_dirs.contains(&dir_name) {
            return Ok(());
        }

        let read_dir = fs::read_dir(current_path)
            .map_err(|e| ListDirectoryError(format!("Cannot read directory: {}", e)))?;

        for entry in read_dir {
            let entry =
                entry.map_err(|e| ListDirectoryError(format!("Error reading entry: {}", e)))?;
            let path = entry.path();
            let metadata = entry.metadata().ok();

            let relative_path = path
                .strip_prefix(base_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

            entries.push(json!({
                "name": entry.file_name().to_string_lossy(),
                "path": relative_path,
                "type": if is_dir { "directory" } else { "file" },
                "size": if is_dir { None::<u64> } else { Some(size) }
            }));

            if recursive && is_dir && depth < max_depth {
                self.list_entries(base_path, &path, recursive, depth + 1, max_depth, entries)?;
            }
        }

        Ok(())
    }
}

impl Tool for ListDirectoryTool {
    const NAME: &'static str = "list_directory";

    type Error = ListDirectoryError;
    type Args = ListDirectoryArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"List the contents of a directory in the project.

**Truncation Limits:**
- Maximum 500 entries returned
- Use more specific paths to explore large directories

**Output Format:**
- Returns entries sorted alphabetically by name
- Each entry includes: name, path, type (file/directory), size (for files)

**Filtering:**
- Automatically skips common non-essential directories: node_modules, .git, target, __pycache__, .venv, venv, dist, build
- Respects .gitignore patterns in recursive mode

**Path Restrictions:**
- Paths must be within the project directory (security boundary)
- Use '.' or empty path for project root"#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the directory (relative to project root). Use '.' or omit for project root."
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "If true, list contents recursively (max depth 3, skips node_modules/.git/etc). Default: false."
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path_str = args.path.as_deref().unwrap_or(".");

        let requested_path = if path_str.is_empty() || path_str == "." {
            self.project_path.clone()
        } else {
            PathBuf::from(path_str)
        };

        let dir_path = match self.validate_path(&requested_path) {
            Ok(path) => path,
            Err(error_msg) => return Ok(error_msg), // Return formatted error as success for LLM
        };
        let recursive = args.recursive.unwrap_or(false);

        let mut entries = Vec::new();
        self.list_entries(&dir_path, &dir_path, recursive, 0, 3, &mut entries)?;

        // Apply truncation to prevent context overflow
        let limits = TruncationLimits::default();
        let truncated = truncate_dir_listing(entries, limits.max_dir_entries);

        // Use response utilities for consistent formatting
        Ok(format_list(
            path_str,
            &truncated.entries,
            truncated.total_entries,
            truncated.was_truncated,
        ))
    }
}

// ============================================================================
// Write File Tool - For writing Dockerfiles, Terraform files, Helm values, etc.
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WriteFileArgs {
    /// Path to the file to write (relative to project root)
    pub path: String,
    /// Content to write to the file
    pub content: String,
    /// If true, create parent directories if they don't exist (default: true)
    pub create_dirs: Option<bool>,
}

#[derive(Debug, thiserror::Error)]
#[error("Write file error: {0}")]
pub struct WriteFileError(String);

/// Session-level tracking of always-allowed file patterns
#[derive(Debug)]
pub struct AllowedFilePatterns {
    patterns: Mutex<HashSet<String>>,
}

impl AllowedFilePatterns {
    pub fn new() -> Self {
        Self {
            patterns: Mutex::new(HashSet::new()),
        }
    }

    /// Check if a file pattern is already allowed
    pub fn is_allowed(&self, filename: &str) -> bool {
        let patterns = self.patterns.lock().unwrap();
        patterns.contains(filename)
    }

    /// Add a file pattern to the allowed list
    pub fn allow(&self, pattern: String) {
        let mut patterns = self.patterns.lock().unwrap();
        patterns.insert(pattern);
    }
}

impl Default for AllowedFilePatterns {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct WriteFileTool {
    project_path: PathBuf,
    /// Whether to require confirmation before writing
    require_confirmation: bool,
    /// Session-level allowed file patterns
    allowed_patterns: std::sync::Arc<AllowedFilePatterns>,
    /// Optional IDE client for native diff viewing
    ide_client: Option<std::sync::Arc<tokio::sync::Mutex<IdeClient>>>,
}

impl WriteFileTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            require_confirmation: true,
            allowed_patterns: std::sync::Arc::new(AllowedFilePatterns::new()),
            ide_client: None,
        }
    }

    /// Create with shared allowed patterns state (for session persistence)
    pub fn with_allowed_patterns(
        project_path: PathBuf,
        allowed_patterns: std::sync::Arc<AllowedFilePatterns>,
    ) -> Self {
        Self {
            project_path,
            require_confirmation: true,
            allowed_patterns,
            ide_client: None,
        }
    }

    /// Set IDE client for native diff viewing
    pub fn with_ide_client(
        mut self,
        ide_client: std::sync::Arc<tokio::sync::Mutex<IdeClient>>,
    ) -> Self {
        self.ide_client = Some(ide_client);
        self
    }

    /// Disable confirmation prompts
    pub fn without_confirmation(mut self) -> Self {
        self.require_confirmation = false;
        self
    }

    /// Validates a path is within the project boundary for writing.
    /// Returns Ok(path) if valid, Err(formatted_error_string) if invalid.
    fn validate_path(&self, requested: &PathBuf) -> Result<PathBuf, String> {
        let canonical_project = self.project_path.canonicalize().map_err(|e| {
            format_error_for_llm(
                "write_file",
                ErrorCategory::InternalError,
                &format!("Invalid project path: {}", e),
                Some(vec!["This is an internal configuration error"]),
            )
        })?;

        let target = if requested.is_absolute() {
            requested.clone()
        } else {
            self.project_path.join(requested)
        };

        // For new files, we can't canonicalize yet, so check the parent
        let parent = target.parent().ok_or_else(|| {
            format_error_for_llm(
                "write_file",
                ErrorCategory::ValidationFailed,
                &format!(
                    "Invalid path '{}': no parent directory",
                    requested.display()
                ),
                Some(vec![
                    "Provide a valid file path with at least a filename",
                    "Example: 'tmp/output.txt' or 'results/analysis.md'",
                ]),
            )
        })?;

        // If parent exists, canonicalize it; otherwise check the path prefix
        let is_within_project = if parent.exists() {
            let canonical_parent = parent.canonicalize().map_err(|e| {
                let kind = e.kind();
                match kind {
                    std::io::ErrorKind::PermissionDenied => format_error_for_llm(
                        "write_file",
                        ErrorCategory::PermissionDenied,
                        &format!(
                            "Permission denied accessing parent directory: {}",
                            parent.display()
                        ),
                        Some(vec!["The parent directory exists but cannot be accessed"]),
                    ),
                    _ => format_error_for_llm(
                        "write_file",
                        ErrorCategory::ValidationFailed,
                        &format!("Invalid parent path '{}': {}", parent.display(), e),
                        Some(vec!["Verify the parent directory path is valid"]),
                    ),
                }
            })?;
            canonical_parent.starts_with(&canonical_project)
        } else {
            // For nested new directories, check if the normalized path stays within project
            let normalized = self.project_path.join(requested);
            !normalized
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        };

        if !is_within_project {
            return Err(format_error_for_llm(
                "write_file",
                ErrorCategory::PathOutsideBoundary,
                &format!("Path '{}' is outside project boundary", requested.display()),
                Some(vec![
                    "SECURITY: Writes are restricted to the project directory",
                    "For temporary files, create a 'tmp/' directory in project root",
                    "Use a project-relative path like 'tmp/output.txt'",
                    &format!("Project root: {}", self.project_path.display()),
                ]),
            ));
        }

        Ok(target)
    }
}

impl Tool for WriteFileTool {
    const NAME: &'static str = "write_file";

    type Error = WriteFileError;
    type Args = WriteFileArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Write content to a file in the project. Creates the file if it doesn't exist, or overwrites if it does.

**SECURITY: Path Restriction (Intentional)**
- Writes are ONLY allowed within the project directory
- Writing to /tmp, /etc, or any path outside the project is blocked
- This is a security feature to prevent unintended system modifications
- For temporary files, create a 'tmp/' directory within your project root

**Confirmation Workflow:**
- All writes show a diff preview before applying
- User can approve, reject, or request modifications
- Use 'Always' option to skip confirmation for repeated file types

**IMPORTANT**: Use this tool IMMEDIATELY when the user asks you to:
- Create ANY file (Dockerfile, .tf, .yaml, .md, .json, etc.)
- Generate configuration files
- Write documentation to a specific location
- Save analysis results or findings

**DO NOT** just describe what you would write - actually call this tool with the content.

Use cases:
- Generate Dockerfiles for applications
- Create Terraform configuration files (.tf)
- Write Helm chart templates and values
- Create docker-compose.yml files
- Generate CI/CD configuration files
- Write Kubernetes manifests
- Save analysis findings to markdown files

The tool will create parent directories automatically if they don't exist."#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file (relative to project root). Must be within project. Examples: 'Dockerfile', 'terraform/main.tf', 'tmp/scratch.txt'"
                    },
                    "content": {
                        "type": "string",
                        "description": "The complete content to write to the file"
                    },
                    "create_dirs": {
                        "type": "boolean",
                        "description": "If true (default), create parent directories if they don't exist"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let requested_path = PathBuf::from(&args.path);
        let file_path = match self.validate_path(&requested_path) {
            Ok(path) => path,
            Err(error_msg) => return Ok(error_msg), // Return formatted error as success for LLM
        };

        // Read existing content for diff (if file exists)
        let old_content = if file_path.exists() {
            fs::read_to_string(&file_path).ok()
        } else {
            None
        };

        // Get filename for pattern matching
        let filename = std::path::Path::new(&args.path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| args.path.clone());

        // Check if confirmation is needed
        let needs_confirmation =
            self.require_confirmation && !self.allowed_patterns.is_allowed(&filename);

        if needs_confirmation {
            // Get IDE client reference if available
            let ide_client_guard = if let Some(ref client) = self.ide_client {
                Some(client.lock().await)
            } else {
                None
            };
            let ide_client_ref = ide_client_guard.as_deref();

            // Show diff with IDE integration if available
            let confirmation = confirm_file_write_with_ide(
                &args.path,
                old_content.as_deref(),
                &args.content,
                ide_client_ref,
            )
            .await;

            match confirmation {
                ConfirmationResult::Proceed => {
                    // Continue with write
                }
                ConfirmationResult::ProceedAlways(pattern) => {
                    // Remember this file pattern for the session
                    self.allowed_patterns.allow(pattern);
                }
                ConfirmationResult::Modify(feedback) => {
                    // Return feedback to the agent using response utility
                    return Ok(format_cancelled(
                        &args.path,
                        "User requested changes",
                        Some(&feedback),
                    ));
                }
                ConfirmationResult::Cancel => {
                    // User cancelled using response utility
                    return Ok(format_cancelled(
                        &args.path,
                        "User cancelled the operation",
                        None,
                    ));
                }
            }
        } else {
            // Auto-accept mode: show the diff without requiring confirmation
            use crate::agent::ui::diff::{render_diff, render_new_file};
            use colored::Colorize;

            if let Some(old) = &old_content {
                render_diff(old, &args.content, &args.path);
            } else {
                render_new_file(&args.content, &args.path);
            }
            println!("  {} Auto-accepted", "✓".green());
        }

        // Create parent directories if needed
        let create_dirs = args.create_dirs.unwrap_or(true);
        if create_dirs
            && let Some(parent) = file_path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)
                .map_err(|e| WriteFileError(format!("Failed to create directories: {}", e)))?;
        }

        // Check if file exists (for reporting)
        let file_existed = file_path.exists();

        // Write the content
        fs::write(&file_path, &args.content)
            .map_err(|e| WriteFileError(format!("Failed to write file: {}", e)))?;

        let action = if file_existed { "Updated" } else { "Created" };
        let lines = args.content.lines().count();

        let result = json!({
            "success": true,
            "action": action,
            "path": args.path,
            "lines_written": lines,
            "bytes_written": args.content.len()
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| WriteFileError(format!("Failed to serialize: {}", e)))
    }
}

// ============================================================================
// Write Files Tool - For writing multiple files (Terraform modules, Helm charts)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct FileToWrite {
    /// Path to the file (relative to project root)
    pub path: String,
    /// Content to write
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct WriteFilesArgs {
    /// List of files to write
    pub files: Vec<FileToWrite>,
    /// If true, create parent directories if they don't exist (default: true)
    pub create_dirs: Option<bool>,
}

#[derive(Debug, thiserror::Error)]
#[error("Write files error: {0}")]
pub struct WriteFilesError(String);

#[derive(Debug, Clone)]
pub struct WriteFilesTool {
    project_path: PathBuf,
    /// Whether to require confirmation before writing
    require_confirmation: bool,
    /// Session-level allowed file patterns
    allowed_patterns: std::sync::Arc<AllowedFilePatterns>,
    /// Optional IDE client for native diff views
    ide_client: Option<std::sync::Arc<tokio::sync::Mutex<IdeClient>>>,
}

impl WriteFilesTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            require_confirmation: true,
            allowed_patterns: std::sync::Arc::new(AllowedFilePatterns::new()),
            ide_client: None,
        }
    }

    /// Create with shared allowed patterns state
    pub fn with_allowed_patterns(
        project_path: PathBuf,
        allowed_patterns: std::sync::Arc<AllowedFilePatterns>,
    ) -> Self {
        Self {
            project_path,
            require_confirmation: true,
            allowed_patterns,
            ide_client: None,
        }
    }

    /// Disable confirmation prompts
    pub fn without_confirmation(mut self) -> Self {
        self.require_confirmation = false;
        self
    }

    /// Set the IDE client for native diff views
    pub fn with_ide_client(
        mut self,
        ide_client: std::sync::Arc<tokio::sync::Mutex<IdeClient>>,
    ) -> Self {
        self.ide_client = Some(ide_client);
        self
    }

    /// Validates a path is within the project boundary for writing.
    /// Returns Ok(path) if valid, Err(formatted_error_string) if invalid.
    fn validate_path(&self, requested: &PathBuf) -> Result<PathBuf, String> {
        let canonical_project = self.project_path.canonicalize().map_err(|e| {
            format_error_for_llm(
                "write_files",
                ErrorCategory::InternalError,
                &format!("Invalid project path: {}", e),
                Some(vec!["This is an internal configuration error"]),
            )
        })?;

        let target = if requested.is_absolute() {
            requested.clone()
        } else {
            self.project_path.join(requested)
        };

        let parent = target.parent().ok_or_else(|| {
            format_error_for_llm(
                "write_files",
                ErrorCategory::ValidationFailed,
                &format!(
                    "Invalid path '{}': no parent directory",
                    requested.display()
                ),
                Some(vec![
                    "Provide a valid file path with at least a filename",
                    "Example: 'tmp/output.txt' or 'results/analysis.md'",
                ]),
            )
        })?;

        let is_within_project = if parent.exists() {
            let canonical_parent = parent.canonicalize().map_err(|e| {
                let kind = e.kind();
                match kind {
                    std::io::ErrorKind::PermissionDenied => format_error_for_llm(
                        "write_files",
                        ErrorCategory::PermissionDenied,
                        &format!(
                            "Permission denied accessing parent directory: {}",
                            parent.display()
                        ),
                        Some(vec!["The parent directory exists but cannot be accessed"]),
                    ),
                    _ => format_error_for_llm(
                        "write_files",
                        ErrorCategory::ValidationFailed,
                        &format!("Invalid parent path '{}': {}", parent.display(), e),
                        Some(vec!["Verify the parent directory path is valid"]),
                    ),
                }
            })?;
            canonical_parent.starts_with(&canonical_project)
        } else {
            let normalized = self.project_path.join(requested);
            !normalized
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        };

        if !is_within_project {
            return Err(format_error_for_llm(
                "write_files",
                ErrorCategory::PathOutsideBoundary,
                &format!("Path '{}' is outside project boundary", requested.display()),
                Some(vec![
                    "SECURITY: Writes are restricted to the project directory",
                    "For temporary files, create a 'tmp/' directory in project root",
                    "Use project-relative paths like 'tmp/output.txt'",
                    &format!("Project root: {}", self.project_path.display()),
                ]),
            ));
        }

        Ok(target)
    }
}

impl Tool for WriteFilesTool {
    const NAME: &'static str = "write_files";

    type Error = WriteFilesError;
    type Args = WriteFilesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Write multiple files at once. Ideal for creating complete infrastructure configurations.

**SECURITY: Path Restriction (Intentional)**
- ALL paths must be within the project directory
- Writing to /tmp, /etc, or any path outside the project is blocked
- This is a security feature to prevent unintended system modifications
- For temporary files, create a 'tmp/' directory within your project root

**Atomicity:**
- All paths are validated BEFORE any files are written
- If any path is invalid, NO files are written
- Confirmation is requested for each file individually

**USE THIS TOOL** (not just describe files) when the user asks for:
- Complete Terraform modules (main.tf, variables.tf, outputs.tf, providers.tf)
- Full Helm charts (Chart.yaml, values.yaml, templates/*.yaml)
- Kubernetes manifests (deployment.yaml, service.yaml, configmap.yaml)
- Multi-file docker-compose setups
- Any set of related files

**DO NOT** just describe the files - actually call this tool to create them.

Parent directories are created automatically."#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "files": {
                        "type": "array",
                        "description": "List of files to write. All paths must be within project directory.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Path to the file (relative to project root). Must be within project."
                                },
                                "content": {
                                    "type": "string",
                                    "description": "Content to write to the file"
                                }
                            },
                            "required": ["path", "content"]
                        }
                    },
                    "create_dirs": {
                        "type": "boolean",
                        "description": "If true (default), create parent directories if they don't exist"
                    }
                },
                "required": ["files"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let create_dirs = args.create_dirs.unwrap_or(true);
        let mut results = Vec::new();
        let mut total_bytes = 0usize;
        let mut total_lines = 0usize;

        // Pre-validate ALL paths before writing ANY files (atomicity)
        let mut validated_paths: Vec<(PathBuf, &FileToWrite)> = Vec::new();
        let mut invalid_paths: Vec<String> = Vec::new();

        for file in &args.files {
            let requested_path = PathBuf::from(&file.path);
            match self.validate_path(&requested_path) {
                Ok(path) => validated_paths.push((path, file)),
                Err(_) => invalid_paths.push(file.path.clone()),
            }
        }

        // If any paths are invalid, return error listing all invalid paths
        if !invalid_paths.is_empty() {
            let invalid_list = invalid_paths.join(", ");
            return Ok(format_error_for_llm(
                "write_files",
                ErrorCategory::PathOutsideBoundary,
                &format!("Invalid paths detected: {}", invalid_list),
                Some(vec![
                    "SECURITY: All paths must be within the project directory",
                    "None of the files were written due to invalid paths",
                    "For temporary files, create a 'tmp/' directory in project root",
                    &format!("Project root: {}", self.project_path.display()),
                ]),
            ));
        }

        // Now process all validated files
        for (file_path, file) in validated_paths {
            // Read existing content for diff
            let old_content = if file_path.exists() {
                fs::read_to_string(&file_path).ok()
            } else {
                None
            };

            // Get filename for pattern matching
            let filename = std::path::Path::new(&file.path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| file.path.clone());

            // Check if confirmation is needed
            let needs_confirmation =
                self.require_confirmation && !self.allowed_patterns.is_allowed(&filename);

            if needs_confirmation {
                // Use IDE diff if client is connected, otherwise terminal diff
                let confirmation = if let Some(ref client) = self.ide_client {
                    let guard = client.lock().await;
                    if guard.is_connected() {
                        confirm_file_write_with_ide(
                            &file.path,
                            old_content.as_deref(),
                            &file.content,
                            Some(&*guard),
                        )
                        .await
                    } else {
                        drop(guard);
                        confirm_file_write(&file.path, old_content.as_deref(), &file.content)
                    }
                } else {
                    confirm_file_write(&file.path, old_content.as_deref(), &file.content)
                };

                match confirmation {
                    ConfirmationResult::Proceed => {
                        // Continue with this file
                    }
                    ConfirmationResult::ProceedAlways(pattern) => {
                        self.allowed_patterns.allow(pattern);
                    }
                    ConfirmationResult::Modify(feedback) => {
                        // User provided feedback - stop ALL remaining files immediately
                        return Ok(format_cancelled(
                            &file.path,
                            "User requested changes",
                            Some(&feedback),
                        ));
                    }
                    ConfirmationResult::Cancel => {
                        // User cancelled - stop ALL remaining files immediately
                        return Ok(format_cancelled(
                            &file.path,
                            "User cancelled the operation",
                            None,
                        ));
                    }
                }
            } else {
                // Auto-accept mode: show the diff without requiring confirmation
                use crate::agent::ui::diff::{render_diff, render_new_file};
                use colored::Colorize;

                if let Some(old) = &old_content {
                    render_diff(old, &file.content, &file.path);
                } else {
                    render_new_file(&file.content, &file.path);
                }
                println!("  {} Auto-accepted", "✓".green());
            }

            // Create parent directories if needed
            if create_dirs
                && let Some(parent) = file_path.parent()
                && !parent.exists()
            {
                fs::create_dir_all(parent).map_err(|e| {
                    WriteFilesError(format!(
                        "Failed to create directories for {}: {}",
                        file.path, e
                    ))
                })?;
            }

            let file_existed = file_path.exists();

            fs::write(&file_path, &file.content)
                .map_err(|e| WriteFilesError(format!("Failed to write {}: {}", file.path, e)))?;

            let lines = file.content.lines().count();
            total_bytes += file.content.len();
            total_lines += lines;

            results.push(json!({
                "path": file.path,
                "action": if file_existed { "updated" } else { "created" },
                "lines": lines,
                "bytes": file.content.len()
            }));
        }

        // If we get here, all files were written successfully
        // (cancellations return early with immediate stop message)
        let result = json!({
            "success": true,
            "files_written": results.len(),
            "total_lines": total_lines,
            "total_bytes": total_bytes,
            "files": results
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| WriteFilesError(format!("Failed to serialize: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // =========================================================================
    // ReadFileTool tests
    // =========================================================================

    #[test]
    fn test_is_likely_binary_text() {
        // Pure ASCII text should not be detected as binary
        let text = b"fn main() {\n    println!(\"Hello, world!\");\n}\n";
        assert!(!ReadFileTool::is_likely_binary(text));
    }

    #[test]
    fn test_is_likely_binary_with_null() {
        // Content with null byte should be detected as binary
        let binary = b"some text\x00more text";
        assert!(ReadFileTool::is_likely_binary(binary));
    }

    #[test]
    fn test_is_likely_binary_empty() {
        // Empty content should not be detected as binary
        let empty: &[u8] = b"";
        assert!(!ReadFileTool::is_likely_binary(empty));
    }

    #[test]
    fn test_is_likely_binary_utf8() {
        // UTF-8 content should not be detected as binary
        let utf8 = "日本語テキスト".as_bytes();
        assert!(!ReadFileTool::is_likely_binary(utf8));
    }

    #[tokio::test]
    async fn test_read_file_within_project() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Hello, world!").unwrap();

        let tool = ReadFileTool::new(dir.path().to_path_buf());
        let args = ReadFileArgs {
            path: "test.txt".to_string(),
            start_line: None,
            end_line: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["file"], "test.txt");
        assert!(
            parsed["content"]
                .as_str()
                .unwrap()
                .contains("Hello, world!")
        );
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let dir = tempdir().unwrap();
        let tool = ReadFileTool::new(dir.path().to_path_buf());
        let args = ReadFileArgs {
            path: "nonexistent.txt".to_string(),
            start_line: None,
            end_line: None,
        };

        let result = tool.call(args).await.unwrap();
        // Should return error formatted for LLM
        assert!(
            result.contains("error")
                || result.contains("not found")
                || result.contains("does not exist")
        );
    }

    // =========================================================================
    // ListDirectoryTool tests
    // =========================================================================

    #[tokio::test]
    async fn test_list_directory_basic() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file1.txt"), "content").unwrap();
        fs::write(dir.path().join("file2.txt"), "content").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let tool = ListDirectoryTool::new(dir.path().to_path_buf());
        let args = ListDirectoryArgs {
            path: Some(".".to_string()),
            recursive: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert!(parsed["entries"].as_array().unwrap().len() >= 2);
    }
}
