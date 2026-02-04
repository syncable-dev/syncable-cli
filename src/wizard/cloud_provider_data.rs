//! Cloud provider regions and machine types for the deployment wizard
//!
//! This module provides both static fallback data and dynamic fetching
//! from the Hetzner API for real-time availability and pricing information.
//!
//! The dynamic functions use the Syncable Platform API which fetches from
//! Hetzner Cloud API using the customer's stored credentials. If the API
//! call fails, static fallback data is returned.

use crate::platform::api::client::PlatformApiClient;
use crate::platform::api::types::{CloudProvider, LocationWithAvailability, ServerTypeSummary};

/// A cloud region/location option
#[derive(Debug, Clone)]
pub struct CloudRegion {
    /// Region ID (e.g., "nbg1", "us-central1")
    pub id: &'static str,
    /// Human-readable name (e.g., "Nuremberg", "Iowa")
    pub name: &'static str,
    /// Geographic location (e.g., "Germany", "US Central")
    pub location: &'static str,
}

/// A machine/instance type option
#[derive(Debug, Clone)]
pub struct MachineType {
    /// Machine type ID (e.g., "cx22", "e2-small")
    pub id: &'static str,
    /// Display name
    pub name: &'static str,
    /// Number of vCPUs (as string to handle fractional)
    pub cpu: &'static str,
    /// Memory amount (e.g., "4 GB")
    pub memory: &'static str,
    /// Optional description (e.g., "Shared Intel", "ARM64")
    pub description: Option<&'static str>,
}

// =============================================================================
// Hetzner Cloud
// =============================================================================

/// Hetzner Cloud locations
pub static HETZNER_LOCATIONS: &[CloudRegion] = &[
    // Europe
    CloudRegion { id: "nbg1", name: "Nuremberg", location: "Germany" },
    CloudRegion { id: "fsn1", name: "Falkenstein", location: "Germany" },
    CloudRegion { id: "hel1", name: "Helsinki", location: "Finland" },
    // Americas
    CloudRegion { id: "ash", name: "Ashburn", location: "US East" },
    CloudRegion { id: "hil", name: "Hillsboro", location: "US West" },
    // Asia Pacific
    CloudRegion { id: "sin", name: "Singapore", location: "Southeast Asia" },
];

/// Hetzner Cloud server types (updated January 2026 naming)
pub static HETZNER_SERVER_TYPES: &[MachineType] = &[
    // Shared vCPU - CX Series (Intel/AMD cost-optimized)
    MachineType { id: "cx23", name: "CX23", cpu: "2", memory: "4 GB", description: Some("Shared Intel/AMD") },
    MachineType { id: "cx33", name: "CX33", cpu: "4", memory: "8 GB", description: Some("Shared Intel/AMD") },
    MachineType { id: "cx43", name: "CX43", cpu: "8", memory: "16 GB", description: Some("Shared Intel/AMD") },
    MachineType { id: "cx53", name: "CX53", cpu: "16", memory: "32 GB", description: Some("Shared Intel/AMD") },
    // Shared vCPU - CPX Series (AMD regular)
    MachineType { id: "cpx22", name: "CPX22", cpu: "2", memory: "4 GB", description: Some("Shared AMD") },
    MachineType { id: "cpx32", name: "CPX32", cpu: "4", memory: "8 GB", description: Some("Shared AMD") },
    MachineType { id: "cpx42", name: "CPX42", cpu: "8", memory: "16 GB", description: Some("Shared AMD") },
    MachineType { id: "cpx52", name: "CPX52", cpu: "12", memory: "24 GB", description: Some("Shared AMD") },
    MachineType { id: "cpx62", name: "CPX62", cpu: "16", memory: "32 GB", description: Some("Shared AMD") },
    // Dedicated vCPU - CCX Series (AMD)
    MachineType { id: "ccx13", name: "CCX13", cpu: "2", memory: "8 GB", description: Some("Dedicated AMD") },
    MachineType { id: "ccx23", name: "CCX23", cpu: "4", memory: "16 GB", description: Some("Dedicated AMD") },
    MachineType { id: "ccx33", name: "CCX33", cpu: "8", memory: "32 GB", description: Some("Dedicated AMD") },
    MachineType { id: "ccx43", name: "CCX43", cpu: "16", memory: "64 GB", description: Some("Dedicated AMD") },
    MachineType { id: "ccx53", name: "CCX53", cpu: "32", memory: "128 GB", description: Some("Dedicated AMD") },
    MachineType { id: "ccx63", name: "CCX63", cpu: "48", memory: "192 GB", description: Some("Dedicated AMD") },
    // ARM - CAX Series (Ampere)
    MachineType { id: "cax11", name: "CAX11", cpu: "2", memory: "4 GB", description: Some("ARM64 Ampere") },
    MachineType { id: "cax21", name: "CAX21", cpu: "4", memory: "8 GB", description: Some("ARM64 Ampere") },
    MachineType { id: "cax31", name: "CAX31", cpu: "8", memory: "16 GB", description: Some("ARM64 Ampere") },
    MachineType { id: "cax41", name: "CAX41", cpu: "16", memory: "32 GB", description: Some("ARM64 Ampere") },
];

