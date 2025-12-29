//! Helmlint tool - Native Helm chart linting using Rig's Tool trait
//!
//! Lints Helm **chart structure and templates** (before rendering).
//! Validates Chart.yaml, values.yaml, Go template syntax, and Helm-specific best practices.
//!
//! **Use this for:** Helm chart development, template syntax issues, chart metadata validation.
//! **Use KubelintTool for:** Security/best practice issues in rendered K8s manifests.
//!
//! Output is optimized for AI agent decision-making with:
//! - Categorized issues (structure, values, template, security, best-practice)
//! - Priority rankings (critical, high, medium, low)
//! - Actionable fix recommendations
//! - Rule documentation

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use crate::analyzer::helmlint::{lint_chart, HelmlintConfig, LintResult, Severity};
use crate::analyzer::helmlint::types::RuleCategory;

/// Arguments for the helmlint tool
#[derive(Debug, Deserialize)]
pub struct HelmlintArgs {
    /// Path to Helm chart directory (relative to project root)
    #[serde(default)]
    pub chart: Option<String>,

    /// Rules to ignore (e.g., ["HL1007", "HL5001"])
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Minimum severity threshold: "error", "warning", "info", "style"
    #[serde(default)]
    pub threshold: Option<String>,
}

/// Error type for helmlint tool
#[derive(Debug, thiserror::Error)]
#[error("Helmlint error: {0}")]
pub struct HelmlintError(String);

/// Tool to lint Helm charts natively
///
/// **When to use:**
/// - Validating Helm chart structure (Chart.yaml, values.yaml)
/// - Checking Go template syntax issues (unclosed blocks, undefined variables)
/// - Helm-specific best practices
///
/// **When to use KubelintTool instead:**
/// - Checking security issues in the rendered K8s manifests
/// - Validating K8s resource configurations (probes, resource limits, RBAC)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmlintTool {
    project_path: PathBuf,
}

