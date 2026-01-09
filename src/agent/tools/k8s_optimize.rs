//! K8s Optimize tool - Native Kubernetes resource optimization using Rig's Tool trait
//!
//! Analyzes Kubernetes manifests for over-provisioned or under-provisioned
//! resources and suggests right-sized values.
//!
//! Output is optimized for AI agent decision-making with:
//! - Categorized issues (over-provisioned, under-provisioned, missing resources)
//! - Priority rankings (critical, high, medium, low)
//! - Actionable fix recommendations with YAML snippets
//! - Cost savings estimates (when available)
//! - Live cluster analysis (optional, via Prometheus)
//!
//! # Prometheus Integration
//!
//! For data-driven recommendations based on actual usage:
//! 1. Use `prometheus_discover` to find Prometheus in cluster
//! 2. Use `prometheus_connect` to establish connection (port-forward or URL)
//! 3. Use `k8s_optimize` with the prometheus URL from step 2

use super::compression::{CompressionConfig, compress_tool_output};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use crate::analyzer::k8s_optimize::{
    K8sOptimizeConfig, OptimizationResult, PrometheusAuth, PrometheusClient, Severity, analyze,
    analyze_content, bytes_to_memory_string, millicores_to_cpu_string, parse_cpu_to_millicores,
    parse_memory_to_bytes, rule_codes, rule_description,
};

/// Arguments for the k8s-optimize tool
#[derive(Debug, Deserialize)]
pub struct K8sOptimizeArgs {
    /// Path to K8s manifest file or directory (relative to project root)
    #[serde(default)]
    pub path: Option<String>,

    /// Inline YAML content to analyze (alternative to path)
    #[serde(default)]
    pub content: Option<String>,

    /// Minimum severity to report: "critical", "high", "medium", "low", "info"
    #[serde(default)]
    pub severity: Option<String>,

    /// Minimum waste percentage to report (default: 10)
    #[serde(default)]
    pub threshold: Option<u8>,

    /// Include info-level suggestions
    #[serde(default)]
    pub include_info: bool,

    /// Include system namespaces (kube-system, etc.)
    #[serde(default)]
    pub include_system: bool,

    /// Run FULL comprehensive analysis (optimize + kubelint security + helmlint)
    #[serde(default)]
    pub full: bool,

    // ========== Live Analysis Options (Phase 2) ==========
    /// Connect to a Kubernetes cluster (kubeconfig context name)
    #[serde(default)]
    pub cluster: Option<String>,

    /// Prometheus URL for historical metrics (e.g., "http://localhost:9090" from port-forward)
    /// Use prometheus_discover and prometheus_connect tools to get this URL
    #[serde(default)]
    pub prometheus: Option<String>,

    /// Prometheus authentication type: "none", "basic", "bearer" (default: "none")
    /// Only needed for externally exposed Prometheus, NOT for port-forward connections
    #[serde(default)]
    pub prometheus_auth_type: Option<String>,

    /// Username for Prometheus basic auth (only for external Prometheus)
    #[serde(default)]
    pub prometheus_username: Option<String>,

    /// Password for Prometheus basic auth (only for external Prometheus)
    #[serde(default)]
    pub prometheus_password: Option<String>,

    /// Bearer token for Prometheus auth (only for external Prometheus)
    #[serde(default)]
    pub prometheus_token: Option<String>,

    /// Analysis period for live metrics (e.g., "7d", "24h", "1h")
    #[serde(default)]
    pub period: Option<String>,

    // ========== Cost Estimation Options (Phase 3) ==========
    /// Cloud provider for cost estimation: "aws", "gcp", "azure", "onprem"
    #[serde(default)]
    pub cloud_provider: Option<String>,

    /// Cloud region for pricing (e.g., "us-east-1", "us-central1")
    #[serde(default)]
    pub region: Option<String>,
}

/// Error type for k8s-optimize tool
#[derive(Debug, thiserror::Error)]
#[error("K8s optimize error: {0}")]
pub struct K8sOptimizeError(String);

/// Result of Prometheus enhancement
struct PrometheusEnhancement {
    /// Number of recommendations enhanced with live data
    enhanced_count: usize,
    /// Number of workloads with no Prometheus data
    no_data_count: usize,
    /// Raw Prometheus data for each workload
    prometheus_data: Vec<serde_json::Value>,
}

