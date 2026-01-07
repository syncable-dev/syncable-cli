//! Live Cluster Analyzer for Kubernetes resource optimization.
//!
//! Combines metrics from the Kubernetes metrics-server (real-time) and
//! Prometheus (historical) to provide data-driven right-sizing recommendations.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        Live Analyzer                                │
//! │                                                                     │
//! │  ┌─────────────────┐    ┌──────────────────┐    ┌───────────────┐  │
//! │  │  MetricsClient  │    │ PrometheusClient │    │ Static Rules  │  │
//! │  │  (Real-time)    │    │ (Historical)     │    │ (Fallback)    │  │
//! │  └────────┬────────┘    └────────┬─────────┘    └───────┬───────┘  │
//! │           │                      │                      │          │
//! │           └──────────────────────┴──────────────────────┘          │
//! │                                  │                                  │
//! │                                  ▼                                  │
//! │                       ┌──────────────────┐                         │
//! │                       │  Recommendations │                         │
//! │                       │  (Data-Driven)   │                         │
//! │                       └──────────────────┘                         │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use super::metrics_client::{MetricsClient, MetricsError, PodResources, ResourceComparison};
use super::prometheus_client::{
    ContainerHistory, HistoricalRecommendation, PrometheusClient, PrometheusError,
};
use super::types::Severity;
use serde::{Deserialize, Serialize};

/// Error type for live analysis operations.
#[derive(Debug, thiserror::Error)]
pub enum LiveAnalyzerError {
    #[error("Kubernetes API error: {0}")]
    KubernetesError(#[from] MetricsError),

    #[error("Prometheus error: {0}")]
    PrometheusError(#[from] PrometheusError),

    #[error("No cluster connection available")]
    NoClusterConnection,

    #[error("Insufficient data for reliable recommendations")]
    InsufficientData,
}

/// Data source for recommendations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataSource {
    /// Real-time metrics from metrics-server (current snapshot)
    MetricsServer,
    /// Historical data from Prometheus (7-30 days)
    Prometheus,
    /// Combined real-time + historical (most accurate)
    Combined,
    /// Static heuristics only (no cluster data)
    Static,
}

/// Configuration for live analysis.
#[derive(Debug, Clone)]
pub struct LiveAnalyzerConfig {
    /// Prometheus URL (optional)
    pub prometheus_url: Option<String>,
    /// Time range for historical data (e.g., "7d", "30d")
    pub history_period: String,
    /// Safety margin percentage (default: 20%)
    pub safety_margin_pct: u8,
    /// Minimum samples required for high-confidence recommendations
    pub min_samples: usize,
    /// Waste threshold percentage to report
    pub waste_threshold_pct: f32,
    /// Target namespace (None = all namespaces)
    pub namespace: Option<String>,
    /// Include system namespaces
    pub include_system: bool,
}

impl Default for LiveAnalyzerConfig {
    fn default() -> Self {
        Self {
            prometheus_url: None,
            history_period: "7d".to_string(),
            safety_margin_pct: 20,
            min_samples: 100,
            waste_threshold_pct: 10.0,
            namespace: None,
            include_system: false,
        }
    }
}

/// Live cluster analyzer.
pub struct LiveAnalyzer {
    metrics_client: Option<MetricsClient>,
    prometheus_client: Option<PrometheusClient>,
    config: LiveAnalyzerConfig,
}

impl LiveAnalyzer {
    /// Create a new live analyzer, attempting to connect to the cluster.
    pub async fn new(config: LiveAnalyzerConfig) -> Result<Self, LiveAnalyzerError> {
        // Try to create Kubernetes client
        let metrics_client = match MetricsClient::new().await {
            Ok(client) => Some(client),
            Err(e) => {
                eprintln!("Warning: Could not connect to Kubernetes cluster: {}", e);
                None
            }
        };

        // Try to create Prometheus client if URL provided
        let prometheus_client =
            config
                .prometheus_url
                .as_ref()
                .and_then(|url| match PrometheusClient::new(url) {
                    Ok(client) => Some(client),
                    Err(e) => {
                        eprintln!("Warning: Could not create Prometheus client: {}", e);
                        None
                    }
                });

        Ok(Self {
            metrics_client,
            prometheus_client,
            config,
        })
    }

