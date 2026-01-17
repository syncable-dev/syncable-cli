//! Provider selection step for deployment wizard

use crate::platform::api::{
    types::{
        CloudProvider, ClusterStatus, ClusterSummary, ProviderDeploymentStatus, RegistryStatus,
        RegistrySummary,
    },
    PlatformApiClient,
};
use std::collections::HashMap;

/// Get deployment status for all providers
///
/// Queries the platform to determine which providers are connected and what
/// resources (clusters, registries) are available for each.
pub async fn get_provider_deployment_statuses(
    client: &PlatformApiClient,
    project_id: &str,
) -> Result<Vec<ProviderDeploymentStatus>, crate::platform::api::PlatformApiError> {
    // Get all clusters and registries for the project
    let clusters = client
        .list_clusters_for_project(project_id)
        .await
        .unwrap_or_default();
    let registries = client
        .list_registries_for_project(project_id)
        .await
        .unwrap_or_default();

    // Group by provider
    let mut provider_clusters: HashMap<CloudProvider, Vec<ClusterSummary>> = HashMap::new();
    let mut provider_registries: HashMap<CloudProvider, Vec<RegistrySummary>> = HashMap::new();

    for cluster in clusters {
        let summary = ClusterSummary {
            id: cluster.id,
            name: cluster.name,
            region: cluster.region,
            is_healthy: cluster.status == ClusterStatus::Running,
        };
        provider_clusters
            .entry(cluster.provider)
            .or_default()
            .push(summary);
    }

    for registry in registries {
        let summary = RegistrySummary {
            id: registry.id,
            name: registry.name,
            region: registry.region,
            is_ready: registry.status == RegistryStatus::Ready,
        };
        provider_registries
            .entry(registry.provider)
            .or_default()
            .push(summary);
    }

    // Build status for each supported provider
    let providers = [
        CloudProvider::Gcp,
        CloudProvider::Hetzner,
        CloudProvider::Aws,
        CloudProvider::Azure,
    ];
    let mut statuses = Vec::new();

    for provider in providers {
        let clusters = provider_clusters.remove(&provider).unwrap_or_default();
        let registries = provider_registries.remove(&provider).unwrap_or_default();

        // Provider is connected if it has any resources (clusters or registries)
        let is_connected = !clusters.is_empty() || !registries.is_empty();

        // Cloud Runner available for GCP and Hetzner
        let cloud_runner_available =
            is_connected && matches!(provider, CloudProvider::Gcp | CloudProvider::Hetzner);

        let summary = build_status_summary(&clusters, &registries, cloud_runner_available);

        statuses.push(ProviderDeploymentStatus {
            provider,
            is_connected,
            clusters,
            registries,
            cloud_runner_available,
            summary,
        });
    }

    Ok(statuses)
}

/// Build a human-readable summary string for a provider
fn build_status_summary(
    clusters: &[ClusterSummary],
    registries: &[RegistrySummary],
    cloud_runner: bool,
) -> String {
    let mut parts = Vec::new();

    if cloud_runner {
        parts.push("Cloud Run".to_string());
    }

    let healthy_clusters = clusters.iter().filter(|c| c.is_healthy).count();
    if healthy_clusters > 0 {
        parts.push(format!(
            "{} cluster{}",
            healthy_clusters,
            if healthy_clusters == 1 { "" } else { "s" }
        ));
    }

    let ready_registries = registries.iter().filter(|r| r.is_ready).count();
    if ready_registries > 0 {
        parts.push(format!(
            "{} registr{}",
            ready_registries,
            if ready_registries == 1 { "y" } else { "ies" }
        ));
    }

    if parts.is_empty() {
        "Not connected".to_string()
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_status_summary_cloud_runner_only() {
        let summary = build_status_summary(&[], &[], true);
        assert_eq!(summary, "Cloud Run");
    }

    #[test]
    fn test_build_status_summary_full() {
        let clusters = vec![
            ClusterSummary {
                id: "c1".to_string(),
                name: "prod".to_string(),
                region: "us-central1".to_string(),
                is_healthy: true,
            },
            ClusterSummary {
                id: "c2".to_string(),
                name: "staging".to_string(),
                region: "us-east1".to_string(),
                is_healthy: false,
            },
        ];
        let registries = vec![RegistrySummary {
            id: "r1".to_string(),
            name: "main".to_string(),
            region: "us-central1".to_string(),
            is_ready: true,
        }];
        let summary = build_status_summary(&clusters, &registries, true);
        assert_eq!(summary, "Cloud Run, 1 cluster, 1 registry");
    }

    #[test]
    fn test_build_status_summary_not_connected() {
        let summary = build_status_summary(&[], &[], false);
        assert_eq!(summary, "Not connected");
    }
}