// =============================================================================
// GCP (Google Cloud Platform)
// =============================================================================

/// GCP regions
pub static GCP_REGIONS: &[CloudRegion] = &[
    // Americas
    CloudRegion { id: "us-central1", name: "Iowa", location: "US Central" },
    CloudRegion { id: "us-east1", name: "South Carolina", location: "US East" },
    CloudRegion { id: "us-east4", name: "Virginia", location: "US East" },
    CloudRegion { id: "us-west1", name: "Oregon", location: "US West" },
    CloudRegion { id: "us-west2", name: "Los Angeles", location: "US West" },
    // Europe
    CloudRegion { id: "europe-west1", name: "Belgium", location: "Europe" },
    CloudRegion { id: "europe-west2", name: "London", location: "UK" },
    CloudRegion { id: "europe-west3", name: "Frankfurt", location: "Germany" },
    CloudRegion { id: "europe-west4", name: "Netherlands", location: "Europe" },
    CloudRegion { id: "europe-north1", name: "Finland", location: "Europe" },
    // Asia Pacific
    CloudRegion { id: "asia-east1", name: "Taiwan", location: "Asia Pacific" },
    CloudRegion { id: "asia-northeast1", name: "Tokyo", location: "Japan" },
    CloudRegion { id: "asia-southeast1", name: "Singapore", location: "Southeast Asia" },
    CloudRegion { id: "australia-southeast1", name: "Sydney", location: "Australia" },
];

/// GCP machine types (Compute Engine)
pub static GCP_MACHINE_TYPES: &[MachineType] = &[
    // E2 Series (Cost-optimized)
    MachineType { id: "e2-micro", name: "e2-micro", cpu: "0.25", memory: "1 GB", description: Some("Shared-core") },
    MachineType { id: "e2-small", name: "e2-small", cpu: "0.5", memory: "2 GB", description: Some("Shared-core") },
    MachineType { id: "e2-medium", name: "e2-medium", cpu: "1", memory: "4 GB", description: Some("Shared-core") },
    MachineType { id: "e2-standard-2", name: "e2-standard-2", cpu: "2", memory: "8 GB", description: None },
    MachineType { id: "e2-standard-4", name: "e2-standard-4", cpu: "4", memory: "16 GB", description: None },
    MachineType { id: "e2-standard-8", name: "e2-standard-8", cpu: "8", memory: "32 GB", description: None },
    // N2 Series (Balanced)
    MachineType { id: "n2-standard-2", name: "n2-standard-2", cpu: "2", memory: "8 GB", description: None },
    MachineType { id: "n2-standard-4", name: "n2-standard-4", cpu: "4", memory: "16 GB", description: None },
    MachineType { id: "n2-standard-8", name: "n2-standard-8", cpu: "8", memory: "32 GB", description: None },
];