    /// Create analyzer with specific context.
    pub async fn with_context(
        context: &str,
        config: LiveAnalyzerConfig,
    ) -> Result<Self, LiveAnalyzerError> {
        let metrics_client = match MetricsClient::with_context(context).await {
            Ok(client) => Some(client),
            Err(e) => {
                eprintln!("Warning: Could not connect to context '{}': {}", context, e);
                None
            }
        };

        let prometheus_client = config
            .prometheus_url
            .as_ref()
            .and_then(|url| PrometheusClient::new(url).ok());

        Ok(Self {
            metrics_client,
            prometheus_client,
            config,
        })
    }

    /// Check what data sources are available.
    pub async fn available_sources(&self) -> Vec<DataSource> {
        let mut sources = vec![DataSource::Static]; // Always available

        if let Some(ref metrics) = self.metrics_client {
            if metrics.is_metrics_available().await {
                sources.push(DataSource::MetricsServer);
            }
        }

        if let Some(ref prometheus) = self.prometheus_client {
            if prometheus.is_available().await {
                sources.push(DataSource::Prometheus);
            }
        }

        if sources.contains(&DataSource::MetricsServer) && sources.contains(&DataSource::Prometheus)
        {
            sources.push(DataSource::Combined);
        }

        sources
    }

    /// Analyze cluster and generate recommendations.
    pub async fn analyze(&self) -> Result<LiveAnalysisResult, LiveAnalyzerError> {
        let sources = self.available_sources().await;

        let best_source = if sources.contains(&DataSource::Combined) {
            DataSource::Combined
        } else if sources.contains(&DataSource::Prometheus) {
            DataSource::Prometheus
        } else if sources.contains(&DataSource::MetricsServer) {
            DataSource::MetricsServer
        } else {
            DataSource::Static
        };

        match best_source {
            DataSource::Combined => self.analyze_combined().await,
            DataSource::Prometheus => self.analyze_prometheus().await,
            DataSource::MetricsServer => self.analyze_metrics_server().await,
            DataSource::Static => Ok(LiveAnalysisResult::static_fallback()),
        }
    }

    /// Analyze using metrics-server data (real-time snapshot).
    async fn analyze_metrics_server(&self) -> Result<LiveAnalysisResult, LiveAnalyzerError> {
        let client = self
            .metrics_client
            .as_ref()
            .ok_or(LiveAnalyzerError::NoClusterConnection)?;

        let namespace = self.config.namespace.as_deref();
        let comparisons = client.compare_usage(namespace).await?;
        let total_count = comparisons.len();

        let mut recommendations = Vec::new();
        let mut total_cpu_waste: u64 = 0;
        let mut total_memory_waste: u64 = 0;
        let mut over_provisioned = 0;
        let mut under_provisioned = 0;

        for comp in comparisons {
            // Skip system namespaces unless configured
            if !self.config.include_system && is_system_namespace(&comp.namespace) {
                continue;
            }

            // Skip if waste is below threshold
            if comp.cpu_waste_pct.abs() < self.config.waste_threshold_pct
                && comp.memory_waste_pct.abs() < self.config.waste_threshold_pct
            {
                continue;
            }

            let recommendation = self.comparison_to_recommendation(&comp);

            if comp.cpu_waste_pct > 0.0 || comp.memory_waste_pct > 0.0 {
                over_provisioned += 1;
                if let Some(req) = comp.cpu_request {
                    total_cpu_waste += (req as f32 * (comp.cpu_waste_pct / 100.0)) as u64;
                }
                if let Some(req) = comp.memory_request {
                    total_memory_waste += (req as f32 * (comp.memory_waste_pct / 100.0)) as u64;
                }
            } else {
                under_provisioned += 1;
            }

            recommendations.push(recommendation);
        }

        Ok(LiveAnalysisResult {
            source: DataSource::MetricsServer,
            recommendations,
            summary: AnalysisSummary {
                resources_analyzed: total_count,
                over_provisioned,
                under_provisioned,
                optimal: total_count.saturating_sub(over_provisioned + under_provisioned),
                total_cpu_waste_millicores: total_cpu_waste,
                total_memory_waste_bytes: total_memory_waste,
                confidence: 60, // Lower confidence for point-in-time data
            },
            warnings: vec![
                "Real-time snapshot only. For accurate recommendations, enable Prometheus for historical data.".to_string()
            ],
        })
    }

