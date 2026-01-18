//! Wizard orchestration - ties all steps together

use crate::analyzer::discover_dockerfiles_for_deployment;
use crate::platform::api::types::{
    build_cloud_runner_config, ConnectRepositoryRequest, CreateDeploymentConfigRequest,
    DeploymentTarget, ProjectRepository, TriggerDeploymentRequest, WizardDeploymentConfig,
};
use crate::platform::api::PlatformApiClient;
use crate::wizard::{
    collect_config, get_provider_deployment_statuses, provision_registry, select_cluster,
    select_dockerfile, select_infrastructure, select_provider, select_registry, select_repository,
    select_target, ClusterSelectionResult, ConfigFormResult, DockerfileSelectionResult,
    InfrastructureSelectionResult, ProviderSelectionResult, RegistryProvisioningResult,
    RegistrySelectionResult, RepositorySelectionResult, TargetSelectionResult,
};
use colored::Colorize;
use inquire::{Confirm, InquireError};
use std::path::Path;

/// Deployment result with task ID for tracking
#[derive(Debug, Clone)]
pub struct DeploymentInfo {
    /// The deployment config ID
    pub config_id: String,
    /// Backstage task ID for tracking progress
    pub task_id: String,
    /// Service name that was deployed
    pub service_name: String,
}

/// Result of running the wizard
#[derive(Debug)]
pub enum WizardResult {
    /// Wizard completed and deployment triggered
    Deployed(DeploymentInfo),
    /// Wizard completed successfully (config created but not deployed)
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

    // Step 0: Repository selection (auto-detect or ask)
    let repository = match select_repository(client, project_id, project_path).await {
        RepositorySelectionResult::Selected(repo) => repo,
        RepositorySelectionResult::ConnectNew(available) => {
            // Connect the repository first
            println!("{} Connecting repository...", "→".cyan());

            // Extract owner from full_name if not provided
            let owner = available
                .owner
                .clone()
                .unwrap_or_else(|| available.full_name.split('/').next().unwrap_or("").to_string());

            let connect_request = ConnectRepositoryRequest {
                project_id: project_id.to_string(),
                repository_id: available.id,
                repository_name: available.name.clone(),
                repository_full_name: available.full_name.clone(),
                repository_owner: owner.clone(),
                repository_private: available.private,
                default_branch: available.default_branch.clone().or(Some("main".to_string())),
                connection_type: Some("app".to_string()),
                github_installation_id: available.installation_id,
                repository_type: Some("application".to_string()),
            };
            match client.connect_repository(&connect_request).await {
                Ok(response) => {
                    println!("{} Repository connected!", "✓".green());
                    // Construct ProjectRepository from the response and available info
                    ProjectRepository {
                        id: response.id,
                        project_id: response.project_id,
                        repository_id: response.repository_id,
                        repository_name: available.name,
                        repository_full_name: response.repository_full_name,
                        repository_owner: owner,
                        repository_private: available.private,
                        default_branch: available.default_branch,
                        is_active: response.is_active,
                        connection_type: Some("app".to_string()),
                        repository_type: Some("application".to_string()),
                        is_primary_git_ops: None,
                        github_installation_id: available.installation_id,
                        user_id: None,
                        created_at: None,
                        updated_at: None,
                    }
                }
                Err(e) => {
                    return WizardResult::Error(format!("Failed to connect repository: {}", e));
                }
            }
        }
        RepositorySelectionResult::NeedsGitHubApp { installation_url, org_name } => {
            println!(
                "\n{} Please install the Syncable GitHub App for organization '{}' first.",
                "⚠".yellow(),
                org_name.cyan()
            );
            println!("Installation URL: {}", installation_url);
            return WizardResult::Cancelled;
        }
        RepositorySelectionResult::NoInstallations { installation_url } => {
            println!(
                "\n{} No GitHub App installations found. Please install the app first.",
                "⚠".yellow()
            );
            println!("Installation URL: {}", installation_url);
            return WizardResult::Cancelled;
        }
        RepositorySelectionResult::NoRepositories => {
            return WizardResult::Error(
                "No repositories available. Please install the Syncable GitHub App first."
                    .to_string(),
            );
        }
        RepositorySelectionResult::Cancelled => return WizardResult::Cancelled,
        RepositorySelectionResult::Error(e) => return WizardResult::Error(e),
    };

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

