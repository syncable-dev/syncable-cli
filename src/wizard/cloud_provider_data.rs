//! Cloud provider regions and machine types for the deployment wizard
//!
//! For Hetzner: Uses DYNAMIC fetching from Hetzner API for real-time
//! availability and pricing. No hardcoded fallback - ensures agent always
//! uses current data for smart resource selection.
//!
//! For GCP: Uses static data (dynamic fetching not yet implemented).

use crate::platform::api::client::PlatformApiClient;
use crate::platform::api::types::{CloudProvider, LocationWithAvailability, ServerTypeSummary};

/// A cloud region/location option (static data for non-Hetzner providers)
#[derive(Debug, Clone)]
pub struct CloudRegion {
    /// Region ID (e.g., "us-central1")
    pub id: &'static str,
    /// Human-readable name (e.g., "Iowa")
    pub name: &'static str,
    /// Geographic location (e.g., "US Central")
    pub location: &'static str,
}

/// A machine/instance type option (static data for non-Hetzner providers)
#[derive(Debug, Clone)]
pub struct MachineType {
    /// Machine type ID (e.g., "e2-small")
    pub id: &'static str,
    /// Display name
    pub name: &'static str,
    /// Number of vCPUs (as string to handle fractional)
    pub cpu: &'static str,
    /// Memory amount (e.g., "4 GB")
    pub memory: &'static str,
    /// Optional description (e.g., "Shared-core")
    pub description: Option<&'static str>,
}

// =============================================================================
// GCP (Google Cloud Platform) - Static data
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
// Static Helper Functions (for non-Hetzner providers only)
// =============================================================================

/// Get static regions for a cloud provider
/// NOTE: For Hetzner, returns empty - use get_hetzner_regions_dynamic() instead
pub fn get_regions_for_provider(provider: &CloudProvider) -> &'static [CloudRegion] {
    match provider {
        CloudProvider::Hetzner => &[], // Use dynamic fetching for Hetzner
        CloudProvider::Gcp => GCP_REGIONS,
        _ => &[], // AWS, Azure not yet supported
    }
}

/// Get static machine types for a cloud provider
/// NOTE: For Hetzner, returns empty - use get_hetzner_server_types_dynamic() instead
pub fn get_machine_types_for_provider(provider: &CloudProvider) -> &'static [MachineType] {
    match provider {
        CloudProvider::Hetzner => &[], // Use dynamic fetching for Hetzner
        CloudProvider::Gcp => GCP_MACHINE_TYPES,
        _ => &[], // AWS, Azure not yet supported
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
        CloudProvider::Hetzner => "cx22",
        CloudProvider::Gcp => "e2-small",
        _ => "",
    }
}

// =============================================================================
// Dynamic Types and Fetching (Hetzner)
// =============================================================================

/// Dynamic cloud region with real-time availability info
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
    /// Server types currently available in this region
    pub available_server_types: Vec<String>,
}

/// Dynamic machine type with real-time pricing and availability
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
    /// Monthly price in EUR (from Hetzner API)
    pub price_monthly: f64,
    /// Hourly price in EUR (from Hetzner API)
    pub price_hourly: f64,
    /// Locations where this type is currently available
    pub available_in: Vec<String>,
}

/// Result of dynamic Hetzner data fetch
#[derive(Debug)]
pub enum HetznerFetchResult<T> {
    /// Successfully fetched data
    Success(T),
    /// Failed to fetch - requires Hetzner credentials
    NoCredentials,
    /// Failed to fetch - API error
    ApiError(String),
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

/// Fetch Hetzner regions dynamically with real-time availability
///
/// Returns regions with availability info directly from Hetzner API.
/// The agent uses this to make smart deployment decisions based on actual capacity.
///
/// # Errors
/// Returns error if credentials are missing or API call fails.
pub async fn get_hetzner_regions_dynamic(
    client: &PlatformApiClient,
    project_id: &str,
) -> HetznerFetchResult<Vec<DynamicCloudRegion>> {
    match client.get_hetzner_locations(project_id).await {
        Ok(locations) => {
            HetznerFetchResult::Success(locations.iter().map(location_to_dynamic_region).collect())
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("credentials") || error_msg.contains("Unauthorized") {
                HetznerFetchResult::NoCredentials
            } else {
                HetznerFetchResult::ApiError(error_msg)
            }
        }
    }
}

/// Fetch Hetzner server types dynamically with pricing and availability
///
/// Returns server types sorted by monthly price (cheapest first) with
/// real-time availability per region. The agent uses this for cost-optimized
/// resource selection.
///
/// # Errors
/// Returns error if credentials are missing or API call fails.
pub async fn get_hetzner_server_types_dynamic(
    client: &PlatformApiClient,
    project_id: &str,
    preferred_location: Option<&str>,
) -> HetznerFetchResult<Vec<DynamicMachineType>> {
    match client.get_hetzner_server_types(project_id, preferred_location).await {
        Ok(server_types) => {
            HetznerFetchResult::Success(server_types.iter().map(server_type_to_dynamic).collect())
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("credentials") || error_msg.contains("Unauthorized") {
                HetznerFetchResult::NoCredentials
            } else {
                HetznerFetchResult::ApiError(error_msg)
            }
        }
    }
}

/// Check availability of a specific server type at a location
///
/// Returns (available, reason, alternative_locations):
/// - available: true if the server type can be provisioned now
/// - reason: None if available, Some("capacity"|"unsupported") if not
/// - alternative_locations: Other locations where this server type IS available
///
/// The agent uses this for pre-deployment validation and smart fallback.
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
            // On error, return unavailable with error message
            (false, Some(format!("Failed to check: {}", e)), vec![])
        }
    }
}