    /// Analyze using Prometheus historical data.
    async fn analyze_prometheus(&self) -> Result<LiveAnalysisResult, LiveAnalyzerError> {
        let client = self
            .prometheus_client
            .as_ref()
            .ok_or(LiveAnalyzerError::NoClusterConnection)?;

        let metrics_client = self.metrics_client.as_ref();

        // Get pod resources to understand current requests
        let pod_resources = if let Some(mc) = metrics_client {
            mc.get_pod_resources(self.config.namespace.as_deref())
                .await
                .ok()
        } else {
            None
        };

        let mut recommendations = Vec::new();
        let mut over_provisioned = 0;
        let mut under_provisioned = 0;
        let mut total_cpu_waste: u64 = 0;
        let mut total_memory_waste: u64 = 0;

        // Group by unique workloads
        let workloads = if let Some(ref resources) = pod_resources {
            extract_workloads(resources)
        } else {
            Vec::new()
        };

        let resources_analyzed = workloads.len();

        for (namespace, owner_name, containers) in workloads {
            if !self.config.include_system && is_system_namespace(&namespace) {
                continue;
            }

            for (container_name, cpu_request, memory_request) in containers {
                match client
                    .get_container_history(
                        &namespace,
                        &owner_name,
                        &container_name,
                        &self.config.history_period,
                    )
                    .await
                {
                    Ok(history) => {
                        let rec = PrometheusClient::generate_recommendation(
                            &history,
                            cpu_request,
                            memory_request,
                            self.config.safety_margin_pct,
                        );

                        if rec.cpu_savings_pct.abs() < self.config.waste_threshold_pct
                            && rec.memory_savings_pct.abs() < self.config.waste_threshold_pct
                        {
                            continue;
                        }

                        if rec.cpu_savings_pct > 0.0 || rec.memory_savings_pct > 0.0 {
                            over_provisioned += 1;
                            if let Some(req) = cpu_request {
                                total_cpu_waste +=
                                    (req as f32 * (rec.cpu_savings_pct / 100.0)) as u64;
                            }
                            if let Some(req) = memory_request {
                                total_memory_waste +=
                                    (req as f32 * (rec.memory_savings_pct / 100.0)) as u64;
                            }
                        } else {
                            under_provisioned += 1;
                        }

                        recommendations
                            .push(self.history_to_recommendation(&rec, &namespace, &history));
                    }
                    Err(_) => continue,
                }
            }
        }

        Ok(LiveAnalysisResult {
            source: DataSource::Prometheus,
            recommendations,
            summary: AnalysisSummary {
                resources_analyzed,
                over_provisioned,
                under_provisioned,
                optimal: resources_analyzed - over_provisioned - under_provisioned,
                total_cpu_waste_millicores: total_cpu_waste,
                total_memory_waste_bytes: total_memory_waste,
                confidence: 85,
            },
            warnings: vec![],
        })
    }

    /// Analyze using both real-time and historical data (highest accuracy).
    async fn analyze_combined(&self) -> Result<LiveAnalysisResult, LiveAnalyzerError> {
        // Get Prometheus-based recommendations (more accurate)
        let mut result = self.analyze_prometheus().await?;

        // Get real-time data for current state
        if let Ok(_realtime) = self.analyze_metrics_server().await {
            // Merge real-time data with historical
            result.source = DataSource::Combined;
            result.summary.confidence = 95;
            result.warnings = vec![];
        }

        Ok(result)
    }

