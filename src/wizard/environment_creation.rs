//! Environment creation wizard for deployment targets
//!
//! Interactive wizard that guides users through creating a new environment
//! with target type selection (Kubernetes or Cloud Runner).

use crate::platform::api::client::PlatformApiClient;
use crate::platform::api::types::{CloudProvider, ClusterSummary, Environment};
use crate::wizard::cloud_provider_data::{get_default_region, get_regions_for_provider};
use crate::wizard::provider_selection::get_provider_deployment_statuses;
use crate::wizard::render::{display_step_header, wizard_render_config};
use colored::Colorize;
use inquire::{InquireError, MultiSelect, Select, Text};
use std::collections::HashMap;

/// Environment type for the API
/// "cluster" = Kubernetes cluster
/// "cloud" = Cloud Runner (serverless)
#[derive(Debug, Clone, PartialEq, Eq)]
enum EnvironmentType {
    Cluster,
    Cloud,
}

impl EnvironmentType {
    fn as_str(&self) -> &'static str {
        match self {
            EnvironmentType::Cluster => "cluster",
            EnvironmentType::Cloud => "cloud",
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            EnvironmentType::Cluster => "Kubernetes",
            EnvironmentType::Cloud => "Cloud Runner",
        }
    }
}

/// Result of environment creation wizard
#[derive(Debug)]
pub enum EnvironmentCreationResult {
    /// Environment created successfully
    Created(Environment),
    /// User cancelled the wizard
    Cancelled,
    /// An error occurred
    Error(String),
}

/// Run the environment creation wizard
///
/// Guides user through:
/// 1. Choosing environment name
/// 2. Selecting target type (Kubernetes or Cloud Runner)
/// 3. If Kubernetes: selecting a cluster
pub async fn create_environment_wizard(
    client: &PlatformApiClient,
    project_id: &str,
) -> EnvironmentCreationResult {
    display_step_header(
        1,
        "Create Environment",
        "Set up a new deployment environment for your project.",
    );

    // Step 1: Get environment name
    let name = match Text::new("Environment name:")
        .with_placeholder("e.g., production, staging, development")
        .with_help_message("Choose a descriptive name for this environment")
        .prompt()
    {
        Ok(name) => {
            if name.trim().is_empty() {
                println!("\n{}", "Environment name cannot be empty.".red());
                return EnvironmentCreationResult::Cancelled;
            }
            name.trim().to_string()
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            println!("\n{}", "Wizard cancelled.".dimmed());
            return EnvironmentCreationResult::Cancelled;
        }
        Err(e) => {
            return EnvironmentCreationResult::Error(format!("Input error: {}", e));
        }
    };

    // Step 2: Select target type
    display_step_header(
        2,
        "Select Target Type",
        "Choose how this environment will deploy services.",
    );

    let target_options = vec![
        format!(
            "{}  {}",
            "Cloud Runner".cyan(),
            "Fully managed, auto-scaling containers".dimmed()
        ),
        format!(
            "{}  {}",
            "Kubernetes".cyan(),
            "Deploy to your own K8s cluster".dimmed()
        ),
    ];

    let target_selection = Select::new("Select target type:", target_options)
        .with_render_config(wizard_render_config())
        .with_help_message("Cloud Runner: serverless, Kubernetes: full control")
        .prompt();

    let env_type = match target_selection {
        Ok(answer) => {
            if answer.contains("Cloud Runner") {
                EnvironmentType::Cloud
            } else {
                EnvironmentType::Cluster
            }
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            println!("\n{}", "Wizard cancelled.".dimmed());
            return EnvironmentCreationResult::Cancelled;
        }
        Err(e) => {
            return EnvironmentCreationResult::Error(format!("Selection error: {}", e));
        }
    };

    println!(
        "\n{} Target: {}",
        "✓".green(),
        env_type.display_name().bold()
    );

    // Step 3: If Kubernetes (cluster), select cluster
    let cluster_id = if env_type == EnvironmentType::Cluster {
        match select_cluster_for_env(client, project_id).await {
            ClusterSelectionResult::Selected(id) => Some(id),
            ClusterSelectionResult::NoClusters => {
                println!(
                    "\n{}",
                    "No Kubernetes clusters available. Please provision a cluster first.".red()
                );
                return EnvironmentCreationResult::Cancelled;
            }
            ClusterSelectionResult::Cancelled => {
                return EnvironmentCreationResult::Cancelled;
            }
            ClusterSelectionResult::Error(e) => {
                return EnvironmentCreationResult::Error(e);
            }
        }
    } else {
        None
    };

    // Step 4 (Cloud Runner only): Optional provider region defaults
    let provider_regions = if env_type == EnvironmentType::Cloud {
        select_provider_regions()
    } else {
        None
    };

    // Create the environment via API
    println!("\n{}", "Creating environment...".dimmed());

    match client
        .create_environment(
            project_id,
            &name,
            env_type.as_str(),
            cluster_id.as_deref(),
            provider_regions.as_ref(),
        )
        .await
    {
        Ok(env) => {
            println!(
                "\n{} Environment {} created successfully!",
                "✓".green().bold(),
                env.name.bold()
            );
            println!("  ID: {}", env.id.dimmed());
            println!("  Type: {}", env.environment_type);
            if let Some(cid) = &env.cluster_id {
                println!("  Cluster: {}", cid);
            }
            EnvironmentCreationResult::Created(env)
        }
        Err(e) => EnvironmentCreationResult::Error(format!("Failed to create environment: {}", e)),
    }
}

