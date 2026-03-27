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

use super::error::{ErrorCategory, format_error_for_llm};
use crate::analyzer::helmlint::types::RuleCategory;
use crate::analyzer::helmlint::{HelmlintConfig, LintResult, Severity, lint_chart};

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
            "HL1004" => {
                "Add a 'version' field with semantic versioning (e.g., '1.0.0') to Chart.yaml."
            }
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
            description: r#"Native Helm chart linting for chart STRUCTURE and TEMPLATES (before rendering).

**What helmlint validates:**
- Chart.yaml (metadata, versioning, dependencies)
- values.yaml (schema, unused values, type consistency)
- Go template syntax (unclosed blocks, undefined variables)
- Helm-specific best practices (naming, labels, probes)

**Rule Categories:**
- HL1xxx (Structure): Chart.yaml metadata, directory structure
- HL2xxx (Values): values.yaml validation, defaults
- HL3xxx (Template): Go template syntax, undefined references
- HL4xxx (Security): Security concerns in templates
- HL5xxx (BestPractice): Helm conventions, standard labels

**Use helmlint for:** Chart development, template syntax issues, metadata validation.
**Use kubelint for:** Security/best practices in the RENDERED K8s manifests (probes, resources, RBAC).

Returns prioritized issues with fix recommendations grouped by priority (critical/high/medium/low)."#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "chart": {
                        "type": "string",
                        "description": "Path to Helm chart directory relative to project root. Must contain Chart.yaml. Examples: 'charts/my-app', 'helm/production', 'deploy/chart'"
                    },
                    "ignore": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Rule codes to skip. Format: HL[1-5]xxx. Examples: ['HL1007', 'HL5001']. Categories: 1=Structure, 2=Values, 3=Template, 4=Security, 5=BestPractice"
                    },
                    "threshold": {
                        "type": "string",
                        "enum": ["error", "warning", "info", "style"],
                        "default": "warning",
                        "description": "Minimum severity to report. 'error'=critical only, 'warning'=errors+warnings (default), 'info'=all except style, 'style'=everything"
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
            let path = self.project_path.join(chart);

            // Check if the path exists at all
            if !path.exists() {
                return Ok(format_error_for_llm(
                    "helmlint",
                    ErrorCategory::FileNotFound,
                    &format!("Chart path '{}' does not exist", chart),
                    Some(vec![
                        "Verify the chart directory path is correct",
                        "Use list_directory to explore available paths",
                        "Helm charts are typically in 'charts/', 'helm/', or 'deploy/' directories",
                    ]),
                ));
            }

            // Check if it's a directory
            if !path.is_dir() {
                return Ok(format_error_for_llm(
                    "helmlint",
                    ErrorCategory::ValidationFailed,
                    &format!("'{}' is not a directory", chart),
                    Some(vec![
                        "The chart parameter must point to a Helm chart directory",
                        "The directory should contain Chart.yaml",
                    ]),
                ));
            }

            path
        } else {
            // Look for Chart.yaml in project root
            if self.project_path.join("Chart.yaml").exists() {
                self.project_path.clone()
            } else {
                return Ok(format_error_for_llm(
                    "helmlint",
                    ErrorCategory::ValidationFailed,
                    "No chart specified and no Chart.yaml found in project root",
                    Some(vec![
                        "Specify a chart directory with the 'chart' parameter",
                        "Use list_directory to find Helm charts (look for Chart.yaml files)",
                        "Common locations: charts/, helm/, deploy/",
                    ]),
                ));
            }
        };

        // Validate it's a Helm chart (has Chart.yaml)
        if !chart_path.join("Chart.yaml").exists() {
            // Check if it's an empty directory
            let is_empty = std::fs::read_dir(&chart_path)
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(false);

            if is_empty {
                return Ok(format_error_for_llm(
                    "helmlint",
                    ErrorCategory::ValidationFailed,
                    &format!("Directory '{}' is empty", chart_path.display()),
                    Some(vec![
                        "The directory must contain Chart.yaml to be a valid Helm chart",
                        "Run 'helm create <name>' to scaffold a new chart",
                    ]),
                ));
            }

            return Ok(format_error_for_llm(
                "helmlint",
                ErrorCategory::ValidationFailed,
                &format!(
                    "Not a valid Helm chart: Chart.yaml not found in '{}'",
                    chart_path.display()
                ),
                Some(vec![
                    "Ensure the path points to a Helm chart directory",
                    "Chart directory must contain Chart.yaml",
                    "For K8s manifest linting (not Helm charts), use kubelint instead",
                    "Use list_directory to explore the directory structure",
                ]),
            ));
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
    use crate::analyzer::helmlint::types::RuleCategory;
    use std::fs;
    use tempfile::TempDir;

    // ==================== Unit Tests ====================

    #[test]
    fn test_parse_threshold() {
        assert_eq!(HelmlintTool::parse_threshold("error"), Severity::Error);
        assert_eq!(HelmlintTool::parse_threshold("warning"), Severity::Warning);
        assert_eq!(HelmlintTool::parse_threshold("info"), Severity::Info);
        assert_eq!(HelmlintTool::parse_threshold("style"), Severity::Style);
        // Case insensitive
        assert_eq!(HelmlintTool::parse_threshold("ERROR"), Severity::Error);
        assert_eq!(HelmlintTool::parse_threshold("Warning"), Severity::Warning);
        // Invalid defaults to warning
        assert_eq!(HelmlintTool::parse_threshold("invalid"), Severity::Warning);
        assert_eq!(HelmlintTool::parse_threshold(""), Severity::Warning);
    }

    #[test]
    fn test_get_priority() {
        // Security errors are always critical
        assert_eq!(
            HelmlintTool::get_priority(Severity::Error, RuleCategory::Security),
            "critical"
        );

        // Non-security errors are high
        assert_eq!(
            HelmlintTool::get_priority(Severity::Error, RuleCategory::Structure),
            "high"
        );
        assert_eq!(
            HelmlintTool::get_priority(Severity::Error, RuleCategory::Template),
            "high"
        );
        assert_eq!(
            HelmlintTool::get_priority(Severity::Error, RuleCategory::Values),
            "high"
        );
        assert_eq!(
            HelmlintTool::get_priority(Severity::Error, RuleCategory::BestPractice),
            "high"
        );

        // Security warnings are high
        assert_eq!(
            HelmlintTool::get_priority(Severity::Warning, RuleCategory::Security),
            "high"
        );

        // Template warnings are high
        assert_eq!(
            HelmlintTool::get_priority(Severity::Warning, RuleCategory::Template),
            "high"
        );

        // Structure warnings are medium
        assert_eq!(
            HelmlintTool::get_priority(Severity::Warning, RuleCategory::Structure),
            "medium"
        );

        // Other warnings are medium
        assert_eq!(
            HelmlintTool::get_priority(Severity::Warning, RuleCategory::BestPractice),
            "medium"
        );
        assert_eq!(
            HelmlintTool::get_priority(Severity::Warning, RuleCategory::Values),
            "medium"
        );

        // Info and Style are low
        assert_eq!(
            HelmlintTool::get_priority(Severity::Info, RuleCategory::Structure),
            "low"
        );
        assert_eq!(
            HelmlintTool::get_priority(Severity::Info, RuleCategory::Security),
            "low"
        );
        assert_eq!(
            HelmlintTool::get_priority(Severity::Style, RuleCategory::Template),
            "low"
        );

        // Ignore is info
        assert_eq!(
            HelmlintTool::get_priority(Severity::Ignore, RuleCategory::Security),
            "info"
        );
    }

    #[test]
    fn test_fix_recommendations() {
        // Structure rules (HL1xxx)
        assert!(HelmlintTool::get_fix_recommendation("HL1001").contains("Chart.yaml"));
        assert!(HelmlintTool::get_fix_recommendation("HL1002").contains("apiVersion"));
        assert!(HelmlintTool::get_fix_recommendation("HL1003").contains("name"));
        assert!(HelmlintTool::get_fix_recommendation("HL1004").contains("version"));
        assert!(HelmlintTool::get_fix_recommendation("HL1005").contains("semantic versioning"));
        assert!(HelmlintTool::get_fix_recommendation("HL1006").contains("description"));
        assert!(HelmlintTool::get_fix_recommendation("HL1007").contains("maintainers"));
        assert!(HelmlintTool::get_fix_recommendation("HL1008").contains("dependencies"));

        // Values rules (HL2xxx)
        assert!(HelmlintTool::get_fix_recommendation("HL2001").contains("values.yaml"));
        assert!(HelmlintTool::get_fix_recommendation("HL2002").contains("default"));
        assert!(HelmlintTool::get_fix_recommendation("HL2003").contains("unused"));
        assert!(HelmlintTool::get_fix_recommendation("HL2004").contains("naming"));
        assert!(HelmlintTool::get_fix_recommendation("HL2005").contains("comments"));

        // Template rules (HL3xxx)
        assert!(HelmlintTool::get_fix_recommendation("HL3001").contains("end"));
        assert!(HelmlintTool::get_fix_recommendation("HL3002").contains("define"));
        assert!(HelmlintTool::get_fix_recommendation("HL3003").contains("Values"));
        assert!(HelmlintTool::get_fix_recommendation("HL3004").contains("nesting"));
        assert!(HelmlintTool::get_fix_recommendation("HL3005").contains("pipeline"));
        assert!(HelmlintTool::get_fix_recommendation("HL3006").contains("whitespace"));

        // Security rules (HL4xxx)
        assert!(HelmlintTool::get_fix_recommendation("HL4001").contains("runAsNonRoot"));
        assert!(HelmlintTool::get_fix_recommendation("HL4002").contains("privileged"));
        assert!(HelmlintTool::get_fix_recommendation("HL4003").contains("resource limits"));
        assert!(HelmlintTool::get_fix_recommendation("HL4004").contains("readOnlyRootFilesystem"));
        assert!(HelmlintTool::get_fix_recommendation("HL4005").contains("capabilities"));

        // Best practice rules (HL5xxx)
        assert!(HelmlintTool::get_fix_recommendation("HL5001").contains("resource"));
        assert!(HelmlintTool::get_fix_recommendation("HL5002").contains("probes"));
        assert!(HelmlintTool::get_fix_recommendation("HL5003").contains("Namespace"));
        assert!(HelmlintTool::get_fix_recommendation("HL5004").contains("NOTES.txt"));
        assert!(HelmlintTool::get_fix_recommendation("HL5005").contains("labels"));
        assert!(HelmlintTool::get_fix_recommendation("HL5006").contains("fullname"));
        assert!(HelmlintTool::get_fix_recommendation("HL5007").contains("selector"));

        // Unknown codes return generic message
        assert!(HelmlintTool::get_fix_recommendation("HL9999").contains("best practices"));
        assert!(HelmlintTool::get_fix_recommendation("INVALID").contains("best practices"));
    }

    // ==================== Integration Tests ====================

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
    async fn test_helmlint_no_chart_returns_error_json() {
        let temp_dir = TempDir::new().unwrap();
        // Don't create a chart

        let tool = HelmlintTool::new(temp_dir.path().to_path_buf());
        let args = HelmlintArgs {
            chart: None,
            ignore: vec![],
            threshold: None,
        };

        // Now returns Ok with error JSON instead of Err
        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["error"], true);
        assert_eq!(parsed["tool"], "helmlint");
        assert_eq!(parsed["code"], "VALIDATION_FAILED");
        assert!(
            parsed["message"]
                .as_str()
                .unwrap()
                .contains("No chart specified")
        );
        assert!(parsed["suggestions"].is_array());
    }

    #[tokio::test]
    async fn test_helmlint_not_a_chart_returns_error_json() {
        let temp_dir = TempDir::new().unwrap();
        // Create a directory without Chart.yaml but with a file so it's not empty
        fs::create_dir_all(temp_dir.path().join("some-dir")).unwrap();
        fs::write(temp_dir.path().join("some-dir/README.md"), "test").unwrap();

        let tool = HelmlintTool::new(temp_dir.path().to_path_buf());
        let args = HelmlintArgs {
            chart: Some("some-dir".to_string()),
            ignore: vec![],
            threshold: None,
        };

        // Now returns Ok with error JSON instead of Err
        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["error"], true);
        assert_eq!(parsed["tool"], "helmlint");
        assert_eq!(parsed["code"], "VALIDATION_FAILED");
        assert!(
            parsed["message"]
                .as_str()
                .unwrap()
                .contains("Chart.yaml not found")
        );
        assert!(parsed["suggestions"].is_array());
    }

    #[tokio::test]
    async fn test_helmlint_nonexistent_path_returns_error_json() {
        let temp_dir = TempDir::new().unwrap();

        let tool = HelmlintTool::new(temp_dir.path().to_path_buf());
        let args = HelmlintArgs {
            chart: Some("nonexistent-dir".to_string()),
            ignore: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["error"], true);
        assert_eq!(parsed["tool"], "helmlint");
        assert_eq!(parsed["code"], "FILE_NOT_FOUND");
        assert!(
            parsed["message"]
                .as_str()
                .unwrap()
                .contains("does not exist")
        );
    }

    #[tokio::test]
    async fn test_helmlint_file_not_directory_returns_error_json() {
        let temp_dir = TempDir::new().unwrap();
        // Create a file instead of a directory
        fs::write(temp_dir.path().join("not-a-dir"), "content").unwrap();

        let tool = HelmlintTool::new(temp_dir.path().to_path_buf());
        let args = HelmlintArgs {
            chart: Some("not-a-dir".to_string()),
            ignore: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["error"], true);
        assert_eq!(parsed["tool"], "helmlint");
        assert_eq!(parsed["code"], "VALIDATION_FAILED");
        assert!(
            parsed["message"]
                .as_str()
                .unwrap()
                .contains("not a directory")
        );
    }

    #[tokio::test]
    async fn test_helmlint_empty_directory_returns_error_json() {
        let temp_dir = TempDir::new().unwrap();
        // Create an empty directory
        fs::create_dir_all(temp_dir.path().join("empty-dir")).unwrap();

        let tool = HelmlintTool::new(temp_dir.path().to_path_buf());
        let args = HelmlintArgs {
            chart: Some("empty-dir".to_string()),
            ignore: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["error"], true);
        assert_eq!(parsed["tool"], "helmlint");
        assert_eq!(parsed["code"], "VALIDATION_FAILED");
        assert!(parsed["message"].as_str().unwrap().contains("empty"));
    }
}
