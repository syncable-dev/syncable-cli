//! Kubernetes Metrics Client for live cluster resource usage.
//!
//! Connects to a Kubernetes cluster and fetches actual CPU/memory usage
//! from the metrics-server API. This provides the "ground truth" data
//! needed for precise right-sizing recommendations.
//!
//! # Prerequisites
//!
//! - Valid kubeconfig (uses default context or specified context)
//! - metrics-server installed in the cluster
//! - RBAC permissions to read pods and metrics
//!
//! # Example
//!
//! ```rust,ignore
//! use syncable_cli::analyzer::k8s_optimize::metrics_client::MetricsClient;
//!
//! let client = MetricsClient::new().await?;
//! let metrics = client.get_pod_metrics("default").await?;
//!
//! for pod in metrics {
//!     println!("{}: CPU={}, Memory={}", pod.name, pod.cpu_usage, pod.memory_usage);
//! }
//! ```

use k8s_openapi::api::core::v1::{Container, Pod};
use kube::{
    Client, Config,
    api::{Api, ListParams},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error type for metrics client operations.
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("Failed to create Kubernetes client: {0}")]
    ClientCreation(#[from] kube::Error),

    #[error("Failed to infer Kubernetes config: {0}")]
    ConfigError(#[from] kube::config::InferConfigError),

    #[error("Failed to read kubeconfig: {0}")]
    KubeconfigError(#[from] kube::config::KubeconfigError),

    #[error("Metrics server not available or not installed")]
    MetricsServerUnavailable,

    #[error("Namespace not found: {0}")]
    NamespaceNotFound(String),

    #[error("Failed to parse resource quantity: {0}")]
    QuantityParse(String),

    #[error("API request failed: {0}")]
    ApiError(String),
}

/// Metrics for a single pod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodMetrics {
    /// Pod name
    pub name: String,
    /// Namespace
    pub namespace: String,
    /// Container metrics
    pub containers: Vec<ContainerMetrics>,
    /// Total CPU usage in millicores
    pub total_cpu_millicores: u64,
    /// Total memory usage in bytes
    pub total_memory_bytes: u64,
    /// Timestamp of the metrics
    pub timestamp: String,
}

/// Metrics for a single container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMetrics {
    /// Container name
    pub name: String,
    /// CPU usage in millicores
    pub cpu_millicores: u64,
    /// Memory usage in bytes
    pub memory_bytes: u64,
}

/// Resource specifications from pod spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodResources {
    /// Pod name
    pub name: String,
    /// Namespace
    pub namespace: String,
    /// Owner reference (Deployment, StatefulSet, etc.)
    pub owner_kind: Option<String>,
    /// Owner name
    pub owner_name: Option<String>,
    /// Container resources
    pub containers: Vec<ContainerResources>,
}

/// Resource specifications for a container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResources {
    /// Container name
    pub name: String,
    /// Container image
    pub image: String,
    /// CPU request in millicores
    pub cpu_request: Option<u64>,
    /// Memory request in bytes
    pub memory_request: Option<u64>,
    /// CPU limit in millicores
    pub cpu_limit: Option<u64>,
    /// Memory limit in bytes
    pub memory_limit: Option<u64>,
}

/// Comparison between requested and actual resource usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceComparison {
    /// Pod name
    pub pod_name: String,
    /// Namespace
    pub namespace: String,
    /// Container name
    pub container_name: String,
    /// Owner kind (Deployment, StatefulSet, etc.)
    pub owner_kind: Option<String>,
    /// Owner name
    pub owner_name: Option<String>,
    /// CPU request in millicores
    pub cpu_request: Option<u64>,
    /// Actual CPU usage in millicores
    pub cpu_actual: u64,
    /// CPU waste percentage (negative if under-provisioned)
    pub cpu_waste_pct: f32,
    /// Memory request in bytes
    pub memory_request: Option<u64>,
    /// Actual memory usage in bytes
    pub memory_actual: u64,
    /// Memory waste percentage (negative if under-provisioned)
    pub memory_waste_pct: f32,
}

/// Kubernetes metrics client.
pub struct MetricsClient {
    client: Client,
}

impl MetricsClient {
    /// Create a new metrics client using the default kubeconfig.
    pub async fn new() -> Result<Self, MetricsError> {
        let config = Config::infer().await?;
        let client = Client::try_from(config)?;
        Ok(Self { client })
    }

