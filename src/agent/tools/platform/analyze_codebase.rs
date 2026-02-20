//! Analyze codebase tool for the agent
//!
//! Wraps the full `analyze_project()` analyzer function to provide comprehensive
//! project analysis including languages, frameworks, entry points, ports,
//! environment variables, and build scripts.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::analyzer::{
    AnalysisConfig, ProjectAnalysis, ProjectType, TechnologyCategory, analyze_project_with_config,
};

/// Arguments for the analyze codebase tool
#[derive(Debug, Deserialize)]
pub struct AnalyzeCodebaseArgs {
    /// Path to the project directory to analyze (defaults to current directory)
    #[serde(default = "default_project_path")]
    pub project_path: String,
    /// Whether to include dev dependencies in analysis (defaults to false)
    #[serde(default)]
    pub include_dev_dependencies: bool,
}

fn default_project_path() -> String {
    ".".to_string()
}

/// Error type for analyze codebase operations
#[derive(Debug, thiserror::Error)]
#[error("Analyze codebase error: {0}")]
pub struct AnalyzeCodebaseError(String);

/// Tool to perform comprehensive codebase analysis
///
/// Provides detailed information about a project's technology stack,
/// build requirements, and deployment configuration recommendations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalyzeCodebaseTool;

impl AnalyzeCodebaseTool {
    /// Create a new AnalyzeCodebaseTool
    pub fn new() -> Self {
        Self
    }
}

impl Tool for AnalyzeCodebaseTool {
    const NAME: &'static str = "analyze_codebase";

    type Error = AnalyzeCodebaseError;
    type Args = AnalyzeCodebaseArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Perform comprehensive analysis of a codebase to understand its technology stack and deployment requirements.

**Use this tool to understand HOW to configure a deployment.** For quick Dockerfile discovery, use `analyze_project` instead.

**What it detects:**
- Programming languages with versions and confidence scores
- Frameworks and libraries (React, Next.js, Express, Django, etc.)
- Entry points and exposed ports
- Environment variables the application needs
- Build scripts (npm run build, etc.)
- Docker configuration if present

**Parameters:**
- project_path: Path to the project directory (defaults to ".")
- include_dev_dependencies: Include dev dependencies in analysis (default: false)

**Use Cases:**
- Understanding a project's technology stack before configuring deployment
- Discovering required environment variables for secrets setup
- Finding available build scripts for CI/CD configuration
- Recommending appropriate Dockerfile base images

**Returns:**
- languages: Detected languages with versions
- technologies: Frameworks, libraries, and tools
- ports: Exposed ports from various sources
- environment_variables: Environment variables the app needs
- build_scripts: Available build commands
- deployment_hints: Derived recommendations for deployment
- next_steps: Guidance on what to do next

**Comparison with analyze_project:**
- `analyze_project`: Fast, focused on Dockerfiles only - "what can I deploy?"
- `analyze_codebase`: Comprehensive analysis - "how should I configure deployment?""#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "project_path": {
                        "type": "string",
                        "description": "Path to the project directory to analyze (defaults to current directory)",
                        "default": "."
                    },
                    "include_dev_dependencies": {
                        "type": "boolean",
                        "description": "Include dev dependencies in analysis (default: false)",
                        "default": false
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
                "analyze_codebase",
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
                "analyze_codebase",
                ErrorCategory::ValidationFailed,
                &format!("Path is not a directory: {}", args.project_path),
                Some(vec!["Provide a directory path, not a file path"]),
            ));
        }

        // Configure analysis
        let config = AnalysisConfig {
            include_dev_dependencies: args.include_dev_dependencies,
            deep_analysis: true,
            ..Default::default()
        };

        // Perform analysis
        match analyze_project_with_config(project_path, &config) {
            Ok(analysis) => {
                let result = format_analysis_for_llm(&args.project_path, &analysis);
                serde_json::to_string_pretty(&result)
                    .map_err(|e| AnalyzeCodebaseError(format!("Failed to serialize: {}", e)))
            }
            Err(e) => Ok(format_error_for_llm(
                "analyze_codebase",
                ErrorCategory::InternalError,
                &format!("Failed to analyze codebase: {}", e),
                Some(vec![
                    "Check that you have read permissions for the project directory",
                    "Ensure the path is accessible",
                    "Try running from the project root directory",
                ]),
            )),
        }
    }
}