// =============================================================================
// Helper Functions
// =============================================================================

/// Get regions for a cloud provider
pub fn get_regions_for_provider(provider: &CloudProvider) -> &'static [CloudRegion] {
    match provider {
        CloudProvider::Hetzner => HETZNER_LOCATIONS,
        CloudProvider::Gcp => GCP_REGIONS,
        _ => &[], // AWS, Azure not yet supported for Cloud Runner
    }
}

/// Get machine types for a cloud provider
pub fn get_machine_types_for_provider(provider: &CloudProvider) -> &'static [MachineType] {
    match provider {
        CloudProvider::Hetzner => HETZNER_SERVER_TYPES,
        CloudProvider::Gcp => GCP_MACHINE_TYPES,
        _ => &[], // AWS, Azure not yet supported for Cloud Runner
    }
}

/// Get default region for a provider
pub fn get_default_region(provider: &CloudProvider) -> &'static str {
    match provider {
        CloudProvider::Hetzner => "nbg1",
        CloudProvider::Gcp => "us-central1",
        _ => "",
    }
}

/// Get default machine type for a provider
pub fn get_default_machine_type(provider: &CloudProvider) -> &'static str {
    match provider {
        CloudProvider::Hetzner => "cx23",
        CloudProvider::Gcp => "e2-small",
        _ => "",
    }
}

/// Format region for display: "Nuremberg (Germany)"
pub fn format_region_display(region: &CloudRegion) -> String {
    format!("{} ({})", region.name, region.location)
}

/// Format machine type for display: "cx22 · 2 vCPU · 4 GB"
pub fn format_machine_type_display(machine: &MachineType) -> String {
    let base = format!("{} · {} vCPU · {}", machine.name, machine.cpu, machine.memory);
    if let Some(desc) = machine.description {
        format!("{} · {}", base, desc)
    } else {
        base
    }
}

// =============================================================================
// Dynamic Fetching (with Static Fallback)
// =============================================================================

/// Dynamic cloud region with availability info
#[derive(Debug, Clone)]
pub struct DynamicCloudRegion {
    /// Region ID (e.g., "nbg1")
    pub id: String,
    /// Human-readable name (e.g., "Nuremberg")
    pub name: String,
    /// Geographic location (e.g., "Germany")
    pub location: String,
    /// Network zone (e.g., "eu-central")
    pub network_zone: String,
    /// Server types available in this region
    pub available_server_types: Vec<String>,
}

/// Dynamic machine type with pricing info
#[derive(Debug, Clone)]
pub struct DynamicMachineType {
    /// Machine type ID (e.g., "cx22")
    pub id: String,
    /// Display name
    pub name: String,
    /// Number of vCPUs
    pub cores: i32,
    /// Memory in GB
    pub memory_gb: f64,
    /// Disk size in GB
    pub disk_gb: i64,
    /// Monthly price in EUR
    pub price_monthly: f64,
    /// Hourly price in EUR
    pub price_hourly: f64,
    /// Locations where available
    pub available_in: Vec<String>,
}

/// Convert API LocationWithAvailability to DynamicCloudRegion
fn location_to_dynamic_region(loc: &LocationWithAvailability) -> DynamicCloudRegion {
    DynamicCloudRegion {
        id: loc.location.name.clone(),
        name: loc.location.city.clone(),
        location: loc.location.country.clone(),
        network_zone: loc.location.network_zone.clone(),
        available_server_types: loc.available_server_types.clone(),
    }
}

/// Convert API ServerTypeSummary to DynamicMachineType
fn server_type_to_dynamic(st: &ServerTypeSummary) -> DynamicMachineType {
    DynamicMachineType {
        id: st.name.clone(),
        name: st.name.clone(),
        cores: st.cores,
        memory_gb: st.memory_gb,
        disk_gb: st.disk_gb,
        price_monthly: st.price_monthly,
        price_hourly: st.price_hourly,
        available_in: st.available_in.clone(),
    }
}

