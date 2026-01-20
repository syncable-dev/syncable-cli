//! Provider selection step for deployment wizard

use crate::platform::api::{
    types::{
        CloudProvider, ClusterStatus, ClusterSummary, ProviderDeploymentStatus, RegistryStatus,
        RegistrySummary,
    },
    PlatformApiClient,
};
use crate::wizard::render::{display_step_header, status_indicator, wizard_render_config};
use colored::Colorize;
use inquire::{InquireError, Select};
use std::collections::HashMap;

/// Get deployment status for all providers
///
/// Queries the platform to determine which providers are connected and what
/// resources (clusters, registries) are available for each.
pub async fn get_provider_deployment_statuses(
    client: &PlatformApiClient,
    project_id: &str,
) -> Result<Vec<ProviderDeploymentStatus>, crate::platform::api::PlatformApiError> {
    // Get all cloud credentials for the project (determines connectivity)
    let credentials = client
        .list_cloud_credentials_for_project(project_id)
        .await
        .unwrap_or_default();

    // Build set of connected providers from credentials
    let connected_providers: std::collections::HashSet<String> = credentials
        .iter()
        .map(|c| c.provider.to_lowercase())
        .collect();

    // Get all clusters and registries for the project
    let clusters = client
        .list_clusters_for_project(project_id)
        .await
        .unwrap_or_default();
    let registries = client
        .list_registries_for_project(project_id)
        .await
        .unwrap_or_default();

    // Group by provider
    let mut provider_clusters: HashMap<CloudProvider, Vec<ClusterSummary>> = HashMap::new();
    let mut provider_registries: HashMap<CloudProvider, Vec<RegistrySummary>> = HashMap::new();

    for cluster in clusters {
        let summary = ClusterSummary {
            id: cluster.id,
            name: cluster.name,
            region: cluster.region,
            is_healthy: cluster.status == ClusterStatus::Running,
        };
        provider_clusters
            .entry(cluster.provider)
            .or_default()
            .push(summary);
    }

    for registry in registries {
        let summary = RegistrySummary {
            id: registry.id,
            name: registry.name,
            region: registry.region,
            is_ready: registry.status == RegistryStatus::Ready,
        };
        provider_registries
            .entry(registry.cloud_provider)
            .or_default()
            .push(summary);
    }

    // Build status for each supported provider
    // Available providers first, then coming soon providers
    let providers = [
        CloudProvider::Gcp,
        CloudProvider::Hetzner,
        CloudProvider::Aws,
        CloudProvider::Azure,
        CloudProvider::Scaleway,
        CloudProvider::Cyso,
    ];
    let mut statuses = Vec::new();

    for provider in providers {
        let clusters = provider_clusters.remove(&provider).unwrap_or_default();
        let registries = provider_registries.remove(&provider).unwrap_or_default();

        // Provider is connected if it has cloud credentials (NOT just resources)
        let is_connected = connected_providers.contains(provider.as_str());

        // Cloud Runner available for GCP and Hetzner when connected
        let cloud_runner_available =
            is_connected && matches!(provider, CloudProvider::Gcp | CloudProvider::Hetzner);

        let summary = build_status_summary(&clusters, &registries, cloud_runner_available);

        statuses.push(ProviderDeploymentStatus {
            provider,
            is_connected,
            clusters,
            registries,
            cloud_runner_available,
            summary,
        });
    }

    Ok(statuses)
}

/// Build a human-readable summary string for a provider
fn build_status_summary(
    clusters: &[ClusterSummary],
    registries: &[RegistrySummary],
    cloud_runner: bool,
) -> String {
    let mut parts = Vec::new();

    if cloud_runner {
        parts.push("Cloud Run".to_string());
    }

    let healthy_clusters = clusters.iter().filter(|c| c.is_healthy).count();
    if healthy_clusters > 0 {
        parts.push(format!(
            "{} cluster{}",
            healthy_clusters,
            if healthy_clusters == 1 { "" } else { "s" }
        ));
    }

    let ready_registries = registries.iter().filter(|r| r.is_ready).count();
    if ready_registries > 0 {
        parts.push(format!(
            "{} registr{}",
            ready_registries,
            if ready_registries == 1 { "y" } else { "ies" }
        ));
    }

    if parts.is_empty() {
        "Not connected".to_string()
    } else {
        parts.join(", ")
    }
}

/// Result of provider selection step
#[derive(Debug, Clone)]
pub enum ProviderSelectionResult {
    /// User selected a provider
    Selected(CloudProvider),
    /// User cancelled the wizard
    Cancelled,
}