/// Format ProjectAnalysis into LLM-friendly JSON
fn format_analysis_for_llm(project_path: &str, analysis: &ProjectAnalysis) -> serde_json::Value {
    // Format languages
    let languages: Vec<serde_json::Value> = analysis
        .languages
        .iter()
        .map(|lang| {
            json!({
                "name": lang.name,
                "version": lang.version,
                "confidence": lang.confidence,
                "package_manager": lang.package_manager,
            })
        })
        .collect();

    // Format technologies (frameworks, libraries)
    let technologies: Vec<serde_json::Value> = analysis
        .technologies
        .iter()
        .map(|tech| {
            json!({
                "name": tech.name,
                "version": tech.version,
                "category": format_category(&tech.category),
                "is_primary": tech.is_primary,
                "confidence": tech.confidence,
            })
        })
        .collect();

    // Format ports
    let ports: Vec<serde_json::Value> = analysis
        .ports
        .iter()
        .map(|port| {
            json!({
                "number": port.number,
                "protocol": format!("{:?}", port.protocol),
                "description": port.description,
            })
        })
        .collect();

    // Format environment variables
    let env_vars: Vec<serde_json::Value> = analysis
        .environment_variables
        .iter()
        .map(|env| {
            json!({
                "name": env.name,
                "required": env.required,
                "default_value": env.default_value,
                "description": env.description,
            })
        })
        .collect();

    // Format build scripts
    let build_scripts: Vec<serde_json::Value> = analysis
        .build_scripts
        .iter()
        .map(|script| {
            json!({
                "name": script.name,
                "command": script.command,
                "description": script.description,
                "is_default": script.is_default,
            })
        })
        .collect();

    // Derive deployment hints
    let deployment_hints = derive_deployment_hints(analysis);

    // Determine next steps
    let next_steps = determine_next_steps(analysis);

    json!({
        "success": true,
        "project_path": project_path,
        "languages": languages,
        "technologies": technologies,
        "ports": ports,
        "environment_variables": env_vars,
        "build_scripts": build_scripts,
        "project_type": format!("{:?}", analysis.project_type),
        "architecture_type": format!("{:?}", analysis.architecture_type),
        "analysis_metadata": {
            "confidence_score": analysis.analysis_metadata.confidence_score,
            "files_analyzed": analysis.analysis_metadata.files_analyzed,
            "duration_ms": analysis.analysis_metadata.analysis_duration_ms,
        },
        "deployment_hints": deployment_hints,
        "summary": format_summary(analysis),
        "next_steps": next_steps,
    })
}

/// Format technology category for output
fn format_category(category: &TechnologyCategory) -> String {
    match category {
        TechnologyCategory::MetaFramework => "MetaFramework".to_string(),
        TechnologyCategory::FrontendFramework => "FrontendFramework".to_string(),
        TechnologyCategory::BackendFramework => "BackendFramework".to_string(),
        TechnologyCategory::Library(lib_type) => format!("Library:{:?}", lib_type),
        TechnologyCategory::BuildTool => "BuildTool".to_string(),
        TechnologyCategory::Database => "Database".to_string(),
        TechnologyCategory::Testing => "Testing".to_string(),
        TechnologyCategory::Runtime => "Runtime".to_string(),
        TechnologyCategory::PackageManager => "PackageManager".to_string(),
    }
}

/// Derive deployment hints from analysis
fn derive_deployment_hints(analysis: &ProjectAnalysis) -> serde_json::Value {
    // Suggested port: first detected port or framework default
    let suggested_port = analysis
        .ports
        .first()
        .map(|p| p.number)
        .or_else(|| infer_default_port(analysis));

    // Check if build step is needed
    let needs_build_step = !analysis.build_scripts.is_empty()
        || analysis.technologies.iter().any(|t| {
            matches!(
                t.category,
                TechnologyCategory::MetaFramework | TechnologyCategory::FrontendFramework
            )
        });

    // Recommend Dockerfile base image
    let recommended_dockerfile_base = infer_dockerfile_base(analysis);

    // Check for Docker presence
    let has_dockerfile = analysis
        .docker_analysis
        .as_ref()
        .map(|d| !d.dockerfiles.is_empty())
        .unwrap_or(false);

    json!({
        "suggested_port": suggested_port,
        "needs_build_step": needs_build_step,
        "recommended_dockerfile_base": recommended_dockerfile_base,
        "has_existing_dockerfile": has_dockerfile,
        "required_env_vars": analysis.environment_variables.iter()
            .filter(|e| e.required)
            .map(|e| e.name.clone())
            .collect::<Vec<_>>(),
    })
}

