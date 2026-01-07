//! Prometheus Discovery Tool
//!
//! Discovers Prometheus services running in a Kubernetes cluster.
//! Used to find Prometheus for live K8s optimization analysis.
//!
//! # Usage Flow
//!
//! 1. Use `prometheus_discover` to find Prometheus in cluster
//! 2. Use `prometheus_connect` to establish connection
//! 3. Use `k8s_optimize` with the connection for live analysis

use crate::agent::ui::prometheus_display::{DiscoveredService, PrometheusDiscoveryDisplay};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;

/// Arguments for the prometheus_discover tool
#[derive(Debug, Deserialize)]
pub struct PrometheusDiscoverArgs {
    /// Kubernetes context (optional, uses current context if not specified)
    #[serde(default)]
    pub cluster: Option<String>,

    /// Namespace to search in (optional, searches all namespaces if not specified)
    #[serde(default)]
    pub namespace: Option<String>,

    /// Service name pattern to match (default: "prometheus")
    #[serde(default)]
    pub service_pattern: Option<String>,
}

/// A discovered Prometheus service
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredPrometheus {
    pub name: String,
    pub namespace: String,
    pub port: u16,
    pub service_type: String,
    pub cluster_ip: Option<String>,
}

/// Error type for prometheus discovery
#[derive(Debug, thiserror::Error)]
#[error("Prometheus discovery error: {0}")]
pub struct PrometheusDiscoverError(String);

/// Tool for discovering Prometheus in Kubernetes clusters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusDiscoverTool;

impl Default for PrometheusDiscoverTool {
    fn default() -> Self {
        Self::new()
    }
}

impl PrometheusDiscoverTool {
    /// Create a new PrometheusDiscoverTool
    pub fn new() -> Self {
        Self
    }