/// Display provider selection and prompt user to choose
pub fn select_provider(statuses: &[ProviderDeploymentStatus]) -> ProviderSelectionResult {
    display_step_header(
        1,
        "Select Provider",
        "Choose which cloud provider to deploy to. You'll need to connect providers in the platform settings first.",
    );

    // Build options with status indicators
    let options: Vec<String> = statuses
        .iter()
        .map(|s| {
            let name = format!("{:?}", s.provider);
            // Check availability first - unavailable providers show "Coming Soon"
            if !s.provider.is_available() {
                format!("○ {}  {}", name.dimmed(), "(Coming Soon)".yellow())
            } else {
                let indicator = status_indicator(s.is_connected);
                if s.is_connected {
                    format!("{} {}  {}", indicator, name, s.summary.dimmed())
                } else {
                    format!("{} {}  {}", indicator, name.dimmed(), "Not connected".dimmed())
                }
            }
        })
        .collect();

    // Find available AND connected providers for validation
    let available_connected_indices: Vec<usize> = statuses
        .iter()
        .enumerate()
        .filter(|(_, s)| s.provider.is_available() && s.is_connected)
        .map(|(i, _)| i)
        .collect();

    if available_connected_indices.is_empty() {
        println!(
            "\n{}",
            "No providers connected. Connect a cloud provider in platform settings first.".red()
        );
        println!(
            "  {}",
            "Visit: https://app.syncable.dev/integrations".dimmed()
        );
        println!(
            "  {}",
            "Note: GCP and Hetzner are currently available. AWS, Azure, Scaleway, and Cyso Cloud are coming soon.".dimmed()
        );
        return ProviderSelectionResult::Cancelled;
    }

    let selection = Select::new("Select a provider:", options)
        .with_render_config(wizard_render_config())
        .with_help_message("↑↓ to move, Enter to select, Esc to cancel")
        .with_page_size(6)
        .prompt();

    match selection {
        Ok(answer) => {
            // Find which provider was selected
            let selected_idx = statuses
                .iter()
                .position(|s| {
                    let display = format!("{:?}", s.provider);
                    answer.contains(&display)
                })
                .unwrap_or(0);

            let selected_status = &statuses[selected_idx];

            // Check availability first - coming soon providers can't be selected
            if !selected_status.provider.is_available() {
                println!(
                    "\n{}",
                    format!(
                        "{} is coming soon! Currently only GCP and Hetzner are available.",
                        selected_status.provider.display_name()
                    )
                    .yellow()
                );
                return ProviderSelectionResult::Cancelled;
            }

            if !selected_status.is_connected {
                println!(
                    "\n{}",
                    format!(
                        "{:?} is not connected. Please connect it in platform settings first.",
                        selected_status.provider
                    )
                    .yellow()
                );
                return ProviderSelectionResult::Cancelled;
            }

            println!(
                "\n{} Selected: {:?}",
                "✓".green(),
                selected_status.provider
            );
            ProviderSelectionResult::Selected(selected_status.provider.clone())
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            println!("\n{}", "Wizard cancelled.".dimmed());
            ProviderSelectionResult::Cancelled
        }
        Err(_) => ProviderSelectionResult::Cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_status_summary_cloud_runner_only() {
        let summary = build_status_summary(&[], &[], true);
        assert_eq!(summary, "Cloud Run");
    }

    #[test]
    fn test_build_status_summary_full() {
        let clusters = vec![
            ClusterSummary {
                id: "c1".to_string(),
                name: "prod".to_string(),
                region: "us-central1".to_string(),
                is_healthy: true,
            },
            ClusterSummary {
                id: "c2".to_string(),
                name: "staging".to_string(),
                region: "us-east1".to_string(),
                is_healthy: false,
            },
        ];
        let registries = vec![RegistrySummary {
            id: "r1".to_string(),
            name: "main".to_string(),
            region: "us-central1".to_string(),
            is_ready: true,
        }];
        let summary = build_status_summary(&clusters, &registries, true);
        assert_eq!(summary, "Cloud Run, 1 cluster, 1 registry");
    }

    #[test]
    fn test_build_status_summary_not_connected() {
        let summary = build_status_summary(&[], &[], false);
        assert_eq!(summary, "Not connected");
    }

    #[test]
    fn test_provider_selection_result_variants() {
        let _ = ProviderSelectionResult::Selected(CloudProvider::Gcp);
        let _ = ProviderSelectionResult::Cancelled;
    }
}
