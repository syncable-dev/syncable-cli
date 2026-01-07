//! Prometheus Client for historical Kubernetes metrics.
//!
//! Fetches historical CPU/memory usage data from Prometheus to calculate
//! percentile values (P50, P95, P99, max) for accurate right-sizing.
//!
//! # Prerequisites
//!
//! - Prometheus accessible (via port-forward, ingress, or direct URL)
//! - Prometheus collecting Kubernetes metrics (typically via kube-state-metrics and cAdvisor)
//!
//! # Authentication
//!
//! Authentication is **optional** and typically not needed when using `kubectl port-forward`
//! because the connection goes directly to the pod, bypassing ingress/auth layers.
//! Auth is only needed for externally exposed Prometheus instances.
//!
//! # Example
//!
//! ```rust,ignore
//! use syncable_cli::analyzer::k8s_optimize::prometheus_client::{PrometheusClient, PrometheusAuth};
//!
//! // Default: No authentication (works with port-forward)
//! let client = PrometheusClient::new("http://localhost:9090")?;
//!
//! // With authentication (for external Prometheus)
//! let client = PrometheusClient::with_auth(
//!     "https://prometheus.example.com",
//!     PrometheusAuth::Bearer("token123".to_string())
//! )?;
//!
//! let history = client.get_container_history("default", "api-gateway", "main", "7d").await?;
//! println!("CPU P99: {}m", history.cpu_p99);
//! ```

use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error type for Prometheus client operations.
#[derive(Debug, thiserror::Error)]
pub enum PrometheusError {
    #[error("Failed to connect to Prometheus: {0}")]
    ConnectionFailed(String),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Invalid Prometheus URL: {0}")]
    InvalidUrl(String),

    #[error("Query failed: {0}")]
    QueryFailed(String),

    #[error("No data available for the specified time range")]
    NoData,

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Authentication failed: {0}")]
    AuthError(String),
}

/// Authentication method for Prometheus (optional).
///
/// Authentication is typically NOT needed when using `kubectl port-forward`
/// because the connection goes directly to the pod, bypassing ingress/auth layers.
/// Auth is only needed for externally exposed Prometheus instances.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum PrometheusAuth {
    /// No authentication (default - works for port-forward)
    #[default]
    None,
    /// Basic auth (for externally exposed Prometheus)
    Basic { username: String, password: String },
    /// Bearer token (for externally exposed Prometheus with OAuth/OIDC)
    Bearer(String),
}

/// Historical resource usage data for a container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerHistory {
    /// Pod name
    pub pod_name: String,
    /// Container name
    pub container_name: String,
    /// Namespace
    pub namespace: String,
    /// Time range queried (e.g., "7d", "30d")
    pub time_range: String,
    /// Number of data points
    pub sample_count: usize,
    /// CPU usage percentiles (in millicores)
    pub cpu_min: u64,
    pub cpu_p50: u64,
    pub cpu_p95: u64,
    pub cpu_p99: u64,
    pub cpu_max: u64,
    pub cpu_avg: u64,
    /// Memory usage percentiles (in bytes)
    pub memory_min: u64,
    pub memory_p50: u64,
    pub memory_p95: u64,
    pub memory_p99: u64,
    pub memory_max: u64,
    pub memory_avg: u64,
}

/// Aggregated history for a workload (Deployment/StatefulSet/etc).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadHistory {
    /// Workload name
    pub workload_name: String,
    /// Workload kind (Deployment, StatefulSet, etc.)
    pub workload_kind: String,
    /// Namespace
    pub namespace: String,
    /// Container histories
    pub containers: Vec<ContainerHistory>,
    /// Time range queried
    pub time_range: String,
}

/// Right-sizing recommendation based on historical data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalRecommendation {
    /// Workload name
    pub workload_name: String,
    /// Container name
    pub container_name: String,
    /// Current CPU request (millicores)
    pub current_cpu_request: Option<u64>,
    /// Recommended CPU request (millicores)
    pub recommended_cpu_request: u64,
    /// CPU savings percentage (negative if under-provisioned)
    pub cpu_savings_pct: f32,
    /// Current memory request (bytes)
    pub current_memory_request: Option<u64>,
    /// Recommended memory request (bytes)
    pub recommended_memory_request: u64,
    /// Memory savings percentage (negative if under-provisioned)
    pub memory_savings_pct: f32,
    /// Confidence level (0-100, based on sample count)
    pub confidence: u8,
    /// Safety margin applied
    pub safety_margin_pct: u8,
}

