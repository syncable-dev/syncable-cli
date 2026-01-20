//! Analyze project tool for the agent
//!
//! Wraps the existing `discover_dockerfiles_for_deployment` analyzer function
//! to allow the agent to analyze projects for deployment.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::analyzer::discover_dockerfiles_for_deployment;

/// Arguments for the analyze project tool
#[derive(Debug, Deserialize)]
pub struct AnalyzeProjectArgs {
    /// Path to the project directory to analyze (defaults to current directory)
    #[serde(default = "default_project_path")]
    pub project_path: String,
}

fn default_project_path() -> String {
    ".".to_string()
}

/// Error type for analyze project operations
#[derive(Debug, thiserror::Error)]
#[error("Analyze project error: {0}")]
pub struct AnalyzeProjectError(String);

/// Tool to analyze a project directory for deployment
///
/// Discovers Dockerfiles and their build configurations to help
/// prepare for deployment.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalyzeProjectTool;

impl AnalyzeProjectTool {
    /// Create a new AnalyzeProjectTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for AnalyzeProjectTool {
    const NAME: &'static str = "analyze_project";

    type Error = AnalyzeProjectError;
    type Args = AnalyzeProjectArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Analyze a project directory to discover Dockerfiles and build configurations for deployment.

Before deploying, use this tool to understand what can be deployed from a project.

**What it detects:**
- Dockerfiles and their variants (Dockerfile.dev, Dockerfile.prod, etc.)
- Build context paths for each Dockerfile
- Exposed ports from EXPOSE instructions or inferred from base images
- Multi-stage build configurations
- Suggested service names based on directory structure

**Parameters:**
- project_path: Path to the project directory (defaults to ".")

**Use Cases:**
- Before creating a deployment config, analyze the project structure
- Understand what services can be deployed from a monorepo
- Find the correct Dockerfile and build context for deployment

**Returns:**
- dockerfiles: Array of discovered Dockerfiles with deployment metadata
- summary: Human-readable summary of what was found"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "project_path": {
                        "type": "string",
                        "description": "Path to the project directory to analyze (defaults to current directory)",
                        "default": "."
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let project_path = Path::new(&args.project_path);

        // Validate path exists
        if !project_path.exists() {
            return Ok(format_error_for_llm(
                "analyze_project",
                ErrorCategory::FileNotFound,
                &format!("Project path does not exist: {}", args.project_path),
                Some(vec![
                    "Check that the path is correct",
                    "Use an absolute path or path relative to current directory",
                ]),
            ));
        }

        if !project_path.is_dir() {
            return Ok(format_error_for_llm(
                "analyze_project",
                ErrorCategory::ValidationFailed,
                &format!("Path is not a directory: {}", args.project_path),
                Some(vec!["Provide a directory path, not a file path"]),
            ));
        }

        // Call the existing analyzer function
        match discover_dockerfiles_for_deployment(project_path) {
            Ok(dockerfiles) => {
                let dockerfile_count = dockerfiles.len();

                // Build response with discovered Dockerfiles
                let dockerfile_data: Vec<serde_json::Value> = dockerfiles
                    .into_iter()
                    .map(|df| {
                        json!({
                            "path": df.path.display().to_string(),
                            "build_context": df.build_context,
                            "suggested_service_name": df.suggested_service_name,
                            "suggested_port": df.suggested_port,
                            "base_image": df.base_image,
                            "is_multistage": df.is_multistage,
                            "environment": df.environment,
                        })
                    })
                    .collect();

                let summary = if dockerfile_count == 0 {
                    "No Dockerfiles found in this project. You may need to create a Dockerfile before deploying.".to_string()
                } else {
                    format!(
                        "Found {} Dockerfile{} suitable for deployment",
                        dockerfile_count,
                        if dockerfile_count == 1 { "" } else { "s" }
                    )
                };

                let result = json!({
                    "success": true,
                    "project_path": args.project_path,
                    "dockerfiles": dockerfile_data,
                    "dockerfile_count": dockerfile_count,
                    "summary": summary,
                    "next_steps": if dockerfile_count > 0 {
                        vec![
                            "Use analyze_codebase for deeper analysis of build requirements and environment variables",
                            "Use list_deployment_capabilities to see available deployment targets",
                            "Use create_deployment_config to create a deployment configuration"
                        ]
                    } else {
                        vec![
                            "Use analyze_codebase to understand the project's technology stack and recommended Dockerfile base image",
                            "Create a Dockerfile for your application",
                            "Consider using a multi-stage build for smaller images"
                        ]
                    }
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| AnalyzeProjectError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_error_for_llm(
                "analyze_project",
                ErrorCategory::InternalError,
                &format!("Failed to analyze project: {}", e),
                Some(vec![
                    "Check that you have read permissions for the project directory",
                    "Ensure the path is accessible",
                ]),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(AnalyzeProjectTool::NAME, "analyze_project");
    }

    #[test]
    fn test_tool_creation() {
        let tool = AnalyzeProjectTool::new();
        assert!(format!("{:?}", tool).contains("AnalyzeProjectTool"));
    }

    #[test]
    fn test_default_project_path() {
        assert_eq!(default_project_path(), ".");
    }
}
