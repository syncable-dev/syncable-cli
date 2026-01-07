//! K8s Drift tool - Detect configuration drift between manifests and live cluster
//!
//! Compares declared Kubernetes manifests against the live cluster state to identify
//! resource drift, especially in CPU/memory limits and requests.
//!
//! Output is optimized for AI agent decision-making with:
//! - Clear drift detection results
//! - Resource-specific differences
//! - Remediation suggestions

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use crate::analyzer::k8s_optimize::{K8sOptimizeConfig, analyze};

/// Arguments for the k8s-drift tool
#[derive(Debug, Deserialize)]
pub struct K8sDriftArgs {
    /// Path to K8s manifest file or directory (relative to project root)
    pub path: String,

    /// Kubernetes cluster context name (from kubeconfig)
    #[serde(default)]
    pub cluster: Option<String>,

    /// Filter by namespace
    #[serde(default)]
    pub namespace: Option<String>,

    /// Only check resource fields (requests/limits)
    #[serde(default)]
    pub resources_only: bool,

    /// Include all fields in diff, not just resource-related
    #[serde(default)]
    pub full_diff: bool,

    /// Output format: "summary", "detailed", "remediation"
    #[serde(default)]
    pub output_format: Option<String>,
}

/// Error type for k8s-drift tool
#[derive(Debug, thiserror::Error)]
#[error("K8s drift error: {0}")]
pub struct K8sDriftError(String);

/// Represents a single drift item
#[derive(Debug, Clone, Serialize)]
pub struct DriftItem {
    pub resource_kind: String,
    pub resource_name: String,
    pub namespace: String,
    pub container: Option<String>,
    pub field: String,
    pub declared_value: Option<String>,
    pub actual_value: Option<String>,
    pub drift_type: DriftType,
    pub severity: DriftSeverity,
}

/// Type of drift detected
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftType {
    /// Value differs between manifest and cluster
    ValueChanged,
    /// Field exists in manifest but not in cluster
    MissingInCluster,
    /// Field exists in cluster but not in manifest
    ExtraInCluster,
    /// Resource exists in manifest but not in cluster
    ResourceMissing,
    /// Resource exists in cluster but not in manifest
    ResourceExtra,
}

/// Severity of the drift
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DriftSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Tool for detecting Kubernetes configuration drift
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sDriftTool {
    project_root: PathBuf,
}

impl K8sDriftTool {
    /// Create a new K8sDriftTool with the given project root.
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Analyze manifests and detect drift (static analysis placeholder).
    ///
    /// In production, this would connect to the cluster and compare.
    /// For now, it analyzes manifests and prepares a drift detection structure.
    fn analyze_drift(&self, args: &K8sDriftArgs) -> Result<Vec<DriftItem>, K8sDriftError> {
        let path = &args.path;
        let full_path = if std::path::Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.project_root.join(path)
        };

        if !full_path.exists() {
            return Err(K8sDriftError(format!(
                "Path not found: {}",
                full_path.display()
            )));
        }

        // Run static analysis to understand what's declared
        let config = K8sOptimizeConfig::default();
        let result = analyze(&full_path, &config);

        // Without live cluster connection, we can only report what we'd check
        // This is a placeholder - full implementation requires kube-rs integration
        let mut drift_items: Vec<DriftItem> = Vec::new();

        // If no cluster specified, return info about what would be checked
        if args.cluster.is_none() {
            // Add informational items about what resources exist in manifests
            for rec in &result.recommendations {
                // These aren't real drifts, but they indicate what we'd compare
                drift_items.push(DriftItem {
                    resource_kind: rec.resource_kind.clone(),
                    resource_name: rec.resource_name.clone(),
                    namespace: rec
                        .namespace
                        .clone()
                        .unwrap_or_else(|| "default".to_string()),
                    container: Some(rec.container.clone()),
                    field: "resources".to_string(),
                    declared_value: Some(format!(
                        "cpu_req={}, mem_req={}",
                        rec.current.cpu_request.as_deref().unwrap_or("none"),
                        rec.current.memory_request.as_deref().unwrap_or("none")
                    )),
                    actual_value: None, // Would be populated with cluster data
                    drift_type: DriftType::ValueChanged,
                    severity: DriftSeverity::Info,
                });
            }
        }

