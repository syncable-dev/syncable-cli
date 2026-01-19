//! Cloud provider regions and machine types for the deployment wizard
//!
//! This module contains static data for cloud provider options,
//! matching the frontend's cloudProviderData.ts for consistency.

use crate::platform::api::types::CloudProvider;

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