/// Result of cluster selection
enum ClusterSelectionResult {
    Selected(String),
    NoClusters,
    Cancelled,
    Error(String),
}

/// Select a Kubernetes cluster from available clusters
async fn select_cluster_for_env(
    client: &PlatformApiClient,
    project_id: &str,
) -> ClusterSelectionResult {
    display_step_header(
        3,
        "Select Cluster",
        "Choose a Kubernetes cluster for this environment.",
    );

    // Get available clusters
    let clusters: Vec<ClusterSummary> =
        match get_available_clusters_for_project(client, project_id).await {
            Ok(c) => c,
            Err(e) => return ClusterSelectionResult::Error(e),
        };

    if clusters.is_empty() {
        return ClusterSelectionResult::NoClusters;
    }

    // Build options
    let options: Vec<String> = clusters
        .iter()
        .map(|c| {
            let health = if c.is_healthy {
                "healthy".green()
            } else {
                "unhealthy".red()
            };
            format!("{} ({}) - {}", c.name.bold(), c.region.dimmed(), health)
        })
        .collect();

    let selection = Select::new("Select cluster:", options.clone())
        .with_render_config(wizard_render_config())
        .with_help_message("Choose the cluster to deploy to")
        .prompt();

    match selection {
        Ok(answer) => {
            // Find the selected cluster by matching the name at the start
            let selected_name = answer.split(" (").next().unwrap_or("");
            if let Some(cluster) = clusters.iter().find(|c| c.name == selected_name) {
                println!("\n{} Selected: {}", "✓".green(), cluster.name.bold());
                ClusterSelectionResult::Selected(cluster.id.clone())
            } else {
                ClusterSelectionResult::Error("Failed to match selected cluster".to_string())
            }
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            println!("\n{}", "Wizard cancelled.".dimmed());
            ClusterSelectionResult::Cancelled
        }
        Err(e) => ClusterSelectionResult::Error(format!("Selection error: {}", e)),
    }
}

/// Get available clusters from all connected providers for a project
async fn get_available_clusters_for_project(
    client: &PlatformApiClient,
    project_id: &str,
) -> Result<Vec<ClusterSummary>, String> {
    // Get provider deployment statuses which include cluster info
    let statuses = get_provider_deployment_statuses(client, project_id)
        .await
        .map_err(|e| format!("Failed to get provider statuses: {}", e))?;

    // Collect all clusters from connected providers
    let mut all_clusters = Vec::new();
    for status in statuses {
        if status.is_connected {
            all_clusters.extend(status.clusters);
        }
    }

    Ok(all_clusters)
}

