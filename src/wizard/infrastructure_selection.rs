//! Infrastructure selection step for the deployment wizard
//!
//! Handles region and machine type selection for Cloud Runner deployments.

use crate::platform::api::types::CloudProvider;
use crate::wizard::cloud_provider_data::{
    get_default_machine_type, get_default_region, get_machine_types_for_provider,
    get_regions_for_provider, CloudRegion, MachineType,
};
use crate::wizard::render::{display_step_header, wizard_render_config};
use colored::Colorize;
use inquire::{InquireError, Select};
use std::fmt;

/// Result of infrastructure selection step
#[derive(Debug, Clone)]
pub enum InfrastructureSelectionResult {
    /// User selected region and machine type
    Selected {
        region: String,
        machine_type: String,
    },
    /// User wants to go back
    Back,
    /// User cancelled the wizard
    Cancelled,
}

/// Wrapper for displaying region options in the selection menu
struct RegionOption<'a> {
    region: &'a CloudRegion,
}

impl<'a> fmt::Display for RegionOption<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}  {}",
            self.region.id.cyan(),
            format!("{}  ({})", self.region.name, self.region.location).dimmed()
        )
    }
}

/// Wrapper for displaying machine type options in the selection menu
struct MachineTypeOption<'a> {
    machine: &'a MachineType,
}

impl<'a> fmt::Display for MachineTypeOption<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let specs = format!("{} vCPU · {}", self.machine.cpu, self.machine.memory);
        let desc = self
            .machine
            .description
            .map(|d| format!(" · {}", d))
            .unwrap_or_default();
        write!(
            f,
            "{}  {}{}",
            self.machine.name.cyan(),
            specs.dimmed(),
            desc.dimmed()
        )
    }
}

/// Select region and machine type for Cloud Runner deployment
pub fn select_infrastructure(
    provider: &CloudProvider,
    step_number: u8,
) -> InfrastructureSelectionResult {
    // Select region first
    let region = match select_region(provider, step_number) {
        Some(r) => r,
        None => return InfrastructureSelectionResult::Back,
    };

    // Then select machine type
    match select_machine_type(provider, &region) {
        Some(machine_type) => InfrastructureSelectionResult::Selected {
            region,
            machine_type,
        },
        None => InfrastructureSelectionResult::Back,
    }
}

/// Select region/location for deployment
fn select_region(provider: &CloudProvider, step_number: u8) -> Option<String> {
    let provider_name = match provider {
        CloudProvider::Hetzner => "Hetzner",
        CloudProvider::Gcp => "GCP",
        _ => "Cloud",
    };

    display_step_header(
        step_number,
        &format!("Select {} Region", provider_name),
        "Choose the geographic location for your deployment.",
    );

    let regions = get_regions_for_provider(provider);
    if regions.is_empty() {
        println!(
            "\n{} No regions available for this provider.",
            "⚠".yellow()
        );
        return None;
    }

    let default_region = get_default_region(provider);
    let default_index = regions
        .iter()
        .position(|r| r.id == default_region)
        .unwrap_or(0);

    let options: Vec<RegionOption> = regions.iter().map(|r| RegionOption { region: r }).collect();

    let selection = Select::new("Select region:", options)
        .with_render_config(wizard_render_config())
        .with_starting_cursor(default_index)
        .with_help_message("Use ↑/↓ to navigate, Enter to select")
        .prompt();

    match selection {
        Ok(selected) => {
            println!(
                "\n{} Selected region: {} ({})",
                "✓".green(),
                selected.region.name.cyan(),
                selected.region.id
            );
            Some(selected.region.id.to_string())
        }
        Err(InquireError::OperationCanceled) => None,
        Err(InquireError::OperationInterrupted) => None,
        Err(_) => None,
    }
}

/// Select machine/instance type for deployment
fn select_machine_type(provider: &CloudProvider, _region: &str) -> Option<String> {
    println!();
    println!(
        "{}",
        "─── Machine Type ────────────────────────────".dimmed()
    );
    println!(
        "  {}",
        "Select the VM size for your deployment.".dimmed()
    );

    let machine_types = get_machine_types_for_provider(provider);
    if machine_types.is_empty() {
        println!(
            "\n{} No machine types available for this provider.",
            "⚠".yellow()
        );
        return None;
    }

    let default_machine = get_default_machine_type(provider);
    let default_index = machine_types
        .iter()
        .position(|m| m.id == default_machine)
        .unwrap_or(0);

    let options: Vec<MachineTypeOption> = machine_types
        .iter()
        .map(|m| MachineTypeOption { machine: m })
        .collect();

    let selection = Select::new("Select machine type:", options)
        .with_render_config(wizard_render_config())
        .with_starting_cursor(default_index)
        .with_help_message("Smaller = cheaper, Larger = more resources")
        .prompt();

    match selection {
        Ok(selected) => {
            println!(
                "\n{} Selected: {} ({} vCPU, {})",
                "✓".green(),
                selected.machine.name.cyan(),
                selected.machine.cpu,
                selected.machine.memory
            );
            Some(selected.machine.id.to_string())
        }
        Err(InquireError::OperationCanceled) => None,
        Err(InquireError::OperationInterrupted) => None,
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_option_display() {
        let region = CloudRegion {
            id: "nbg1",
            name: "Nuremberg",
            location: "Germany",
        };
        let option = RegionOption { region: &region };
        let display = format!("{}", option);
        assert!(display.contains("nbg1"));
        assert!(display.contains("Nuremberg"));
    }

    #[test]
    fn test_machine_type_option_display() {
        let machine = MachineType {
            id: "cx22",
            name: "CX22",
            cpu: "2",
            memory: "4 GB",
            description: Some("Shared Intel"),
        };
        let option = MachineTypeOption { machine: &machine };
        let display = format!("{}", option);
        assert!(display.contains("CX22"));
        assert!(display.contains("2 vCPU"));
        assert!(display.contains("4 GB"));
    }

    #[test]
    fn test_infrastructure_selection_result_variants() {
        let selected = InfrastructureSelectionResult::Selected {
            region: "nbg1".to_string(),
            machine_type: "cx22".to_string(),
        };
        matches!(selected, InfrastructureSelectionResult::Selected { .. });

        let _ = InfrastructureSelectionResult::Back;
        let _ = InfrastructureSelectionResult::Cancelled;
    }
}
