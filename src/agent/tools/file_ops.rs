//! File operation tools for reading and exploring the project using Rig's Tool trait

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

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
        let canonical_project = self.project_path.canonicalize()
            .map_err(|e| ReadFileError(format!("Invalid project path: {}", e)))?;
        
        let target = if requested.is_absolute() {
            requested.clone()
        } else {
            self.project_path.join(requested)
        };

        let canonical_target = target.canonicalize()
            .map_err(|e| ReadFileError(format!("File not found: {}", e)))?;

        if !canonical_target.starts_with(&canonical_project) {
            return Err(ReadFileError("Access denied: path is outside project directory".to_string()));
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
            return Ok(json!({
                "error": format!("File too large ({} bytes). Maximum size is {} bytes.", metadata.len(), MAX_SIZE)
            }).to_string());
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|e| ReadFileError(format!("Failed to read file: {}", e)))?;

        let output = if let Some(start) = args.start_line {
            let lines: Vec<&str> = content.lines().collect();
            let start_idx = (start as usize).saturating_sub(1);
            let end_idx = args.end_line.map(|e| (e as usize).min(lines.len())).unwrap_or(lines.len());
            
            if start_idx >= lines.len() {
                return Ok(json!({
                    "error": format!("Start line {} exceeds file length ({})", start, lines.len())
                }).to_string());
            }

            let selected: Vec<String> = lines[start_idx..end_idx]
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>4} | {}", start_idx + i + 1, line))
                .collect();

            json!({
                "file": args.path,
                "lines": format!("{}-{}", start, end_idx),
                "total_lines": lines.len(),
                "content": selected.join("\n")
            })
        } else {
            json!({
                "file": args.path,
                "total_lines": content.lines().count(),
                "content": content
            })
        };

        serde_json::to_string_pretty(&output)
            .map_err(|e| ReadFileError(format!("Failed to serialize: {}", e)))
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
        let canonical_project = self.project_path.canonicalize()
            .map_err(|e| ListDirectoryError(format!("Invalid project path: {}", e)))?;
        
        let target = if requested.is_absolute() {
            requested.clone()
        } else {
            self.project_path.join(requested)
        };

        let canonical_target = target.canonicalize()
            .map_err(|e| ListDirectoryError(format!("Directory not found: {}", e)))?;

        if !canonical_target.starts_with(&canonical_project) {
            return Err(ListDirectoryError("Access denied: path is outside project directory".to_string()));
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
        let skip_dirs = ["node_modules", ".git", "target", "__pycache__", ".venv", "venv", "dist", "build"];
        
        let dir_name = current_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        
        if depth > 0 && skip_dirs.contains(&dir_name) {
            return Ok(());
        }

        let read_dir = fs::read_dir(current_path)
            .map_err(|e| ListDirectoryError(format!("Cannot read directory: {}", e)))?;

        for entry in read_dir {
            let entry = entry.map_err(|e| ListDirectoryError(format!("Error reading entry: {}", e)))?;
            let path = entry.path();
            let metadata = entry.metadata().ok();
            
            let relative_path = path.strip_prefix(base_path).unwrap_or(&path).to_string_lossy().to_string();
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

        let result = json!({
            "path": path_str,
            "entries": entries,
            "total_count": entries.len()
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| ListDirectoryError(format!("Failed to serialize: {}", e)))
    }
}