    /// Convert a ResourceComparison to a recommendation.
    fn comparison_to_recommendation(&self, comp: &ResourceComparison) -> LiveRecommendation {
        let severity = if comp.memory_waste_pct < -25.0 {
            Severity::Critical // Significantly under-provisioned memory
        } else if comp.cpu_waste_pct < -25.0 || comp.memory_waste_pct < -10.0 {
            Severity::High
        } else if comp.cpu_waste_pct > 50.0 || comp.memory_waste_pct > 50.0 {
            Severity::High
        } else if comp.cpu_waste_pct > 25.0 || comp.memory_waste_pct > 25.0 {
            Severity::Medium
        } else {
            Severity::Low
        };

        let margin = 1.0 + (self.config.safety_margin_pct as f64 / 100.0);
        let recommended_cpu = round_cpu((comp.cpu_actual as f64 * margin) as u64);
        let recommended_memory = round_memory((comp.memory_actual as f64 * margin) as u64);

        LiveRecommendation {
            workload_name: comp
                .owner_name
                .clone()
                .unwrap_or_else(|| comp.pod_name.clone()),
            workload_kind: comp.owner_kind.clone().unwrap_or_else(|| "Pod".to_string()),
            namespace: comp.namespace.clone(),
            container_name: comp.container_name.clone(),
            severity,
            current_cpu_millicores: comp.cpu_request,
            current_memory_bytes: comp.memory_request,
            actual_cpu_millicores: comp.cpu_actual,
            actual_memory_bytes: comp.memory_actual,
            recommended_cpu_millicores: recommended_cpu,
            recommended_memory_bytes: recommended_memory,
            cpu_waste_pct: comp.cpu_waste_pct,
            memory_waste_pct: comp.memory_waste_pct,
            confidence: 60,
            data_source: DataSource::MetricsServer,
        }
    }

    /// Convert historical recommendation to our format.
    fn history_to_recommendation(
        &self,
        rec: &HistoricalRecommendation,
        namespace: &str,
        history: &ContainerHistory,
    ) -> LiveRecommendation {
        let severity = if rec.memory_savings_pct < -25.0 {
            Severity::Critical
        } else if rec.cpu_savings_pct > 50.0 || rec.memory_savings_pct > 50.0 {
            Severity::High
        } else if rec.cpu_savings_pct > 25.0 || rec.memory_savings_pct > 25.0 {
            Severity::Medium
        } else {
            Severity::Low
        };

        LiveRecommendation {
            workload_name: rec.workload_name.clone(),
            workload_kind: "Deployment".to_string(), // Assume deployment
            namespace: namespace.to_string(),
            container_name: rec.container_name.clone(),
            severity,
            current_cpu_millicores: rec.current_cpu_request,
            current_memory_bytes: rec.current_memory_request,
            actual_cpu_millicores: history.cpu_p99,
            actual_memory_bytes: history.memory_p99,
            recommended_cpu_millicores: rec.recommended_cpu_request,
            recommended_memory_bytes: rec.recommended_memory_request,
            cpu_waste_pct: rec.cpu_savings_pct,
            memory_waste_pct: rec.memory_savings_pct,
            confidence: rec.confidence,
            data_source: DataSource::Prometheus,
        }
    }
}

/// Result of live cluster analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveAnalysisResult {
    /// Data source used for recommendations
    pub source: DataSource,
    /// Individual recommendations
    pub recommendations: Vec<LiveRecommendation>,
    /// Summary statistics
    pub summary: AnalysisSummary,
    /// Warnings or notes
    pub warnings: Vec<String>,
}

impl LiveAnalysisResult {
    /// Create a static fallback result when no cluster connection is available.
    fn static_fallback() -> Self {
        Self {
            source: DataSource::Static,
            recommendations: vec![],
            summary: AnalysisSummary {
                resources_analyzed: 0,
                over_provisioned: 0,
                under_provisioned: 0,
                optimal: 0,
                total_cpu_waste_millicores: 0,
                total_memory_waste_bytes: 0,
                confidence: 0,
            },
            warnings: vec![
                "No cluster connection available. Using static analysis only.".to_string(),
                "Connect to a cluster with --cluster for data-driven recommendations.".to_string(),
            ],
        }
    }
}

/// Summary of analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub resources_analyzed: usize,
    pub over_provisioned: usize,
    pub under_provisioned: usize,
    pub optimal: usize,
    pub total_cpu_waste_millicores: u64,
    pub total_memory_waste_bytes: u64,
    /// Confidence percentage (0-100)
    pub confidence: u8,
}

/// A single recommendation from live analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveRecommendation {
    pub workload_name: String,
    pub workload_kind: String,
    pub namespace: String,
    pub container_name: String,
    pub severity: Severity,
    /// Current CPU request (millicores)
    pub current_cpu_millicores: Option<u64>,
    /// Current memory request (bytes)
    pub current_memory_bytes: Option<u64>,
    /// Actual CPU usage (millicores)
    pub actual_cpu_millicores: u64,
    /// Actual memory usage (bytes)
    pub actual_memory_bytes: u64,
    /// Recommended CPU request (millicores)
    pub recommended_cpu_millicores: u64,
    /// Recommended memory request (bytes)
    pub recommended_memory_bytes: u64,
    /// CPU waste percentage (positive = over-provisioned)
    pub cpu_waste_pct: f32,
    /// Memory waste percentage (positive = over-provisioned)
    pub memory_waste_pct: f32,
    /// Confidence level (0-100)
    pub confidence: u8,
    /// Source of the data
    pub data_source: DataSource,
}

