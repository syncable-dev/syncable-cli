//! Analyze tool - wraps the analyze command using Rig's Tool trait

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use crate::analyzer::display::{DisplayMode, display_analysis_to_string};
use crate::analyzer::analyze_monorepo;

/// Arguments for the analyze tool
#[derive(Debug, Deserialize)]
pub struct AnalyzeArgs {
    /// Optional subdirectory path to analyze
    pub path: Option<String>,
    /// Display mode: "matrix" (default), "detailed", "summary", or "json"
    pub mode: Option<String>,
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
            description: "Analyze the project to detect programming languages, frameworks, dependencies, build tools, and architecture patterns. Returns a comprehensive overview of the project's technology stack. Use 'detailed' mode for full analysis, 'summary' for quick overview, 'json' for structured data.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Optional subdirectory path to analyze (relative to project root). If not provided, analyzes the entire project."
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["matrix", "detailed", "summary", "json"],
                        "description": "Display mode: 'matrix' for compact dashboard, 'detailed' for full analysis with Docker info, 'summary' for brief overview, 'json' for structured data. Default is 'json' for best agent parsing."
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

        // Parse display mode - default to JSON for agent consumption
        let display_mode = match args.mode.as_deref() {
            Some("matrix") => DisplayMode::Matrix,
            Some("detailed") => DisplayMode::Detailed,
            Some("summary") => DisplayMode::Summary,
            Some("json") | None => DisplayMode::Json,
            _ => DisplayMode::Json,
        };

        match analyze_monorepo(&path) {
            Ok(analysis) => {
                // Use the display system to format output
                let output = display_analysis_to_string(&analysis, display_mode);
                Ok(output)
            }
            Err(e) => Err(AnalyzeError(format!("Analysis failed: {}", e))),
        }
    }
}
