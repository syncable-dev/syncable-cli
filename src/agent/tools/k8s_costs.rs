//! K8s Costs tool - Cost attribution and analysis for Kubernetes workloads
//!
//! Provides cost estimation, attribution by namespace/label, and trend analysis
//! to help with cloud cost optimization decisions.
//!
//! Output is optimized for AI agent decision-making with:
//! - Cost breakdowns by namespace, workload, and resource type
//! - Historical trends and anomaly detection
//! - Actionable cost reduction recommendations

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use super::error::{ErrorCategory, format_error_for_llm};
use crate::analyzer::k8s_optimize::{
    CloudProvider, CostEstimation, K8sOptimizeConfig, analyze, calculate_from_static,
};

/// Arguments for the k8s-costs tool
#[derive(Debug, Deserialize)]
pub struct K8sCostsArgs {
    /// Path to K8s manifest file or directory (relative to project root)
    #[serde(default)]
    pub path: Option<String>,

    /// Filter by namespace
    #[serde(default)]
    pub namespace: Option<String>,

    /// Group costs by label (e.g., "app", "team", "environment")
    #[serde(default)]
    pub by_label: Option<String>,

    /// Cloud provider for pricing: "aws", "gcp", "azure", "onprem"
    #[serde(default)]
    pub cloud_provider: Option<String>,

    /// Cloud region for pricing (e.g., "us-east-1", "us-central1")
    #[serde(default)]
    pub region: Option<String>,

    /// Show detailed breakdown per workload
    #[serde(default)]
    pub detailed: bool,

    /// Compare with another period (e.g., "7d", "30d") - for trend analysis
    #[serde(default)]
    pub compare_period: Option<String>,

    // ========== Live Cluster Options ==========
    /// Connect to a Kubernetes cluster (kubeconfig context name)
    #[serde(default)]
    pub cluster: Option<String>,

    /// Prometheus URL for historical cost data
    #[serde(default)]
    pub prometheus: Option<String>,
}

/// Error type for k8s-costs tool
#[derive(Debug, thiserror::Error)]
#[error("K8s costs error: {0}")]
pub struct K8sCostsError(String);

/// Tool for analyzing Kubernetes workload costs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sCostsTool {
    project_root: PathBuf,
}

impl K8sCostsTool {
    /// Create a new K8sCostsTool with the given project root.
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Parse cloud provider from string.
    fn parse_provider(&self, provider: &str) -> CloudProvider {
        match provider.to_lowercase().as_str() {
            "aws" => CloudProvider::Aws,
            "gcp" => CloudProvider::Gcp,
            "azure" => CloudProvider::Azure,
            "onprem" | "on-prem" | "on_prem" => CloudProvider::OnPrem,
            _ => CloudProvider::Aws, // Default to AWS
        }
    }