    /// Run kubectl to get services
    async fn get_services(
        &self,
        namespace: Option<&str>,
        context: Option<&str>,
    ) -> Result<String, PrometheusDiscoverError> {
        let mut cmd = Command::new("kubectl");
        cmd.arg("get").arg("svc");

        if let Some(ns) = namespace {
            cmd.arg("-n").arg(ns);
        } else {
            cmd.arg("-A"); // All namespaces
        }

        cmd.arg("-o").arg("json");

        if let Some(ctx) = context {
            cmd.arg("--context").arg(ctx);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd
            .output()
            .await
            .map_err(|e| PrometheusDiscoverError(format!("Failed to run kubectl: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PrometheusDiscoverError(format!(
                "kubectl failed: {}",
                stderr.trim()
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Parse services JSON and find Prometheus SERVER services specifically
    /// We need to be precise - only find the actual Prometheus server, not every monitoring component
    fn find_prometheus_services(
        &self,
        json_str: &str,
        _pattern: &str,
    ) -> Vec<DiscoveredPrometheus> {
        let mut discovered = Vec::new();

        // Parse JSON
        let json: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => return discovered,
        };

        // Get items array
        let items = match json.get("items").and_then(|v| v.as_array()) {
            Some(items) => items,
            None => return discovered,
        };

        for item in items {
            let metadata = match item.get("metadata") {
                Some(m) => m,
                None => continue,
            };

            let name = metadata.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let namespace = metadata
                .get("namespace")
                .and_then(|v| v.as_str())
                .unwrap_or("default");

            // Get spec and check for port 9090 (Prometheus API port)
            let spec = match item.get("spec") {
                Some(s) => s,
                None => continue,
            };

            let ports = spec.get("ports").and_then(|v| v.as_array());
            let has_prometheus_port = ports
                .map(|p| {
                    p.iter()
                        .any(|port| port.get("port").and_then(|v| v.as_u64()) == Some(9090))
                })
                .unwrap_or(false);

            // STRICT FILTERING: Must be the actual Prometheus server
            // Method 1: Service name is specifically prometheus-like AND has port 9090
            let name_lower = name.to_lowercase();
            let is_prometheus_by_name = has_prometheus_port
                && (
                    // Exact patterns for Prometheus server services
                    name_lower == "prometheus" ||
                name_lower == "prometheus-server" ||
                name_lower == "prometheus-operated" ||
                name_lower.ends_with("-prometheus") ||        // e.g., monitoring-prometheus
                name_lower.ends_with("-prometheus-server") ||
                // But NOT node-exporter, alertmanager, etc.
                (name_lower.contains("prometheus") &&
                 !name_lower.contains("node-exporter") &&
                 !name_lower.contains("alertmanager") &&
                 !name_lower.contains("pushgateway") &&
                 !name_lower.contains("blackbox") &&
                 !name_lower.contains("adapter"))
                );

            // Method 2: Check for app.kubernetes.io/name=prometheus label
            let labels = metadata.get("labels").and_then(|l| l.as_object());
            let is_prometheus_by_label = has_prometheus_port
                && labels
                    .map(|obj| {
                        // Check for specific Prometheus server labels
                        obj.get("app.kubernetes.io/name")
                            .and_then(|v| v.as_str())
                            .map(|s| s == "prometheus")
                            .unwrap_or(false)
                            || obj
                                .get("app")
                                .and_then(|v| v.as_str())
                                .map(|s| {
                                    s == "prometheus" || s.contains("prometheus-stack-prometheus")
                                })
                                .unwrap_or(false)
                    })
                    .unwrap_or(false);

            if !is_prometheus_by_name && !is_prometheus_by_label {
                continue;
            }

            let service_type = spec
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("ClusterIP");
            let cluster_ip = spec.get("clusterIP").and_then(|v| v.as_str());

            discovered.push(DiscoveredPrometheus {
                name: name.to_string(),
                namespace: namespace.to_string(),
                port: 9090, // Always 9090 for Prometheus server
                service_type: service_type.to_string(),
                cluster_ip: cluster_ip.map(|s| s.to_string()),
            });
        }

        // Deduplicate - prefer the main service over -operated
        if discovered.len() > 1 {
            // Sort so main service comes first (not -operated)
            discovered.sort_by(|a, b| {
                let a_is_operated = a.name.contains("operated");
                let b_is_operated = b.name.contains("operated");
                a_is_operated.cmp(&b_is_operated)
            });
        }

        discovered
    }
}

impl Tool for PrometheusDiscoverTool {
    const NAME: &'static str = "prometheus_discover";

    type Args = PrometheusDiscoverArgs;
    type Output = String;
    type Error = PrometheusDiscoverError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Discover Prometheus services in a Kubernetes cluster.

**Use this tool when:**
- User asks for K8s optimization with live/historical metrics
- Need to find Prometheus for data-driven recommendations

**What it does:**
- Searches for services with "prometheus" in the name or labels
- Returns discovered services with namespace, port, and type
- Suggests using prometheus_connect to establish connection

**Returns:**
- List of discovered Prometheus services
- Connection suggestions

**Next steps after discovery:**
1. Use `prometheus_connect` with the discovered service
2. Then use `k8s_optimize` with the established connection"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "cluster": {
                        "type": "string",
                        "description": "Kubernetes context name (optional, uses current context)"
                    },
                    "namespace": {
                        "type": "string",
                        "description": "Namespace to search (optional, searches all namespaces)"
                    },
                    "service_pattern": {
                        "type": "string",
                        "description": "Pattern to match service names (default: 'prometheus')"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let pattern = args.service_pattern.as_deref().unwrap_or("prometheus");

        // Start display
        let mut display = PrometheusDiscoveryDisplay::new();
        display.start(args.namespace.as_deref());

        // Get services from cluster
        let services_json = match self
            .get_services(args.namespace.as_deref(), args.cluster.as_deref())
            .await
        {
            Ok(json) => json,
            Err(e) => {
                display.error(&e.to_string());
                return Err(e);
            }
        };

        // Find Prometheus services
        let mut discovered = self.find_prometheus_services(&services_json, pattern);
        let mut used_fallback = false;
        let original_namespace = args.namespace.clone();

        // FALLBACK: If specific namespace was provided but no results found, try ALL namespaces
        // This handles the common case where agent assumes "prometheus" namespace but services
        // are actually in "monitoring" or other namespace
        if discovered.is_empty() && args.namespace.is_some() {
            log::info!(
                "No Prometheus found in '{}' namespace, searching all namespaces...",
                args.namespace.as_deref().unwrap_or("")
            );
            display.searching_all_namespaces();

            if let Ok(all_json) = self.get_services(None, args.cluster.as_deref()).await {
                discovered = self.find_prometheus_services(&all_json, pattern);
                if !discovered.is_empty() {
                    used_fallback = true;
                }
            }
        }

        // Convert to display format
        let display_services: Vec<DiscoveredService> = discovered
            .iter()
            .map(|d| DiscoveredService {
                name: d.name.clone(),
                namespace: d.namespace.clone(),
                port: d.port,
                service_type: d.service_type.clone(),
            })
            .collect();

        // Show results in terminal UI
        display.found_services(&display_services);

        // Show suggestion if services found
        if let Some(first) = display_services.first() {
            display.show_suggestion(first);
        }

        // Build JSON response for agent
        let response = if discovered.is_empty() {
            json!({
                "found": false,
                "discovered": [],
                "message": "No Prometheus services found in cluster",
                "suggestions": [
                    "Check if Prometheus is installed in a different namespace",
                    "Provide an external Prometheus URL using prometheus_connect with url parameter",
                    "Install Prometheus using Helm: helm install prometheus prometheus-community/prometheus"
                ]
            })
        } else {
            let message = if used_fallback {
                format!(
                    "Found {} Prometheus service(s) (note: not in '{}' namespace as specified, but found in other namespaces)",
                    discovered.len(),
                    original_namespace.as_deref().unwrap_or("")
                )
            } else {
                format!("Found {} Prometheus service(s)", discovered.len())
            };

            json!({
                "found": true,
                "used_fallback_search": used_fallback,
                "discovered": discovered.iter().map(|d| json!({
                    "name": d.name,
                    "namespace": d.namespace,
                    "port": d.port,
                    "type": d.service_type,
                    "cluster_ip": d.cluster_ip,
                    "resource": format!("svc/{}", d.name)
                })).collect::<Vec<_>>(),
                "message": message,
                "next_step": "Use prometheus_connect to establish connection",
                "example": {
                    "tool": "prometheus_connect",
                    "args": {
                        "service": discovered.first().map(|d| d.name.clone()),
                        "namespace": discovered.first().map(|d| d.namespace.clone()),
                        "port": discovered.first().map(|d| d.port)
                    }
                }
            })
        };

        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(PrometheusDiscoverTool::NAME, "prometheus_discover");
    }

    #[test]
    fn test_find_prometheus_services() {
        let tool = PrometheusDiscoverTool::new();

        let json = r#"{
            "items": [
                {
                    "metadata": {
                        "name": "prometheus-server",
                        "namespace": "monitoring"
                    },
                    "spec": {
                        "type": "ClusterIP",
                        "clusterIP": "10.0.0.100",
                        "ports": [{"port": 9090, "name": "web"}]
                    }
                },
                {
                    "metadata": {
                        "name": "grafana",
                        "namespace": "monitoring"
                    },
                    "spec": {
                        "type": "ClusterIP",
                        "ports": [{"port": 3000}]
                    }
                }
            ]
        }"#;

        let discovered = tool.find_prometheus_services(json, "prometheus");
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].name, "prometheus-server");
        assert_eq!(discovered[0].namespace, "monitoring");
        assert_eq!(discovered[0].port, 9090);
    }

