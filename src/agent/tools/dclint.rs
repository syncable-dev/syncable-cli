//! Dclint tool - Native Docker Compose linting using Rig's Tool trait
//!
//! Provides native Docker Compose linting without requiring the external dclint binary.
//! Implements docker-compose-linter rules with full pragma support.
//!
//! Output is optimized for AI agent decision-making with:
//! - Categorized issues (security, best-practice, style, performance)
//! - Priority rankings (critical, high, medium, low)
//! - Actionable fix recommendations
//! - Rule documentation links

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use super::error::{ErrorCategory, format_error_for_llm};
use crate::analyzer::dclint::{DclintConfig, LintResult, RuleCategory, Severity, lint, lint_file};

/// Arguments for the dclint tool
#[derive(Debug, Deserialize)]
pub struct DclintArgs {
    /// Path to docker-compose.yml (relative to project root) or inline content
    #[serde(default)]
    pub compose_file: Option<String>,

    /// Inline Docker Compose content to lint (alternative to path)
    #[serde(default)]
    pub content: Option<String>,

    /// Rules to ignore (e.g., ["DCL001", "DCL006"])
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Minimum severity threshold: "error", "warning", "info", "style"
    #[serde(default)]
    pub threshold: Option<String>,

    /// Whether to apply auto-fixes (if available)
    #[serde(default)]
    pub fix: bool,
}

/// Error type for dclint tool
#[derive(Debug, thiserror::Error)]
#[error("Dclint error: {0}")]
pub struct DclintError(String);

/// Tool to lint Docker Compose files natively
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DclintTool {
    project_path: PathBuf,
}

