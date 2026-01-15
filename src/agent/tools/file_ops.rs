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
use super::response::{format_cancelled, format_file_content, format_file_content_range, format_list};
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

    fn validate_path(&self, requested: &PathBuf) -> Result<PathBuf, ReadFileError> {
        let canonical_project = self
            .project_path
            .canonicalize()
            .map_err(|e| ReadFileError(format!("Invalid project path: {}", e)))?;

        let target = if requested.is_absolute() {
            requested.clone()
        } else {
            self.project_path.join(requested)
        };

        let canonical_target = target.canonicalize().map_err(|e| {
            let kind = e.kind();
            let msg = match kind {
                std::io::ErrorKind::NotFound => {
                    format!("File not found: {}", requested.display())
                }
                std::io::ErrorKind::PermissionDenied => {
                    format!("Permission denied: {}", requested.display())
                }
                _ => format!("Cannot access file '{}': {}", requested.display(), e),
            };
            ReadFileError(msg)
        })?;

        if !canonical_target.starts_with(&canonical_project) {
            return Err(ReadFileError(format!(
                "Access denied: path '{}' is outside project directory",
                requested.display()
            )));
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
            description: "Read the contents of a file in the project. Use this to examine source code, configuration files, or any text file.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read (relative to project root)"
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "Optional starting line number (1-based)"
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "Optional ending line number (1-based, inclusive)"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let requested_path = PathBuf::from(&args.path);
        let file_path = self.validate_path(&requested_path)?;

        let metadata = fs::metadata(&file_path)
            .map_err(|e| ReadFileError(format!("Cannot read file: {}", e)))?;

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

        let content = fs::read_to_string(&file_path)
            .map_err(|e| ReadFileError(format!("Failed to read file: {}", e)))?;

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

    fn validate_path(&self, requested: &PathBuf) -> Result<PathBuf, ListDirectoryError> {
        let canonical_project = self
            .project_path
            .canonicalize()
            .map_err(|e| ListDirectoryError(format!("Invalid project path: {}", e)))?;

        let target = if requested.is_absolute() {
            requested.clone()
        } else {
            self.project_path.join(requested)
        };

        let canonical_target = target.canonicalize().map_err(|e| {
            let kind = e.kind();
            let msg = match kind {
                std::io::ErrorKind::NotFound => {
                    format!("Directory not found: {}", requested.display())
                }
                std::io::ErrorKind::PermissionDenied => {
                    format!("Permission denied: {}", requested.display())
                }
                _ => format!("Cannot access directory '{}': {}", requested.display(), e),
            };
            ListDirectoryError(msg)
        })?;

        if !canonical_target.starts_with(&canonical_project) {
            return Err(ListDirectoryError(format!(
                "Access denied: path '{}' is outside project directory",
                requested.display()
            )));
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
            description: "List the contents of a directory in the project. Returns file and subdirectory names with their types and sizes.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the directory to list (relative to project root). Use '.' for root."
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "If true, list contents recursively (max depth 3). Default is false."
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

        let dir_path = self.validate_path(&requested_path)?;
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

    fn validate_path(&self, requested: &PathBuf) -> Result<PathBuf, WriteFileError> {
        let canonical_project = self
            .project_path
            .canonicalize()
            .map_err(|e| WriteFileError(format!("Invalid project path: {}", e)))?;

        let target = if requested.is_absolute() {
            requested.clone()
        } else {
            self.project_path.join(requested)
        };

        // For new files, we can't canonicalize yet, so check the parent
        let parent = target.parent().ok_or_else(|| {
            WriteFileError(format!(
                "Invalid path '{}': no parent directory",
                requested.display()
            ))
        })?;

        // If parent exists, canonicalize it; otherwise check the path prefix
        let is_within_project = if parent.exists() {
            let canonical_parent = parent.canonicalize().map_err(|e| {
                let kind = e.kind();
                let msg = match kind {
                    std::io::ErrorKind::PermissionDenied => {
                        format!("Permission denied accessing parent directory: {}", parent.display())
                    }
                    _ => format!("Invalid parent path '{}': {}", parent.display(), e),
                };
                WriteFileError(msg)
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
            return Err(WriteFileError(format!(
                "Access denied: path '{}' is outside project directory",
                requested.display()
            )));
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

**IMPORTANT**: Use this tool IMMEDIATELY when the user asks you to:
- Create ANY file (Dockerfile, .tf, .yaml, .md, .json, etc.)
- Generate configuration files
- Write documentation to a specific location
- "Put content in" or "under" a directory
- Save analysis results or findings
- Document anything in a file

**DO NOT** just describe what you would write - actually call this tool with the content.

Use cases:
- Generate Dockerfiles for applications
- Create Terraform configuration files (.tf)
- Write Helm chart templates and values
- Create docker-compose.yml files
- Generate CI/CD configuration files (.github/workflows, .gitlab-ci.yml)
- Write Kubernetes manifests
- Save analysis findings to markdown files
- Create any text file the user requests

The tool will create parent directories automatically if they don't exist."#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to write (relative to project root). Example: 'Dockerfile', 'terraform/main.tf', 'helm/values.yaml'"
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
        let file_path = self.validate_path(&requested_path)?;

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

    fn validate_path(&self, requested: &PathBuf) -> Result<PathBuf, WriteFilesError> {
        let canonical_project = self
            .project_path
            .canonicalize()
            .map_err(|e| WriteFilesError(format!("Invalid project path: {}", e)))?;

        let target = if requested.is_absolute() {
            requested.clone()
        } else {
            self.project_path.join(requested)
        };

        let parent = target.parent().ok_or_else(|| {
            WriteFilesError(format!(
                "Invalid path '{}': no parent directory",
                requested.display()
            ))
        })?;

        let is_within_project = if parent.exists() {
            let canonical_parent = parent.canonicalize().map_err(|e| {
                let kind = e.kind();
                let msg = match kind {
                    std::io::ErrorKind::PermissionDenied => {
                        format!("Permission denied accessing parent directory: {}", parent.display())
                    }
                    _ => format!("Invalid parent path '{}': {}", parent.display(), e),
                };
                WriteFilesError(msg)
            })?;
            canonical_parent.starts_with(&canonical_project)
        } else {
            let normalized = self.project_path.join(requested);
            !normalized
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        };

        if !is_within_project {
            return Err(WriteFilesError(format!(
                "Access denied: path '{}' is outside project directory",
                requested.display()
            )));
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

**IMPORTANT**: Use this tool when you need to create multiple related files together.

**USE THIS TOOL** (not just describe files) when the user asks for:
- Complete Terraform modules (main.tf, variables.tf, outputs.tf, providers.tf)
- Full Helm charts (Chart.yaml, values.yaml, templates/*.yaml)
- Kubernetes manifests (deployment.yaml, service.yaml, configmap.yaml)
- Multi-file docker-compose setups
- Multiple documentation files in a directory
- Any set of related files

**DO NOT** just describe the files - actually call this tool to create them.

All files are written atomically. Parent directories are created automatically."#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "files": {
                        "type": "array",
                        "description": "List of files to write",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Path to the file (relative to project root)"
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

        for file in &args.files {
            let requested_path = PathBuf::from(&file.path);
            let file_path = self.validate_path(&requested_path)?;

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
