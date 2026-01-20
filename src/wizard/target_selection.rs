//! Target selection step for deployment wizard

use crate::platform::api::types::{DeploymentTarget, ProviderDeploymentStatus};
use crate::wizard::render::{display_step_header, wizard_render_config};
use colored::Colorize;
use inquire::{InquireError, Select};

/// Result of target selection step
#[derive(Debug, Clone)]
pub enum TargetSelectionResult {
    /// User selected a deployment target
    Selected(DeploymentTarget),
    /// User wants to go back to provider selection
    Back,
    /// User cancelled the wizard
    Cancelled,
}

/// Display target selection based on provider capabilities
pub fn select_target(provider_status: &ProviderDeploymentStatus) -> TargetSelectionResult {
    display_step_header(
        2,
        "Select Target",
        "Choose how to deploy your service. Cloud Runner is fully managed. Kubernetes gives you more control.",
    );

    let available_targets = provider_status.available_targets();

    if available_targets.is_empty() {
        println!(
            "\n{}",
            "No deployment targets available for this provider.".red()
        );
        return TargetSelectionResult::Cancelled;
    }

    // Build options with descriptions
    let mut options: Vec<String> = available_targets
        .iter()
        .map(|t| {
            match t {
                DeploymentTarget::CloudRunner => {
                    format!(
                        "{}  {}",
                        "Cloud Runner".cyan(),
                        "Fully managed, auto-scaling containers".dimmed()
                    )
                }
                DeploymentTarget::Kubernetes => {
                    let cluster_count = provider_status.clusters.iter().filter(|c| c.is_healthy).count();
                    format!(
                        "{}  {} cluster{} available",
                        "Kubernetes".cyan(),
                        cluster_count,
                        if cluster_count == 1 { "" } else { "s" }
                    )
                }
            }
        })
        .collect();

    // Add back option
    options.push("← Back to provider selection".dimmed().to_string());

    let selection = Select::new("Select deployment target:", options.clone())
        .with_render_config(wizard_render_config())
        .with_help_message("↑↓ to move, Enter to select, Esc to cancel")
        .with_page_size(4)
        .prompt();

    match selection {
        Ok(answer) => {
            if answer.contains("Back") {
                return TargetSelectionResult::Back;
            }

            let target = if answer.contains("Cloud Runner") {
                DeploymentTarget::CloudRunner
            } else {
                DeploymentTarget::Kubernetes
            };

            println!("\n{} Selected: {}", "✓".green(), target.display_name());
            TargetSelectionResult::Selected(target)
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            println!("\n{}", "Wizard cancelled.".dimmed());
            TargetSelectionResult::Cancelled
        }
        Err(_) => TargetSelectionResult::Cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_selection_result_variants() {
        let _ = TargetSelectionResult::Selected(DeploymentTarget::CloudRunner);
        let _ = TargetSelectionResult::Selected(DeploymentTarget::Kubernetes);
        let _ = TargetSelectionResult::Back;
        let _ = TargetSelectionResult::Cancelled;
    }
}
