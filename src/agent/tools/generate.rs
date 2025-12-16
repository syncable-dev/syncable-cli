//! IaC Generation tool for the agent
//!
//! Wraps the existing generator functionality for the agent to use.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use crate::analyzer::analyze_monorepo;
use crate::generator;

/// Arguments for the generate IaC tool
#[derive(Debug, Deserialize)]
pub struct GenerateIaCArgs {
    /// Type of IaC to generate: "dockerfile", "compose", "terraform", or "all"
    pub generate_type: String,
    /// Optional subdirectory to generate for
    pub path: Option<String>,
}

/// Error type for generate tool
#[derive(Debug, thiserror::Error)]
#[error("Generation error: {0}")]
pub struct GenerateIaCError(String);

/// Tool to generate Infrastructure as Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateIaCTool {
    project_path: PathBuf,
}

impl GenerateIaCTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }
}

impl Tool for GenerateIaCTool {
    const NAME: &'static str = "generate_iac";

    type Error = GenerateIaCError;
    type Args = GenerateIaCArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Generate Infrastructure as Code files based on project analysis. Can generate Dockerfiles, Docker Compose configurations, or Terraform files. Returns the generated content as a preview without writing to disk.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "generate_type": {
                        "type": "string",
                        "enum": ["dockerfile", "compose", "terraform", "all"],
                        "description": "Type of IaC to generate: 'dockerfile' for container config, 'compose' for Docker Compose, 'terraform' for infrastructure, 'all' for everything"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional subdirectory to analyze for generation (relative to project root)"
                    }
                },
                "required": ["generate_type"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = if let Some(subpath) = args.path {
            self.project_path.join(subpath)
        } else {
            self.project_path.clone()
        };

        // Run analysis
        let monorepo_analysis = analyze_monorepo(&path)
            .map_err(|e| GenerateIaCError(format!("Analysis failed: {}", e)))?;

        // Get the main project analysis
        let main_project = &monorepo_analysis.projects[0];
        let analysis = &main_project.analysis;

        let generate_type = args.generate_type.to_lowercase();
        let generate_all = generate_type == "all";

        let mut results = Vec::new();

        // Generate Dockerfile
        if generate_all || generate_type == "dockerfile" {
            match generator::generate_dockerfile(analysis) {
                Ok(content) => {
                    results.push(json!({
                        "type": "Dockerfile",
                        "content": content,
                        "filename": "Dockerfile"
                    }));
                }
                Err(e) => {
                    results.push(json!({
                        "type": "Dockerfile",
                        "error": e.to_string()
                    }));
                }
            }
        }

        // Generate Docker Compose
        if generate_all || generate_type == "compose" {
            match generator::generate_compose(analysis) {
                Ok(content) => {
                    results.push(json!({
                        "type": "Docker Compose",
                        "content": content,
                        "filename": "docker-compose.yml"
                    }));
                }
                Err(e) => {
                    results.push(json!({
                        "type": "Docker Compose",
                        "error": e.to_string()
                    }));
                }
            }
        }

        // Generate Terraform
        if generate_all || generate_type == "terraform" {
            match generator::generate_terraform(analysis) {
                Ok(content) => {
                    results.push(json!({
                        "type": "Terraform",
                        "content": content,
                        "filename": "main.tf"
                    }));
                }
                Err(e) => {
                    results.push(json!({
                        "type": "Terraform",
                        "error": e.to_string()
                    }));
                }
            }
        }

        // Add project context to help the agent
        let project_info = json!({
            "project_name": main_project.name,
            "languages": monorepo_analysis.technology_summary.languages,
            "frameworks": monorepo_analysis.technology_summary.frameworks,
            "is_monorepo": monorepo_analysis.is_monorepo,
            "project_count": monorepo_analysis.projects.len()
        });

        let result = json!({
            "generated": results,
            "project_info": project_info,
            "note": "This is a preview. The content has not been written to disk. Share with the user and ask if they want to save these files."
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| GenerateIaCError(format!("Serialization error: {}", e)))
    }
}
