use crate::analyzer::{ProjectAnalysis, ProjectCategory};
use serde_json::Value as JsonValue;
use std::path::Path;

/// Extracts a meaningful project name from path and analysis
pub(crate) fn extract_project_name(project_path: &Path, _analysis: &ProjectAnalysis) -> String {
    // Try to get name from package.json
    let package_json_path = project_path.join("package.json");
    if package_json_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&package_json_path) {
            if let Ok(package_json) = serde_json::from_str::<JsonValue>(&content) {
                if let Some(name) = package_json.get("name").and_then(|n| n.as_str()) {
                    return name.to_string();
                }
            }
        }
    }

    // Try to get name from Cargo.toml
    let cargo_toml_path = project_path.join("Cargo.toml");
    if cargo_toml_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_toml_path) {
            if let Ok(cargo_toml) = toml::from_str::<toml::Value>(&content) {
                if let Some(name) = cargo_toml.get("package")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str()) {
                    return name.to_string();
                }
            }
        }
    }

    // Try to get name from pyproject.toml
    let pyproject_toml_path = project_path.join("pyproject.toml");
    if pyproject_toml_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&pyproject_toml_path) {
            if let Ok(pyproject) = toml::from_str::<toml::Value>(&content) {
                if let Some(name) = pyproject.get("project")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str()) {
                    return name.to_string();
                } else if let Some(name) = pyproject.get("tool")
                    .and_then(|t| t.get("poetry"))
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str()) {
                    return name.to_string();
                }
            }
        }
    }

    // Fall back to directory name
    project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Determines the category of a project based on its analysis
pub(crate) fn determine_project_category(analysis: &ProjectAnalysis, project_path: &Path) -> ProjectCategory {
    let dir_name = project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Check directory name patterns first
    let category_from_name = match dir_name.as_str() {
        name if name.contains("frontend") || name.contains("client") || name.contains("web") => Some(ProjectCategory::Frontend),
        name if name.contains("backend") || name.contains("server") => Some(ProjectCategory::Backend),
        name if name.contains("api") => Some(ProjectCategory::Api),
        name if name.contains("service") => Some(ProjectCategory::Service),
        name if name.contains("lib") || name.contains("library") => Some(ProjectCategory::Library),
        name if name.contains("tool") || name.contains("cli") => Some(ProjectCategory::Tool),
        name if name.contains("docs") || name.contains("doc") => Some(ProjectCategory::Documentation),
        name if name.contains("infra") || name.contains("deploy") => Some(ProjectCategory::Infrastructure),
        _ => None,
    };

    // If we found a category from the directory name, return it
    if let Some(category) = category_from_name {
        return category;
    }

    // Analyze technologies to determine category
    let has_frontend_tech = analysis.technologies.iter().any(|t| {
        matches!(t.name.as_str(),
            "React" | "Vue.js" | "Angular" | "Next.js" | "Nuxt.js" | "Svelte" |
            "Astro" | "Gatsby" | "Vite" | "Webpack" | "Parcel"
        )
    });

    let has_backend_tech = analysis.technologies.iter().any(|t| {
        matches!(t.name.as_str(),
            "Express.js" | "FastAPI" | "Django" | "Flask" | "Actix Web" | "Rocket" |
            "Spring Boot" | "Gin" | "Echo" | "Fiber" | "ASP.NET"
        )
    });

    let has_api_tech = analysis.technologies.iter().any(|t| {
        matches!(t.name.as_str(),
            "REST API" | "GraphQL" | "gRPC" | "FastAPI" | "Express.js"
        )
    });

    let has_database = analysis.technologies.iter().any(|t| {
        matches!(t.category, crate::analyzer::TechnologyCategory::Database)
    });

    if has_frontend_tech && !has_backend_tech {
        ProjectCategory::Frontend
    } else if has_backend_tech && !has_frontend_tech {
        ProjectCategory::Backend
    } else if has_api_tech || (has_backend_tech && has_database) {
        ProjectCategory::Api
    } else if matches!(analysis.project_type, crate::analyzer::ProjectType::Library) {
        ProjectCategory::Library
    } else if matches!(analysis.project_type, crate::analyzer::ProjectType::CliTool) {
        ProjectCategory::Tool
    } else {
        ProjectCategory::Unknown
    }
} 