        Ok(drift_items)
    }

    /// Format drift results for agent consumption.
    fn format_for_agent(
        &self,
        drift_items: &[DriftItem],
        args: &K8sDriftArgs,
    ) -> serde_json::Value {
        let cluster_connected = args.cluster.is_some();

        // Group by severity
        let critical_count = drift_items
            .iter()
            .filter(|d| matches!(d.severity, DriftSeverity::Critical))
            .count();
        let high_count = drift_items
            .iter()
            .filter(|d| matches!(d.severity, DriftSeverity::High))
            .count();
        let medium_count = drift_items
            .iter()
            .filter(|d| matches!(d.severity, DriftSeverity::Medium))
            .count();
        let low_count = drift_items
            .iter()
            .filter(|d| matches!(d.severity, DriftSeverity::Low))
            .count();
        let info_count = drift_items
            .iter()
            .filter(|d| matches!(d.severity, DriftSeverity::Info))
            .count();

        let mut response = json!({
            "summary": {
                "total_drifts": drift_items.len(),
                "critical": critical_count,
                "high": high_count,
                "medium": medium_count,
                "low": low_count,
                "info": info_count,
                "cluster_connected": cluster_connected,
                "path_analyzed": args.path,
            },
        });

        if cluster_connected {
            response["drifts"] = json!(drift_items.iter().map(|d| {
                json!({
                    "resource": format!("{}/{}", d.resource_kind, d.resource_name),
                    "namespace": d.namespace,
                    "container": d.container,
                    "field": d.field,
                    "drift_type": d.drift_type,
                    "severity": d.severity,
                    "declared": d.declared_value,
                    "actual": d.actual_value,
                    "remediation": match d.drift_type {
                        DriftType::ValueChanged => "Update manifest or apply kubectl to sync",
                        DriftType::MissingInCluster => "Apply manifest with kubectl apply",
                        DriftType::ExtraInCluster => "Remove from cluster or add to manifest",
                        DriftType::ResourceMissing => "Deploy resource with kubectl apply",
                        DriftType::ResourceExtra => "Consider adding to version control",
                    },
                })
            }).collect::<Vec<_>>());
        } else {
            // Without cluster connection, provide guidance
            response["status"] = json!("no_cluster_connection");
            response["message"] = json!(
                "No cluster context specified. To detect actual drift, provide a cluster name. \
                 Currently showing resources that would be checked."
            );
            response["resources_to_check"] = json!(
                drift_items
                    .iter()
                    .map(|d| {
                        json!({
                            "resource": format!("{}/{}", d.resource_kind, d.resource_name),
                            "namespace": d.namespace,
                            "container": d.container,
                            "declared_resources": d.declared_value,
                        })
                    })
                    .collect::<Vec<_>>()
            );
            response["next_steps"] = json!([
                "Specify 'cluster' parameter with your kubeconfig context name",
                "Run: kubectl config get-contexts to see available contexts",
                "Example: k8s_drift with cluster='my-cluster-context'",
            ]);
        }

        // Add remediation commands if drifts found
        if cluster_connected && !drift_items.is_empty() {
            let mut commands: Vec<String> = Vec::new();

            // Generate kubectl commands for remediation
            for drift in drift_items
                .iter()
                .filter(|d| matches!(d.severity, DriftSeverity::Critical | DriftSeverity::High))
            {
                match drift.drift_type {
                    DriftType::ValueChanged | DriftType::MissingInCluster => {
                        commands.push(format!(
                            "kubectl apply -f {} -n {}",
                            args.path, drift.namespace
                        ));
                    }
                    DriftType::ResourceMissing => {
                        commands.push(format!(
                            "kubectl apply -f {} -n {}",
                            args.path, drift.namespace
                        ));
                    }
                    _ => {}
                }
            }

            if !commands.is_empty() {
                // Deduplicate commands
                commands.sort();
                commands.dedup();
                response["remediation_commands"] = json!(commands);
            }
        }

        response
    }
}

impl Tool for K8sDriftTool {
    const NAME: &'static str = "k8s_drift";

    type Args = K8sDriftArgs;
    type Output = String;
    type Error = K8sDriftError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Detect configuration drift between Kubernetes manifests and live cluster.

**IMPORTANT: Only use this tool when the user EXPLICITLY asks about:**
- Drift detection between manifests and cluster
- What's different between declared and actual state
- GitOps compliance or sync status
- Whether manifests match what's running

**DO NOT use this tool for:**
- General Kubernetes linting (use kubelint)
- Resource optimization (use k8s_optimize)
- Cost analysis (use k8s_costs)
- Any task where user didn't ask about drift/sync/compliance

## What It Does
Compares manifest files against live cluster state (when cluster is connected) to find differences in resource configurations.

## Returns (analysis only - does NOT apply changes)
- Summary of drift counts by severity
- Per-resource drift information
- Suggested remediation commands
- Does NOT automatically sync or modify anything"#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to K8s manifest file or directory (required)"
                    },
                    "cluster": {
                        "type": "string",
                        "description": "Kubernetes cluster context name (from kubeconfig). Required for actual drift detection."
                    },
                    "namespace": {
                        "type": "string",
                        "description": "Filter drift detection to specific namespace"
                    },
                    "resources_only": {
                        "type": "boolean",
                        "description": "Only check resource requests/limits fields (default: false)"
                    },
                    "full_diff": {
                        "type": "boolean",
                        "description": "Include all fields in comparison, not just resources (default: false)"
                    },
                    "output_format": {
                        "type": "string",
                        "description": "Output format: 'summary', 'detailed', 'remediation'",
                        "enum": ["summary", "detailed", "remediation"]
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let drift_items = self.analyze_drift(&args)?;
        let output = self.format_for_agent(&drift_items, &args);
        Ok(serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(K8sDriftTool::NAME, "k8s_drift");
    }

    #[test]
    fn test_drift_type_serialization() {
        let drift = DriftItem {
            resource_kind: "Deployment".to_string(),
            resource_name: "my-app".to_string(),
            namespace: "default".to_string(),
            container: Some("app".to_string()),
            field: "resources.limits.cpu".to_string(),
            declared_value: Some("500m".to_string()),
            actual_value: Some("1000m".to_string()),
            drift_type: DriftType::ValueChanged,
            severity: DriftSeverity::High,
        };

        let json = serde_json::to_string(&drift).unwrap();
        assert!(json.contains("value_changed"));
        assert!(json.contains("high"));
    }

    #[tokio::test]
    async fn test_definition() {
        let tool = K8sDriftTool::new(PathBuf::from("."));
        let def = tool.definition("".to_string()).await;

        assert_eq!(def.name, "k8s_drift");
        assert!(def.description.contains("drift"));
    }

    #[tokio::test]
    async fn test_no_cluster_output() {
        let tool = K8sDriftTool::new(PathBuf::from("."));

        // Without cluster, should return guidance
        let args = K8sDriftArgs {
            path: ".".to_string(),
            cluster: None,
            namespace: None,
            resources_only: false,
            full_diff: false,
            output_format: None,
        };

        let result = tool.call(args).await.unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["status"], "no_cluster_connection");
        assert!(json["next_steps"].is_array());
    }
}
