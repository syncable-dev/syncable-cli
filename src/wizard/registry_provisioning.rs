//! Registry provisioning step for deployment wizard

use crate::platform::api::types::{
    CloudProvider, CreateRegistryRequest, RegistrySummary, RegistryTaskState,
};
use crate::platform::api::PlatformApiClient;
use crate::wizard::render::{display_step_header, wizard_render_config};
use colored::Colorize;
use inquire::{InquireError, Text};
use std::io::Write;
use std::time::Duration;
use tokio::time::sleep;

/// Result of registry provisioning
#[derive(Debug)]
pub enum RegistryProvisioningResult {
    /// Successfully provisioned
    Success(RegistrySummary),
    /// User cancelled
    Cancelled,
    /// Error during provisioning
    Error(String),
}

/// Provision a new artifact registry
pub async fn provision_registry(
    client: &PlatformApiClient,
    project_id: &str,
    cluster_id: &str,
    cluster_name: &str,
    provider: CloudProvider,
    region: &str,
    gcp_project_id: Option<&str>,
) -> RegistryProvisioningResult {
    display_step_header(
        4,
        "Provision Registry",
        "Create a new container registry for storing images.",
    );

    // Get registry name from user
    let registry_name = match Text::new("Registry name:")
        .with_default("main")
        .with_help_message("Lowercase alphanumeric with hyphens (e.g., main, staging)")
        .with_render_config(wizard_render_config())
        .prompt()
    {
        Ok(name) => sanitize_registry_name(&name),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return RegistryProvisioningResult::Cancelled;
        }
        Err(_) => return RegistryProvisioningResult::Cancelled,
    };

    println!(
        "\n{} Provisioning registry: {}",
        "⏳".yellow(),
        registry_name.cyan()
    );

    // Build request
    let request = CreateRegistryRequest {
        project_id: project_id.to_string(),
        cluster_id: cluster_id.to_string(),
        cluster_name: cluster_name.to_string(),
        registry_name: registry_name.clone(),
        cloud_provider: provider.as_str().to_string(),
        region: region.to_string(),
        gcp_project_id: gcp_project_id.map(|s| s.to_string()),
    };

    // Start provisioning
    let response = match client.create_registry(project_id, &request).await {
        Ok(r) => r,
        Err(e) => {
            return RegistryProvisioningResult::Error(format!(
                "Failed to start registry provisioning: {}",
                e
            ));
        }
    };

    let task_id = response.task_id;
    println!("  Task started: {}", task_id.dimmed());

    // Poll for completion with progress display
    let mut last_progress = 0;
    loop {
        sleep(Duration::from_secs(3)).await;

        let status = match client.get_registry_task_status(&task_id).await {
            Ok(s) => s,
            Err(e) => {
                return RegistryProvisioningResult::Error(format!(
                    "Failed to get task status: {}",
                    e
                ));
            }
        };

        // Show progress
        let progress = status.progress.unwrap_or(0);
        if progress > last_progress {
            let bar = progress_bar(progress);
            let message = status
                .overall_message
                .as_deref()
                .unwrap_or("Processing...");
            print!(
                "\r  {} {} {}",
                bar,
                format!("{}%", progress).cyan(),
                message.dimmed()
            );
            std::io::stdout().flush().ok();
            last_progress = progress;
        }

        match status.status {
            RegistryTaskState::Completed => {
                println!("\n{} Registry provisioned successfully!", "✓".green());

                let registry = RegistrySummary {
                    id: task_id.clone(), // Will be updated when we fetch actual registry
                    name: status.output.registry_name.unwrap_or(registry_name),
                    region: region.to_string(),
                    is_ready: true,
                };

                if let Some(url) = status.output.registry_url {
                    println!("  URL: {}", url.cyan());
                }

                return RegistryProvisioningResult::Success(registry);
            }
            RegistryTaskState::Failed => {
                println!();
                let error_msg = status
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| "Unknown error".to_string());
                return RegistryProvisioningResult::Error(error_msg);
            }
            RegistryTaskState::Cancelled => {
                println!();
                return RegistryProvisioningResult::Cancelled;
            }
            RegistryTaskState::Processing | RegistryTaskState::Unknown => {
                // Continue polling
            }
        }
    }
}

/// Create a simple progress bar
fn progress_bar(percent: u8) -> String {
    let filled = (percent as usize * 20) / 100;
    let empty = 20 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// Sanitize registry name (lowercase, alphanumeric, hyphens)
fn sanitize_registry_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_registry_name() {
        assert_eq!(sanitize_registry_name("My Registry"), "my-registry");
        assert_eq!(sanitize_registry_name("test_name"), "test-name");
        assert_eq!(sanitize_registry_name("--test--"), "test");
        assert_eq!(sanitize_registry_name("MAIN"), "main");
        assert_eq!(sanitize_registry_name("prod-123"), "prod-123");
    }

    #[test]
    fn test_progress_bar() {
        assert_eq!(progress_bar(0), "[░░░░░░░░░░░░░░░░░░░░]");
        assert_eq!(progress_bar(50), "[██████████░░░░░░░░░░]");
        assert_eq!(progress_bar(100), "[████████████████████]");
    }
}
