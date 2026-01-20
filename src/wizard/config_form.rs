//! Deployment configuration form for the wizard

use crate::analyzer::DiscoveredDockerfile;
use crate::platform::api::types::{CloudProvider, DeploymentTarget, WizardDeploymentConfig};
use crate::wizard::render::display_step_header;
use colored::Colorize;
use inquire::{Confirm, InquireError, Text};

/// Result of config form step
#[derive(Debug, Clone)]
pub enum ConfigFormResult {
    /// User completed the form
    Completed(WizardDeploymentConfig),
    /// User wants to go back
    Back,
    /// User cancelled the wizard
    Cancelled,
}

/// Collect deployment configuration details from user
///
/// Region, machine type, Dockerfile path, and build context are already selected
/// in previous steps. This form collects service name, port, branch, public access,
/// health check, and auto-deploy settings.
#[allow(clippy::too_many_arguments)]
pub fn collect_config(
    provider: CloudProvider,
    target: DeploymentTarget,
    cluster_id: Option<String>,
    registry_id: Option<String>,
    environment_id: &str,
    dockerfile_path: &str,
    build_context: &str,
    discovered_dockerfile: &DiscoveredDockerfile,
    region: Option<String>,
    machine_type: Option<String>,
    step_number: u8,
) -> ConfigFormResult {
    display_step_header(
        step_number,
        "Configure Service",
        "Provide details for your service deployment.",
    );

    // Show previously selected options
    println!(
        "  {} Dockerfile: {}",
        "│".dimmed(),
        dockerfile_path.cyan()
    );
    println!(
        "  {} Build context: {}",
        "│".dimmed(),
        build_context.cyan()
    );
    if let Some(ref r) = region {
        println!("  {} Region: {}", "│".dimmed(), r.cyan());
    }
    if let Some(ref m) = machine_type {
        println!("  {} Machine: {}", "│".dimmed(), m.cyan());
    }
    println!();

    // Pre-populate from discovery
    let default_name = discovered_dockerfile.suggested_service_name.clone();
    let default_port = discovered_dockerfile.suggested_port.unwrap_or(8080);

    // Get current git branch for default
    let default_branch = get_current_branch().unwrap_or_else(|| "main".to_string());

    // Service name
    let service_name = match Text::new("Service name:")
        .with_default(&default_name)
        .with_help_message("K8s-compatible name (lowercase, hyphens)")
        .prompt()
    {
        Ok(name) => sanitize_service_name(&name),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return ConfigFormResult::Cancelled;
        }
        Err(_) => return ConfigFormResult::Cancelled,
    };

    // Port
    let port_str = default_port.to_string();
    let port = match Text::new("Service port:")
        .with_default(&port_str)
        .with_help_message("Port your service listens on")
        .prompt()
    {
        Ok(p) => p.parse::<u16>().unwrap_or(default_port),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return ConfigFormResult::Cancelled;
        }
        Err(_) => return ConfigFormResult::Cancelled,
    };

    // Branch
    let branch = match Text::new("Git branch:")
        .with_default(&default_branch)
        .with_help_message("Branch to deploy from")
        .prompt()
    {
        Ok(b) => b,
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return ConfigFormResult::Cancelled;
        }
        Err(_) => return ConfigFormResult::Cancelled,
    };

    // Public access toggle (for Cloud Runner)
    let is_public = if target == DeploymentTarget::CloudRunner {
        println!();
        println!(
            "{}",
            "─── Access Configuration ────────────────────".dimmed()
        );
        match Confirm::new("Enable public access?")
            .with_default(true)
            .with_help_message("Make service accessible via public IP/URL")
            .prompt()
        {
            Ok(v) => v,
            Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
                return ConfigFormResult::Cancelled;
            }
            Err(_) => return ConfigFormResult::Cancelled,
        }
    } else {
        true // Default to public for K8s
    };

    // Health check (optional)
    let health_check_path = if target == DeploymentTarget::CloudRunner {
        match Confirm::new("Configure health check endpoint?")
            .with_default(false)
            .with_help_message("Optional HTTP health probe for your service")
            .prompt()
        {
            Ok(true) => {
                match Text::new("Health check path:")
                    .with_default("/health")
                    .with_help_message("e.g., /health, /healthz, /api/health")
                    .prompt()
                {
                    Ok(path) => Some(path),
                    Err(InquireError::OperationCanceled)
                    | Err(InquireError::OperationInterrupted) => {
                        return ConfigFormResult::Cancelled;
                    }
                    Err(_) => return ConfigFormResult::Cancelled,
                }
            }
            Ok(false) => None,
            Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
                return ConfigFormResult::Cancelled;
            }
            Err(_) => return ConfigFormResult::Cancelled,
        }
    } else {
        None
    };

    // Auto-deploy disabled by default (CI/CD not ready yet)
    let auto_deploy = false;

    // Build the config
    let config = WizardDeploymentConfig {
        service_name: Some(service_name.clone()),
        dockerfile_path: Some(dockerfile_path.to_string()),
        build_context: Some(build_context.to_string()),
        port: Some(port),
        branch: Some(branch),
        target: Some(target),
        provider: Some(provider),
        cluster_id,
        registry_id,
        environment_id: Some(environment_id.to_string()),
        auto_deploy,
        region,
        machine_type,
        is_public,
        health_check_path,
    };

    println!("\n{} Configuration complete: {}", "✓".green(), service_name);

    ConfigFormResult::Completed(config)
}

/// Get current git branch name
fn get_current_branch() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Sanitize service name for K8s compatibility
fn sanitize_service_name(name: &str) -> String {
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
    fn test_sanitize_service_name() {
        assert_eq!(sanitize_service_name("My Service"), "my-service");
        assert_eq!(sanitize_service_name("foo_bar"), "foo-bar");
        assert_eq!(sanitize_service_name("--test--"), "test");
        assert_eq!(sanitize_service_name("API Server"), "api-server");
    }

    #[test]
    fn test_config_form_result_variants() {
        let config = WizardDeploymentConfig::default();
        let _ = ConfigFormResult::Completed(config);
        let _ = ConfigFormResult::Back;
        let _ = ConfigFormResult::Cancelled;
    }
}