    // Step 3: Infrastructure selection for Cloud Runner OR Cluster selection for K8s
    let (cluster_id, region, machine_type) = if target == DeploymentTarget::CloudRunner {
        // Cloud Runner: Select region and machine type
        match select_infrastructure(&provider, 3) {
            InfrastructureSelectionResult::Selected {
                region,
                machine_type,
            } => (None, Some(region), Some(machine_type)),
            InfrastructureSelectionResult::Back => {
                // Go back (restart wizard for simplicity)
                return Box::pin(run_wizard(client, project_id, environment_id, project_path)).await;
            }
            InfrastructureSelectionResult::Cancelled => return WizardResult::Cancelled,
        }
    } else {
        // Kubernetes: Select cluster
        match select_cluster(&provider_status.clusters) {
            ClusterSelectionResult::Selected(c) => (Some(c.id), None, None),
            ClusterSelectionResult::Back => {
                // Go back to target selection (restart wizard for simplicity)
                return Box::pin(run_wizard(client, project_id, environment_id, project_path))
                    .await;
            }
            ClusterSelectionResult::Cancelled => return WizardResult::Cancelled,
        }
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
    let config = match collect_config(
        provider.clone(),
        target.clone(),
        cluster_id.clone(),
        registry_id.clone(),
        environment_id,
        &dockerfile_path,
        &build_context,
        &selected_dockerfile,
        region.clone(),
        machine_type.clone(),
        6,
    ) {
        ConfigFormResult::Completed(config) => config,
        ConfigFormResult::Back => {
            // Restart wizard
            return Box::pin(run_wizard(client, project_id, environment_id, project_path)).await;
        }
        ConfigFormResult::Cancelled => return WizardResult::Cancelled,
    };

    // Show summary
    display_summary(&config);

    // Step 7: Confirm and deploy
    println!();
    let should_deploy = match Confirm::new("Deploy now?")
        .with_default(true)
        .with_help_message("This will create the deployment configuration and start the deployment")
        .prompt()
    {
        Ok(v) => v,
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return WizardResult::Cancelled;
        }
        Err(_) => return WizardResult::Cancelled,
    };

    if !should_deploy {
        println!("{}", "Deployment skipped. Configuration saved.".dimmed());
        return WizardResult::Success(config);
    }

    // Create deployment configuration
    println!();
    println!("{}", "Creating deployment configuration...".dimmed());

    let deploy_request = CreateDeploymentConfigRequest {
        project_id: project_id.to_string(),
        service_name: config.service_name.clone().unwrap_or_default(),
        repository_id: repository.repository_id,
        repository_full_name: repository.repository_full_name.clone(),
        dockerfile_path: config.dockerfile_path.clone(),
        build_context: config.build_context.clone(),
        port: config.port.unwrap_or(8080) as i32,
        branch: config.branch.clone().unwrap_or_else(|| "main".to_string()),
        target_type: target.as_str().to_string(),
        cloud_provider: provider.as_str().to_string(),
        environment_id: environment_id.to_string(),
        cluster_id: cluster_id.clone(),
        registry_id: registry_id.clone(),
        auto_deploy_enabled: config.auto_deploy,
        is_public: Some(config.is_public),
        cloud_runner_config: if target == DeploymentTarget::CloudRunner {
            Some(build_cloud_runner_config(
                &provider,
                region.as_deref().unwrap_or(""),
                machine_type.as_deref().unwrap_or(""),
                config.is_public,
                config.health_check_path.as_deref(),
            ))
        } else {
            None
        },
    };

    let deployment_config = match client.create_deployment_config(&deploy_request).await {
        Ok(config) => config,
        Err(e) => {
            return WizardResult::Error(format!("Failed to create deployment config: {}", e));
        }
    };

    println!(
        "{} Deployment configuration created: {}",
        "✓".green(),
        deployment_config.id.dimmed()
    );

    // Trigger deployment
    println!("{}", "Triggering deployment...".dimmed());

    let trigger_request = TriggerDeploymentRequest {
        project_id: project_id.to_string(),
        config_id: deployment_config.id.clone(),
        commit_sha: None, // Use latest from branch
    };

    match client.trigger_deployment(&trigger_request).await {
        Ok(response) => {
            println!();
            println!(
                "{}",
                "═══════════════════════════════════════════════════════════════".bright_green()
            );
            println!(
                "{}  Deployment started!",
                "✓".bright_green().bold()
            );
            println!(
                "{}",
                "═══════════════════════════════════════════════════════════════".bright_green()
            );
            println!();
            println!("  Service:  {}", config.service_name.as_deref().unwrap_or("").cyan());
            println!("  Task ID:  {}", response.backstage_task_id.dimmed());
            println!("  Status:   {}", response.status.yellow());
            println!();
            println!(
                "{}",
                "Track progress: sync-ctl deploy status <task-id>".dimmed()
            );
            println!();

            WizardResult::Deployed(DeploymentInfo {
                config_id: deployment_config.id,
                task_id: response.backstage_task_id,
                service_name: config.service_name.unwrap_or_default(),
            })
        }
        Err(e) => {
            WizardResult::Error(format!("Failed to trigger deployment: {}", e))
        }
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
    if let Some(ref region) = config.region {
        println!("  Region:       {}", region.cyan());
    }
    if let Some(ref machine) = config.machine_type {
        println!("  Machine:      {}", machine.cyan());
    }
    if let Some(ref branch) = config.branch {
        println!("  Branch:       {}", branch);
    }
    if let Some(port) = config.port {
        println!("  Port:         {}", port);
    }
    println!(
        "  Public:       {}",
        if config.is_public {
            "Yes".green()
        } else {
            "No".yellow()
        }
    );
    if let Some(ref health) = config.health_check_path {
        println!("  Health check: {}", health.cyan());
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
