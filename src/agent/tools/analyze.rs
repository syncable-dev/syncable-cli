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

        // Use monorepo analyzer to detect ALL projects in monorepos
        // This returns MonorepoAnalysis with full project list instead of flat ProjectAnalysis
        match crate::analyzer::analyze_monorepo(&path) {
            Ok(analysis) => {
                let json_value = serde_json::to_value(&analysis).map_err(|e| {
                    AnalyzeError(format!(
                        "Failed to serialize analysis results: {}",
                        e
                    ))
                })?;

                // Use smart compression with RAG retrieval pattern
                // This preserves all data while keeping context size manageable
                let config = CompressionConfig::default();
                Ok(compress_analysis_output(&json_value, &config))
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