/// Find Helm charts in a directory.
fn find_helm_charts(path: &std::path::Path) -> Vec<PathBuf> {
    let mut charts = Vec::new();

    if path.join("Chart.yaml").exists() {
        charts.push(path.to_path_buf());
        return charts;
    }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                if entry_path.join("Chart.yaml").exists() {
                    charts.push(entry_path);
                } else if let Ok(sub_entries) = std::fs::read_dir(&entry_path) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.is_dir() && sub_path.join("Chart.yaml").exists() {
                            charts.push(sub_path);
                        }
                    }
                }
            }
        }
    }

    charts
}

/// Tool for analyzing Kubernetes resource configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sOptimizeTool {
    project_root: PathBuf,
}

impl K8sOptimizeTool {
    /// Create a new K8sOptimizeTool with the given project root.
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Build PrometheusAuth from arguments (optional, only for external URLs)
    fn build_prometheus_auth(args: &K8sOptimizeArgs) -> PrometheusAuth {
        match args.prometheus_auth_type.as_deref() {
            Some("basic") => {
                if let (Some(username), Some(password)) =
                    (&args.prometheus_username, &args.prometheus_password)
                {
                    PrometheusAuth::Basic {
                        username: username.clone(),
                        password: password.clone(),
                    }
                } else {
                    PrometheusAuth::None
                }
            }
            Some("bearer") => {
                if let Some(token) = &args.prometheus_token {
                    PrometheusAuth::Bearer(token.clone())
                } else {
                    PrometheusAuth::None
                }
            }
            _ => PrometheusAuth::None,
        }
    }

    /// Enhance recommendations with live Prometheus data.
    ///
    /// For each workload in the static analysis, query Prometheus for historical
    /// CPU/memory usage and replace heuristic recommendations with data-driven ones.
    async fn enhance_with_prometheus(
        &self,
        result: &mut OptimizationResult,
        client: &PrometheusClient,
        period: &str,
    ) -> PrometheusEnhancement {
        let mut enhanced_count = 0;
        let mut no_data_count = 0;
        let mut prometheus_data: Vec<serde_json::Value> = Vec::new();

        for rec in &mut result.recommendations {
            let namespace = rec.namespace.as_deref().unwrap_or("default");
            let workload_name = &rec.resource_name;
            let container = &rec.container;

            // Parse current resource values from String to u64
            let current_cpu_millicores = rec
                .current
                .cpu_request
                .as_ref()
                .and_then(|s| parse_cpu_to_millicores(s));
            let current_memory_bytes = rec
                .current
                .memory_request
                .as_ref()
                .and_then(|s| parse_memory_to_bytes(s));

            // Query Prometheus for historical data
            match client
                .get_container_history(namespace, workload_name, container, period)
                .await
            {
                Ok(history) => {
                    // Generate data-driven recommendation
                    let historical_rec = PrometheusClient::generate_recommendation(
                        &history,
                        current_cpu_millicores,
                        current_memory_bytes,
                        20, // 20% safety margin
                    );

                    // Convert recommended values back to strings
                    let cpu_str = millicores_to_cpu_string(historical_rec.recommended_cpu_request);
                    let mem_str = bytes_to_memory_string(historical_rec.recommended_memory_request);
                    let cpu_limit_str =
                        millicores_to_cpu_string(historical_rec.recommended_cpu_request * 2);

                    // Store the prometheus data for output
                    prometheus_data.push(serde_json::json!({
                        "workload": format!("{}/{}", namespace, workload_name),
                        "container": container,
                        "period": period,
                        "samples": history.sample_count,
                        "cpu_usage": {
                            "min": history.cpu_min,
                            "p50": history.cpu_p50,
                            "p95": history.cpu_p95,
                            "p99": history.cpu_p99,
                            "max": history.cpu_max,
                            "avg": history.cpu_avg,
                        },
                        "memory_usage": {
                            "min_bytes": history.memory_min,
                            "p50_bytes": history.memory_p50,
                            "p95_bytes": history.memory_p95,
                            "p99_bytes": history.memory_p99,
                            "max_bytes": history.memory_max,
                            "avg_bytes": history.memory_avg,
                        },
                        "recommendation": {
                            "cpu_request": cpu_str,
                            "memory_request": mem_str,
                            "cpu_savings_pct": historical_rec.cpu_savings_pct,
                            "memory_savings_pct": historical_rec.memory_savings_pct,
                            "confidence": historical_rec.confidence,
                        }
                    }));

                    // Update the recommendation with data-driven values (as strings)
                    rec.recommended.cpu_request = Some(cpu_str.clone());
                    rec.recommended.memory_request = Some(mem_str.clone());

                    // Update fix_yaml with data-driven values
                    rec.fix_yaml = format!(
                        "resources:\n  requests:\n    cpu: \"{}\"\n    memory: \"{}\"\n  limits:\n    cpu: \"{}\"  # 2x request\n    memory: \"{}\"",
                        cpu_str, mem_str, cpu_limit_str, mem_str,
                    );

                    // Update message to indicate data-driven
                    rec.message = format!(
                        "{} [DATA-DRIVEN: P99 usage CPU={}m, Memory={}Mi over {}, confidence={}%]",
                        rec.message,
                        history.cpu_p99,
                        history.memory_p99 / (1024 * 1024),
                        period,
                        historical_rec.confidence
                    );

                    enhanced_count += 1;
                }
                Err(_) => {
                    // No Prometheus data for this workload, keep heuristic
                    no_data_count += 1;
                }
            }
        }

        PrometheusEnhancement {
            enhanced_count,
            no_data_count,
            prometheus_data,
        }
    }