    /// Create a new metrics client with a specific kubeconfig context.
    pub async fn with_context(context: &str) -> Result<Self, MetricsError> {
        let kubeconfig = kube::config::Kubeconfig::read()?;
        let config = Config::from_custom_kubeconfig(
            kubeconfig,
            &kube::config::KubeConfigOptions {
                context: Some(context.to_string()),
                ..Default::default()
            },
        )
        .await?;
        let client = Client::try_from(config)?;
        Ok(Self { client })
    }

    /// Get the current context name.
    pub async fn current_context() -> Result<String, MetricsError> {
        let kubeconfig = kube::config::Kubeconfig::read()?;
        Ok(kubeconfig
            .current_context
            .unwrap_or_else(|| "default".to_string()))
    }

    /// List available contexts.
    pub async fn list_contexts() -> Result<Vec<String>, MetricsError> {
        let kubeconfig = kube::config::Kubeconfig::read()?;
        Ok(kubeconfig.contexts.into_iter().map(|c| c.name).collect())
    }

    /// Get pod resource specifications from the cluster.
    pub async fn get_pod_resources(
        &self,
        namespace: Option<&str>,
    ) -> Result<Vec<PodResources>, MetricsError> {
        let pods: Api<Pod> = match namespace {
            Some(ns) => Api::namespaced(self.client.clone(), ns),
            None => Api::all(self.client.clone()),
        };

        let pod_list = pods
            .list(&ListParams::default())
            .await
            .map_err(|e| MetricsError::ApiError(format!("Failed to list pods: {}", e)))?;

        let mut results = Vec::new();

        for pod in pod_list.items {
            let metadata = pod.metadata;
            let spec = match pod.spec {
                Some(s) => s,
                None => continue,
            };

            let name = metadata.name.unwrap_or_default();
            let namespace = metadata.namespace.unwrap_or_else(|| "default".to_string());

            // Get owner reference
            let (owner_kind, owner_name) = metadata
                .owner_references
                .and_then(|refs| refs.into_iter().next())
                .map(|owner| (Some(owner.kind), Some(owner.name)))
                .unwrap_or((None, None));

            let containers: Vec<ContainerResources> = spec
                .containers
                .into_iter()
                .map(|c| container_to_resources(&c))
                .collect();

            results.push(PodResources {
                name,
                namespace,
                owner_kind,
                owner_name,
                containers,
            });
        }

        Ok(results)
    }

