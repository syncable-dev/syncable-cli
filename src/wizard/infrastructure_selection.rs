//! Infrastructure selection step for the deployment wizard
//!
//! Handles region and machine type selection for Cloud Runner deployments.
//!
//! For Hetzner: Uses DYNAMIC fetching from Hetzner API - no hardcoded fallback.
//! The agent gets real-time availability and pricing for smart resource selection.
//!
//! For GCP: Uses static data (dynamic fetching not yet implemented).

use crate::platform::api::client::PlatformApiClient;
use crate::platform::api::types::CloudProvider;
use crate::wizard::cloud_provider_data::{
    get_default_machine_type, get_default_region,
    get_hetzner_regions_dynamic, get_hetzner_server_types_dynamic,
    get_machine_types_for_provider, get_regions_for_provider,
    DynamicCloudRegion, DynamicMachineType, HetznerFetchResult,
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

/// Wrapper for displaying dynamic region options with availability info
struct DynamicRegionOption {
    region: DynamicCloudRegion,
}

impl fmt::Display for DynamicRegionOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let availability = if !self.region.available_server_types.is_empty() {
            format!(" · {} types available", self.region.available_server_types.len())
        } else {
            String::new()
        };
        write!(
            f,
            "{}  {}{}",
            self.region.id.cyan(),
            format!("{}  ({})", self.region.name, self.region.location).dimmed(),
            availability.green()
        )
    }
}

/// Wrapper for displaying dynamic machine type options with pricing
struct DynamicMachineTypeOption {
    machine: DynamicMachineType,
}

impl fmt::Display for DynamicMachineTypeOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let specs = format!("{} vCPU · {:.0} GB", self.machine.cores, self.machine.memory_gb);
        let price = if self.machine.price_monthly > 0.0 {
            format!(" · €{:.2}/mo", self.machine.price_monthly)
        } else {
            String::new()
        };
        write!(
            f,
            "{}  {}{}",
            self.machine.name.cyan(),
            specs.dimmed(),
            price.green()
        )
    }
}

/// Select region and machine type for Cloud Runner deployment
///
/// Uses dynamic fetching for Hetzner to get real-time availability and pricing.
/// Falls back to static data for other providers or if API fails.
pub async fn select_infrastructure(
    provider: &CloudProvider,
    step_number: u8,
    client: Option<&PlatformApiClient>,
    project_id: Option<&str>,
) -> InfrastructureSelectionResult {
    // Select region first
    let region = match select_region(provider, step_number, client, project_id).await {
        Some(r) => r,
        None => return InfrastructureSelectionResult::Back,
    };

    // Then select machine type
    match select_machine_type(provider, &region, client, project_id).await {
        Some(machine_type) => InfrastructureSelectionResult::Selected {
            region,
            machine_type,
        },
        None => InfrastructureSelectionResult::Back,
    }
}

/// Legacy synchronous version for backward compatibility
pub fn select_infrastructure_sync(
    provider: &CloudProvider,
    step_number: u8,
) -> InfrastructureSelectionResult {
    // Select region first using static data
    let region = match select_region_static(provider, step_number) {
        Some(r) => r,
        None => return InfrastructureSelectionResult::Back,
    };

    // Then select machine type using static data
    match select_machine_type_static(provider) {
        Some(machine_type) => InfrastructureSelectionResult::Selected {
            region,
            machine_type,
        },
        None => InfrastructureSelectionResult::Back,
    }
}

/// Select region/location for deployment with dynamic fetching
async fn select_region(
    provider: &CloudProvider,
    step_number: u8,
    client: Option<&PlatformApiClient>,
    project_id: Option<&str>,
) -> Option<String> {
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

    // For Hetzner: REQUIRE dynamic fetching - no static fallback
    if *provider == CloudProvider::Hetzner {
        if let (Some(client), Some(project_id)) = (client, project_id) {
            match get_hetzner_regions_dynamic(client, project_id).await {
                HetznerFetchResult::Success(regions) => {
                    if regions.is_empty() {
                        println!(
                            "\n{} No Hetzner regions available. Please check your Hetzner account.",
                            "✗".red()
                        );
                        return None;
                    }
                    return select_region_from_dynamic(regions, provider);
                }
                HetznerFetchResult::NoCredentials => {
                    println!(
                        "\n{} Hetzner credentials not configured for this project.",
                        "✗".red()
                    );
                    println!(
                        "  {} Please add your Hetzner API token in project settings.",
                        "→".dimmed()
                    );
                    return None;
                }
                HetznerFetchResult::ApiError(err) => {
                    println!(
                        "\n{} Failed to fetch Hetzner regions: {}",
                        "✗".red(),
                        err
                    );
                    return None;
                }
            }
        } else {
            println!(
                "\n{} Cannot fetch Hetzner regions without authentication.",
                "✗".red()
            );
            return None;
        }
    }

    // For other providers: Use static data
    select_region_static(provider, step_number)
}