    /// Build config from arguments.
    fn build_config(&self, args: &K8sOptimizeArgs) -> K8sOptimizeConfig {
        let mut config = K8sOptimizeConfig::default();

        if let Some(severity_str) = &args.severity {
            if let Some(severity) = Severity::parse(severity_str) {
                config = config.with_severity(severity);
            }
        }

        if let Some(threshold) = args.threshold {
            config = config.with_threshold(threshold);
        }

        if args.include_info {
            config = config.with_info();
        }

        if args.include_system {
            config = config.with_system();
        }

        config
    }

    /// Format result for AI agent consumption.
    fn format_for_agent(
        &self,
        result: &OptimizationResult,
        args: &K8sOptimizeArgs,
    ) -> serde_json::Value {
        // Create a summary for the agent
        let mut response = json!({
            "summary": {
                "resources_analyzed": result.summary.resources_analyzed,
                "containers_analyzed": result.summary.containers_analyzed,
                "over_provisioned": result.summary.over_provisioned,
                "under_provisioned": result.summary.under_provisioned,
                "missing_requests": result.summary.missing_requests,
                "missing_limits": result.summary.missing_limits,
                "optimal": result.summary.optimal,
                "total_waste_percentage": result.summary.total_waste_percentage,
                "mode": result.metadata.mode.to_string(),
            },
            "recommendations": result.recommendations.iter().map(|r| {
                json!({
                    "resource": format!("{}/{}", r.resource_kind, r.resource_name),
                    "container": r.container,
                    "namespace": r.namespace,
                    "file": r.file_path.display().to_string(),
                    "line": r.line,
                    "issue": r.issue.to_string(),
                    "severity": r.severity.as_str(),
                    "message": r.message,
                    "workload_type": r.workload_type.as_str(),
                    "rule_code": r.rule_code.as_str(),
                    "rule_description": rule_description(r.rule_code.as_str()),
                    "current": {
                        "cpu_request": r.current.cpu_request,
                        "cpu_limit": r.current.cpu_limit,
                        "memory_request": r.current.memory_request,
                        "memory_limit": r.current.memory_limit,
                    },
                    "recommended": {
                        "cpu_request": r.recommended.cpu_request,
                        "cpu_limit": r.recommended.cpu_limit,
                        "memory_request": r.recommended.memory_request,
                        "memory_limit": r.recommended.memory_limit,
                    },
                    "fix_yaml": r.fix_yaml,
                    // Quick fix for agent to apply
                    "quick_fix": {
                        "action": "replace_resources",
                        "file": r.file_path.display().to_string(),
                        "container": r.container.clone(),
                        "yaml": r.fix_yaml.clone(),
                    }
                })
            }).collect::<Vec<_>>(),
            "analysis_metadata": {
                "duration_ms": result.metadata.duration_ms,
                "path": result.metadata.path.display().to_string(),
                "version": result.metadata.version.clone(),
                "timestamp": result.metadata.timestamp.clone(),
            }
        });

        // Add warnings if any
        if !result.warnings.is_empty() {
            response["warnings"] = json!(
                result
                    .warnings
                    .iter()
                    .map(|w| {
                        json!({
                            "resource": w.resource,
                            "issue": w.issue.to_string(),
                            "severity": w.severity.as_str(),
                            "message": w.message,
                        })
                    })
                    .collect::<Vec<_>>()
            );
        }

        // Add savings estimate if available
        if let Some(savings) = result.summary.estimated_monthly_savings_usd {
            response["estimated_savings"] = json!({
                "monthly_usd": savings,
                "annual_usd": savings * 12.0,
            });
        }

        // Add rule reference for agent
        response["rule_codes"] = json!({
            rule_codes::NO_CPU_REQUEST: rule_description(rule_codes::NO_CPU_REQUEST),
            rule_codes::NO_MEMORY_REQUEST: rule_description(rule_codes::NO_MEMORY_REQUEST),
            rule_codes::NO_CPU_LIMIT: rule_description(rule_codes::NO_CPU_LIMIT),
            rule_codes::NO_MEMORY_LIMIT: rule_description(rule_codes::NO_MEMORY_LIMIT),
            rule_codes::HIGH_CPU_REQUEST: rule_description(rule_codes::HIGH_CPU_REQUEST),
            rule_codes::HIGH_MEMORY_REQUEST: rule_description(rule_codes::HIGH_MEMORY_REQUEST),
            rule_codes::EXCESSIVE_CPU_RATIO: rule_description(rule_codes::EXCESSIVE_CPU_RATIO),
            rule_codes::EXCESSIVE_MEMORY_RATIO: rule_description(rule_codes::EXCESSIVE_MEMORY_RATIO),
            rule_codes::REQUESTS_EQUAL_LIMITS: rule_description(rule_codes::REQUESTS_EQUAL_LIMITS),
            rule_codes::UNBALANCED_RESOURCES: rule_description(rule_codes::UNBALANCED_RESOURCES),
        });

        // Add live analysis info if cluster or prometheus was specified
        if args.cluster.is_some() || args.prometheus.is_some() {
            response["live_analysis"] = json!({
                "enabled": args.prometheus.is_some(),
                "cluster": args.cluster.clone(),
                "prometheus": args.prometheus.clone(),
                "prometheus_auth": if args.prometheus_auth_type.is_some() {
                    args.prometheus_auth_type.clone()
                } else {
                    Some("none".to_string())
                },
                "period": args.period.clone().unwrap_or_else(|| "7d".to_string()),
                "note": if args.prometheus.is_some() {
                    "Historical metrics analysis using Prometheus data."
                } else {
                    "Live analysis requires Prometheus. Use prometheus_discover and prometheus_connect to set up."
                },
            });
        }

        // Add cost estimation info if provider was specified
        if args.cloud_provider.is_some() {
            response["cost_estimation"] = json!({
                "enabled": true,
                "provider": args.cloud_provider.clone(),
                "region": args.region.clone().unwrap_or_else(|| "us-east-1".to_string()),
                "note": "Cost estimation uses approximate on-demand pricing. Actual costs may vary.",
            });
        }

        // Add actionable summary for agent
        let action_items: Vec<String> = result
            .recommendations
            .iter()
            .filter(|r| r.severity >= Severity::Medium)
            .map(|r| {
                format!(
                    "[{}] {} in {}/{}",
                    r.rule_code.as_str(),
                    r.message,
                    r.resource_kind,
                    r.resource_name
                )
            })
            .collect();

        if !action_items.is_empty() {
            response["action_items"] = json!(action_items);
        }

        response
    }
}

