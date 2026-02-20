//! Registry selection step for deployment wizard

use crate::platform::api::types::RegistrySummary;
use crate::wizard::render::{display_step_header, status_indicator, wizard_render_config};
use colored::Colorize;
use inquire::{InquireError, Select};

/// Result of registry selection step
#[derive(Debug, Clone)]
pub enum RegistrySelectionResult {
    /// User selected an existing registry
    Selected(RegistrySummary),
    /// User wants to provision a new registry
    ProvisionNew,
    /// User wants to go back
    Back,
    /// User cancelled the wizard
    Cancelled,
}

/// Display registry selection for container image storage
pub fn select_registry(registries: &[RegistrySummary]) -> RegistrySelectionResult {
    display_step_header(
        4,
        "Select Registry",
        "Choose where to store container images. You can use an existing registry or provision a new one.",
    );

    // Filter to ready registries
    let ready_registries: Vec<&RegistrySummary> =
        registries.iter().filter(|r| r.is_ready).collect();

    // Build options
    let mut options: Vec<String> = ready_registries
        .iter()
        .map(|r| {
            format!(
                "{} {}  {}",
                status_indicator(r.is_ready),
                r.name.cyan(),
                r.region.dimmed()
            )
        })
        .collect();

    // Always offer to provision new
    options.push(format!("{} Provision new registry", "+".green()));

    // Add back option
    options.push("← Back".dimmed().to_string());

    let selection = Select::new("Select registry:", options.clone())
        .with_render_config(wizard_render_config())
        .with_help_message("↑↓ to move, Enter to select, Esc to cancel")
        .with_page_size(6)
        .prompt();

    match selection {
        Ok(answer) => {
            if answer.contains("Back") {
                return RegistrySelectionResult::Back;
            }

            if answer.contains("Provision new") {
                println!(
                    "\n{} Will provision new registry during deployment",
                    "→".cyan()
                );
                return RegistrySelectionResult::ProvisionNew;
            }

            // Find selected registry by name
            let selected = ready_registries
                .iter()
                .find(|r| answer.contains(&r.name))
                .copied();

            match selected {
                Some(registry) => {
                    println!(
                        "\n{} Selected registry: {} ({})",
                        "✓".green(),
                        registry.name,
                        registry.region
                    );
                    RegistrySelectionResult::Selected(registry.clone())
                }
                None => RegistrySelectionResult::Cancelled,
            }
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            println!("\n{}", "Wizard cancelled.".dimmed());
            RegistrySelectionResult::Cancelled
        }
        Err(_) => RegistrySelectionResult::Cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_selection_result_variants() {
        let registry = RegistrySummary {
            id: "r1".to_string(),
            name: "main".to_string(),
            region: "us-central1".to_string(),
            is_ready: true,
        };
        let _ = RegistrySelectionResult::Selected(registry);
        let _ = RegistrySelectionResult::ProvisionNew;
        let _ = RegistrySelectionResult::Back;
        let _ = RegistrySelectionResult::Cancelled;
    }
}