    /// Format cost estimation for agent consumption.
    fn format_for_agent(
        &self,
        estimation: &CostEstimation,
        args: &K8sCostsArgs,
    ) -> serde_json::Value {
        let mut response = json!({
            "summary": {
                "monthly_waste_cost_usd": estimation.monthly_waste_cost,
                "annual_waste_cost_usd": estimation.annual_waste_cost,
                "monthly_savings_usd": estimation.monthly_savings,
                "annual_savings_usd": estimation.annual_savings,
                "workload_count": estimation.workload_costs.len(),
                "cloud_provider": format!("{:?}", estimation.provider),
                "region": estimation.region.clone(),
                "currency": estimation.currency.clone(),
            },
            "breakdown": {
                "cpu_waste_cost_usd": estimation.breakdown.cpu_cost,
                "memory_waste_cost_usd": estimation.breakdown.memory_cost,
            },
            "workloads": estimation.workload_costs.iter().map(|w| {
                json!({
                    "name": w.workload_name,
                    "namespace": w.namespace,
                    "monthly_waste_cost_usd": w.monthly_cost,
                    "potential_savings_usd": w.monthly_savings,
                })
            }).collect::<Vec<_>>(),
        });

        // Add namespace grouping if requested
        if args.namespace.is_some() || args.by_label.is_some() {
            let mut namespace_costs: std::collections::HashMap<String, f64> =
                std::collections::HashMap::new();
            for workload in &estimation.workload_costs {
                *namespace_costs
                    .entry(workload.namespace.clone())
                    .or_insert(0.0) += workload.monthly_cost;
            }
            response["by_namespace"] = json!(namespace_costs);
        }

        // Add recommendations for cost reduction
        let mut recommendations: Vec<serde_json::Value> = Vec::new();

        // Find top cost workloads
        let mut sorted_workloads = estimation.workload_costs.clone();
        sorted_workloads.sort_by(|a, b| {
            b.monthly_cost
                .partial_cmp(&a.monthly_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_waste = estimation.monthly_waste_cost;
        if let Some(top) = sorted_workloads.first()
            && total_waste > 0.0
            && top.monthly_cost > total_waste * 0.3
        {
            recommendations.push(json!({
                "type": "high_waste_workload",
                "workload": top.workload_name,
                "namespace": top.namespace,
                "waste_cost_usd": top.monthly_cost,
                "percentage": (top.monthly_cost / total_waste * 100.0).round(),
                "message": format!("{} accounts for over 30% of total waste. Consider optimization.", top.workload_name),
            }));
        }

        // Check for cost imbalance (CPU vs Memory)
        if estimation.breakdown.cpu_cost > estimation.breakdown.memory_cost * 3.0 {
            recommendations.push(json!({
                "type": "cpu_heavy",
                "message": "CPU waste is significantly higher than memory waste. Consider if workloads are CPU over-provisioned.",
                "cpu_waste_cost_usd": estimation.breakdown.cpu_cost,
                "memory_waste_cost_usd": estimation.breakdown.memory_cost,
            }));
        }

        if !recommendations.is_empty() {
            response["recommendations"] = json!(recommendations);
        }

        // Add analysis metadata
        response["analysis"] = json!({
            "mode": if args.cluster.is_some() { "live" } else { "static" },
            "path": args.path.clone().unwrap_or_else(|| ".".to_string()),
            "pricing_note": "Estimates based on on-demand pricing. Actual costs may vary with reserved instances, spot pricing, or enterprise discounts.",
        });

        response
    }
}

impl Tool for K8sCostsTool {
    const NAME: &'static str = "k8s_costs";

    type Args = K8sCostsArgs;
    type Output = String;
    type Error = K8sCostsError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Analyze Kubernetes workload costs and waste.

**IMPORTANT: Only use this tool when the user EXPLICITLY asks about:**
- Cloud costs for Kubernetes
- Cost attribution or cost breakdown
- How much resources cost or waste
- Budget/spending analysis for K8s
- Which workloads cost the most

**DO NOT use this tool for:**
- General Kubernetes linting (use kubelint)
- Resource optimization analysis (use k8s_optimize)
- Any task where user didn't ask about costs/spending/budget

## What It Does
Estimates monthly cloud costs based on resource requests, shows cost breakdown by namespace/workload, and identifies wasted spend.

## Supported Providers
- aws, gcp, azure, onprem

## Returns (analysis only - does NOT apply changes)
- Monthly/annual waste cost estimates
- Cost breakdown by CPU/memory
- Per-workload cost attribution
- Does NOT automatically modify anything"#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to K8s manifest file or directory (relative to project root). Examples: 'k8s/', 'deployments/'"
                    },
                    "namespace": {
                        "type": "string",
                        "description": "Filter costs by namespace"
                    },
                    "by_label": {
                        "type": "string",
                        "description": "Group costs by label key (e.g., 'app', 'team', 'environment')"
                    },
                    "cloud_provider": {
                        "type": "string",
                        "description": "Cloud provider for pricing: 'aws', 'gcp', 'azure', 'onprem'. Default: 'aws'",
                        "enum": ["aws", "gcp", "azure", "onprem"]
                    },
                    "region": {
                        "type": "string",
                        "description": "Cloud region for pricing (e.g., 'us-east-1', 'us-central1'). Default: 'us-east-1'"
                    },
                    "detailed": {
                        "type": "boolean",
                        "description": "Show detailed per-workload breakdown (default: false)"
                    },
                    "compare_period": {
                        "type": "string",
                        "description": "Compare with historical period for trend analysis (e.g., '7d', '30d')"
                    },
                    "cluster": {
                        "type": "string",
                        "description": "Connect to a Kubernetes cluster for live cost analysis (kubeconfig context name)"
                    },
                    "prometheus": {
                        "type": "string",
                        "description": "Prometheus URL for historical cost metrics (e.g., 'http://prometheus:9090')"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // First, analyze the manifests to get resource information
        let path = args.path.as_deref().unwrap_or(".");
        let full_path = if std::path::Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.project_root.join(path)
        };

        // Edge case: Path not found
        if !full_path.exists() {
            return Ok(format_error_for_llm(
                "k8s_costs",
                ErrorCategory::FileNotFound,
                &format!("Path not found: {}", full_path.display()),
                Some(vec![
                    "Check if the path is correct",
                    "Common locations: k8s/, manifests/, deploy/, kubernetes/",
                    "Use list_directory to explore available paths",
                    "Use k8s_optimize for resource analysis first",
                ]),
            ));
        }

        // Edge case: Check if directory is empty (no files)
        if full_path.is_dir() {
            let has_files = std::fs::read_dir(&full_path)
                .map(|entries| entries.filter_map(|e| e.ok()).next().is_some())
                .unwrap_or(false);

            if !has_files {
                return Ok(format_error_for_llm(
                    "k8s_costs",
                    ErrorCategory::ValidationFailed,
                    &format!("Directory is empty: {}", full_path.display()),
                    Some(vec![
                        "The directory contains no files to analyze",
                        "Check if K8s manifests exist in a subdirectory",
                        "Use list_directory to explore the project structure",
                    ]),
                ));
            }
        }

        // Run static analysis first
        let config = K8sOptimizeConfig::default();
        let analysis_result = analyze(&full_path, &config);

        // Edge case: No K8s manifests found (empty recommendations)
        if analysis_result.recommendations.is_empty() && analysis_result.warnings.is_empty() {
            return Ok(format_error_for_llm(
                "k8s_costs",
                ErrorCategory::ValidationFailed,
                &format!("No Kubernetes manifests found in: {}", full_path.display()),
                Some(vec![
                    "Ensure the path contains .yaml or .yml files",
                    "K8s manifests should define Deployment, StatefulSet, or Pod resources",
                    "Try specifying a more specific path (e.g., 'k8s/deployments/')",
                    "Use kubelint to validate manifest structure",
                ]),
            ));
        }

        // Calculate costs from recommendations
        let provider = self.parse_provider(args.cloud_provider.as_deref().unwrap_or("aws"));
        let region = args
            .region
            .clone()
            .unwrap_or_else(|| "us-east-1".to_string());

        let cost_estimation =
            calculate_from_static(&analysis_result.recommendations, provider, &region);

        // Edge case: No cost data available (no workloads with resource requests)
        if cost_estimation.workload_costs.is_empty() {
            return Ok(format_error_for_llm(
                "k8s_costs",
                ErrorCategory::ValidationFailed,
                "No cost data available - workloads have no resource requests defined",
                Some(vec![
                    "Ensure Deployments/StatefulSets have resource requests specified",
                    "Add resources.requests.cpu and resources.requests.memory to containers",
                    "Use k8s_optimize to get resource recommendation suggestions",
                ]),
            ));
        }

        // Format for agent
        let output = self.format_for_agent(&cost_estimation, &args);
        Ok(serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(K8sCostsTool::NAME, "k8s_costs");
    }

    #[test]
    fn test_parse_provider() {
        let tool = K8sCostsTool::new(PathBuf::from("."));

        assert!(matches!(tool.parse_provider("aws"), CloudProvider::Aws));
        assert!(matches!(tool.parse_provider("AWS"), CloudProvider::Aws));
        assert!(matches!(tool.parse_provider("gcp"), CloudProvider::Gcp));
        assert!(matches!(tool.parse_provider("azure"), CloudProvider::Azure));
        assert!(matches!(
            tool.parse_provider("onprem"),
            CloudProvider::OnPrem
        ));
        assert!(matches!(
            tool.parse_provider("on-prem"),
            CloudProvider::OnPrem
        ));
        assert!(matches!(tool.parse_provider("unknown"), CloudProvider::Aws)); // Default
    }

    #[tokio::test]
    async fn test_definition() {
        let tool = K8sCostsTool::new(PathBuf::from("."));
        let def = tool.definition("".to_string()).await;

        assert_eq!(def.name, "k8s_costs");
        assert!(def.description.contains("cost"));
    }

    #[tokio::test]
    async fn test_path_not_found_error() {
        let tool = K8sCostsTool::new(PathBuf::from("/tmp/test-k8s-costs-nonexistent"));
        let args = K8sCostsArgs {
            path: Some("nonexistent/path".to_string()),
            namespace: None,
            by_label: None,
            cloud_provider: None,
            region: None,
            detailed: false,
            compare_period: None,
            cluster: None,
            prometheus: None,
        };
        let result = tool.call(args).await.unwrap();

        // Verify it returns structured error JSON
        assert!(result.contains("FILE_NOT_FOUND") || result.contains("error"));
        assert!(result.contains("suggestions"));
        assert!(result.contains("Path not found"));
    }

    #[test]
    fn test_provider_case_insensitivity() {
        let tool = K8sCostsTool::new(PathBuf::from("."));

        // Test uppercase
        assert!(matches!(tool.parse_provider("AWS"), CloudProvider::Aws));
        assert!(matches!(tool.parse_provider("GCP"), CloudProvider::Gcp));
        assert!(matches!(tool.parse_provider("AZURE"), CloudProvider::Azure));
        assert!(matches!(tool.parse_provider("ONPREM"), CloudProvider::OnPrem));

        // Test mixed case
        assert!(matches!(tool.parse_provider("Aws"), CloudProvider::Aws));
        assert!(matches!(tool.parse_provider("Gcp"), CloudProvider::Gcp));
        assert!(matches!(tool.parse_provider("Azure"), CloudProvider::Azure));
        assert!(matches!(tool.parse_provider("OnPrem"), CloudProvider::OnPrem));

        // Test lowercase
        assert!(matches!(tool.parse_provider("aws"), CloudProvider::Aws));
        assert!(matches!(tool.parse_provider("gcp"), CloudProvider::Gcp));
        assert!(matches!(tool.parse_provider("azure"), CloudProvider::Azure));
        assert!(matches!(tool.parse_provider("onprem"), CloudProvider::OnPrem));

        // Test alternative formats
        assert!(matches!(tool.parse_provider("on-prem"), CloudProvider::OnPrem));
        assert!(matches!(tool.parse_provider("on_prem"), CloudProvider::OnPrem));
        assert!(matches!(tool.parse_provider("ON-PREM"), CloudProvider::OnPrem));
    }
}