    /// Get pod metrics from the metrics-server.
    ///
    /// Note: This requires the metrics-server to be installed in the cluster.
    /// The metrics API is a custom resource, so we use a raw request.
    pub async fn get_pod_metrics(
        &self,
        namespace: Option<&str>,
    ) -> Result<Vec<PodMetrics>, MetricsError> {
        // The metrics API path depends on whether we're querying a specific namespace
        let path = match namespace {
            Some(ns) => format!("/apis/metrics.k8s.io/v1beta1/namespaces/{}/pods", ns),
            None => "/apis/metrics.k8s.io/v1beta1/pods".to_string(),
        };

        // Make raw API request
        let request = http::Request::builder()
            .method("GET")
            .uri(&path)
            .body(Vec::new())
            .map_err(|e| MetricsError::ApiError(format!("Failed to build request: {}", e)))?;

        let response = self
            .client
            .request::<PodMetricsList>(request)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("not found") {
                    MetricsError::MetricsServerUnavailable
                } else {
                    MetricsError::ApiError(format!("Metrics API error: {}", e))
                }
            })?;

        let results: Vec<PodMetrics> = response
            .items
            .into_iter()
            .map(|pm| {
                let containers: Vec<ContainerMetrics> = pm
                    .containers
                    .into_iter()
                    .map(|c| ContainerMetrics {
                        name: c.name,
                        cpu_millicores: parse_cpu_quantity(&c.usage.cpu),
                        memory_bytes: parse_memory_quantity(&c.usage.memory),
                    })
                    .collect();

                let total_cpu: u64 = containers.iter().map(|c| c.cpu_millicores).sum();
                let total_memory: u64 = containers.iter().map(|c| c.memory_bytes).sum();

                PodMetrics {
                    name: pm.metadata.name,
                    namespace: pm.metadata.namespace,
                    containers,
                    total_cpu_millicores: total_cpu,
                    total_memory_bytes: total_memory,
                    timestamp: pm.timestamp,
                }
            })
            .collect();

        Ok(results)
    }

    /// Compare actual usage against requested resources.
    pub async fn compare_usage(
        &self,
        namespace: Option<&str>,
    ) -> Result<Vec<ResourceComparison>, MetricsError> {
        let resources = self.get_pod_resources(namespace).await?;
        let metrics = self.get_pod_metrics(namespace).await?;

        // Create a map of pod/container -> metrics
        let mut metrics_map: HashMap<(String, String, String), (u64, u64)> = HashMap::new();
        for pm in &metrics {
            for cm in &pm.containers {
                metrics_map.insert(
                    (pm.namespace.clone(), pm.name.clone(), cm.name.clone()),
                    (cm.cpu_millicores, cm.memory_bytes),
                );
            }
        }

        let mut comparisons = Vec::new();

        for pod in resources {
            for container in pod.containers {
                let key = (
                    pod.namespace.clone(),
                    pod.name.clone(),
                    container.name.clone(),
                );

                if let Some((cpu_actual, memory_actual)) = metrics_map.get(&key) {
                    let cpu_waste_pct = calculate_waste_pct(container.cpu_request, *cpu_actual);
                    let memory_waste_pct =
                        calculate_waste_pct(container.memory_request, *memory_actual);

                    comparisons.push(ResourceComparison {
                        pod_name: pod.name.clone(),
                        namespace: pod.namespace.clone(),
                        container_name: container.name,
                        owner_kind: pod.owner_kind.clone(),
                        owner_name: pod.owner_name.clone(),
                        cpu_request: container.cpu_request,
                        cpu_actual: *cpu_actual,
                        cpu_waste_pct,
                        memory_request: container.memory_request,
                        memory_actual: *memory_actual,
                        memory_waste_pct,
                    });
                }
            }
        }

        Ok(comparisons)
    }

    /// Check if metrics-server is available.
    pub async fn is_metrics_available(&self) -> bool {
        let request = http::Request::builder()
            .method("GET")
            .uri("/apis/metrics.k8s.io/v1beta1")
            .body(Vec::new());

        match request {
            Ok(req) => self.client.request::<serde_json::Value>(req).await.is_ok(),
            Err(_) => false,
        }
    }
}

// ============================================================================
// Internal types for metrics API responses
// ============================================================================

#[derive(Debug, Deserialize)]
struct PodMetricsList {
    items: Vec<PodMetricsItem>,
}

#[derive(Debug, Deserialize)]
struct PodMetricsItem {
    metadata: PodMetricsMetadata,
    timestamp: String,
    containers: Vec<ContainerMetricsItem>,
}

#[derive(Debug, Deserialize)]
struct PodMetricsMetadata {
    name: String,
    namespace: String,
}

#[derive(Debug, Deserialize)]
struct ContainerMetricsItem {
    name: String,
    usage: ResourceUsage,
}

#[derive(Debug, Deserialize)]
struct ResourceUsage {
    cpu: String,
    memory: String,
}

// ============================================================================
// Helper functions
// ============================================================================

/// Convert a K8s container spec to our resource struct.
fn container_to_resources(container: &Container) -> ContainerResources {
    let resources = container.resources.as_ref();

    let cpu_request = resources
        .and_then(|r| r.requests.as_ref())
        .and_then(|req| req.get("cpu"))
        .map(|q| parse_cpu_quantity(&q.0));

    let memory_request = resources
        .and_then(|r| r.requests.as_ref())
        .and_then(|req| req.get("memory"))
        .map(|q| parse_memory_quantity(&q.0));

    let cpu_limit = resources
        .and_then(|r| r.limits.as_ref())
        .and_then(|lim| lim.get("cpu"))
        .map(|q| parse_cpu_quantity(&q.0));

    let memory_limit = resources
        .and_then(|r| r.limits.as_ref())
        .and_then(|lim| lim.get("memory"))
        .map(|q| parse_memory_quantity(&q.0));

    ContainerResources {
        name: container.name.clone(),
        image: container.image.clone().unwrap_or_default(),
        cpu_request,
        memory_request,
        cpu_limit,
        memory_limit,
    }
}

