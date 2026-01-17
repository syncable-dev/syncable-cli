//! Wizard orchestration - ties all steps together

use crate::analyzer::discover_dockerfiles_for_deployment;
use crate::platform::api::types::{DeploymentTarget, WizardDeploymentConfig};
use crate::platform::api::PlatformApiClient;
use crate::wizard::{
    collect_config, get_provider_deployment_statuses, provision_registry, select_cluster,
    select_dockerfile, select_provider, select_registry, select_target, ClusterSelectionResult,
    ConfigFormResult, DockerfileSelectionResult, ProviderSelectionResult,
    RegistryProvisioningResult, RegistrySelectionResult, TargetSelectionResult,
};
use colored::Colorize;
use std::path::Path;

/// Result of running the wizard
#[derive(Debug)]
pub enum WizardResult {
    /// Wizard completed successfully
    Success(WizardDeploymentConfig),
    /// User wants to start agent to create Dockerfile
    StartAgent(String),
    /// User cancelled the wizard
    Cancelled,
    /// An error occurred
    Error(String),
}

/// Run the deployment wizard
pub async fn run_wizard(
    client: &PlatformApiClient,
    project_id: &str,
    environment_id: &str,
    project_path: &Path,
) -> WizardResult {
    println!();
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════".bright_cyan()
    );
    println!(
        "{}",
        "                    Deployment Wizard                          "
            .bright_cyan()
            .bold()
    );
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════".bright_cyan()
    );

    // Step 1: Provider selection
    let provider_statuses = match get_provider_deployment_statuses(client, project_id).await {
        Ok(s) => s,
        Err(e) => {
            return WizardResult::Error(format!("Failed to fetch provider status: {}", e));
        }
    };

    let provider = match select_provider(&provider_statuses) {
        ProviderSelectionResult::Selected(p) => p,
        ProviderSelectionResult::Cancelled => return WizardResult::Cancelled,
    };

    // Get status for selected provider
    let provider_status = provider_statuses
        .iter()
        .find(|s| s.provider == provider)
        .expect("Selected provider must exist in statuses");

    // Step 2: Target selection (with back navigation)
    let target = match select_target(provider_status) {
        TargetSelectionResult::Selected(t) => t,
        TargetSelectionResult::Back => {
            // Restart from provider selection
            return Box::pin(run_wizard(client, project_id, environment_id, project_path)).await;
        }
        TargetSelectionResult::Cancelled => return WizardResult::Cancelled,
    };

    // Step 3: Cluster selection (if Kubernetes)
    let cluster_id = if target == DeploymentTarget::Kubernetes {
        match select_cluster(&provider_status.clusters) {
            ClusterSelectionResult::Selected(c) => Some(c.id),
            ClusterSelectionResult::Back => {
                // Go back to target selection (restart wizard for simplicity)
                return Box::pin(run_wizard(client, project_id, environment_id, project_path))
                    .await;
            }
            ClusterSelectionResult::Cancelled => return WizardResult::Cancelled,
        }
    } else {
        None
    };

    // Step 4: Registry selection
    let registry_id = loop {
        match select_registry(&provider_status.registries) {
            RegistrySelectionResult::Selected(r) => break Some(r.id),
            RegistrySelectionResult::ProvisionNew => {
                // Get cluster info for provisioning
                let (prov_cluster_id, prov_cluster_name, prov_region) =
                    if let Some(ref cid) = cluster_id {
                        // Use selected cluster
                        let cluster = provider_status
                            .clusters
                            .iter()
                            .find(|c| c.id == *cid)
                            .expect("Selected cluster must exist");
                        (cid.clone(), cluster.name.clone(), cluster.region.clone())
                    } else {
                        // For Cloud Runner, use first available cluster for registry provisioning
                        if let Some(cluster) = provider_status.clusters.first() {
                            (
                                cluster.id.clone(),
                                cluster.name.clone(),
                                cluster.region.clone(),
                            )
                        } else {
                            return WizardResult::Error(
                                "No cluster available for registry provisioning".to_string(),
                            );
                        }
                    };

                // Provision the registry
                match provision_registry(
                    client,
                    project_id,
                    &prov_cluster_id,
                    &prov_cluster_name,
                    provider.clone(),
                    &prov_region,
                    None, // GCP project ID resolved by backend
                )
                .await
                {
                    RegistryProvisioningResult::Success(registry) => {
                        break Some(registry.id);
                    }
                    RegistryProvisioningResult::Cancelled => {
                        return WizardResult::Cancelled;
                    }
                    RegistryProvisioningResult::Error(e) => {
                        eprintln!("{} {}", "Registry provisioning failed:".red(), e);
                        // Allow retry - loop back to selection
                        continue;
                    }
                }
            }
            RegistrySelectionResult::Back => {
                // Go back (restart wizard for simplicity)
                return Box::pin(run_wizard(client, project_id, environment_id, project_path)).await;
            }
            RegistrySelectionResult::Cancelled => return WizardResult::Cancelled,
        }
    };

    // Step 5: Dockerfile selection
    let dockerfiles = discover_dockerfiles_for_deployment(project_path).unwrap_or_default();
    let (selected_dockerfile, build_context) = match select_dockerfile(&dockerfiles, project_path) {
        DockerfileSelectionResult::Selected {
            dockerfile,
            build_context,
        } => (dockerfile, build_context),
        DockerfileSelectionResult::StartAgent(prompt) => {
            return WizardResult::StartAgent(prompt);
        }
        DockerfileSelectionResult::Back => {
            // Go back (restart wizard for simplicity)
            return Box::pin(run_wizard(client, project_id, environment_id, project_path)).await;
        }
        DockerfileSelectionResult::Cancelled => return WizardResult::Cancelled,
    };

    // Get relative dockerfile path for config
    let dockerfile_path = selected_dockerfile
        .path
        .strip_prefix(project_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| selected_dockerfile.path.to_string_lossy().to_string());

    // Step 6: Config form
    match collect_config(
        provider,
        target,
        cluster_id,
        registry_id,
        environment_id,
        &dockerfile_path,
        &build_context,
        &selected_dockerfile,
    ) {
        ConfigFormResult::Completed(config) => {
            // Show summary
            display_summary(&config);
            WizardResult::Success(config)
        }
        ConfigFormResult::Back => {
            // Restart wizard
            Box::pin(run_wizard(client, project_id, environment_id, project_path)).await
        }
        ConfigFormResult::Cancelled => WizardResult::Cancelled,
    }
}

/// Display a summary of the deployment configuration
fn display_summary(config: &WizardDeploymentConfig) {
    println!();
    println!(
        "{}",
        "─────────────────────────────────────────────────────────────────".dimmed()
    );
    println!("{}", " Deployment Summary ".bright_green().bold());
    println!(
        "{}",
        "─────────────────────────────────────────────────────────────────".dimmed()
    );

    if let Some(ref name) = config.service_name {
        println!("  Service:      {}", name.cyan());
    }
    if let Some(ref target) = config.target {
        println!("  Target:       {}", target.display_name());
    }
    if let Some(ref provider) = config.provider {
        println!("  Provider:     {:?}", provider);
    }
    if let Some(ref branch) = config.branch {
        println!("  Branch:       {}", branch);
    }
    if let Some(port) = config.port {
        println!("  Port:         {}", port);
    }
    println!(
        "  Auto-deploy:  {}",
        if config.auto_deploy {
            "Yes".green()
        } else {
            "No".yellow()
        }
    );

    println!(
        "{}",
        "─────────────────────────────────────────────────────────────────".dimmed()
    );
    println!();
}