/// Convert static CloudRegion to DynamicCloudRegion (for fallback)
fn static_to_dynamic_region(region: &CloudRegion) -> DynamicCloudRegion {
    DynamicCloudRegion {
        id: region.id.to_string(),
        name: region.name.to_string(),
        location: region.location.to_string(),
        network_zone: match region.id {
            "fsn1" | "nbg1" | "hel1" => "eu-central".to_string(),
            "ash" => "us-east".to_string(),
            "hil" => "us-west".to_string(),
            "sin" => "ap-southeast".to_string(),
            _ => "unknown".to_string(),
        },
        available_server_types: vec![], // Unknown when using static data
    }
}

/// Convert static MachineType to DynamicMachineType (for fallback)
fn static_to_dynamic_machine(machine: &MachineType) -> DynamicMachineType {
    DynamicMachineType {
        id: machine.id.to_string(),
        name: machine.name.to_string(),
        cores: machine.cpu.parse().unwrap_or(2),
        memory_gb: machine.memory.replace(" GB", "").parse().unwrap_or(4.0),
        disk_gb: 40, // Default, unknown from static data
        price_monthly: 0.0, // Unknown from static data
        price_hourly: 0.0,
        available_in: vec![], // Unknown from static data
    }
}

/// Fetch Hetzner regions dynamically with real-time availability
///
/// Falls back to static data if the API call fails.
/// The agent can use this to make smart deployment decisions based on actual availability.
pub async fn get_hetzner_regions_dynamic(
    client: &PlatformApiClient,
    project_id: &str,
) -> Vec<DynamicCloudRegion> {
    match client.get_hetzner_locations(project_id).await {
        Ok(locations) => locations.iter().map(location_to_dynamic_region).collect(),
        Err(e) => {
            eprintln!(
                "Warning: Failed to fetch Hetzner regions dynamically ({}), using static data",
                e
            );
            HETZNER_LOCATIONS.iter().map(static_to_dynamic_region).collect()
        }
    }
}

/// Fetch Hetzner server types dynamically with pricing and availability
///
/// Falls back to static data if the API call fails.
/// Returns server types sorted by monthly price (cheapest first).
pub async fn get_hetzner_server_types_dynamic(
    client: &PlatformApiClient,
    project_id: &str,
    preferred_location: Option<&str>,
) -> Vec<DynamicMachineType> {
    match client.get_hetzner_server_types(project_id, preferred_location).await {
        Ok(server_types) => server_types.iter().map(server_type_to_dynamic).collect(),
        Err(e) => {
            eprintln!(
                "Warning: Failed to fetch Hetzner server types dynamically ({}), using static data",
                e
            );
            HETZNER_SERVER_TYPES.iter().map(static_to_dynamic_machine).collect()
        }
    }
}

/// Check availability of a specific server type at a location
///
/// Returns (available, reason, alternative_locations)
/// - available: true if the server type can be provisioned
/// - reason: None if available, Some("capacity"|"unsupported") if not
/// - alternative_locations: Locations where this server type IS available
pub async fn check_hetzner_availability(
    client: &PlatformApiClient,
    project_id: &str,
    location: &str,
    server_type: &str,
) -> (bool, Option<String>, Vec<String>) {
    match client.check_hetzner_availability(project_id, location, server_type).await {
        Ok(result) => (
            result.available,
            result.reason,
            result.alternative_locations.unwrap_or_default(),
        ),
        Err(e) => {
            eprintln!(
                "Warning: Failed to check Hetzner availability ({}), assuming available",
                e
            );
            // Assume available when we can't check (optimistic fallback)
            (true, None, vec![])
        }
    }
}