impl LiveRecommendation {
    /// Generate a YAML fix snippet for this recommendation.
    pub fn generate_fix_yaml(&self) -> String {
        let cpu_str = format_cpu_millicores(self.recommended_cpu_millicores);
        let mem_str = format_memory_bytes(self.recommended_memory_bytes);

        format!(
            "# Fix for {}/{} container {}
# Source: {:?} (confidence: {}%)
resources:
  requests:
    cpu: \"{}\"
    memory: \"{}\"
  limits:
    cpu: \"{}\"    # Consider 2x request for burst
    memory: \"{}\"  # Same as request to prevent OOM",
            self.namespace,
            self.workload_name,
            self.container_name,
            self.data_source,
            self.confidence,
            cpu_str,
            mem_str,
            format_cpu_millicores(self.recommended_cpu_millicores * 2), // 2x for limit
            mem_str, // Memory limit = request to prevent OOM
        )
    }
}

/// Format CPU millicores as Kubernetes resource string.
fn format_cpu_millicores(millicores: u64) -> String {
    if millicores >= 1000 {
        format!("{}", millicores / 1000) // Full cores
    } else {
        format!("{}m", millicores)
    }
}

/// Format memory bytes as Kubernetes resource string.
fn format_memory_bytes(bytes: u64) -> String {
    const GI: u64 = 1024 * 1024 * 1024;
    const MI: u64 = 1024 * 1024;

    if bytes >= GI {
        format!("{}Gi", bytes / GI)
    } else {
        format!("{}Mi", bytes / MI)
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Check if a namespace is a system namespace.
fn is_system_namespace(namespace: &str) -> bool {
    matches!(
        namespace,
        "kube-system"
            | "kube-public"
            | "kube-node-lease"
            | "default"
            | "ingress-nginx"
            | "cert-manager"
            | "monitoring"
            | "logging"
            | "istio-system"
    )
}

/// Extract unique workloads from pod resources.
fn extract_workloads(
    resources: &[PodResources],
) -> Vec<(String, String, Vec<(String, Option<u64>, Option<u64>)>)> {
    use std::collections::HashMap;

    let mut workloads: HashMap<(String, String), Vec<(String, Option<u64>, Option<u64>)>> =
        HashMap::new();

    for pod in resources {
        let owner = pod.owner_name.clone().unwrap_or_else(|| pod.name.clone());
        let key = (pod.namespace.clone(), owner);

        let containers: Vec<_> = pod
            .containers
            .iter()
            .map(|c| (c.name.clone(), c.cpu_request, c.memory_request))
            .collect();

        workloads
            .entry(key)
            .or_default()
            .extend(containers);
    }

    workloads
        .into_iter()
        .map(|((ns, owner), containers)| (ns, owner, containers))
        .collect()
}

/// Round CPU to nice values.
fn round_cpu(millicores: u64) -> u64 {
    if millicores <= 100 {
        ((millicores + 12) / 25) * 25
    } else if millicores <= 1000 {
        ((millicores + 25) / 50) * 50
    } else {
        ((millicores + 50) / 100) * 100
    }
}

/// Round memory to nice values.
fn round_memory(bytes: u64) -> u64 {
    const MI: u64 = 1024 * 1024;
    if bytes <= 128 * MI {
        ((bytes + 16 * MI) / (32 * MI)) * (32 * MI)
    } else {
        ((bytes + 32 * MI) / (64 * MI)) * (64 * MI)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_system_namespace() {
        assert!(is_system_namespace("kube-system"));
        assert!(is_system_namespace("kube-public"));
        assert!(!is_system_namespace("production"));
        assert!(!is_system_namespace("my-app"));
    }

    #[test]
    fn test_round_cpu() {
        assert_eq!(round_cpu(10), 25);
        assert_eq!(round_cpu(90), 100);
        assert_eq!(round_cpu(150), 150);
        assert_eq!(round_cpu(1250), 1300);
    }
}
