//! Kubelint tool - Native Kubernetes manifest linting using Rig's Tool trait
//!
//! Lints **rendered Kubernetes manifests** for security and best practices.
//! Works on raw YAML files, Helm charts (renders them), and Kustomize directories.
//!
//! **Use this for:** Security issues, K8s resource best practices, RBAC, probes, resource limits.
//! **Use HelmlintTool for:** Helm chart structure, template syntax, Chart.yaml validation.
//!
//! Output is optimized for AI agent decision-making with:
//! - Categorized issues (security, best-practice, validation, rbac)
//! - Priority rankings (critical, high, medium, low)
//! - Actionable remediation recommendations

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use crate::analyzer::kubelint::{
    KubelintConfig, LintResult, Severity, lint, lint_content, lint_file,
};

/// Arguments for the kubelint tool
#[derive(Debug, Deserialize)]
pub struct KubelintArgs {
    /// Path to K8s manifest file or directory (relative to project root)
    /// Can be: YAML file, directory with YAMLs, Helm chart dir, Kustomize dir
    #[serde(default)]
    pub path: Option<String>,

    /// Inline YAML content to lint (alternative to path)
    #[serde(default)]
    pub content: Option<String>,

    /// Checks to include (if empty, uses defaults)
    #[serde(default)]
    pub include: Vec<String>,

    /// Checks to exclude
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Minimum severity threshold: "error", "warning", "info"
    #[serde(default)]
    pub threshold: Option<String>,
}

/// Error type for kubelint tool
#[derive(Debug, thiserror::Error)]
#[error("Kubelint error: {0}")]
pub struct KubelintError(String);

/// Tool to lint Kubernetes manifests natively
///
/// **When to use:**
/// - Checking security issues (privileged containers, missing probes, etc.)
/// - Validating K8s resource best practices
/// - RBAC configuration validation
/// - Resource limits and requests checking
///
/// **When to use HelmlintTool instead:**
/// - Helm chart structure validation (Chart.yaml, values.yaml)
/// - Go template syntax checking
/// - Helm-specific best practices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubelintTool {
    project_path: PathBuf,
}