/// Get recommended server type for a workload profile
///
/// Returns the cheapest server type that meets the requirements:
/// - minimal: 1 core, 2GB RAM (development/testing)
/// - standard: 2 cores, 4GB RAM (small production)
/// - performance: 4 cores, 8GB RAM (production with dedicated CPU)
/// - high-memory: 2 cores, 16GB RAM (memory-intensive)
pub async fn get_recommended_server_type(
    client: &PlatformApiClient,
    project_id: &str,
    profile: &str,
    preferred_location: Option<&str>,
) -> Option<DynamicMachineType> {
    let (min_cores, min_memory, prefer_dedicated) = match profile {
        "minimal" => (1, 2.0, false),
        "standard" => (2, 4.0, false),
        "performance" => (4, 8.0, true),
        "high-memory" => (2, 16.0, false),
        _ => (2, 4.0, false), // Default to standard
    };

    let server_types = get_hetzner_server_types_dynamic(client, project_id, preferred_location).await;

    // Filter by requirements and find cheapest
    server_types
        .into_iter()
        .filter(|st| {
            st.cores >= min_cores
                && st.memory_gb >= min_memory
                && (!prefer_dedicated || st.name.starts_with("ccx"))
        })
        .filter(|st| {
            // If preferred location is set, only include types available there
            preferred_location.map_or(true, |loc| st.available_in.contains(&loc.to_string()))
        })
        .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
}

/// Format dynamic region for display
pub fn format_dynamic_region_display(region: &DynamicCloudRegion) -> String {
    if region.available_server_types.is_empty() {
        format!("{} ({})", region.name, region.location)
    } else {
        format!(
            "{} ({}) - {} server types available",
            region.name,
            region.location,
            region.available_server_types.len()
        )
    }
}

/// Format dynamic machine type for display
pub fn format_dynamic_machine_type_display(machine: &DynamicMachineType) -> String {
    if machine.price_monthly > 0.0 {
        format!(
            "{} · {} vCPU · {:.0} GB · €{:.2}/mo",
            machine.name, machine.cores, machine.memory_gb, machine.price_monthly
        )
    } else {
        format!(
            "{} · {} vCPU · {:.0} GB",
            machine.name, machine.cores, machine.memory_gb
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hetzner_locations() {
        assert!(!HETZNER_LOCATIONS.is_empty());
        assert!(HETZNER_LOCATIONS.iter().any(|r| r.id == "nbg1"));
    }

    #[test]
    fn test_hetzner_machine_types() {
        assert!(!HETZNER_SERVER_TYPES.is_empty());
        assert!(HETZNER_SERVER_TYPES.iter().any(|m| m.id == "cx23"));
    }

    #[test]
    fn test_gcp_regions() {
        assert!(!GCP_REGIONS.is_empty());
        assert!(GCP_REGIONS.iter().any(|r| r.id == "us-central1"));
    }

    #[test]
    fn test_gcp_machine_types() {
        assert!(!GCP_MACHINE_TYPES.is_empty());
        assert!(GCP_MACHINE_TYPES.iter().any(|m| m.id == "e2-small"));
    }

    #[test]
    fn test_get_regions_for_provider() {
        let hetzner_regions = get_regions_for_provider(&CloudProvider::Hetzner);
        assert!(!hetzner_regions.is_empty());

        let gcp_regions = get_regions_for_provider(&CloudProvider::Gcp);
        assert!(!gcp_regions.is_empty());
    }

    #[test]
    fn test_format_region_display() {
        let region = &HETZNER_LOCATIONS[0];
        let display = format_region_display(region);
        assert!(display.contains("Nuremberg"));
        assert!(display.contains("Germany"));
    }

    #[test]
    fn test_format_machine_type_display() {
        let machine = &HETZNER_SERVER_TYPES[0];
        let display = format_machine_type_display(machine);
        assert!(display.contains("CX23"));
        assert!(display.contains("2 vCPU"));
        assert!(display.contains("4 GB"));
    }
}