/// Prometheus client for querying historical metrics.
pub struct PrometheusClient {
    base_url: String,
    http_client: Client,
    auth: PrometheusAuth,
}

impl PrometheusClient {
    /// Create a new Prometheus client without authentication.
    ///
    /// This is the default and works for `kubectl port-forward` connections
    /// where no authentication is needed.
    pub fn new(url: &str) -> Result<Self, PrometheusError> {
        Self::with_auth(url, PrometheusAuth::None)
    }

    /// Create a new Prometheus client with optional authentication.
    ///
    /// Use this for externally exposed Prometheus instances that require auth.
    pub fn with_auth(url: &str, auth: PrometheusAuth) -> Result<Self, PrometheusError> {
        let base_url = url.trim_end_matches('/').to_string();

        // Validate URL format
        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            return Err(PrometheusError::InvalidUrl(
                "URL must start with http:// or https://".to_string(),
            ));
        }

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            base_url,
            http_client,
            auth,
        })
    }

    /// Add authentication headers to a request (if configured).
    fn add_auth(&self, req: RequestBuilder) -> RequestBuilder {
        match &self.auth {
            PrometheusAuth::None => req,
            PrometheusAuth::Basic { username, password } => {
                req.basic_auth(username, Some(password))
            }
            PrometheusAuth::Bearer(token) => req.bearer_auth(token),
        }
    }

    /// Check if Prometheus is reachable.
    pub async fn is_available(&self) -> bool {
        // Use the health endpoint which is faster and simpler
        let url = format!("{}/-/healthy", self.base_url);
        let req = self
            .http_client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5));
        match self.add_auth(req).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// Get container CPU/memory history.
    pub async fn get_container_history(
        &self,
        namespace: &str,
        pod_pattern: &str,
        container: &str,
        time_range: &str,
    ) -> Result<ContainerHistory, PrometheusError> {
        let duration = parse_duration(time_range)?;

        // Query CPU usage (rate of CPU seconds over time, converted to millicores)
        let cpu_query = format!(
            r#"rate(container_cpu_usage_seconds_total{{namespace="{}", pod=~"{}.*", container="{}"}}[5m]) * 1000"#,
            namespace, pod_pattern, container
        );

        // Query memory usage
        let memory_query = format!(
            r#"container_memory_working_set_bytes{{namespace="{}", pod=~"{}.*", container="{}"}}"#,
            namespace, pod_pattern, container
        );

        let cpu_values = self.query_range(&cpu_query, &duration).await?;
        let memory_values = self.query_range(&memory_query, &duration).await?;

        if cpu_values.is_empty() && memory_values.is_empty() {
            return Err(PrometheusError::NoData);
        }

        Ok(ContainerHistory {
            pod_name: pod_pattern.to_string(),
            container_name: container.to_string(),
            namespace: namespace.to_string(),
            time_range: time_range.to_string(),
            sample_count: cpu_values.len().max(memory_values.len()),
            cpu_min: percentile(&cpu_values, 0.0) as u64,
            cpu_p50: percentile(&cpu_values, 0.50) as u64,
            cpu_p95: percentile(&cpu_values, 0.95) as u64,
            cpu_p99: percentile(&cpu_values, 0.99) as u64,
            cpu_max: percentile(&cpu_values, 1.0) as u64,
            cpu_avg: average(&cpu_values) as u64,
            memory_min: percentile(&memory_values, 0.0) as u64,
            memory_p50: percentile(&memory_values, 0.50) as u64,
            memory_p95: percentile(&memory_values, 0.95) as u64,
            memory_p99: percentile(&memory_values, 0.99) as u64,
            memory_max: percentile(&memory_values, 1.0) as u64,
            memory_avg: average(&memory_values) as u64,
        })
    }

    /// Get history for all containers in a workload.
    pub async fn get_workload_history(
        &self,
        namespace: &str,
        workload_name: &str,
        workload_kind: &str,
        time_range: &str,
    ) -> Result<WorkloadHistory, PrometheusError> {
        // First, discover containers in this workload
        let containers = self.discover_containers(namespace, workload_name).await?;

        let mut container_histories = Vec::new();

        for container_name in containers {
            match self
                .get_container_history(namespace, workload_name, &container_name, time_range)
                .await
            {
                Ok(history) => container_histories.push(history),
                Err(PrometheusError::NoData) => continue, // Skip containers with no data
                Err(e) => return Err(e),
            }
        }

        Ok(WorkloadHistory {
            workload_name: workload_name.to_string(),
            workload_kind: workload_kind.to_string(),
            namespace: namespace.to_string(),
            containers: container_histories,
            time_range: time_range.to_string(),
        })
    }

    /// Generate right-sizing recommendations based on historical data.
    pub fn generate_recommendation(
        history: &ContainerHistory,
        current_cpu_request: Option<u64>,
        current_memory_request: Option<u64>,
        safety_margin_pct: u8,
    ) -> HistoricalRecommendation {
        let margin_multiplier = 1.0 + (safety_margin_pct as f64 / 100.0);

        // Use P99 + safety margin for recommendations
        let recommended_cpu = (history.cpu_p99 as f64 * margin_multiplier).ceil() as u64;
        let recommended_memory = (history.memory_p99 as f64 * margin_multiplier).ceil() as u64;

        // Round CPU to nice values (nearest 25m for small, 100m for larger)
        let recommended_cpu = round_cpu(recommended_cpu);
        // Round memory to nice values (nearest 64Mi)
        let recommended_memory = round_memory(recommended_memory);

        let cpu_savings_pct = current_cpu_request
            .map(|curr| ((curr as f32 - recommended_cpu as f32) / curr as f32) * 100.0)
            .unwrap_or(0.0);

        let memory_savings_pct = current_memory_request
            .map(|curr| ((curr as f32 - recommended_memory as f32) / curr as f32) * 100.0)
            .unwrap_or(0.0);

        // Confidence based on sample count
        let confidence = match history.sample_count {
            0..=10 => 20,
            11..=50 => 40,
            51..=100 => 60,
            101..=500 => 80,
            _ => 95,
        };

        HistoricalRecommendation {
            workload_name: history.pod_name.clone(),
            container_name: history.container_name.clone(),
            current_cpu_request,
            recommended_cpu_request: recommended_cpu,
            cpu_savings_pct,
            current_memory_request,
            recommended_memory_request: recommended_memory,
            memory_savings_pct,
            confidence,
            safety_margin_pct,
        }
    }

    /// Query Prometheus for a range of values.
    async fn query_range(&self, query: &str, duration: &str) -> Result<Vec<f64>, PrometheusError> {
        // Prometheus API requires Unix timestamps, not relative strings like "now-7d"
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let duration_secs = parse_duration_to_seconds(duration)?;
        let start = now - duration_secs;

        // Use 1h step for 7d+ queries to avoid too many data points
        let step = if duration_secs > 86400 * 3 {
            "1h"
        } else {
            "5m"
        };

        let url = format!(
            "{}/api/v1/query_range?query={}&start={}&end={}&step={}",
            self.base_url,
            urlencoding::encode(query),
            start,
            now,
            step
        );

        let req = self.http_client.get(&url);
        let response = self.add_auth(req).send().await?;

        if !response.status().is_success() {
            return Err(PrometheusError::QueryFailed(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let body: PrometheusResponse = response
            .json()
            .await
            .map_err(|e| PrometheusError::ParseError(format!("Failed to parse response: {}", e)))?;

        if body.status != "success" {
            return Err(PrometheusError::QueryFailed(
                body.error.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        // Extract all values from the result
        let mut values = Vec::new();
        if let Some(result) = body.data.result {
            for series in result {
                for (_, value) in series.values.unwrap_or_default() {
                    if let Ok(v) = value.parse::<f64>() {
                        if !v.is_nan() && v.is_finite() {
                            values.push(v);
                        }
                    }
                }
            }
        }

        Ok(values)
    }

    /// Discover containers in a workload.
    async fn discover_containers(
        &self,
        namespace: &str,
        workload_pattern: &str,
    ) -> Result<Vec<String>, PrometheusError> {
        let query = format!(
            r#"count by (container) (container_cpu_usage_seconds_total{{namespace="{}", pod=~"{}.*", container!="POD", container!=""}})"#,
            namespace, workload_pattern
        );

        let url = format!(
            "{}/api/v1/query?query={}",
            self.base_url,
            urlencoding::encode(&query)
        );

        let req = self.http_client.get(&url);
        let response = self.add_auth(req).send().await?;

        if !response.status().is_success() {
            return Err(PrometheusError::QueryFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let body: PrometheusResponse = response
            .json()
            .await
            .map_err(|e| PrometheusError::ParseError(format!("Failed to parse response: {}", e)))?;

        let mut containers = Vec::new();
        if let Some(result) = body.data.result {
            for series in result {
                if let Some(container) = series.metric.get("container") {
                    containers.push(container.clone());
                }
            }
        }

        Ok(containers)
    }
}

// ============================================================================
// Prometheus API response types
// ============================================================================

#[derive(Debug, Deserialize)]
struct PrometheusResponse {
    status: String,
    error: Option<String>,
    data: PrometheusData,
}

#[derive(Debug, Deserialize)]
struct PrometheusData {
    #[serde(rename = "resultType")]
    #[allow(dead_code)]
    result_type: Option<String>,
    result: Option<Vec<PrometheusResult>>,
}

#[derive(Debug, Deserialize)]
struct PrometheusResult {
    metric: HashMap<String, String>,
    #[allow(dead_code)]
    value: Option<(f64, String)>, // For instant queries
    values: Option<Vec<(f64, String)>>, // For range queries
}

// ============================================================================
// Helper functions
// ============================================================================

/// Parse a duration string (e.g., "7d", "24h", "30m") to Prometheus format.
fn parse_duration(duration: &str) -> Result<String, PrometheusError> {
    let duration = duration.trim().to_lowercase();

    // Prometheus already understands these formats
    if duration.ends_with('d')
        || duration.ends_with('h')
        || duration.ends_with('m')
        || duration.ends_with('s')
    {
        Ok(duration)
    } else if duration.ends_with("day") || duration.ends_with("days") {
        let num: u32 = duration
            .trim_end_matches(|c: char| c.is_alphabetic())
            .trim()
            .parse()
            .map_err(|_| PrometheusError::ParseError("Invalid duration number".to_string()))?;
        Ok(format!("{}d", num))
    } else if duration.ends_with("week") || duration.ends_with("weeks") {
        let num: u32 = duration
            .trim_end_matches(|c: char| c.is_alphabetic())
            .trim()
            .parse()
            .map_err(|_| PrometheusError::ParseError("Invalid duration number".to_string()))?;
        Ok(format!("{}d", num * 7))
    } else {
        // Default to treating as days
        let num: u32 = duration
            .parse()
            .map_err(|_| PrometheusError::ParseError(format!("Invalid duration: {}", duration)))?;
        Ok(format!("{}d", num))
    }
}

/// Parse a duration string (e.g., "7d", "24h", "30m") to seconds.
fn parse_duration_to_seconds(duration: &str) -> Result<u64, PrometheusError> {
    let duration = duration.trim().to_lowercase();

    // Extract the numeric part and unit
    let (num_str, unit) = if duration.ends_with("days") {
        (duration.trim_end_matches("days").trim(), "d")
    } else if duration.ends_with("day") {
        (duration.trim_end_matches("day").trim(), "d")
    } else if duration.ends_with("weeks") {
        (duration.trim_end_matches("weeks").trim(), "w")
    } else if duration.ends_with("week") {
        (duration.trim_end_matches("week").trim(), "w")
    } else if duration.ends_with('d') {
        (duration.trim_end_matches('d'), "d")
    } else if duration.ends_with('h') {
        (duration.trim_end_matches('h'), "h")
    } else if duration.ends_with('m') {
        (duration.trim_end_matches('m'), "m")
    } else if duration.ends_with('s') {
        (duration.trim_end_matches('s'), "s")
    } else {
        // Default to days
        (duration.as_str(), "d")
    };

    let num: u64 = num_str.parse().map_err(|_| {
        PrometheusError::ParseError(format!("Invalid duration number: {}", duration))
    })?;

    let seconds = match unit {
        "w" => num * 7 * 24 * 60 * 60,
        "d" => num * 24 * 60 * 60,
        "h" => num * 60 * 60,
        "m" => num * 60,
        "s" => num,
        _ => num * 24 * 60 * 60, // Default to days
    };

    Ok(seconds)
}

/// Calculate percentile of a sorted slice.
fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if p <= 0.0 {
        return sorted[0];
    }
    if p >= 1.0 {
        return sorted[sorted.len() - 1];
    }

    let index = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[index]
}

/// Calculate average of values.
fn average(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Round CPU millicores to nice values.
fn round_cpu(millicores: u64) -> u64 {
    if millicores <= 100 {
        // Round to nearest 25m
        ((millicores + 12) / 25) * 25
    } else if millicores <= 1000 {
        // Round to nearest 50m
        ((millicores + 25) / 50) * 50
    } else {
        // Round to nearest 100m
        ((millicores + 50) / 100) * 100
    }
}

/// Round memory bytes to nice values (64Mi increments).
fn round_memory(bytes: u64) -> u64 {
    const MI: u64 = 1024 * 1024;
    const INCREMENT: u64 = 64 * MI;

    if bytes <= 128 * MI {
        // Round to nearest 32Mi for small values
        let increment = 32 * MI;
        ((bytes + increment / 2) / increment) * increment
    } else {
        // Round to nearest 64Mi
        ((bytes + INCREMENT / 2) / INCREMENT) * INCREMENT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("7d").unwrap(), "7d");
        assert_eq!(parse_duration("24h").unwrap(), "24h");
        assert_eq!(parse_duration("30m").unwrap(), "30m");
        assert_eq!(parse_duration("1week").unwrap(), "7d");
        assert_eq!(parse_duration("2weeks").unwrap(), "14d");
    }

    #[test]
    fn test_percentile() {
        let values = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
        assert!((percentile(&values, 0.0) - 10.0).abs() < 0.1);
        assert!((percentile(&values, 0.5) - 55.0).abs() < 5.1); // ~50th percentile
        assert!((percentile(&values, 1.0) - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_round_cpu() {
        assert_eq!(round_cpu(12), 25);
        assert_eq!(round_cpu(23), 25);
        assert_eq!(round_cpu(37), 50);
        assert_eq!(round_cpu(120), 100);
        assert_eq!(round_cpu(175), 200);
        assert_eq!(round_cpu(1234), 1200);
    }

    #[test]
    fn test_round_memory() {
        const MI: u64 = 1024 * 1024;
        assert_eq!(round_memory(50 * MI), 64 * MI);
        assert_eq!(round_memory(100 * MI), 96 * MI);
        assert_eq!(round_memory(200 * MI), 192 * MI);
        assert_eq!(round_memory(500 * MI), 512 * MI);
    }

    #[test]
    fn test_parse_duration_to_seconds() {
        // Days
        assert_eq!(parse_duration_to_seconds("7d").unwrap(), 7 * 24 * 60 * 60);
        assert_eq!(parse_duration_to_seconds("1d").unwrap(), 24 * 60 * 60);
        // Hours
        assert_eq!(parse_duration_to_seconds("24h").unwrap(), 24 * 60 * 60);
        assert_eq!(parse_duration_to_seconds("1h").unwrap(), 60 * 60);
        // Minutes
        assert_eq!(parse_duration_to_seconds("30m").unwrap(), 30 * 60);
        // Weeks
        assert_eq!(
            parse_duration_to_seconds("1week").unwrap(),
            7 * 24 * 60 * 60
        );
        assert_eq!(
            parse_duration_to_seconds("2weeks").unwrap(),
            14 * 24 * 60 * 60
        );
    }
}