impl KubelintTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    fn parse_threshold(threshold: &str) -> Severity {
        match threshold.to_lowercase().as_str() {
            "error" => Severity::Error,
            "warning" => Severity::Warning,
            "info" => Severity::Info,
            _ => Severity::Warning,
        }
    }

    /// Get category for a check code
    fn get_check_category(code: &str) -> &'static str {
        match code {
            // Security checks
            "privileged-container"
            | "privilege-escalation"
            | "run-as-non-root"
            | "read-only-root-fs"
            | "drop-net-raw-capability"
            | "hostnetwork"
            | "hostpid"
            | "hostipc"
            | "host-mounts"
            | "writable-host-mount"
            | "docker-sock"
            | "unsafe-proc-mount"
            | "scc-deny-privileged-container" => "security",

            // Best practice checks
            "latest-tag"
            | "no-liveness-probe"
            | "no-readiness-probe"
            | "unset-cpu-requirements"
            | "unset-memory-requirements"
            | "minimum-replicas"
            | "no-anti-affinity"
            | "no-rolling-update-strategy"
            | "default-service-account"
            | "deprecated-service-account"
            | "env-var-secret"
            | "read-secret-from-env-var"
            | "priority-class-name"
            | "no-node-affinity"
            | "restart-policy"
            | "sysctls"
            | "dnsconfig-options" => "best-practice",

            // RBAC checks
            "access-to-secrets"
            | "access-to-create-pods"
            | "cluster-admin-role-binding"
            | "wildcard-in-rules" => "rbac",

            // Validation checks
            "dangling-service"
            | "dangling-ingress"
            | "dangling-horizontalpodautoscaler"
            | "dangling-networkpolicy"
            | "mismatching-selector"
            | "duplicate-env-var"
            | "invalid-target-ports"
            | "non-existent-service-account"
            | "non-isolated-pod"
            | "use-namespace"
            | "env-var-value-from"
            | "job-ttl-seconds-after-finished" => "validation",

            // Port checks
            "ssh-port" | "privileged-ports" | "liveness-port" | "readiness-port"
            | "startup-port" => "ports",

            // PDB checks
            "pdb-max-unavailable" | "pdb-min-available" | "pdb-unhealthy-pod-eviction-policy" => {
                "disruption-budget"
            }

            // HPA checks
            "hpa-minimum-replicas" => "autoscaling",

            // Deprecated API checks
            "no-extensions-v1beta" => "deprecated-api",

            // Service checks
            "service-type" => "service",

            _ => "other",
        }
    }

    /// Get priority based on severity and check code
    fn get_priority(severity: Severity, code: &str) -> &'static str {
        let category = Self::get_check_category(code);
        match (severity, category) {
            (Severity::Error, "security") => "critical",
            (Severity::Error, "rbac") => "critical",
            (Severity::Error, _) => "high",
            (Severity::Warning, "security") => "high",
            (Severity::Warning, "rbac") => "high",
            (Severity::Warning, "validation") => "medium",
            (Severity::Warning, "best-practice") => "medium",
            (Severity::Warning, _) => "medium",
            (Severity::Info, _) => "low",
        }
    }

    /// Format result optimized for agent decision-making
    fn format_result(result: &LintResult, source: &str) -> String {
        // Categorize and enrich failures
        let enriched_failures: Vec<serde_json::Value> = result
            .failures
            .iter()
            .map(|f| {
                let code = f.code.as_str();
                let category = Self::get_check_category(code);
                let priority = Self::get_priority(f.severity, code);

                json!({
                    "check": code,
                    "severity": format!("{:?}", f.severity).to_lowercase(),
                    "priority": priority,
                    "category": category,
                    "message": f.message,
                    "object": {
                        "name": f.object_name,
                        "kind": f.object_kind,
                        "namespace": f.object_namespace,
                    },
                    "file": f.file_path.display().to_string(),
                    "line": f.line,
                    "remediation": f.remediation,
                })
            })
            .collect();

        // Group by priority
        let critical: Vec<_> = enriched_failures
            .iter()
            .filter(|f| f["priority"] == "critical")
            .cloned()
            .collect();
        let high: Vec<_> = enriched_failures
            .iter()
            .filter(|f| f["priority"] == "high")
            .cloned()
            .collect();
        let medium: Vec<_> = enriched_failures
            .iter()
            .filter(|f| f["priority"] == "medium")
            .cloned()
            .collect();
        let low: Vec<_> = enriched_failures
            .iter()
            .filter(|f| f["priority"] == "low")
            .cloned()
            .collect();

        // Group by category
        let mut by_category: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for f in &result.failures {
            let cat = Self::get_check_category(f.code.as_str());
            *by_category.entry(cat).or_default() += 1;
        }

        // Build decision context
        let decision_context = if critical.is_empty() && high.is_empty() {
            if medium.is_empty() && low.is_empty() {
                "Kubernetes manifests follow security best practices. No issues found."
            } else if medium.is_empty() {
                "Minor improvements possible. Low priority issues only."
            } else {
                "Good baseline. Medium priority improvements recommended."
            }
        } else if !critical.is_empty() {
            "CRITICAL security issues found. Fix before deployment to production."
        } else {
            "High priority issues found. Review security and best practice violations."
        };

        // Build agent-optimized output
        let mut output = json!({
            "source": source,
            "success": result.summary.passed,
            "decision_context": decision_context,
            "tool_guidance": "Use kubelint for K8s manifest security/best practices. Use helmlint for Helm chart structure/template syntax.",
            "summary": {
                "total_issues": result.failures.len(),
                "objects_analyzed": result.summary.objects_analyzed,
                "checks_run": result.summary.checks_run,
                "by_priority": {
                    "critical": critical.len(),
                    "high": high.len(),
                    "medium": medium.len(),
                    "low": low.len(),
                },
                "by_category": by_category,
            },
            "action_plan": {
                "critical": critical,
                "high": high,
                "medium": medium,
                "low": low,
            },
        });

        // Add quick fixes summary
        if !enriched_failures.is_empty() {
            let quick_fixes: Vec<String> = enriched_failures
                .iter()
                .filter(|f| f["priority"] == "critical" || f["priority"] == "high")
                .take(5)
                .map(|f| {
                    let remediation = f["remediation"]
                        .as_str()
                        .unwrap_or("Review the check documentation.");
                    format!(
                        "{}/{}: {} - {}",
                        f["object"]["kind"].as_str().unwrap_or(""),
                        f["object"]["name"].as_str().unwrap_or(""),
                        f["check"].as_str().unwrap_or(""),
                        remediation
                    )
                })
                .collect();

            if !quick_fixes.is_empty() {
                output["quick_fixes"] = json!(quick_fixes);
            }
        }

        if !result.parse_errors.is_empty() {
            output["parse_errors"] = json!(result.parse_errors);
        }

        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    }
}

