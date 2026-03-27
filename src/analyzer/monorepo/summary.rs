use crate::analyzer::{ArchitecturePattern, ProjectCategory, ProjectInfo, TechnologySummary};
use std::collections::HashSet;

/// Generates a summary of technologies across all projects
pub(crate) fn generate_technology_summary(projects: &[ProjectInfo]) -> TechnologySummary {
    let mut all_languages = HashSet::new();
    let mut all_frameworks = HashSet::new();
    let mut all_databases = HashSet::new();

    for project in projects {
        // Collect languages
        for lang in &project.analysis.languages {
            all_languages.insert(lang.name.clone());
        }

        // Collect technologies
        for tech in &project.analysis.technologies {
            match tech.category {
                crate::analyzer::TechnologyCategory::FrontendFramework
                | crate::analyzer::TechnologyCategory::BackendFramework
                | crate::analyzer::TechnologyCategory::MetaFramework => {
                    all_frameworks.insert(tech.name.clone());
                }
                crate::analyzer::TechnologyCategory::Database => {
                    all_databases.insert(tech.name.clone());
                }
                _ => {}
            }
        }
    }

    let architecture_pattern = determine_architecture_pattern(projects);

    TechnologySummary {
        languages: all_languages.into_iter().collect(),
        frameworks: all_frameworks.into_iter().collect(),
        databases: all_databases.into_iter().collect(),
        total_projects: projects.len(),
        architecture_pattern,
    }
}

/// Determines the overall architecture pattern
fn determine_architecture_pattern(projects: &[ProjectInfo]) -> ArchitecturePattern {
    if projects.len() == 1 {
        return ArchitecturePattern::Monolithic;
    }

    let has_frontend = projects
        .iter()
        .any(|p| p.project_category == ProjectCategory::Frontend);
    let has_backend = projects.iter().any(|p| {
        matches!(
            p.project_category,
            ProjectCategory::Backend | ProjectCategory::Api
        )
    });
    let service_count = projects
        .iter()
        .filter(|p| p.project_category == ProjectCategory::Service)
        .count();

    if service_count >= 2 {
        ArchitecturePattern::Microservices
    } else if has_frontend && has_backend {
        ArchitecturePattern::Fullstack
    } else if projects
        .iter()
        .all(|p| p.project_category == ProjectCategory::Api)
    {
        ArchitecturePattern::ApiFirst
    } else {
        ArchitecturePattern::Mixed
    }
}