/// Select region from dynamic data with availability info
fn select_region_from_dynamic(
    regions: Vec<DynamicCloudRegion>,
    provider: &CloudProvider,
) -> Option<String> {
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

    let options: Vec<DynamicRegionOption> = regions
        .into_iter()
        .map(|r| DynamicRegionOption { region: r })
        .collect();

    let selection = Select::new("Select region:", options)
        .with_render_config(wizard_render_config())
        .with_starting_cursor(default_index)
        .with_help_message("Use ↑/↓ to navigate, Enter to select · Real-time availability shown")
        .prompt();

    match selection {
        Ok(selected) => {
            println!(
                "\n{} Selected region: {} ({})",
                "✓".green(),
                selected.region.name.cyan(),
                selected.region.id
            );
            Some(selected.region.id)
        }
        Err(InquireError::OperationCanceled) => None,
        Err(InquireError::OperationInterrupted) => None,
        Err(_) => None,
    }
}

/// Select region using static data (fallback)
fn select_region_static(provider: &CloudProvider, step_number: u8) -> Option<String> {
    display_step_header(
        step_number,
        &format!("Select {} Region", match provider {
            CloudProvider::Hetzner => "Hetzner",
            CloudProvider::Gcp => "GCP",
            _ => "Cloud",
        }),
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

    // Convert static regions to dynamic format for consistent display
    let options: Vec<DynamicRegionOption> = regions
        .iter()
        .map(|r| DynamicRegionOption {
            region: DynamicCloudRegion {
                id: r.id.to_string(),
                name: r.name.to_string(),
                location: r.location.to_string(),
                network_zone: String::new(),
                available_server_types: vec![],
            },
        })
        .collect();

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
            Some(selected.region.id)
        }
        Err(InquireError::OperationCanceled) => None,
        Err(InquireError::OperationInterrupted) => None,
        Err(_) => None,
    }
}

/// Select machine/instance type for deployment with dynamic fetching
async fn select_machine_type(
    provider: &CloudProvider,
    region: &str,
    client: Option<&PlatformApiClient>,
    project_id: Option<&str>,
) -> Option<String> {
    println!();
    println!(
        "{}",
        "─── Machine Type ────────────────────────────".dimmed()
    );
    println!(
        "  {}",
        "Select the VM size for your deployment.".dimmed()
    );

    // For Hetzner: REQUIRE dynamic fetching - no static fallback
    if *provider == CloudProvider::Hetzner {
        if let (Some(client), Some(project_id)) = (client, project_id) {
            match get_hetzner_server_types_dynamic(client, project_id, Some(region)).await {
                HetznerFetchResult::Success(machine_types) => {
                    if machine_types.is_empty() {
                        println!(
                            "\n{} No Hetzner server types available. Please check your Hetzner account.",
                            "✗".red()
                        );
                        return None;
                    }
                    return select_machine_type_from_dynamic(machine_types, provider, region);
                }
                HetznerFetchResult::NoCredentials => {
                    println!(
                        "\n{} Hetzner credentials not configured for this project.",
                        "✗".red()
                    );
                    println!(
                        "  {} Please add your Hetzner API token in project settings.",
                        "→".dimmed()
                    );
                    return None;
                }
                HetznerFetchResult::ApiError(err) => {
                    println!(
                        "\n{} Failed to fetch Hetzner server types: {}",
                        "✗".red(),
                        err
                    );
                    return None;
                }
            }
        } else {
            println!(
                "\n{} Cannot fetch Hetzner server types without authentication.",
                "✗".red()
            );
            return None;
        }
    }

    // For other providers: Use static data
    select_machine_type_static(provider)
}