/// Interactive provider region selection for Cloud Runner environments
///
/// Asks user which providers they want to set default regions for,
/// then presents region list per provider.
fn select_provider_regions() -> Option<HashMap<String, String>> {
    display_step_header(
        4,
        "Provider Regions (Optional)",
        "Set default regions for each cloud provider. Press Esc to skip.",
    );

    let available_providers = [
        ("GCP", CloudProvider::Gcp),
        ("Hetzner", CloudProvider::Hetzner),
        ("Azure", CloudProvider::Azure),
    ];

    let provider_labels: Vec<String> = available_providers
        .iter()
        .map(|(label, _)| label.to_string())
        .collect();

    let selected = match MultiSelect::new(
        "Select providers to set default regions for:",
        provider_labels,
    )
    .with_render_config(wizard_render_config())
    .with_help_message("Space to select, Enter to confirm, Esc to skip")
    .prompt()
    {
        Ok(s) if !s.is_empty() => s,
        _ => return None,
    };

    let mut regions = HashMap::new();

    for provider_label in &selected {
        let (_, provider) = available_providers
            .iter()
            .find(|(label, _)| label == provider_label)
            .unwrap();

        let provider_regions = get_regions_for_provider(provider);
        let default_region = get_default_region(provider);

        if provider_regions.is_empty() {
            // For providers with dynamic regions (Hetzner), use a text input
            let region = match Text::new(&format!("{} region:", provider_label))
                .with_default(default_region)
                .with_render_config(wizard_render_config())
                .prompt()
            {
                Ok(r) => r,
                Err(_) => continue,
            };
            regions.insert(provider.as_str().to_string(), region);
        } else {
            let region_labels: Vec<String> = provider_regions
                .iter()
                .map(|r| format!("{} - {} ({})", r.id, r.name, r.location))
                .collect();

            let default_idx = provider_regions
                .iter()
                .position(|r| r.id == default_region)
                .unwrap_or(0);

            let region = match Select::new(
                &format!("{} region:", provider_label),
                region_labels,
            )
            .with_render_config(wizard_render_config())
            .with_starting_cursor(default_idx)
            .prompt()
            {
                Ok(r) => {
                    // Extract region ID from the display string (before first " - ")
                    r.split(" - ").next().unwrap_or(default_region).to_string()
                }
                Err(_) => continue,
            };
            regions.insert(provider.as_str().to_string(), region);
        }
    }

    if regions.is_empty() {
        None
    } else {
        println!("\n{} Provider regions configured:", "✓".green());
        for (provider, region) in &regions {
            println!("  {}: {}", provider, region.bold());
        }
        Some(regions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_creation_result_variants() {
        let created = EnvironmentCreationResult::Created(Environment {
            id: "env-1".to_string(),
            name: "test".to_string(),
            project_id: "proj-1".to_string(),
            environment_type: "cloud".to_string(),
            cluster_id: None,
            namespace: None,
            description: None,
            is_active: true,
            created_at: None,
            updated_at: None,
            provider_regions: None,
        });
        assert!(matches!(created, EnvironmentCreationResult::Created(_)));

        let cancelled = EnvironmentCreationResult::Cancelled;
        assert!(matches!(cancelled, EnvironmentCreationResult::Cancelled));

        let error = EnvironmentCreationResult::Error("test error".to_string());
        assert!(matches!(error, EnvironmentCreationResult::Error(_)));
    }

    #[test]
    fn test_environment_type_as_str() {
        assert_eq!(EnvironmentType::Cluster.as_str(), "cluster");
        assert_eq!(EnvironmentType::Cloud.as_str(), "cloud");
    }

    #[test]
    fn test_environment_type_display_name() {
        assert_eq!(EnvironmentType::Cluster.display_name(), "Kubernetes");
        assert_eq!(EnvironmentType::Cloud.display_name(), "Cloud Runner");
    }
}