/// Get recommended server type for a workload profile
///
/// Fetches real-time pricing and returns the cheapest server type meeting requirements:
/// - minimal: 1 core, 2GB RAM (development/testing)
/// - standard: 2 cores, 4GB RAM (small production)
/// - performance: 4 cores, 8GB RAM with dedicated CPU (production)
/// - high-memory: 2 cores, 16GB RAM (memory-intensive workloads)
///
/// The agent uses this for intelligent resource recommendations.
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

    let server_types = match get_hetzner_server_types_dynamic(client, project_id, preferred_location).await {
        HetznerFetchResult::Success(types) => types,
        _ => return None,
    };

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

/// Find the best region for a workload based on availability
///
/// Returns the region with the most available server types,
/// preferring regions in the specified network zone.
pub async fn find_best_region(
    client: &PlatformApiClient,
    project_id: &str,
    preferred_zone: Option<&str>,
) -> Option<DynamicCloudRegion> {
    let regions = match get_hetzner_regions_dynamic(client, project_id).await {
        HetznerFetchResult::Success(r) => r,
        _ => return None,
    };

    // Sort by availability count, preferring specified zone
    let mut sorted_regions = regions;
    sorted_regions.sort_by(|a, b| {
        let a_zone_match = preferred_zone.map_or(false, |z| a.network_zone == z);
        let b_zone_match = preferred_zone.map_or(false, |z| b.network_zone == z);

        match (a_zone_match, b_zone_match) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.available_server_types.len().cmp(&a.available_server_types.len()),
        }
    });

    sorted_regions.into_iter().next()
}

/// Find cheapest available server type for a region
///
/// Returns the cheapest server type that is currently available
/// in the specified region.
pub async fn find_cheapest_available(
    client: &PlatformApiClient,
    project_id: &str,
    region: &str,
) -> Option<DynamicMachineType> {
    let server_types = match get_hetzner_server_types_dynamic(client, project_id, Some(region)).await {
        HetznerFetchResult::Success(types) => types,
        _ => return None,
    };

    // Filter to only available types in this region, sort by price
    server_types
        .into_iter()
        .filter(|st| st.available_in.contains(&region.to_string()))
        .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
}

// =============================================================================
// Display Formatting
// =============================================================================

/// Format dynamic region for display
pub fn format_dynamic_region_display(region: &DynamicCloudRegion) -> String {
    if region.available_server_types.is_empty() {
        format!("{} ({}) - checking availability...", region.name, region.location)
    } else {
        format!(
            "{} ({}) · {} server types available",
            region.name,
            region.location,
            region.available_server_types.len()
        )
    }
}

/// Format dynamic machine type for display with pricing
pub fn format_dynamic_machine_type_display(machine: &DynamicMachineType) -> String {
    format!(
        "{} · {} vCPU · {:.0} GB · €{:.2}/mo",
        machine.name, machine.cores, machine.memory_gb, machine.price_monthly
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_hetzner_returns_empty_static() {
        // Hetzner should return empty from static functions
        // because we want to force dynamic fetching
        let regions = get_regions_for_provider(&CloudProvider::Hetzner);
        assert!(regions.is_empty());

        let machines = get_machine_types_for_provider(&CloudProvider::Hetzner);
        assert!(machines.is_empty());
    }

    #[test]
    fn test_gcp_returns_static_data() {
        let regions = get_regions_for_provider(&CloudProvider::Gcp);
        assert!(!regions.is_empty());

        let machines = get_machine_types_for_provider(&CloudProvider::Gcp);
        assert!(!machines.is_empty());
    }

    #[test]
    fn test_defaults() {
        assert_eq!(get_default_region(&CloudProvider::Hetzner), "nbg1");
        assert_eq!(get_default_region(&CloudProvider::Gcp), "us-central1");
        assert_eq!(get_default_machine_type(&CloudProvider::Hetzner), "cx22");
        assert_eq!(get_default_machine_type(&CloudProvider::Gcp), "e2-small");
    }

    #[test]
    fn test_dynamic_region_display() {
        let region = DynamicCloudRegion {
            id: "nbg1".to_string(),
            name: "Nuremberg".to_string(),
            location: "Germany".to_string(),
            network_zone: "eu-central".to_string(),
            available_server_types: vec!["cx22".to_string(), "cx32".to_string()],
        };
        let display = format_dynamic_region_display(&region);
        assert!(display.contains("Nuremberg"));
        assert!(display.contains("2 server types"));
    }

    #[test]
    fn test_dynamic_machine_display() {
        let machine = DynamicMachineType {
            id: "cx22".to_string(),
            name: "cx22".to_string(),
            cores: 2,
            memory_gb: 4.0,
            disk_gb: 40,
            price_monthly: 5.95,
            price_hourly: 0.008,
            available_in: vec!["nbg1".to_string()],
        };
        let display = format_dynamic_machine_type_display(&machine);
        assert!(display.contains("cx22"));
        assert!(display.contains("2 vCPU"));
        assert!(display.contains("€5.95/mo"));
    }
}
