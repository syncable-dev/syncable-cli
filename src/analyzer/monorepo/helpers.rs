use crate::analyzer::ProjectInfo;

/// Calculates overall confidence score across all projects
pub(crate) fn calculate_overall_confidence(projects: &[ProjectInfo]) -> f32 {
    if projects.is_empty() {
        return 0.0;
    }

    let total_confidence: f32 = projects.iter()
        .map(|p| p.analysis.analysis_metadata.confidence_score)
        .sum();

    total_confidence / projects.len() as f32
} 