    #[test]
    fn test_find_prometheus_by_label() {
        let tool = PrometheusDiscoverTool::new();

        let json = r#"{
            "items": [
                {
                    "metadata": {
                        "name": "kube-prometheus-stack-prometheus",
                        "namespace": "monitoring",
                        "labels": {
                            "app": "prometheus"
                        }
                    },
                    "spec": {
                        "type": "ClusterIP",
                        "ports": [{"port": 9090}]
                    }
                }
            ]
        }"#;

        let discovered = tool.find_prometheus_services(json, "prometheus");
        assert_eq!(discovered.len(), 1);
    }

    #[test]
    fn test_no_prometheus_found() {
        let tool = PrometheusDiscoverTool::new();

        let json = r#"{"items": []}"#;

        let discovered = tool.find_prometheus_services(json, "prometheus");
        assert!(discovered.is_empty());
    }

    #[test]
    fn test_filters_out_non_prometheus_services() {
        let tool = PrometheusDiscoverTool::new();

        // This JSON includes services that should be filtered OUT:
        // - node-exporter (different service)
        // - alertmanager (different service)
        // - monitoring-coredns (unrelated, but might have prometheus labels)
        // Only monitoring-prometheus should match
        let json = r#"{
            "items": [
                {
                    "metadata": {
                        "name": "monitoring-prometheus",
                        "namespace": "monitoring",
                        "labels": {"app": "prometheus"}
                    },
                    "spec": {
                        "type": "ClusterIP",
                        "ports": [{"port": 9090}]
                    }
                },
                {
                    "metadata": {
                        "name": "monitoring-prometheus-node-exporter",
                        "namespace": "monitoring",
                        "labels": {"app": "prometheus-node-exporter"}
                    },
                    "spec": {
                        "type": "ClusterIP",
                        "ports": [{"port": 9100}]
                    }
                },
                {
                    "metadata": {
                        "name": "alertmanager-operated",
                        "namespace": "monitoring",
                        "labels": {"app": "alertmanager"}
                    },
                    "spec": {
                        "type": "ClusterIP",
                        "ports": [{"port": 9093}]
                    }
                },
                {
                    "metadata": {
                        "name": "monitoring-coredns",
                        "namespace": "kube-system",
                        "labels": {"prometheus.io/scrape": "true"}
                    },
                    "spec": {
                        "type": "ClusterIP",
                        "ports": [{"port": 9153}]
                    }
                }
            ]
        }"#;

        let discovered = tool.find_prometheus_services(json, "prometheus");
        // Only monitoring-prometheus should be found
        assert_eq!(
            discovered.len(),
            1,
            "Should only find 1 service, found: {:?}",
            discovered
        );
        assert_eq!(discovered[0].name, "monitoring-prometheus");
    }
}