impl Tool for KubelintTool {
    const NAME: &'static str = "kubelint";

    type Error = KubelintError;
    type Args = KubelintArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Lint Kubernetes manifests for SECURITY and BEST PRACTICES. \
                Works on raw YAML files, Helm charts (renders them first), and Kustomize directories. \
                \n\n**IMPORTANT:** Always specify the `path` parameter to lint specific files or directories. \
                \n\n**Use kubelint for:** Security issues (privileged containers, missing probes), \
                resource best practices (limits, RBAC), manifest validation. \
                \n**Use helmlint for:** Helm chart structure, template syntax, Chart.yaml/values.yaml validation. \
                \n\nReturns AI-optimized JSON with issues categorized by priority (critical/high/medium/low) \
                and type (security/rbac/best-practice/validation). Each issue includes remediation steps."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to K8s manifest(s) relative to project root. Can be: \
                            single YAML file, directory with YAMLs, Helm chart directory, or Kustomize directory."
                    },
                    "content": {
                        "type": "string",
                        "description": "Inline YAML content to lint. Use this to validate generated manifests before writing."
                    },
                    "include": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Specific checks to run (e.g., ['privileged-container', 'latest-tag']). If empty, runs all default checks."
                    },
                    "exclude": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Checks to skip (e.g., ['no-liveness-probe', 'minimum-replicas'])"
                    },
                    "threshold": {
                        "type": "string",
                        "enum": ["error", "warning", "info"],
                        "description": "Minimum severity to report. Default is 'warning'."
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Build configuration
        let mut config = KubelintConfig::default().with_all_builtin();

        // Apply includes
        for check in &args.include {
            config = config.include(check.as_str());
        }

        // Apply excludes
        for check in &args.exclude {
            config = config.exclude(check.as_str());
        }

        // Apply threshold
        if let Some(threshold) = &args.threshold {
            config = config.with_threshold(Self::parse_threshold(threshold));
        }

        // Determine source and lint
        let (result, source) = if let Some(content) = &args.content {
            // Lint inline content
            (lint_content(content, &config), "<inline>".to_string())
        } else if let Some(path) = &args.path {
            // Lint file or directory
            let full_path = self.project_path.join(path);

            if !full_path.exists() {
                return Err(KubelintError(format!(
                    "Path '{}' does not exist.",
                    full_path.display()
                )));
            }

            if full_path.is_file() {
                (lint_file(&full_path, &config), path.clone())
            } else {
                (lint(&full_path, &config), path.clone())
            }
        } else {
            // Look for common K8s manifest locations
            let candidates = [
                "kubernetes",
                "k8s",
                "manifests",
                "deploy",
                "deployment",
                "helm",
                "charts",
                "test-lint",     // For testing
                "test-lint/k8s", // For testing
                ".",
            ];

            let mut found = None;
            for candidate in &candidates {
                let candidate_path = self.project_path.join(candidate);
                if candidate_path.exists() {
                    // Check if it has YAML files or is a Helm/Kustomize directory
                    if candidate_path.join("Chart.yaml").exists()
                        || candidate_path.join("kustomization.yaml").exists()
                        || candidate_path.join("kustomization.yml").exists()
                    {
                        found = Some((candidate_path, candidate.to_string()));
                        break;
                    }
                    // Check for YAML files
                    if let Ok(entries) = std::fs::read_dir(&candidate_path) {
                        let has_yaml = entries.filter_map(|e| e.ok()).any(|e| {
                            e.path()
                                .extension()
                                .map(|ext| ext == "yaml" || ext == "yml")
                                .unwrap_or(false)
                        });
                        if has_yaml {
                            found = Some((candidate_path, candidate.to_string()));
                            break;
                        }
                    }
                }
            }

            if let Some((path, name)) = found {
                (lint(&path, &config), name)
            } else {
                return Err(KubelintError(
                    "No path specified and no K8s manifests found. \
                    Specify a path with 'path' parameter or provide 'content' to lint."
                        .to_string(),
                ));
            }
        };

        // Check for parse errors
        if !result.parse_errors.is_empty() {
            log::warn!("K8s manifest parse errors: {:?}", result.parse_errors);
        }

        Ok(Self::format_result(&result, &source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_kubelint_inline_content() {
        let temp_dir = TempDir::new().unwrap();
        let tool = KubelintTool::new(temp_dir.path().to_path_buf());

        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: insecure-deploy
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:latest
        securityContext:
          privileged: true
"#;

        let args = KubelintArgs {
            path: None,
            content: Some(yaml.to_string()),
            include: vec!["privileged-container".to_string(), "latest-tag".to_string()],
            exclude: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Should find issues
        assert!(parsed["summary"]["total_issues"].as_u64().unwrap_or(0) > 0);
        assert!(parsed["decision_context"].is_string());
        assert!(parsed["tool_guidance"].is_string());
    }

    #[tokio::test]
    async fn test_kubelint_secure_deployment() {
        let temp_dir = TempDir::new().unwrap();
        let tool = KubelintTool::new(temp_dir.path().to_path_buf());

        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: secure-deploy
spec:
  replicas: 3
  selector:
    matchLabels:
      app: test
  template:
    spec:
      serviceAccountName: my-service-account
      securityContext:
        runAsNonRoot: true
      containers:
      - name: nginx
        image: nginx:1.25.0
        securityContext:
          privileged: false
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop:
            - ALL
"#;

        let args = KubelintArgs {
            path: None,
            content: Some(yaml.to_string()),
            include: vec!["privileged-container".to_string(), "latest-tag".to_string()],
            exclude: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Should pass for privileged and latest-tag checks
        let critical = parsed["summary"]["by_priority"]["critical"]
            .as_u64()
            .unwrap_or(99);
        let high = parsed["summary"]["by_priority"]["high"]
            .as_u64()
            .unwrap_or(99);
        assert_eq!(critical, 0);
        assert_eq!(high, 0);
    }

    #[tokio::test]
    async fn test_kubelint_file() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("deployment.yaml");

        fs::write(
            &manifest_path,
            r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: test
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.25.0
"#,
        )
        .unwrap();

        let tool = KubelintTool::new(temp_dir.path().to_path_buf());
        let args = KubelintArgs {
            path: Some("deployment.yaml".to_string()),
            content: None,
            include: vec![],
            exclude: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert!(
            parsed["source"]
                .as_str()
                .unwrap()
                .contains("deployment.yaml")
        );
        assert!(parsed["summary"]["objects_analyzed"].as_u64().unwrap_or(0) >= 1);
    }

    #[tokio::test]
    async fn test_kubelint_output_format() {
        let temp_dir = TempDir::new().unwrap();
        let tool = KubelintTool::new(temp_dir.path().to_path_buf());

        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: insecure-deploy
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:latest
        securityContext:
          privileged: true
"#;

        let args = KubelintArgs {
            path: None,
            content: Some(yaml.to_string()),
            include: vec![], // Use all defaults + builtin
            exclude: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        println!("\n=== KUBELINT OUTPUT ===\n{}\n", result);

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Verify structure
        assert!(
            parsed["summary"]["total_issues"].as_u64().unwrap() > 0,
            "Expected issues but got none. Output: {}",
            result
        );
        assert!(
            !parsed["action_plan"]["critical"]
                .as_array()
                .unwrap()
                .is_empty()
                || !parsed["action_plan"]["high"].as_array().unwrap().is_empty(),
            "Expected critical or high priority issues"
        );
    }

    #[tokio::test]
    async fn test_kubelint_excludes() {
        let temp_dir = TempDir::new().unwrap();
        let tool = KubelintTool::new(temp_dir.path().to_path_buf());

        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: test
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:latest
        securityContext:
          privileged: true
"#;

        let args = KubelintArgs {
            path: None,
            content: Some(yaml.to_string()),
            include: vec![],
            exclude: vec!["privileged-container".to_string(), "latest-tag".to_string()],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Excluded checks should not appear
        let all_issues: Vec<_> = ["critical", "high", "medium", "low"]
            .iter()
            .flat_map(|p| {
                parsed["action_plan"][p]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
            })
            .collect();

        assert!(
            !all_issues
                .iter()
                .any(|i| i["check"] == "privileged-container")
        );
        assert!(!all_issues.iter().any(|i| i["check"] == "latest-tag"));
    }
}
