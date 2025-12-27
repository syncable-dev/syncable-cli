use crate::analyzer::{
    AnalysisConfig, AnalysisMetadata, MonorepoAnalysis, ProjectInfo, analyze_project_with_config,
};
use crate::common::file_utils;
use crate::error::Result;
use chrono::Utc;
use std::path::{Path, PathBuf};

use super::config::MonorepoDetectionConfig;
use super::detection::{detect_potential_projects, determine_if_monorepo};
use super::helpers::calculate_overall_confidence;
use super::project_info::{determine_project_category, extract_project_name};
use super::summary::generate_technology_summary;

/// Detects if a path contains a monorepo and analyzes all projects within it
pub fn analyze_monorepo(path: &Path) -> Result<MonorepoAnalysis> {
    analyze_monorepo_with_config(
        path,
        &MonorepoDetectionConfig::default(),
        &AnalysisConfig::default(),
    )
}

/// Analyzes a monorepo with custom configuration
pub fn analyze_monorepo_with_config(
    path: &Path,
    monorepo_config: &MonorepoDetectionConfig,
    analysis_config: &AnalysisConfig,
) -> Result<MonorepoAnalysis> {
    let start_time = std::time::Instant::now();
    let root_path = file_utils::validate_project_path(path)?;

    log::info!("Starting monorepo analysis of: {}", root_path.display());

    // Detect potential projects within the path
    let potential_projects = detect_potential_projects(&root_path, monorepo_config)?;

    log::debug!("Found {} potential projects", potential_projects.len());

    // Determine if this is actually a monorepo or just a single project
    let is_monorepo = determine_if_monorepo(&root_path, &potential_projects, monorepo_config)?;

    let mut projects = Vec::new();

    if is_monorepo && potential_projects.len() > 1 {
        // Analyze each project separately
        for project_path in potential_projects {
            if let Ok(project_info) =
                analyze_individual_project(&root_path, &project_path, analysis_config)
            {
                projects.push(project_info);
            }
        }

        // If we didn't find multiple valid projects, treat as single project
        if projects.len() <= 1 {
            log::info!(
                "Detected potential monorepo but only found {} valid project(s), treating as single project",
                projects.len()
            );
            projects.clear();
            let single_analysis = analyze_project_with_config(&root_path, analysis_config)?;
            projects.push(ProjectInfo {
                path: PathBuf::from("."),
                name: extract_project_name(&root_path, &single_analysis),
                project_category: determine_project_category(&single_analysis, &root_path),
                analysis: single_analysis,
            });
        }
    } else {
        // Single project analysis
        let single_analysis = analyze_project_with_config(&root_path, analysis_config)?;
        projects.push(ProjectInfo {
            path: PathBuf::from("."),
            name: extract_project_name(&root_path, &single_analysis),
            project_category: determine_project_category(&single_analysis, &root_path),
            analysis: single_analysis,
        });
    }

    // Generate technology summary
    let technology_summary = generate_technology_summary(&projects);

    let duration = start_time.elapsed();
    let metadata = AnalysisMetadata {
        timestamp: Utc::now().to_rfc3339(),
        analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
        analysis_duration_ms: duration.as_millis() as u64,
        files_analyzed: projects
            .iter()
            .map(|p| p.analysis.analysis_metadata.files_analyzed)
            .sum(),
        confidence_score: calculate_overall_confidence(&projects),
    };

    Ok(MonorepoAnalysis {
        root_path,
        is_monorepo: projects.len() > 1,
        projects,
        metadata,
        technology_summary,
    })
}

/// Analyzes an individual project within a monorepo
fn analyze_individual_project(
    root_path: &Path,
    project_path: &Path,
    config: &AnalysisConfig,
) -> Result<ProjectInfo> {
    log::debug!("Analyzing individual project: {}", project_path.display());

    let analysis = analyze_project_with_config(project_path, config)?;
    let relative_path = project_path
        .strip_prefix(root_path)
        .unwrap_or(project_path)
        .to_path_buf();

    let name = extract_project_name(project_path, &analysis);
    let category = determine_project_category(&analysis, project_path);

    Ok(ProjectInfo {
        path: relative_path,
        name,
        project_category: category,
        analysis,
    })
}