/// Select machine type from dynamic data with pricing info
fn select_machine_type_from_dynamic(
    machine_types: Vec<DynamicMachineType>,
    provider: &CloudProvider,
    region: &str,
) -> Option<String> {
    if machine_types.is_empty() {
        println!(
            "\n{} No machine types available for this provider.",
            "⚠".yellow()
        );
        return None;
    }

    // Filter to only show types available in selected region
    let available_types: Vec<DynamicMachineType> = machine_types
        .into_iter()
        .filter(|m| m.available_in.is_empty() || m.available_in.contains(&region.to_string()))
        .collect();

    if available_types.is_empty() {
        println!(
            "\n{} No machine types available in {} region.",
            "⚠".yellow(),
            region
        );
        return None;
    }

    let default_machine = get_default_machine_type(provider);
    let default_index = available_types
        .iter()
        .position(|m| m.id == default_machine)
        .unwrap_or(0);

    let options: Vec<DynamicMachineTypeOption> = available_types
        .into_iter()
        .map(|m| DynamicMachineTypeOption { machine: m })
        .collect();

    let selection = Select::new("Select machine type:", options)
        .with_render_config(wizard_render_config())
        .with_starting_cursor(default_index)
        .with_help_message("Sorted by price · Real-time pricing shown")
        .prompt();

    match selection {
        Ok(selected) => {
            let price_info = if selected.machine.price_monthly > 0.0 {
                format!(" · €{:.2}/mo", selected.machine.price_monthly)
            } else {
                String::new()
            };
            println!(
                "\n{} Selected: {} ({} vCPU, {:.0} GB){}",
                "✓".green(),
                selected.machine.name.cyan(),
                selected.machine.cores,
                selected.machine.memory_gb,
                price_info.green()
            );
            Some(selected.machine.id)
        }
        Err(InquireError::OperationCanceled) => None,
        Err(InquireError::OperationInterrupted) => None,
        Err(_) => None,
    }
}

/// Select machine type using static data (fallback)
fn select_machine_type_static(provider: &CloudProvider) -> Option<String> {
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

    // Convert static machine types to dynamic format for consistent display
    let options: Vec<DynamicMachineTypeOption> = machine_types
        .iter()
        .map(|m| DynamicMachineTypeOption {
            machine: DynamicMachineType {
                id: m.id.to_string(),
                name: m.name.to_string(),
                cores: m.cpu.parse().unwrap_or(2),
                memory_gb: m.memory.replace(" GB", "").parse().unwrap_or(4.0),
                disk_gb: 40,
                price_monthly: 0.0,
                price_hourly: 0.0,
                available_in: vec![],
            },
        })
        .collect();

    let selection = Select::new("Select machine type:", options)
        .with_render_config(wizard_render_config())
        .with_starting_cursor(default_index)
        .with_help_message("Smaller = cheaper, Larger = more resources")
        .prompt();

    match selection {
        Ok(selected) => {
            println!(
                "\n{} Selected: {} ({} vCPU, {:.0} GB)",
                "✓".green(),
                selected.machine.name.cyan(),
                selected.machine.cores,
                selected.machine.memory_gb
            );
            Some(selected.machine.id)
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
    fn test_dynamic_region_option_display() {
        let region = DynamicCloudRegion {
            id: "nbg1".to_string(),
            name: "Nuremberg".to_string(),
            location: "Germany".to_string(),
            network_zone: "eu-central".to_string(),
            available_server_types: vec!["cx22".to_string(), "cx32".to_string()],
        };
        let option = DynamicRegionOption { region };
        let display = format!("{}", option);
        assert!(display.contains("nbg1"));
        assert!(display.contains("Nuremberg"));
        assert!(display.contains("2 types available"));
    }

    #[test]
    fn test_dynamic_machine_type_option_display() {
        let machine = DynamicMachineType {
            id: "cx22".to_string(),
            name: "CX22".to_string(),
            cores: 2,
            memory_gb: 4.0,
            disk_gb: 40,
            price_monthly: 5.95,
            price_hourly: 0.008,
            available_in: vec!["nbg1".to_string()],
        };
        let option = DynamicMachineTypeOption { machine };
        let display = format!("{}", option);
        assert!(display.contains("CX22"));
        assert!(display.contains("2 vCPU"));
        assert!(display.contains("4 GB"));
        assert!(display.contains("€5.95/mo"));
    }

    #[test]
    fn test_dynamic_machine_type_option_display_no_price() {
        let machine = DynamicMachineType {
            id: "cx22".to_string(),
            name: "CX22".to_string(),
            cores: 2,
            memory_gb: 4.0,
            disk_gb: 40,
            price_monthly: 0.0,
            price_hourly: 0.0,
            available_in: vec![],
        };
        let option = DynamicMachineTypeOption { machine };
        let display = format!("{}", option);
        assert!(display.contains("CX22"));
        assert!(!display.contains("€"));
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