impl Tool for K8sOptimizeTool {
    const NAME: &'static str = "k8s_optimize";

    type Args = K8sOptimizeArgs;
    type Output = String;
    type Error = K8sOptimizeError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Analyze Kubernetes manifests for resource optimization.

**IMPORTANT: Only use when user EXPLICITLY asks about:**
- "optimize my K8s resources" / "right-size my pods"
- "full analysis" / "comprehensive check" (use full=true)
- Over-provisioned or under-provisioned resources
- Cost optimization for Kubernetes

**DO NOT use for:**
- General K8s linting without optimization focus (use kubelint)
- Tasks where user didn't ask about optimization

## For Live Cluster Analysis with Historical Metrics

**RECOMMENDED FLOW when user wants data-driven optimization:**
1. First use `prometheus_discover` to find Prometheus in cluster
2. Use `prometheus_connect` to establish connection (starts port-forward)
3. Call `k8s_optimize` with the prometheus URL from step 2

Port-forward is preferred (no auth needed). Auth is only needed for external Prometheus URLs.

## Modes
- **Standard**: Resource optimization analysis only
- **Full** (full=true): Comprehensive analysis including:
  - Resource optimization (CPU/memory waste)
  - Security checks (kubelint - privileged, RBAC, etc.)
  - Helm validation (if charts present)
- **Live**: With prometheus URL for historical metrics (data-driven recommendations)

## Returns (analysis only - does NOT apply changes)
- Summary with issue counts and waste percentage
- Recommendations with suggested values (based on actual usage if Prometheus provided)
- Security findings (if full=true)
- Does NOT automatically modify files"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to K8s manifest file or directory (relative to project root). Examples: 'k8s/', 'deployments/api.yaml', 'charts/myapp/', 'terraform/'"
                    },
                    "content": {
                        "type": "string",
                        "description": "Inline YAML content to analyze (alternative to path)"
                    },
                    "severity": {
                        "type": "string",
                        "description": "Minimum severity to report: 'critical', 'high', 'medium', 'low', 'info'. Default: 'medium'",
                        "enum": ["critical", "high", "medium", "low", "info"]
                    },
                    "threshold": {
                        "type": "integer",
                        "description": "Minimum waste percentage to report (default: 10)"
                    },
                    "include_info": {
                        "type": "boolean",
                        "description": "Include info-level suggestions (default: false)"
                    },
                    "include_system": {
                        "type": "boolean",
                        "description": "Include system namespaces like kube-system (default: false)"
                    },
                    "full": {
                        "type": "boolean",
                        "description": "Run FULL comprehensive analysis: optimize + kubelint security + helmlint. Use when user asks for 'full analysis' or 'check everything'."
                    },
                    "cluster": {
                        "type": "string",
                        "description": "Connect to a Kubernetes cluster for live analysis (kubeconfig context name). Requires cluster connectivity."
                    },
                    "prometheus": {
                        "type": "string",
                        "description": "Prometheus URL for historical metrics (from prometheus_connect tool, e.g., 'http://localhost:52431')"
                    },
                    "prometheus_auth_type": {
                        "type": "string",
                        "description": "Prometheus auth type (only for external URL, NOT for port-forward): 'none', 'basic', 'bearer'",
                        "enum": ["none", "basic", "bearer"]
                    },
                    "prometheus_username": {
                        "type": "string",
                        "description": "Username for Prometheus basic auth (only for external URL)"
                    },
                    "prometheus_password": {
                        "type": "string",
                        "description": "Password for Prometheus basic auth (only for external URL)"
                    },
                    "prometheus_token": {
                        "type": "string",
                        "description": "Bearer token for Prometheus auth (only for external URL)"
                    },
                    "period": {
                        "type": "string",
                        "description": "Analysis period for live metrics (e.g., '7d', '24h', '1h'). Default: '7d'"
                    },
                    "cloud_provider": {
                        "type": "string",
                        "description": "Cloud provider for cost estimation: 'aws', 'gcp', 'azure', 'onprem'",
                        "enum": ["aws", "gcp", "azure", "onprem"]
                    },
                    "region": {
                        "type": "string",
                        "description": "Cloud region for pricing (e.g., 'us-east-1', 'us-central1')"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let config = self.build_config(&args);

        // IMPORTANT: Treat empty content as None - fixes AI agents passing empty strings
        let mut result = if args.content.as_ref().is_some_and(|c| !c.trim().is_empty()) {
            // Analyze non-empty inline content
            analyze_content(args.content.as_ref().unwrap(), &config)
        } else {
            // Analyze path
            let path = args.path.as_deref().unwrap_or(".");
            let full_path = if std::path::Path::new(path).is_absolute() {
                PathBuf::from(path)
            } else {
                self.project_root.join(path)
            };

            if !full_path.exists() {
                return Err(K8sOptimizeError(format!(
                    "Path not found: {}",
                    full_path.display()
                )));
            }

            analyze(&full_path, &config)
        };

        // If prometheus URL provided, enhance recommendations with live data
        let prometheus_enhancement = if let Some(prometheus_url) = &args.prometheus {
            let auth = Self::build_prometheus_auth(&args);
            match PrometheusClient::with_auth(prometheus_url, auth) {
                Ok(client) => {
                    if client.is_available().await {
                        let period = args.period.as_deref().unwrap_or("7d");
                        Some(
                            self.enhance_with_prometheus(&mut result, &client, period)
                                .await,
                        )
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        } else {
            None
        };

        // If full mode, also run kubelint and helmlint
        let mut output = self.format_for_agent(&result, &args);

        if args.full {
            let path = args.path.as_deref().unwrap_or(".");
            let full_path = if std::path::Path::new(path).is_absolute() {
                PathBuf::from(path)
            } else {
                self.project_root.join(path)
            };

            // Run kubelint for security
            let kubelint_config =
                crate::analyzer::kubelint::KubelintConfig::default().with_all_builtin();
            let kubelint_result = crate::analyzer::kubelint::lint(&full_path, &kubelint_config);

            output["security_analysis"] = json!({
                "objects_analyzed": kubelint_result.summary.objects_analyzed,
                "checks_run": kubelint_result.summary.checks_run,
                "issues_found": kubelint_result.failures.len(),
                "findings": kubelint_result.failures.iter().take(20).map(|f| {
                    json!({
                        "code": f.code.to_string(),
                        "severity": format!("{:?}", f.severity).to_lowercase(),
                        "object": format!("{}/{}", f.object_kind, f.object_name),
                        "message": f.message,
                        "remediation": f.remediation,
                    })
                }).collect::<Vec<_>>(),
            });

            // Run helmlint on Helm charts if any
            let helm_charts = find_helm_charts(&full_path);
            if !helm_charts.is_empty() {
                let helmlint_config = crate::analyzer::helmlint::HelmlintConfig::default();
                let mut chart_results: Vec<serde_json::Value> = Vec::new();

                for chart_path in &helm_charts {
                    let chart_name = chart_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let helmlint_result =
                        crate::analyzer::helmlint::lint_chart(chart_path, &helmlint_config);

                    chart_results.push(json!({
                        "chart": chart_name,
                        "issues": helmlint_result.failures.iter().map(|f| {
                            json!({
                                "code": f.code.to_string(),
                                "severity": format!("{:?}", f.severity).to_lowercase(),
                                "message": f.message,
                            })
                        }).collect::<Vec<_>>(),
                    }));
                }

                output["helm_validation"] = json!({
                    "charts_analyzed": helm_charts.len(),
                    "results": chart_results,
                });
            }

            output["analysis_mode"] = json!("full");
        }

        // Add Prometheus enhancement data if available
        if let Some(enhancement) = prometheus_enhancement {
            output["prometheus_analysis"] = json!({
                "enabled": true,
                "url": args.prometheus,
                "period": args.period.clone().unwrap_or_else(|| "7d".to_string()),
                "workloads_enhanced": enhancement.enhanced_count,
                "workloads_no_data": enhancement.no_data_count,
                "mode": if enhancement.enhanced_count > 0 { "data-driven" } else { "static" },
                "historical_data": enhancement.prometheus_data,
                "note": if enhancement.enhanced_count > 0 {
                    format!(
                        "Recommendations for {} workloads are based on actual P99 usage from Prometheus. {} workloads had no historical data.",
                        enhancement.enhanced_count,
                        enhancement.no_data_count
                    )
                } else {
                    "No historical data found in Prometheus for the analyzed workloads. Recommendations are heuristic-based.".to_string()
                }
            });

            // Update summary mode
            if enhancement.enhanced_count > 0 {
                output["summary"]["mode"] = json!("prometheus");
            }
        }

        // Use smart compression with RAG retrieval pattern
        // This preserves all data while keeping context size manageable
        let config = CompressionConfig::default();
        Ok(compress_tool_output(&output, "k8s_optimize", &config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(K8sOptimizeTool::NAME, "k8s_optimize");
    }

    #[tokio::test]
    async fn test_analyze_content() {
        let tool = K8sOptimizeTool::new(PathBuf::from("."));

        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: test-app
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    spec:
      containers:
      - name: app
        image: myapp:v1
"#;

        let args = K8sOptimizeArgs {
            path: None,
            content: Some(yaml.to_string()),
            severity: None,
            threshold: None,
            include_info: false,
            include_system: true,
            full: false,
            cluster: None,
            prometheus: None,
            prometheus_auth_type: None,
            prometheus_username: None,
            prometheus_password: None,
            prometheus_token: None,
            period: None,
            cloud_provider: None,
            region: None,
        };

        let result = tool.call(args).await.unwrap();
        assert!(result.contains("summary"));
        assert!(result.contains("recommendations"));
        assert!(result.contains("rule_codes"));
    }

    #[tokio::test]
    async fn test_build_config() {
        let tool = K8sOptimizeTool::new(PathBuf::from("."));

        let args = K8sOptimizeArgs {
            path: None,
            content: None,
            severity: Some("high".to_string()),
            threshold: Some(20),
            include_info: true,
            include_system: true,
            full: false,
            cluster: None,
            prometheus: None,
            prometheus_auth_type: None,
            prometheus_username: None,
            prometheus_password: None,
            prometheus_token: None,
            period: None,
            cloud_provider: None,
            region: None,
        };

        let config = tool.build_config(&args);
        assert_eq!(config.waste_threshold_percent, 20);
        assert!(config.include_info);
        assert!(config.include_system);
    }

    #[tokio::test]
    async fn test_output_format() {
        let tool = K8sOptimizeTool::new(PathBuf::from("."));

        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: over-provisioned
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21
        resources:
          requests:
            cpu: 4000m
            memory: 8Gi
          limits:
            cpu: 8000m
            memory: 16Gi
"#;

        let args = K8sOptimizeArgs {
            path: None,
            content: Some(yaml.to_string()),
            severity: None,
            threshold: None,
            include_info: false,
            include_system: true,
            full: false,
            cluster: None,
            prometheus: None,
            prometheus_auth_type: None,
            prometheus_username: None,
            prometheus_password: None,
            prometheus_token: None,
            period: None,
            cloud_provider: Some("aws".to_string()),
            region: Some("us-east-1".to_string()),
        };

        let result = tool.call(args).await.unwrap();

        // Parse and verify structure
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert!(json.get("summary").is_some());
        assert!(json.get("recommendations").is_some());
        assert!(json.get("rule_codes").is_some());
        assert!(json.get("cost_estimation").is_some());
    }

    #[test]
    fn test_build_prometheus_auth_none() {
        let args = K8sOptimizeArgs {
            path: None,
            content: None,
            severity: None,
            threshold: None,
            include_info: false,
            include_system: false,
            full: false,
            cluster: None,
            prometheus: Some("http://localhost:9090".to_string()),
            prometheus_auth_type: None,
            prometheus_username: None,
            prometheus_password: None,
            prometheus_token: None,
            period: None,
            cloud_provider: None,
            region: None,
        };

        let auth = K8sOptimizeTool::build_prometheus_auth(&args);
        assert!(matches!(auth, PrometheusAuth::None));
    }

    #[test]
    fn test_build_prometheus_auth_basic() {
        let args = K8sOptimizeArgs {
            path: None,
            content: None,
            severity: None,
            threshold: None,
            include_info: false,
            include_system: false,
            full: false,
            cluster: None,
            prometheus: Some("https://prometheus.example.com".to_string()),
            prometheus_auth_type: Some("basic".to_string()),
            prometheus_username: Some("admin".to_string()),
            prometheus_password: Some("secret".to_string()),
            prometheus_token: None,
            period: None,
            cloud_provider: None,
            region: None,
        };

        let auth = K8sOptimizeTool::build_prometheus_auth(&args);
        match auth {
            PrometheusAuth::Basic { username, password } => {
                assert_eq!(username, "admin");
                assert_eq!(password, "secret");
            }
            _ => panic!("Expected Basic auth"),
        }
    }
}