/// Infer default port based on detected frameworks
fn infer_default_port(analysis: &ProjectAnalysis) -> Option<u16> {
    for tech in &analysis.technologies {
        let name_lower = tech.name.to_lowercase();
        if name_lower.contains("next") || name_lower.contains("nuxt") {
            return Some(3000);
        }
        if name_lower.contains("vite") || name_lower.contains("vue") {
            return Some(5173);
        }
        if name_lower.contains("angular") {
            return Some(4200);
        }
        if name_lower.contains("django") {
            return Some(8000);
        }
        if name_lower.contains("flask") {
            return Some(5000);
        }
        if name_lower.contains("express") || name_lower.contains("fastify") {
            return Some(3000);
        }
        if name_lower.contains("spring") {
            return Some(8080);
        }
        if name_lower.contains("actix") || name_lower.contains("axum") {
            return Some(8080);
        }
    }

    // Default based on language
    for lang in &analysis.languages {
        match lang.name.to_lowercase().as_str() {
            "python" => return Some(8000),
            "go" => return Some(8080),
            "rust" => return Some(8080),
            "java" | "kotlin" => return Some(8080),
            "javascript" | "typescript" => return Some(3000),
            _ => {}
        }
    }

    None
}

/// Infer recommended Dockerfile base image
fn infer_dockerfile_base(analysis: &ProjectAnalysis) -> Option<String> {
    // Check primary language
    for lang in &analysis.languages {
        match lang.name.to_lowercase().as_str() {
            "javascript" | "typescript" => {
                // Check for Bun
                if analysis
                    .technologies
                    .iter()
                    .any(|t| t.name.to_lowercase() == "bun")
                {
                    return Some("oven/bun:1-alpine".to_string());
                }
                return Some("node:20-alpine".to_string());
            }
            "python" => return Some("python:3.12-slim".to_string()),
            "go" => return Some("golang:1.22-alpine".to_string()),
            "rust" => return Some("rust:1.75-alpine".to_string()),
            "java" => return Some("eclipse-temurin:21-jre-alpine".to_string()),
            "kotlin" => return Some("eclipse-temurin:21-jre-alpine".to_string()),
            _ => {}
        }
    }

    None
}

/// Determine next steps based on analysis
fn determine_next_steps(analysis: &ProjectAnalysis) -> Vec<String> {
    let mut steps = Vec::new();

    let has_dockerfile = analysis
        .docker_analysis
        .as_ref()
        .map(|d| !d.dockerfiles.is_empty())
        .unwrap_or(false);

    if has_dockerfile {
        steps.push("Use analyze_project to get specific Dockerfile details".to_string());
        steps.push(
            "Use list_deployment_capabilities to see available deployment targets".to_string(),
        );
        steps.push("Use create_deployment_config to create a deployment configuration".to_string());
    } else {
        steps.push(
            "Create a Dockerfile for your application (recommended base image in deployment_hints)"
                .to_string(),
        );
        steps.push(
            "After creating Dockerfile, use analyze_project to verify it's detected".to_string(),
        );
    }

    if !analysis.environment_variables.is_empty() {
        let required_count = analysis
            .environment_variables
            .iter()
            .filter(|e| e.required)
            .count();
        if required_count > 0 {
            steps.push(format!(
                "Configure {} required environment variable{} before deployment",
                required_count,
                if required_count == 1 { "" } else { "s" }
            ));
        }
    }

    steps
}

/// Format a human-readable summary
fn format_summary(analysis: &ProjectAnalysis) -> String {
    let lang_names: Vec<&str> = analysis.languages.iter().map(|l| l.name.as_str()).collect();

    let primary_tech: Vec<&str> = analysis
        .technologies
        .iter()
        .filter(|t| t.is_primary)
        .map(|t| t.name.as_str())
        .collect();

    let project_type = match analysis.project_type {
        ProjectType::WebApplication => "web application",
        ProjectType::ApiService => "API service",
        ProjectType::CliTool => "CLI tool",
        ProjectType::Library => "library",
        ProjectType::MobileApp => "mobile app",
        ProjectType::DesktopApp => "desktop app",
        ProjectType::Microservice => "microservice",
        ProjectType::StaticSite => "static site",
        ProjectType::Hybrid => "hybrid project",
        ProjectType::Unknown => "project",
    };

    let lang_str = if lang_names.is_empty() {
        "Unknown language".to_string()
    } else {
        lang_names.join(", ")
    };

    let tech_str = if primary_tech.is_empty() {
        String::new()
    } else {
        format!(" using {}", primary_tech.join(", "))
    };

    format!("{} {}{}", lang_str, project_type, tech_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(AnalyzeCodebaseTool::NAME, "analyze_codebase");
    }

    #[test]
    fn test_tool_creation() {
        let tool = AnalyzeCodebaseTool::new();
        assert!(format!("{:?}", tool).contains("AnalyzeCodebaseTool"));
    }

    #[test]
    fn test_default_project_path() {
        assert_eq!(default_project_path(), ".");
    }

    #[test]
    fn test_format_category() {
        assert_eq!(
            format_category(&TechnologyCategory::MetaFramework),
            "MetaFramework"
        );
        assert_eq!(
            format_category(&TechnologyCategory::BackendFramework),
            "BackendFramework"
        );
    }
}
