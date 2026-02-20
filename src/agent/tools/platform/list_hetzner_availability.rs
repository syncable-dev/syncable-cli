//! List Hetzner availability tool for the agent
//!
//! Fetches real-time Hetzner Cloud region and server type availability with pricing.
//! The agent uses this to make smart deployment decisions based on current capacity.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tools::error::{ErrorCategory, format_error_for_llm};
use crate::platform::PlatformSession;
use crate::platform::api::PlatformApiClient;
use crate::wizard::{
    DynamicCloudRegion, DynamicMachineType, HetznerFetchResult, get_hetzner_regions_dynamic,
    get_hetzner_server_types_dynamic,
};

/// Arguments for the list_hetzner_availability tool
#[derive(Debug, Deserialize)]
pub struct ListHetznerAvailabilityArgs {
    /// Optional: filter server types by location
    pub location: Option<String>,
}

/// Error type for availability operations
#[derive(Debug, thiserror::Error)]
#[error("Hetzner availability error: {0}")]
pub struct ListHetznerAvailabilityError(String);

/// Tool to fetch real-time Hetzner Cloud availability
///
/// Returns current regions/locations and server types with:
/// - Real-time availability per region
/// - Current pricing (hourly and monthly in EUR)
/// - CPU, memory, and disk specs for each server type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListHetznerAvailabilityTool;

impl ListHetznerAvailabilityTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ListHetznerAvailabilityTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ListHetznerAvailabilityTool {
    const NAME: &'static str = "list_hetzner_availability";

    type Error = ListHetznerAvailabilityError;
    type Args = ListHetznerAvailabilityArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Fetch real-time Hetzner Cloud region and server type availability.

**IMPORTANT:** Use this tool BEFORE recommending Hetzner regions or server types.
This provides current data directly from Hetzner API - never use hardcoded/static data.

**What it returns:**
- Available regions/locations with:
  - Region ID (e.g., "nbg1", "fsn1", "hel1", "ash", "hil", "sin")
  - City name and country
  - Network zone (eu-central, us-east, us-west, ap-southeast)
  - List of server types currently available in that region

- Available server types with:
  - Server type ID (e.g., "cx22", "cx32", "cpx21")
  - CPU cores and memory (GB)
  - Disk size (GB)
  - Current pricing (EUR/hour and EUR/month)
  - Which regions this type is available in

**When to use:**
- When user asks about Hetzner regions/locations
- When recommending infrastructure for Hetzner deployment
- When user wants to compare Hetzner server types and pricing
- Before deploying to Hetzner to verify availability

**Parameters:**
- location: Optional. Filter server types by specific location (e.g., "nbg1")

**Prerequisites:**
- User must be authenticated
- A project with Hetzner credentials must be selected"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "Optional: Filter server types by location (e.g., 'nbg1', 'fsn1')"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Get API client
        let client = match PlatformApiClient::new() {
            Ok(c) => c,
            Err(_) => {
                return Ok(format_error_for_llm(
                    "list_hetzner_availability",
                    ErrorCategory::PermissionDenied,
                    "Not authenticated",
                    Some(vec!["Run: sync-ctl auth login"]),
                ));
            }
        };

        // Load platform session for project context
        let session = match PlatformSession::load() {
            Ok(s) => s,
            Err(_) => {
                return Ok(format_error_for_llm(
                    "list_hetzner_availability",
                    ErrorCategory::InternalError,
                    "Failed to load platform session",
                    Some(vec!["Try selecting a project with select_project"]),
                ));
            }
        };

        if !session.is_project_selected() {
            return Ok(format_error_for_llm(
                "list_hetzner_availability",
                ErrorCategory::ValidationFailed,
                "No project selected",
                Some(vec!["Use select_project to choose a project first"]),
            ));
        }

        let project_id = session.project_id.clone().unwrap_or_default();

        // Fetch regions
        let regions: Vec<DynamicCloudRegion> =
            match get_hetzner_regions_dynamic(&client, &project_id).await {
                HetznerFetchResult::Success(r) => r,
                HetznerFetchResult::NoCredentials => {
                    return Ok(format_error_for_llm(
                        "list_hetzner_availability",
                        ErrorCategory::PermissionDenied,
                        "Hetzner credentials not configured for this project",
                        Some(vec![
                            "Add Hetzner API token in project settings",
                            "Use open_provider_settings to configure Hetzner",
                        ]),
                    ));
                }
                HetznerFetchResult::ApiError(err) => {
                    return Ok(format_error_for_llm(
                        "list_hetzner_availability",
                        ErrorCategory::NetworkError,
                        &format!("Failed to fetch Hetzner regions: {}", err),
                        None,
                    ));
                }
            };

