//! Analyze tool - wraps the analyze command using Rig's Tool trait

use super::compression::{CompressionConfig, compress_analysis_output};
use super::error::{ErrorCategory, format_error_for_llm};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

/// Arguments for the analyze tool
#[derive(Debug, Deserialize)]
pub struct AnalyzeArgs {
    /// Optional subdirectory path to analyze
    pub path: Option<String>,
}

/// Error type for analyze tool
#[derive(Debug, thiserror::Error)]
#[error("Analysis error: {0}")]
pub struct AnalyzeError(String);

/// Tool to analyze a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeTool {
    project_path: PathBuf,
}

impl AnalyzeTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }
}

impl Tool for AnalyzeTool {
    const NAME: &'static str = "analyze_project";

    type Error = AnalyzeError;
    type Args = AnalyzeArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Analyze the project to detect programming languages, frameworks, dependencies, build tools, and architecture patterns.

**What gets analyzed:**
- Languages: Java, Go, JavaScript/TypeScript, Rust, Python
- Frameworks: Spring Boot, Express, React, Vue, Django, FastAPI, Actix, etc.
- Dependencies: package.json, go.mod, Cargo.toml, pom.xml, requirements.txt
- Build tools: Maven, Gradle, npm/yarn/pnpm, Cargo, Make
- Architecture: microservices, monolith, monorepo structure

**Monorepo detection:**
Automatically detects and analyzes all sub-projects in monorepos (Nx, Turborepo, Lerna, Yarn workspaces, etc.). Returns analysis for each discovered project.

**Output format:**
Returns a compressed summary with key findings. Full analysis is stored and can be retrieved using the `retrieve_output` tool with the returned `retrieval_id`.

**When to use:**
- Start of analysis to understand project structure
- After major changes to verify project configuration
- To identify all languages/frameworks before linting or optimization"#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Subdirectory path to analyze (relative to project root). Use to target a specific sub-project in a monorepo. Leave empty/omit to analyze the entire project from root."
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = if let Some(ref subpath) = args.path {
            let joined = self.project_path.join(subpath);
            // Validate the path exists
            if !joined.exists() {
                return Ok(format_error_for_llm(
                    "analyze_project",
                    ErrorCategory::FileNotFound,
                    &format!("Path not found: {}", subpath),
                    Some(vec![
                        "Check if the path exists",
                        "Use list_directory to explore available paths",
                        "Omit path parameter to analyze the entire project",
                    ]),
                ));
            }
            joined
        } else {
            self.project_path.clone()
        };

        // Edge case: Check if directory is empty or has no analyzable content
        let entries: Vec<_> = match std::fs::read_dir(&path) {
            Ok(dir) => dir.filter_map(Result::ok).collect(),
            Err(e) => {
                return Ok(format_error_for_llm(
                    "analyze_project",
                    ErrorCategory::PermissionDenied,
                    &format!("Cannot read directory: {}", e),
                    Some(vec![
                        "Check file permissions",
                        "Ensure the path is a directory, not a file",
                    ]),
                ));
            }
        };

        if entries.is_empty() {
            return Ok(format_error_for_llm(
                "analyze_project",
                ErrorCategory::ValidationFailed,
                "Directory appears to be empty",
                Some(vec![
                    "Check if the path is correct",
                    "Hidden files (starting with .) are included in analysis",
                    "Use list_directory to see what's in this path",
                ]),
            ));
        }

        // Edge case: Warn about very large projects (rough estimate)
        // Count visible entries recursively up to a limit
        let file_count = count_files_recursive(&path, 15000);
        let large_project_warning = if file_count >= 10000 {
            Some(format!(
                "Note: Large project detected (~{}+ files). Analysis may take longer.",
                file_count
            ))
        } else {
            None
        };

        // Use monorepo analyzer to detect ALL projects in monorepos
        // This returns MonorepoAnalysis with full project list instead of flat ProjectAnalysis
        match crate::analyzer::analyze_monorepo(&path) {
            Ok(analysis) => {
                // Edge case: Check if no languages were detected (unsupported project type)
                if analysis.technology_summary.languages.is_empty() {
                    return Ok(format_error_for_llm(
                        "analyze_project",
                        ErrorCategory::ValidationFailed,
                        "No supported programming languages detected in this directory",
                        Some(vec![
                            "Supported languages: Java, Go, JavaScript/TypeScript, Rust, Python",
                            "Check if source files exist in this directory or subdirectories",
                            "For non-code projects, use list_directory to explore contents",
                            "Try analyzing a specific subdirectory if this is a monorepo",
                        ]),
                    ));
                }

                let json_value = serde_json::to_value(&analysis).map_err(|e| {
                    AnalyzeError(format!(
                        "Failed to serialize analysis results: {}",
                        e
                    ))
                })?;

                // Use smart compression with RAG retrieval pattern
                // This preserves all data while keeping context size manageable
                let config = CompressionConfig::default();
                let mut result = compress_analysis_output(&json_value, &config);

                // Append large project warning if applicable
                if let Some(warning) = large_project_warning {
                    result = format!("{}\n\n{}", warning, result);
                }

                Ok(result)
            }
            Err(e) => {
                // Provide structured error with suggestions
                let error_str = e.to_string();
                let (category, suggestions) = if error_str.contains("permission")
                    || error_str.contains("Permission")
                {
                    (
                        ErrorCategory::PermissionDenied,
                        vec!["Check file permissions", "Try a different subdirectory"],
                    )
                } else if error_str.contains("not found") || error_str.contains("No such file") {
                    (
                        ErrorCategory::FileNotFound,
                        vec![
                            "Verify the path exists",
                            "Use list_directory to explore",
                        ],
                    )
                } else {
                    (
                        ErrorCategory::InternalError,
                        vec!["Try analyzing a subdirectory", "Check project structure"],
                    )
                };

                Ok(format_error_for_llm(
                    "analyze_project",
                    category,
                    &format!("Analysis failed: {}", e),
                    Some(suggestions),
                ))
            }
        }
    }
}

/// Count files recursively up to a limit (to avoid long waits on huge directories)
fn count_files_recursive(path: &std::path::Path, limit: usize) -> usize {
    let mut count = 0;
    let mut dirs_to_visit = vec![path.to_path_buf()];

    while let Some(dir) = dirs_to_visit.pop() {
        if count >= limit {
            break;
        }

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(Result::ok) {
                if count >= limit {
                    break;
                }

                let path = entry.path();
                // Skip common non-source directories for efficiency
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if matches!(
                        name,
                        "node_modules"
                            | "target"
                            | ".git"
                            | "vendor"
                            | "dist"
                            | "build"
                            | "__pycache__"
                            | ".venv"
                            | "venv"
                    ) {
                        continue;
                    }
                }

                if path.is_file() {
                    count += 1;
                } else if path.is_dir() {
                    dirs_to_visit.push(path);
                }
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_count_files_empty_dir() {
        let dir = tempdir().unwrap();
        let count = count_files_recursive(dir.path(), 10000);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_files_with_files() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("file1.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.path().join("file2.go"), "package main").unwrap();
        let count = count_files_recursive(dir.path(), 10000);
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_analyze_nonexistent_path() {
        let dir = tempdir().unwrap();
        let tool = AnalyzeTool::new(dir.path().to_path_buf());
        let args = AnalyzeArgs {
            path: Some("nonexistent".to_string()),
        };

        let result = tool.call(args).await.unwrap();
        // Should return error formatted for LLM
        assert!(result.contains("error") || result.contains("not found") || result.contains("Path not found"));
    }
}
