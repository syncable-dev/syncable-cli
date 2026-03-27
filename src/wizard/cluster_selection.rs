//! Cluster selection step for deployment wizard

use crate::platform::api::types::ClusterSummary;
use crate::wizard::render::{display_step_header, status_indicator, wizard_render_config};
use colored::Colorize;
use inquire::{InquireError, Select};

/// Result of cluster selection step
#[derive(Debug, Clone)]
pub enum ClusterSelectionResult {
    /// User selected a cluster
    Selected(ClusterSummary),
    /// User wants to go back
    Back,
    /// User cancelled the wizard
    Cancelled,
}

/// Display cluster selection for Kubernetes deployments
pub fn select_cluster(clusters: &[ClusterSummary]) -> ClusterSelectionResult {
    display_step_header(
        3,
        "Select Cluster",
        "Choose which Kubernetes cluster to deploy to.",
    );

    // Filter to only healthy clusters
    let healthy_clusters: Vec<&ClusterSummary> = clusters.iter().filter(|c| c.is_healthy).collect();

    if healthy_clusters.is_empty() {
        println!(
            "\n{}",
            "No healthy clusters available. Provision a cluster in platform settings.".red()
        );
        return ClusterSelectionResult::Cancelled;
    }

    // Build options with status and region
    let mut options: Vec<String> = healthy_clusters
        .iter()
        .map(|c| {
            format!(
                "{} {}  {}",
                status_indicator(c.is_healthy),
                c.name.cyan(),
                c.region.dimmed()
            )
        })
        .collect();

    // Add back option
    options.push("← Back to target selection".dimmed().to_string());

    let selection = Select::new("Select cluster:", options.clone())
        .with_render_config(wizard_render_config())
        .with_help_message("↑↓ to move, Enter to select, Esc to cancel")
        .with_page_size(6)
        .prompt();

    match selection {
        Ok(answer) => {
            if answer.contains("Back") {
                return ClusterSelectionResult::Back;
            }

            // Find selected cluster by name
            let selected = healthy_clusters
                .iter()
                .find(|c| answer.contains(&c.name))
                .copied();

            match selected {
                Some(cluster) => {
                    println!(
                        "\n{} Selected cluster: {} ({})",
                        "✓".green(),
                        cluster.name,
                        cluster.region
                    );
                    ClusterSelectionResult::Selected(cluster.clone())
                }
                None => ClusterSelectionResult::Cancelled,
            }
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            println!("\n{}", "Wizard cancelled.".dimmed());
            ClusterSelectionResult::Cancelled
        }
        Err(_) => ClusterSelectionResult::Cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_selection_result_variants() {
        let cluster = ClusterSummary {
            id: "c1".to_string(),
            name: "prod".to_string(),
            region: "us-central1".to_string(),
            is_healthy: true,
        };
        let _ = ClusterSelectionResult::Selected(cluster);
        let _ = ClusterSelectionResult::Back;
        let _ = ClusterSelectionResult::Cancelled;
    }
}
