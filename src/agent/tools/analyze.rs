//! Analyze tool - wraps the analyze command using Rig's Tool trait

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
            description: "Analyze the project to detect programming languages, frameworks, dependencies, build tools, and architecture patterns. Returns a comprehensive overview of the project's technology stack.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Optional subdirectory path to analyze (relative to project root). If not provided, analyzes the entire project."
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = if let Some(subpath) = args.path {
            self.project_path.join(subpath)
        } else {
            self.project_path.clone()
        };

        match crate::analyzer::analyze_project(&path) {
            Ok(analysis) => serde_json::to_string_pretty(&analysis)
                .map_err(|e| AnalyzeError(format!("Failed to serialize: {}", e))),
            Err(e) => Err(AnalyzeError(format!("Analysis failed: {}", e))),
        }
    }
}