        // Fetch server types
        let server_types: Vec<DynamicMachineType> =
            match get_hetzner_server_types_dynamic(&client, &project_id, args.location.as_deref())
                .await
            {
                HetznerFetchResult::Success(s) => s,
                HetznerFetchResult::NoCredentials => Vec::new(), // Already handled above
                HetznerFetchResult::ApiError(_) => Vec::new(),   // Non-fatal, continue with regions
            };

        // Format response
        let regions_json: Vec<serde_json::Value> = regions
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "name": r.name,
                    "country": r.location,
                    "network_zone": r.network_zone,
                    "available_server_types_count": r.available_server_types.len(),
                    "available_server_types": r.available_server_types,
                })
            })
            .collect();

        let server_types_json: Vec<serde_json::Value> = server_types
            .iter()
            .map(|s| {
                json!({
                    "id": s.id,
                    "name": s.name,
                    "cores": s.cores,
                    "memory_gb": s.memory_gb,
                    "disk_gb": s.disk_gb,
                    "price_hourly_eur": s.price_hourly,
                    "price_monthly_eur": s.price_monthly,
                    "available_in": s.available_in,
                })
            })
            .collect();

        // Group server types by category for easier reading
        let shared_cpu: Vec<&serde_json::Value> = server_types_json
            .iter()
            .filter(|s| s["id"].as_str().map_or(false, |id| id.starts_with("cx")))
            .collect();

        let dedicated_cpu: Vec<&serde_json::Value> = server_types_json
            .iter()
            .filter(|s| s["id"].as_str().map_or(false, |id| id.starts_with("ccx")))
            .collect();

        let performance: Vec<&serde_json::Value> = server_types_json
            .iter()
            .filter(|s| s["id"].as_str().map_or(false, |id| id.starts_with("cpx")))
            .collect();

        let response = json!({
            "status": "success",
            "summary": {
                "total_regions": regions.len(),
                "total_server_types": server_types.len(),
                "filter_applied": args.location,
            },
            "regions": regions_json,
            "server_types": {
                "shared_cpu_cx": shared_cpu,
                "dedicated_cpu_ccx": dedicated_cpu,
                "performance_cpx": performance,
                "all": server_types_json,
            },
            "recommendations": {
                "cheapest": server_types.iter()
                    .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
                    .map(|s| json!({
                        "id": s.id,
                        "price_monthly_eur": s.price_monthly,
                        "specs": format!("{} vCPU, {:.0} GB RAM", s.cores, s.memory_gb),
                    })),
                "best_value_4gb": server_types.iter()
                    .filter(|s| s.memory_gb >= 4.0)
                    .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
                    .map(|s| json!({
                        "id": s.id,
                        "price_monthly_eur": s.price_monthly,
                        "specs": format!("{} vCPU, {:.0} GB RAM", s.cores, s.memory_gb),
                    })),
                "best_value_8gb": server_types.iter()
                    .filter(|s| s.memory_gb >= 8.0)
                    .min_by(|a, b| a.price_monthly.partial_cmp(&b.price_monthly).unwrap())
                    .map(|s| json!({
                        "id": s.id,
                        "price_monthly_eur": s.price_monthly,
                        "specs": format!("{} vCPU, {:.0} GB RAM", s.cores, s.memory_gb),
                    })),
            },
            "usage_notes": [
                "Use region IDs (nbg1, fsn1, hel1, ash, hil, sin) when deploying",
                "EU regions (nbg1, fsn1, hel1) have lowest pricing",
                "CX series: shared CPU, best for most workloads",
                "CCX series: dedicated CPU, best for CPU-intensive workloads",
                "CPX series: AMD performance, good balance of price/performance",
            ],
        });

        serde_json::to_string_pretty(&response)
            .map_err(|e| ListHetznerAvailabilityError(format!("Failed to serialize: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(
            ListHetznerAvailabilityTool::NAME,
            "list_hetzner_availability"
        );
    }

    #[test]
    fn test_tool_creation() {
        let tool = ListHetznerAvailabilityTool::new();
        assert!(format!("{:?}", tool).contains("ListHetznerAvailabilityTool"));
    }
}