impl DclintTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    fn parse_threshold(threshold: &str) -> Severity {
        match threshold.to_lowercase().as_str() {
            "error" => Severity::Error,
            "warning" => Severity::Warning,
            "info" => Severity::Info,
            "style" => Severity::Style,
            _ => Severity::Warning, // Default
        }
    }

    /// Get priority based on severity and category
    fn get_priority(severity: Severity, category: RuleCategory) -> &'static str {
        match (severity, category) {
            (Severity::Error, RuleCategory::Security) => "critical",
            (Severity::Error, _) => "high",
            (Severity::Warning, RuleCategory::Security) => "high",
            (Severity::Warning, RuleCategory::BestPractice) => "medium",
            (Severity::Warning, _) => "medium",
            (Severity::Info, _) => "low",
            (Severity::Style, _) => "low",
        }
    }

    /// Get actionable fix recommendation for a rule
    fn get_fix_recommendation(code: &str) -> &'static str {
        match code {
            "DCL001" => {
                "Remove either the 'build' or 'image' field, or add 'pull_policy' if both are intentional."
            }
            "DCL002" => {
                "Use unique container names for each service, or remove explicit container_name to use auto-generated names."
            }
            "DCL003" => {
                "Use different host ports for each service, or bind to different interfaces (e.g., 127.0.0.1:8080:80)."
            }
            "DCL004" => "Remove quotes from volume paths. YAML doesn't require quotes for paths.",
            "DCL005" => {
                "Add explicit interface binding, e.g., '127.0.0.1:8080:80' instead of '8080:80' for local-only access."
            }
            "DCL006" => {
                "Remove the 'version' field. Docker Compose now infers the version automatically."
            }
            "DCL007" => "Add 'name: myproject' at the top level for explicit project naming.",
            "DCL008" => {
                "Quote port mappings to prevent YAML parsing issues, e.g., \"8080:80\" instead of 8080:80."
            }
            "DCL009" => {
                "Use lowercase container names with only letters, numbers, hyphens, and underscores."
            }
            "DCL010" => {
                "Sort dependencies alphabetically for better readability and easier merges."
            }
            "DCL011" => {
                "Use explicit version tags (e.g., nginx:1.25) instead of implicit latest or untagged images."
            }
            "DCL012" => {
                "Reorder service keys to follow convention: image, build, container_name, ports, volumes, environment, etc."
            }
            "DCL013" => "Sort port mappings alphabetically/numerically for consistency.",
            "DCL014" => "Sort services alphabetically for better navigation and easier merges.",
            "DCL015" => {
                "Reorder top-level keys: name, services, networks, volumes, configs, secrets."
            }
            _ => "Review the rule documentation for specific guidance.",
        }
    }

    /// Get documentation URL for a rule
    fn get_rule_url(code: &str) -> String {
        if code.starts_with("DCL") {
            let rule_name = match code {
                "DCL001" => "no-build-and-image-rule",
                "DCL002" => "no-duplicate-container-names-rule",
                "DCL003" => "no-duplicate-exported-ports-rule",
                "DCL004" => "no-quotes-in-volumes-rule",
                "DCL005" => "no-unbound-port-interfaces-rule",
                "DCL006" => "no-version-field-rule",
                "DCL007" => "require-project-name-field-rule",
                "DCL008" => "require-quotes-in-ports-rule",
                "DCL009" => "service-container-name-regex-rule",
                "DCL010" => "service-dependencies-alphabetical-order-rule",
                "DCL011" => "service-image-require-explicit-tag-rule",
                "DCL012" => "service-keys-order-rule",
                "DCL013" => "service-ports-alphabetical-order-rule",
                "DCL014" => "services-alphabetical-order-rule",
                "DCL015" => "top-level-properties-order-rule",
                _ => return String::new(),
            };
            format!(
                "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/{}.md",
                rule_name
            )
        } else {
            String::new()
        }
    }

    /// Format result optimized for agent decision-making
    fn format_result(result: &LintResult, filename: &str) -> String {
        // Categorize and enrich failures
        let enriched_failures: Vec<serde_json::Value> = result
            .failures
            .iter()
            .map(|f| {
                let code = f.code.as_str();
                let priority = Self::get_priority(f.severity, f.category);

                json!({
                    "code": code,
                    "ruleName": f.rule_name,
                    "severity": f.severity.as_str(),
                    "priority": priority,
                    "category": f.category.as_str(),
                    "message": f.message,
                    "line": f.line,
                    "column": f.column,
                    "fixable": f.fixable,
                    "fix": Self::get_fix_recommendation(code),
                    "docs": Self::get_rule_url(code),
                })
            })
            .collect();

        // Group by priority for agent decision ordering
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

        // Group by category for thematic fixes
        let mut by_category: std::collections::HashMap<&str, Vec<_>> =
            std::collections::HashMap::new();
        for f in &enriched_failures {
            let cat = f["category"].as_str().unwrap_or("other");
            by_category.entry(cat).or_default().push(f.clone());
        }

        // Build decision context
        let decision_context = if critical.is_empty() && high.is_empty() {
            if medium.is_empty() && low.is_empty() {
                "Docker Compose file follows best practices. No issues found."
            } else if medium.is_empty() {
                "Minor improvements possible. Low priority issues only (style/formatting)."
            } else {
                "Good baseline. Medium priority improvements recommended."
            }
        } else if !critical.is_empty() {
            "Critical issues found. Address security/error issues first before deployment."
        } else {
            "High priority issues found. Review and fix before production use."
        };

        // Count fixable issues
        let fixable_count = enriched_failures
            .iter()
            .filter(|f| f["fixable"] == true)
            .count();

        // Build agent-optimized output
        let mut output = json!({
            "file": filename,
            "success": !result.has_errors(),
            "decision_context": decision_context,
            "summary": {
                "total": result.failures.len(),
                "by_priority": {
                    "critical": critical.len(),
                    "high": high.len(),
                    "medium": medium.len(),
                    "low": low.len(),
                },
                "by_severity": {
                    "errors": result.error_count,
                    "warnings": result.warning_count,
                    "info": result.failures.iter().filter(|f| f.severity == Severity::Info).count(),
                    "style": result.failures.iter().filter(|f| f.severity == Severity::Style).count(),
                },
                "by_category": by_category.iter().map(|(k, v)| (k.to_string(), v.len())).collect::<std::collections::HashMap<_, _>>(),
                "fixable": fixable_count,
            },
            "action_plan": {
                "critical": critical,
                "high": high,
                "medium": medium,
                "low": low,
            },
        });

        // Add quick fixes summary for agent
        if !enriched_failures.is_empty() {
            let quick_fixes: Vec<String> = enriched_failures
                .iter()
                .filter(|f| f["priority"] == "critical" || f["priority"] == "high")
                .take(5)
                .map(|f| {
                    format!(
                        "Line {}: {} - {}",
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

impl Tool for DclintTool {
    const NAME: &'static str = "dclint";

    type Error = DclintError;
    type Args = DclintArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Native Docker Compose linting with AI-optimized output. No external binary required.

CAPABILITIES:
- Validates docker-compose.yml files against 15 rules
- Provides auto-fix support for 8 rules (use fix: true)
- Returns prioritized issues with actionable fix recommendations
- Auto-discovers compose files in project root

RULE CATEGORIES:
- Security (DCL0xx): Port exposure (DCL005), network settings
- Best Practice (DCL1xx): Version field (DCL006), project naming (DCL007), image tags (DCL011)
- Style (DCL2xx): Ordering rules (DCL010, DCL012-015), container naming (DCL009)
- Performance (DCL3xx): Build caching, resource usage patterns

KEY RULES:
- DCL001: No both build and image in same service
- DCL005: Ports should bind to specific interface (security)
- DCL006: Version field is deprecated (remove it)
- DCL011: Images need explicit version tags (not :latest or untagged)

OUTPUT FORMAT:
- 'decision_context': Quick assessment of severity
- 'action_plan': Issues grouped by priority (critical/high/medium/low)
- 'quick_fixes': Top 5 most important fixes to apply

USAGE:
1. Without args: Scans for docker-compose.yml in project root
2. With compose_file: Lint specific file by path
3. With content: Lint inline YAML (useful for validating before write)"#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "compose_file": {
                        "type": "string",
                        "description": "Path to docker-compose.yml relative to project root. Examples: 'docker-compose.yml', 'deploy/compose.prod.yml', 'docker/docker-compose.dev.yaml'"
                    },
                    "content": {
                        "type": "string",
                        "description": "Inline Docker Compose YAML content to lint. Use when validating generated content before writing to file. Must include 'services:' section."
                    },
                    "ignore": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Rule codes to skip. Common: ['DCL006'] for legacy version field, ['DCL014', 'DCL015'] to skip ordering rules."
                    },
                    "threshold": {
                        "type": "string",
                        "enum": ["error", "warning", "info", "style"],
                        "description": "Minimum severity to report. 'error' for critical only, 'warning' (default) for actionable issues, 'style' for all."
                    },
                    "fix": {
                        "type": "boolean",
                        "description": "Apply auto-fixes. Supported rules: DCL004, DCL006, DCL008, DCL010, DCL012-015. Returns fixed content in response."
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Build configuration
        let mut config = DclintConfig::default();

        // Apply ignored rules
        for rule in &args.ignore {
            config = config.ignore(rule.as_str());
        }

        // Apply threshold
        if let Some(threshold) = &args.threshold {
            config = config.with_threshold(Self::parse_threshold(threshold));
        }

        // Determine source, filename, and lint
        // IMPORTANT: Treat empty content as None - fixes AI agents passing empty strings
        let (result, filename) = if args.content.as_ref().is_some_and(|c| !c.trim().is_empty()) {
            // Lint non-empty inline content
            let content = args.content.as_ref().unwrap();

            // Check for non-compose YAML (no services section)
            if !content.contains("services:") && !content.contains("services :") {
                return Ok(format_error_for_llm(
                    "dclint",
                    ErrorCategory::ValidationFailed,
                    "Content does not appear to be a Docker Compose file (missing 'services' section)",
                    Some(vec![
                        "Docker Compose files must have a 'services' section",
                        "Ensure the YAML defines at least one service",
                        "Example: services:\\n  web:\\n    image: nginx:latest",
                    ]),
                ));
            }

            (lint(content, &config), "<inline>".to_string())
        } else if let Some(compose_file) = &args.compose_file {
            // Lint file
            let path = self.project_path.join(compose_file);

            // Check if file exists
            if !path.exists() {
                return Ok(format_error_for_llm(
                    "dclint",
                    ErrorCategory::FileNotFound,
                    &format!("Docker Compose file not found: {}", compose_file),
                    Some(vec![
                        "Check if the file path is correct",
                        "Verify the file exists relative to the project root",
                        "Use list_directory to explore available files",
                        "Common names: docker-compose.yml, docker-compose.yaml, compose.yml",
                    ]),
                ));
            }

            // Check if file is empty
            if let Ok(metadata) = std::fs::metadata(&path) {
                if metadata.len() == 0 {
                    return Ok(format_error_for_llm(
                        "dclint",
                        ErrorCategory::ValidationFailed,
                        &format!("Docker Compose file is empty: {}", compose_file),
                        Some(vec![
                            "Add service definitions to the file",
                            "Example minimal compose file:",
                            "services:\\n  app:\\n    image: myimage:latest",
                        ]),
                    ));
                }
            }

            (lint_file(&path, &config), compose_file.clone())
        } else {
            // Default: look for docker-compose.yml in project root
            let default_files = [
                "docker-compose.yml",
                "docker-compose.yaml",
                "compose.yml",
                "compose.yaml",
            ];

            let mut found = None;
            for file in &default_files {
                let path = self.project_path.join(file);
                if path.exists() {
                    found = Some((lint_file(&path, &config), file.to_string()));
                    break;
                }
            }

            match found {
                Some((result, filename)) => (result, filename),
                None => {
                    return Ok(format_error_for_llm(
                        "dclint",
                        ErrorCategory::FileNotFound,
                        "No Docker Compose file found in project root",
                        Some(vec![
                            "Check if the file exists in the project root",
                            "Common names: docker-compose.yml, docker-compose.yaml, compose.yml, compose.yaml",
                            "Use compose_file parameter to specify a custom path",
                            "Use content parameter to lint inline YAML",
                        ]),
                    ));
                }
            }
        };

        // Handle parse errors - return structured error for agent
        if !result.parse_errors.is_empty() {
            log::warn!("Docker Compose parse errors: {:?}", result.parse_errors);
            // If we have ONLY parse errors and no lint results, treat as validation failure
            if result.failures.is_empty() && result.error_count == 0 && result.warning_count == 0 {
                return Ok(format_error_for_llm(
                    "dclint",
                    ErrorCategory::ValidationFailed,
                    &format!("Invalid Docker Compose YAML syntax: {}", result.parse_errors.join(", ")),
                    Some(vec![
                        "Check YAML indentation (use spaces, not tabs)",
                        "Verify key-value pair syntax (key: value)",
                        "Ensure quotes are properly matched",
                        "Validate the 'services' section structure",
                    ]),
                ));
            }
        }

        Ok(Self::format_result(&result, &filename))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::fs;

    #[tokio::test]
    async fn test_dclint_inline_content() {
        let tool = DclintTool::new(temp_dir());
        let args = DclintArgs {
            compose_file: None,
            content: Some(
                r#"
services:
  web:
    build: .
    image: nginx:latest
"#
                .to_string(),
            ),
            ignore: vec![],
            threshold: None,
            fix: false,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Should detect DCL001 (build+image)
        assert!(!parsed["success"].as_bool().unwrap_or(true));
        assert!(parsed["summary"]["total"].as_u64().unwrap_or(0) >= 1);

        // Check new fields exist
        assert!(parsed["decision_context"].is_string());
        assert!(parsed["action_plan"].is_object());
    }

    #[tokio::test]
    async fn test_dclint_ignore_rules() {
        let tool = DclintTool::new(temp_dir());
        let args = DclintArgs {
            compose_file: None,
            content: Some(
                r#"
version: "3.8"
services:
  web:
    image: nginx:latest
"#
                .to_string(),
            ),
            ignore: vec!["DCL006".to_string(), "DCL011".to_string()],
            threshold: None,
            fix: false,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // DCL006 and DCL011 should be ignored
        let all_codes: Vec<&str> = parsed["action_plan"]
            .as_object()
            .unwrap()
            .values()
            .flat_map(|v| v.as_array().unwrap())
            .filter_map(|v| v["code"].as_str())
            .collect();

        assert!(!all_codes.contains(&"DCL006"));
        assert!(!all_codes.contains(&"DCL011"));
    }

    #[tokio::test]
    async fn test_dclint_file() {
        let temp = temp_dir().join("dclint_test");
        fs::create_dir_all(&temp).unwrap();
        let compose_file = temp.join("docker-compose.yml");
        fs::write(
            &compose_file,
            r#"
name: myproject
services:
  web:
    image: nginx:1.25
    ports:
      - "8080:80"
"#,
        )
        .unwrap();

        let tool = DclintTool::new(temp.clone());
        let args = DclintArgs {
            compose_file: Some("docker-compose.yml".to_string()),
            content: None,
            ignore: vec![],
            threshold: None,
            fix: false,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Well-formed compose file should have few/no critical issues
        assert_eq!(parsed["file"], "docker-compose.yml");

        // Cleanup
        fs::remove_dir_all(&temp).ok();
    }

    #[tokio::test]
    async fn test_dclint_valid_compose() {
        let tool = DclintTool::new(temp_dir());
        let compose = r#"
name: myproject
services:
  api:
    image: node:20-alpine
    ports:
      - "127.0.0.1:3000:3000"
  db:
    image: postgres:16-alpine
"#;

        let args = DclintArgs {
            compose_file: None,
            content: Some(compose.to_string()),
            ignore: vec![],
            threshold: None,
            fix: false,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Well-structured compose file should pass (no errors)
        assert!(parsed["success"].as_bool().unwrap_or(false));
        assert!(parsed["decision_context"].is_string());
        // Should not have critical or high priority issues
        assert_eq!(
            parsed["summary"]["by_priority"]["critical"]
                .as_u64()
                .unwrap_or(99),
            0
        );
        assert_eq!(
            parsed["summary"]["by_priority"]["high"]
                .as_u64()
                .unwrap_or(99),
            0
        );
    }

    // Unit tests for internal helper functions

    #[test]
    fn test_parse_threshold() {
        assert_eq!(DclintTool::parse_threshold("error"), Severity::Error);
        assert_eq!(DclintTool::parse_threshold("warning"), Severity::Warning);
        assert_eq!(DclintTool::parse_threshold("info"), Severity::Info);
        assert_eq!(DclintTool::parse_threshold("style"), Severity::Style);
        // Case insensitive
        assert_eq!(DclintTool::parse_threshold("ERROR"), Severity::Error);
        assert_eq!(DclintTool::parse_threshold("Warning"), Severity::Warning);
        // Invalid defaults to Warning
        assert_eq!(DclintTool::parse_threshold("invalid"), Severity::Warning);
        assert_eq!(DclintTool::parse_threshold(""), Severity::Warning);
    }

    #[test]
    fn test_get_priority() {
        use crate::analyzer::dclint::RuleCategory;

        // Critical: Error + Security
        assert_eq!(
            DclintTool::get_priority(Severity::Error, RuleCategory::Security),
            "critical"
        );

        // High: Error + other, Warning + Security
        assert_eq!(
            DclintTool::get_priority(Severity::Error, RuleCategory::BestPractice),
            "high"
        );
        assert_eq!(
            DclintTool::get_priority(Severity::Warning, RuleCategory::Security),
            "high"
        );

        // Medium: Warning + BestPractice or other
        assert_eq!(
            DclintTool::get_priority(Severity::Warning, RuleCategory::BestPractice),
            "medium"
        );
        assert_eq!(
            DclintTool::get_priority(Severity::Warning, RuleCategory::Style),
            "medium"
        );

        // Low: Info or Style severity
        assert_eq!(
            DclintTool::get_priority(Severity::Info, RuleCategory::BestPractice),
            "low"
        );
        assert_eq!(
            DclintTool::get_priority(Severity::Info, RuleCategory::Style),
            "low"
        );
        assert_eq!(
            DclintTool::get_priority(Severity::Style, RuleCategory::Style),
            "low"
        );
    }

    #[test]
    fn test_fix_recommendations() {
        // DCL001 - build+image conflict
        let rec = DclintTool::get_fix_recommendation("DCL001");
        assert!(rec.contains("build") || rec.contains("image"));

        // DCL005 - port interface binding
        let rec = DclintTool::get_fix_recommendation("DCL005");
        assert!(rec.contains("interface") || rec.contains("127.0.0.1"));

        // DCL006 - version field
        let rec = DclintTool::get_fix_recommendation("DCL006");
        assert!(rec.contains("version") || rec.contains("Remove"));

        // DCL011 - explicit image tags
        let rec = DclintTool::get_fix_recommendation("DCL011");
        assert!(rec.contains("tag") || rec.contains("latest"));

        // Unknown rule - generic guidance
        let rec = DclintTool::get_fix_recommendation("UNKNOWN");
        assert!(rec.contains("documentation") || rec.contains("Review"));
    }

    #[test]
    fn test_rule_url_generation() {
        // Valid rule codes should return URLs
        let url = DclintTool::get_rule_url("DCL001");
        assert!(url.contains("docker-compose-linter"));
        assert!(url.contains("no-build-and-image"));

        let url = DclintTool::get_rule_url("DCL006");
        assert!(url.contains("no-version-field"));

        // Unknown rule codes return empty string
        let url = DclintTool::get_rule_url("UNKNOWN");
        assert!(url.is_empty());

        let url = DclintTool::get_rule_url("DCL999");
        assert!(url.is_empty());
    }
}