impl HelmlintTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    fn parse_threshold(threshold: &str) -> Severity {
        match threshold.to_lowercase().as_str() {
            "error" => Severity::Error,
            "warning" => Severity::Warning,
            "info" => Severity::Info,
            "style" => Severity::Style,
            _ => Severity::Warning,
        }
    }

    /// Get priority based on severity and category
    fn get_priority(severity: Severity, category: RuleCategory) -> &'static str {
        match (severity, category) {
            (Severity::Error, RuleCategory::Security) => "critical",
            (Severity::Error, _) => "high",
            (Severity::Warning, RuleCategory::Security) => "high",
            (Severity::Warning, RuleCategory::Template) => "high",
            (Severity::Warning, RuleCategory::Structure) => "medium",
            (Severity::Warning, _) => "medium",
            (Severity::Info, _) => "low",
            (Severity::Style, _) => "low",
            (Severity::Ignore, _) => "info",
        }
    }

    /// Get fix recommendation for common rules
    fn get_fix_recommendation(code: &str) -> &'static str {
        match code {
            // Structure rules (HL1xxx)
            "HL1001" => "Create a Chart.yaml file in the chart root directory.",
            "HL1002" => "Add 'apiVersion: v2' (for Helm 3) or 'apiVersion: v1' to Chart.yaml.",
            "HL1003" => "Add a 'name' field to Chart.yaml matching the chart directory name.",
            "HL1004" => "Add a 'version' field with semantic versioning (e.g., '1.0.0') to Chart.yaml.",
            "HL1005" => "Use semantic versioning format (MAJOR.MINOR.PATCH) for the version field.",
            "HL1006" => "Add a 'description' field explaining what the chart does.",
            "HL1007" => "Add a 'maintainers' list with name and email for chart ownership.",
            "HL1008" => "Ensure all dependencies listed in Chart.yaml are available and versioned.",

            // Values rules (HL2xxx)
            "HL2001" => "Create a values.yaml file with default configuration values.",
            "HL2002" => "Define this value in values.yaml or provide a default in the template.",
            "HL2003" => "Remove unused values from values.yaml or use them in templates.",
            "HL2004" => "Use consistent naming (camelCase or snake_case) for all values.",
            "HL2005" => "Add comments documenting the purpose and valid options for values.",

            // Template rules (HL3xxx)
            "HL3001" => "Close the unclosed template block ({{- end }}).",
            "HL3002" => "Define this template with {{ define \"name\" }} or check for typos.",
            "HL3003" => "Use {{ .Values.key }} or {{ .Release.Name }} for valid references.",
            "HL3004" => "Check nesting of if/range/with blocks - each needs matching {{ end }}.",
            "HL3005" => "Ensure the pipeline uses valid functions and proper syntax.",
            "HL3006" => "Add whitespace control with {{- and -}} to avoid extra blank lines.",

            // Security rules (HL4xxx)
            "HL4001" => "Add 'securityContext.runAsNonRoot: true' to container specs.",
            "HL4002" => "Remove 'privileged: true' or add explicit justification annotation.",
            "HL4003" => "Add resource limits (cpu, memory) to prevent resource exhaustion.",
            "HL4004" => "Use 'readOnlyRootFilesystem: true' in securityContext.",
            "HL4005" => "Drop all capabilities and add only required ones explicitly.",

            // Best practice rules (HL5xxx)
            "HL5001" => "Add resource requests and limits for all containers.",
            "HL5002" => "Add liveness and readiness probes for health checking.",
            "HL5003" => "Use '{{ .Release.Namespace }}' for namespace-aware resources.",
            "HL5004" => "Include NOTES.txt with post-install instructions.",
            "HL5005" => "Add labels including 'app.kubernetes.io/name' and 'helm.sh/chart'.",
            "HL5006" => "Use '{{ include \"chart.fullname\" . }}' for consistent naming.",
            "HL5007" => "Add selector labels to connect Services with Deployments.",

            _ => "Review the Helm chart best practices documentation.",
        }
    }

    /// Format result optimized for agent decision-making
    fn format_result(result: &LintResult) -> String {
        // Categorize and enrich failures
        let enriched_failures: Vec<serde_json::Value> = result
            .failures
            .iter()
            .map(|f| {
                let code = f.code.as_str();
                let priority = Self::get_priority(f.severity, f.category);

                json!({
                    "code": code,
                    "severity": f.severity.as_str(),
                    "priority": priority,
                    "category": f.category.display_name(),
                    "message": f.message,
                    "file": f.file.display().to_string(),
                    "line": f.line,
                    "column": f.column,
                    "fixable": f.fixable,
                    "fix": Self::get_fix_recommendation(code),
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
            *by_category.entry(f.category.display_name()).or_default() += 1;
        }

        // Build decision context
        let decision_context = if critical.is_empty() && high.is_empty() {
            if medium.is_empty() && low.is_empty() {
                "Helm chart follows best practices. No issues found."
            } else if medium.is_empty() {
                "Minor improvements possible. Low priority issues only."
            } else {
                "Good baseline. Medium priority improvements recommended."
            }
        } else if !critical.is_empty() {
            "Critical issues found. Fix template/security issues before deployment."
        } else {
            "High priority issues found. Fix template syntax or structure issues."
        };

        // Build agent-optimized output
        let mut output = json!({
            "chart": result.chart_path,
            "success": !result.has_errors(),
            "decision_context": decision_context,
            "tool_guidance": "Use helmlint for chart structure/template issues. Use kubelint for K8s resource security/best practices.",
            "summary": {
                "total": result.failures.len(),
                "files_checked": result.files_checked,
                "by_priority": {
                    "critical": critical.len(),
                    "high": high.len(),
                    "medium": medium.len(),
                    "low": low.len(),
                },
                "by_severity": {
                    "errors": result.error_count,
                    "warnings": result.warning_count,
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
                    format!(
                        "{} line {}: {} - {}",
                        f["file"].as_str().unwrap_or(""),
                        f["line"],
                        f["code"].as_str().unwrap_or(""),
                        f["fix"].as_str().unwrap_or("")
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

impl Tool for HelmlintTool {
    const NAME: &'static str = "helmlint";

    type Error = HelmlintError;
    type Args = HelmlintArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Lint Helm chart STRUCTURE and TEMPLATES (before rendering). \
                Validates Chart.yaml, values.yaml, Go template syntax, and Helm-specific best practices. \
                \n\n**Use helmlint for:** Chart metadata, template syntax errors, undefined values, unclosed blocks. \
                \n**Use kubelint for:** Security/best practices in rendered K8s manifests (probes, resources, RBAC). \
                \n\nReturns AI-optimized JSON with issues categorized by priority and type. \
                Each issue includes an actionable fix recommendation."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "chart": {
                        "type": "string",
                        "description": "Path to Helm chart directory relative to project root (e.g., 'charts/my-app', 'helm/production'). Must contain Chart.yaml."
                    },
                    "ignore": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of rule codes to ignore (e.g., ['HL1007', 'HL5001']). See rule categories: HL1xxx=Structure, HL2xxx=Values, HL3xxx=Template, HL4xxx=Security, HL5xxx=BestPractice"
                    },
                    "threshold": {
                        "type": "string",
                        "enum": ["error", "warning", "info", "style"],
                        "description": "Minimum severity to report. Default is 'warning'."
                    }
                },
                "required": ["chart"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Build configuration
        let mut config = HelmlintConfig::default();

        // Apply ignored rules
        for rule in &args.ignore {
            config = config.ignore(rule.as_str());
        }

        // Apply threshold
        if let Some(threshold) = &args.threshold {
            config = config.with_threshold(Self::parse_threshold(threshold));
        }

        // Determine chart path
        let chart_path = if let Some(chart) = &args.chart {
            self.project_path.join(chart)
        } else {
            // Look for Chart.yaml in project root
            if self.project_path.join("Chart.yaml").exists() {
                self.project_path.clone()
            } else {
                return Err(HelmlintError(
                    "No chart specified and no Chart.yaml found in project root. \
                    Specify a chart directory with 'chart' parameter."
                        .to_string(),
                ));
            }
        };

        // Validate it's a Helm chart
        if !chart_path.join("Chart.yaml").exists() {
            return Err(HelmlintError(format!(
                "No Chart.yaml found in '{}'. This doesn't appear to be a Helm chart directory. \
                For K8s manifest linting, use the kubelint tool instead.",
                chart_path.display()
            )));
        }

        // Lint the chart
        let result = lint_chart(&chart_path, &config);

        // Check for parse errors
        if !result.parse_errors.is_empty() {
            log::warn!("Helm chart parse errors: {:?}", result.parse_errors);
        }

        Ok(Self::format_result(&result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_chart(dir: &std::path::Path) {
        fs::create_dir_all(dir.join("templates")).unwrap();

        fs::write(
            dir.join("Chart.yaml"),
            r#"apiVersion: v2
name: test-chart
version: 1.0.0
description: A test chart
"#,
        )
        .unwrap();

        fs::write(
            dir.join("values.yaml"),
            r#"replicaCount: 1
image:
  repository: nginx
  tag: "1.25"
"#,
        )
        .unwrap();

        fs::write(
            dir.join("templates/deployment.yaml"),
            r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ .Release.Name }}
spec:
  replicas: {{ .Values.replicaCount }}
"#,
        )
        .unwrap();
    }

    #[tokio::test]
    async fn test_helmlint_valid_chart() {
        let temp_dir = TempDir::new().unwrap();
        create_test_chart(temp_dir.path());

        let tool = HelmlintTool::new(temp_dir.path().to_path_buf());
        let args = HelmlintArgs {
            chart: Some(".".to_string()),
            ignore: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert!(parsed["decision_context"].is_string());
        assert!(parsed["tool_guidance"].is_string());
        assert!(parsed["summary"]["files_checked"].is_number());
    }

    #[tokio::test]
    async fn test_helmlint_no_chart() {
        let temp_dir = TempDir::new().unwrap();
        // Don't create a chart

        let tool = HelmlintTool::new(temp_dir.path().to_path_buf());
        let args = HelmlintArgs {
            chart: None,
            ignore: vec![],
            threshold: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No chart specified"));
    }

    #[tokio::test]
    async fn test_helmlint_not_a_chart() {
        let temp_dir = TempDir::new().unwrap();
        // Create a directory without Chart.yaml
        fs::create_dir_all(temp_dir.path().join("some-dir")).unwrap();

        let tool = HelmlintTool::new(temp_dir.path().to_path_buf());
        let args = HelmlintArgs {
            chart: Some("some-dir".to_string()),
            ignore: vec![],
            threshold: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No Chart.yaml"));
    }
}
