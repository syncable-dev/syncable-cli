use super::microservices::MicroserviceInfo;
use crate::analyzer::{DetectedLanguage, DetectedTechnology, EntryPoint, Port, ProjectType};

/// Enhanced project type determination including microservice structure analysis
pub(crate) fn determine_project_type_with_structure(
    languages: &[DetectedLanguage],
    technologies: &[DetectedTechnology],
    entry_points: &[EntryPoint],
    ports: &[Port],
    microservices: &[MicroserviceInfo],
) -> ProjectType {
    // If we have multiple services with databases, it's likely a microservice architecture
    let services_with_db = microservices.iter().filter(|s| s.has_db).count();
    if services_with_db >= 2 || microservices.len() >= 3 {
        return ProjectType::Microservice;
    }

    // Fall back to original determination logic
    determine_project_type(languages, technologies, entry_points, ports)
}

/// Determines the project type based on analysis
fn determine_project_type(
    languages: &[DetectedLanguage],
    technologies: &[DetectedTechnology],
    entry_points: &[EntryPoint],
    ports: &[Port],
) -> ProjectType {
    // Check for microservice architecture indicators
    let has_database_ports = ports.iter().any(|p| {
        if let Some(desc) = &p.description {
            let desc_lower = desc.to_lowercase();
            desc_lower.contains("postgres")
                || desc_lower.contains("mysql")
                || desc_lower.contains("mongodb")
                || desc_lower.contains("database")
        } else {
            false
        }
    });

    let has_multiple_services = ports
        .iter()
        .filter_map(|p| p.description.as_ref())
        .filter(|desc| {
            let desc_lower = desc.to_lowercase();
            desc_lower.contains("service") || desc_lower.contains("application")
        })
        .count()
        > 1;

    let has_orchestration_framework = technologies
        .iter()
        .any(|t| t.name == "Encore" || t.name == "Dapr" || t.name == "Temporal");

    // Check for web frameworks
    let web_frameworks = [
        "Express",
        "Fastify",
        "Koa",
        "Next.js",
        "React",
        "Vue",
        "Angular",
        "Django",
        "Flask",
        "FastAPI",
        "Spring Boot",
        "Actix Web",
        "Rocket",
        "Gin",
        "Echo",
        "Fiber",
        "Svelte",
        "SvelteKit",
        "SolidJS",
        "Astro",
        "Encore",
        "Hono",
        "Elysia",
        "React Router v7",
        "Tanstack Start",
        "SolidStart",
        "Qwik",
        "Nuxt.js",
        "Gatsby",
    ];

    let has_web_framework = technologies
        .iter()
        .any(|t| web_frameworks.contains(&t.name.as_str()));

    // Check for CLI indicators
    let cli_indicators = ["cobra", "clap", "argparse", "commander"];
    let has_cli_framework = technologies
        .iter()
        .any(|t| cli_indicators.contains(&t.name.to_lowercase().as_str()));

    // Check for API indicators
    let api_frameworks = [
        "FastAPI",
        "Express",
        "Gin",
        "Echo",
        "Actix Web",
        "Spring Boot",
        "Fastify",
        "Koa",
        "Nest.js",
        "Encore",
        "Hono",
        "Elysia",
    ];
    let has_api_framework = technologies
        .iter()
        .any(|t| api_frameworks.contains(&t.name.as_str()));

    // Check for static site generators
    let static_generators = ["Gatsby", "Hugo", "Jekyll", "Eleventy", "Astro"];
    let has_static_generator = technologies
        .iter()
        .any(|t| static_generators.contains(&t.name.as_str()));

    // Determine type based on indicators
    if (has_database_ports || has_multiple_services)
        && (has_orchestration_framework || has_api_framework)
    {
        ProjectType::Microservice
    } else if has_static_generator {
        ProjectType::StaticSite
    } else if has_api_framework && !has_web_framework {
        ProjectType::ApiService
    } else if has_web_framework {
        ProjectType::WebApplication
    } else if has_cli_framework || (entry_points.len() == 1 && ports.is_empty()) {
        ProjectType::CliTool
    } else if entry_points.is_empty() && ports.is_empty() {
        // Check if it's a library
        let has_lib_indicators = languages.iter().any(|l| match l.name.as_str() {
            "Rust" => l
                .files
                .iter()
                .any(|f| f.to_string_lossy().contains("lib.rs")),
            "Python" => l
                .files
                .iter()
                .any(|f| f.to_string_lossy().contains("__init__.py")),
            "JavaScript" | "TypeScript" => l.main_dependencies.is_empty(),
            _ => false,
        });

        if has_lib_indicators {
            ProjectType::Library
        } else {
            ProjectType::Unknown
        }
    } else {
        ProjectType::Unknown
    }
}