/// Parse a CPU quantity string (e.g., "100m", "1", "500n") to millicores.
fn parse_cpu_quantity(quantity: &str) -> u64 {
    let quantity = quantity.trim();

    if let Some(val) = quantity.strip_suffix('n') {
        // Nanocores to millicores
        val.parse::<u64>().map(|n| n / 1_000_000).unwrap_or(0)
    } else if let Some(val) = quantity.strip_suffix('u') {
        // Microcores to millicores
        val.parse::<u64>().map(|u| u / 1_000).unwrap_or(0)
    } else if let Some(val) = quantity.strip_suffix('m') {
        // Already in millicores
        val.parse::<u64>().unwrap_or(0)
    } else {
        // Whole cores to millicores
        quantity
            .parse::<f64>()
            .map(|c| (c * 1000.0) as u64)
            .unwrap_or(0)
    }
}

/// Parse a memory quantity string (e.g., "128Mi", "1Gi", "256000Ki") to bytes.
fn parse_memory_quantity(quantity: &str) -> u64 {
    let quantity = quantity.trim();

    if let Some(val) = quantity.strip_suffix("Ki") {
        val.parse::<u64>().map(|k| k * 1024).unwrap_or(0)
    } else if let Some(val) = quantity.strip_suffix("Mi") {
        val.parse::<u64>().map(|m| m * 1024 * 1024).unwrap_or(0)
    } else if let Some(val) = quantity.strip_suffix("Gi") {
        val.parse::<u64>()
            .map(|g| g * 1024 * 1024 * 1024)
            .unwrap_or(0)
    } else if let Some(val) = quantity.strip_suffix("Ti") {
        val.parse::<u64>()
            .map(|t| t * 1024 * 1024 * 1024 * 1024)
            .unwrap_or(0)
    } else if let Some(val) = quantity.strip_suffix('K').or_else(|| quantity.strip_suffix('k')) {
        val.parse::<u64>().map(|k| k * 1000).unwrap_or(0)
    } else if let Some(val) = quantity.strip_suffix('M') {
        val.parse::<u64>().map(|m| m * 1_000_000).unwrap_or(0)
    } else if let Some(val) = quantity.strip_suffix('G') {
        val.parse::<u64>().map(|g| g * 1_000_000_000).unwrap_or(0)
    } else {
        // Plain bytes
        quantity.parse::<u64>().unwrap_or(0)
    }
}

/// Calculate waste percentage.
/// Positive = over-provisioned, Negative = under-provisioned
fn calculate_waste_pct(request: Option<u64>, actual: u64) -> f32 {
    match request {
        Some(req) if req > 0 => {
            let waste = req as f32 - actual as f32;
            (waste / req as f32) * 100.0
        }
        _ => 0.0, // No request defined, can't calculate waste
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu_quantity() {
        assert_eq!(parse_cpu_quantity("100m"), 100);
        assert_eq!(parse_cpu_quantity("1"), 1000);
        assert_eq!(parse_cpu_quantity("0.5"), 500);
        assert_eq!(parse_cpu_quantity("2.5"), 2500);
        assert_eq!(parse_cpu_quantity("500000000n"), 500);
        assert_eq!(parse_cpu_quantity("500000u"), 500);
    }

    #[test]
    fn test_parse_memory_quantity() {
        assert_eq!(parse_memory_quantity("128Mi"), 128 * 1024 * 1024);
        assert_eq!(parse_memory_quantity("1Gi"), 1024 * 1024 * 1024);
        assert_eq!(parse_memory_quantity("256Ki"), 256 * 1024);
        assert_eq!(parse_memory_quantity("500M"), 500_000_000);
        assert_eq!(parse_memory_quantity("1G"), 1_000_000_000);
        assert_eq!(parse_memory_quantity("1000000"), 1_000_000);
    }

    #[test]
    fn test_calculate_waste_pct() {
        // 50% over-provisioned
        assert!((calculate_waste_pct(Some(1000), 500) - 50.0).abs() < 0.1);
        // 100% over-provisioned (no usage)
        assert!((calculate_waste_pct(Some(1000), 0) - 100.0).abs() < 0.1);
        // Under-provisioned (using more than requested)
        assert!((calculate_waste_pct(Some(500), 1000) - (-100.0)).abs() < 0.1);
        // No request defined
        assert!((calculate_waste_pct(None, 500) - 0.0).abs() < 0.1);
    }
}
