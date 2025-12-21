//! Hadolint tool - Native Dockerfile linting using Rig's Tool trait
//!
//! Provides native Dockerfile linting without requiring the external hadolint binary.
//! Implements hadolint rules with full pragma support.
//!
//! Output is optimized for AI agent decision-making with:
//! - Categorized issues (security, best-practice, maintainability, performance)
//! - Priority rankings (critical, high, medium, low)
//! - Actionable fix recommendations
//! - Rule documentation links

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use crate::analyzer::hadolint::{lint, lint_file, HadolintConfig, LintResult, Severity};

/// Arguments for the hadolint tool
#[derive(Debug, Deserialize)]
pub struct HadolintArgs {
    /// Path to Dockerfile (relative to project root) or inline content
    #[serde(default)]
    pub dockerfile: Option<String>,

    /// Inline Dockerfile content to lint (alternative to path)
    #[serde(default)]
    pub content: Option<String>,

    /// Rules to ignore (e.g., ["DL3008", "DL3013"])
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Minimum severity threshold: "error", "warning", "info", "style"
    #[serde(default)]
    pub threshold: Option<String>,
}

/// Error type for hadolint tool
#[derive(Debug, thiserror::Error)]
#[error("Hadolint error: {0}")]
pub struct HadolintError(String);

/// Tool to lint Dockerfiles natively
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HadolintTool {
    project_path: PathBuf,
}

impl HadolintTool {
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

    /// Get the category for a rule code
    fn get_rule_category(code: &str) -> &'static str {
        match code {
            // Security rules
            "DL3000" | "DL3002" | "DL3004" | "DL3047" => "security",
            // Best practice rules
            "DL3003" | "DL3006" | "DL3007" | "DL3008" | "DL3009" | "DL3013" |
            "DL3014" | "DL3015" | "DL3016" | "DL3018" | "DL3019" | "DL3020" |
            "DL3025" | "DL3027" | "DL3028" | "DL3033" | "DL3042" | "DL3059" => "best-practice",
            // Maintainability rules
            "DL3005" | "DL3010" | "DL3021" | "DL3022" | "DL3023" | "DL3024" |
            "DL3026" | "DL3029" | "DL3030" | "DL3032" | "DL3034" | "DL3035" |
            "DL3036" | "DL3044" | "DL3045" | "DL3048" | "DL3049" | "DL3050" |
            "DL3051" | "DL3052" | "DL3053" | "DL3054" | "DL3055" | "DL3056" |
            "DL3057" | "DL3058" | "DL3060" | "DL3061" => "maintainability",
            // Performance rules
            "DL3001" | "DL3011" | "DL3017" | "DL3031" | "DL3037" | "DL3038" |
            "DL3039" | "DL3040" | "DL3041" | "DL3046" | "DL3062" => "performance",
            // Deprecated instructions
            "DL4000" | "DL4001" | "DL4003" | "DL4005" | "DL4006" => "deprecated",
            // ShellCheck rules
            _ if code.starts_with("SC") => "shell",
            _ => "other",
        }
    }

    /// Get priority based on severity and category
    fn get_priority(severity: Severity, category: &str) -> &'static str {
        match (severity, category) {
            (Severity::Error, "security") => "critical",
            (Severity::Error, _) => "high",
            (Severity::Warning, "security") => "high",
            (Severity::Warning, "best-practice") => "medium",
            (Severity::Warning, _) => "medium",
            (Severity::Info, _) => "low",
            (Severity::Style, _) => "low",
            (Severity::Ignore, _) => "info",
        }
    }

    /// Get actionable fix recommendation for a rule
    fn get_fix_recommendation(code: &str) -> &'static str {
        match code {
            "DL3000" => "Use absolute WORKDIR paths like '/app' instead of relative paths.",
            "DL3001" => "Remove commands that have no effect in Docker (like 'ssh', 'mount').",
            "DL3002" => "Remove the last USER instruction setting root, or add 'USER <non-root>' at the end.",
            "DL3003" => "Use WORKDIR to change directories instead of 'cd' in RUN commands.",
            "DL3004" => "Remove 'sudo' from RUN commands. Docker runs as root by default, or use proper USER switching.",
            "DL3005" => "Remove 'apt-get upgrade' or 'dist-upgrade'. Pin packages instead for reproducibility.",
            "DL3006" => "Add explicit version tag to base image, e.g., 'FROM node:18-alpine' instead of 'FROM node'.",
            "DL3007" => "Use specific version tag instead of ':latest', e.g., 'nginx:1.25-alpine'.",
            "DL3008" => "Pin apt package versions: 'apt-get install package=version' or use '--no-install-recommends'.",
            "DL3009" => "Add 'rm -rf /var/lib/apt/lists/*' after apt-get install to reduce image size.",
            "DL3010" => "Use ADD only for extracting archives. For other files, use COPY.",
            "DL3011" => "Use valid port numbers (0-65535) in EXPOSE.",
            "DL3013" => "Pin pip package versions: 'pip install package==version'.",
            "DL3014" => "Add '-y' flag to apt-get install for non-interactive mode.",
            "DL3015" => "Add '--no-install-recommends' to apt-get install to minimize image size.",
            "DL3016" => "Pin npm package versions: 'npm install package@version'.",
            "DL3017" => "Remove 'apt-get upgrade'. Pin specific package versions instead.",
            "DL3018" => "Pin apk package versions: 'apk add package=version'.",
            "DL3019" => "Add '--no-cache' to apk add instead of separate cache cleanup.",
            "DL3020" => "Use COPY instead of ADD for files from build context. ADD is for URLs and archives.",
            "DL3021" => "Use COPY with --from for multi-stage builds instead of COPY from external images.",
            "DL3022" => "Use COPY --from=stage instead of --from=image for multi-stage builds.",
            "DL3023" => "Reference build stage by name instead of number in COPY --from.",
            "DL3024" => "Use lowercase for 'as' in multi-stage builds: 'FROM image AS builder'.",
            "DL3025" => "Use JSON array format for CMD/ENTRYPOINT: CMD [\"executable\", \"arg1\"].",
            "DL3026" => "Use official Docker images when possible, or document why unofficial is needed.",
            "DL3027" => "Remove 'apt' and use 'apt-get' for scripting in Dockerfiles.",
            "DL3028" => "Pin gem versions: 'gem install package:version'.",
            "DL3029" => "Specify --platform explicitly for multi-arch builds.",
            "DL3030" => "Pin yum/dnf package versions: 'yum install package-version'.",
            "DL3032" => "Replace 'yum clean all' with 'dnf clean all' for newer distros.",
            "DL3033" => "Add 'yum clean all' after yum install to reduce image size.",
            "DL3034" => "Add '--setopt=install_weak_deps=False' to dnf install.",
            "DL3035" => "Add 'dnf clean all' after dnf install to reduce image size.",
            "DL3036" => "Pin zypper package versions: 'zypper install package=version'.",
            "DL3037" => "Add 'zypper clean' after zypper install.",
            "DL3038" => "Add '--no-recommends' to zypper install.",
            "DL3039" => "Add 'zypper clean' after zypper install.",
            "DL3040" => "Add 'dnf clean all && rm -rf /var/cache/dnf' after dnf install.",
            "DL3041" => "Add 'microdnf clean all' after microdnf install.",
            "DL3042" => "Avoid pip cache in builds. Use '--no-cache-dir' or set PIP_NO_CACHE_DIR=1.",
            "DL3044" => "Only use 'HEALTHCHECK' once per Dockerfile, or it won't work correctly.",
            "DL3045" => "Use COPY instead of ADD for local files.",
            "DL3046" => "Use 'useradd' instead of 'adduser' for better compatibility.",
            "DL3047" => "Add 'wget --progress=dot:giga' or 'curl --progress-bar' to show progress during download.",
            "DL3048" => "Prefer setting flag with 'SHELL' instruction instead of inline in RUN.",
            "DL3049" => "Add a 'LABEL maintainer=\"name\"' for documentation.",
            "DL3050" => "Add 'LABEL version=\"x.y\"' for versioning.",
            "DL3051" => "Add 'LABEL description=\"...\"' for documentation.",
            "DL3052" => "Prefer relative paths with LABEL for better portability.",
            "DL3053" => "Remove unused LABEL instructions.",
            "DL3054" => "Use recommended labels from OCI spec (org.opencontainers.image.*).",
            "DL3055" => "Add 'LABEL org.opencontainers.image.created' with ISO 8601 date.",
            "DL3056" => "Add 'LABEL org.opencontainers.image.description'.",
            "DL3057" => "Add a HEALTHCHECK instruction for container health monitoring.",
            "DL3058" => "Add 'LABEL org.opencontainers.image.title'.",
            "DL3059" => "Combine consecutive RUN instructions with '&&' to reduce layers.",
            "DL3060" => "Pin package versions in yarn add: 'yarn add package@version'.",
            "DL3061" => "Use specific image digest or tag instead of implicit latest.",
            "DL3062" => "Prefer single RUN with '&&' over multiple RUN for related commands.",
            "DL4000" => "Replace MAINTAINER with 'LABEL maintainer=\"name <email>\"'.",
            "DL4001" => "Use wget or curl instead of ADD for downloading from URLs.",
            "DL4003" => "Use 'ENTRYPOINT' and 'CMD' together properly for container startup.",
            "DL4005" => "Prefer JSON notation for SHELL: SHELL [\"/bin/bash\", \"-c\"].",
            "DL4006" => "Add 'SHELL [\"/bin/bash\", \"-o\", \"pipefail\", \"-c\"]' before RUN with pipes.",
            _ if code.starts_with("SC") => "See ShellCheck wiki for shell scripting fix.",
            _ => "Review the rule documentation for specific guidance.",
        }
    }

    /// Get documentation URL for a rule
    fn get_rule_url(code: &str) -> String {
        if code.starts_with("DL") || code.starts_with("SC") {
            if code.starts_with("SC") {
                format!("https://www.shellcheck.net/wiki/{}", code)
            } else {
                format!("https://github.com/hadolint/hadolint/wiki/{}", code)
            }
        } else {
            String::new()
        }
    }

    /// Format result optimized for agent decision-making
    fn format_result(result: &LintResult, filename: &str) -> String {
        // Categorize and enrich failures
        let enriched_failures: Vec<serde_json::Value> = result.failures.iter().map(|f| {
            let code = f.code.as_str();
            let category = Self::get_rule_category(code);
            let priority = Self::get_priority(f.severity, category);

            json!({
                "code": code,
                "severity": format!("{:?}", f.severity).to_lowercase(),
                "priority": priority,
                "category": category,
                "message": f.message,
                "line": f.line,
                "column": f.column,
                "fix": Self::get_fix_recommendation(code),
                "docs": Self::get_rule_url(code),
            })
        }).collect();

        // Group by priority for agent decision ordering
        let critical: Vec<_> = enriched_failures.iter()
            .filter(|f| f["priority"] == "critical")
            .cloned().collect();
        let high: Vec<_> = enriched_failures.iter()
            .filter(|f| f["priority"] == "high")
            .cloned().collect();
        let medium: Vec<_> = enriched_failures.iter()
            .filter(|f| f["priority"] == "medium")
            .cloned().collect();
        let low: Vec<_> = enriched_failures.iter()
            .filter(|f| f["priority"] == "low")
            .cloned().collect();

        // Group by category for thematic fixes
        let mut by_category: std::collections::HashMap<&str, Vec<_>> = std::collections::HashMap::new();
        for f in &enriched_failures {
            let cat = f["category"].as_str().unwrap_or("other");
            by_category.entry(cat).or_default().push(f.clone());
        }

        // Build decision context
        let decision_context = if critical.is_empty() && high.is_empty() {
            if medium.is_empty() && low.is_empty() {
                "Dockerfile follows best practices. No issues found."
            } else if medium.is_empty() {
                "Minor improvements possible. Low priority issues only."
            } else {
                "Good baseline. Medium priority improvements recommended."
            }
        } else if !critical.is_empty() {
            "Critical issues found. Address security/error issues first before deployment."
        } else {
            "High priority issues found. Review and fix before production use."
        };

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
                    "errors": result.failures.iter().filter(|f| f.severity == Severity::Error).count(),
                    "warnings": result.failures.iter().filter(|f| f.severity == Severity::Warning).count(),
                    "info": result.failures.iter().filter(|f| f.severity == Severity::Info).count(),
                },
                "by_category": by_category.iter().map(|(k, v)| (k.to_string(), v.len())).collect::<std::collections::HashMap<_, _>>(),
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
            let quick_fixes: Vec<String> = enriched_failures.iter()
                .filter(|f| f["priority"] == "critical" || f["priority"] == "high")
                .take(5)
                .map(|f| format!("Line {}: {} - {}",
                    f["line"],
                    f["code"].as_str().unwrap_or(""),
                    f["fix"].as_str().unwrap_or("")
                ))
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

impl Tool for HadolintTool {
    const NAME: &'static str = "hadolint";

    type Error = HadolintError;
    type Args = HadolintArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Lint Dockerfiles for best practices, security issues, and common mistakes. \
                Returns AI-optimized JSON with issues categorized by priority (critical/high/medium/low) \
                and type (security/best-practice/maintainability/performance/deprecated). \
                Each issue includes an actionable fix recommendation. Use this to analyze Dockerfiles \
                before deployment or to improve existing ones. The 'decision_context' field provides \
                a summary for quick assessment, and 'quick_fixes' lists the most important changes."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "dockerfile": {
                        "type": "string",
                        "description": "Path to Dockerfile relative to project root (e.g., 'Dockerfile', 'docker/Dockerfile.prod')"
                    },
                    "content": {
                        "type": "string",
                        "description": "Inline Dockerfile content to lint. Use this when you want to validate generated Dockerfile content before writing."
                    },
                    "ignore": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of rule codes to ignore (e.g., ['DL3008', 'DL3013'])"
                    },
                    "threshold": {
                        "type": "string",
                        "enum": ["error", "warning", "info", "style"],
                        "description": "Minimum severity to report. Default is 'warning'."
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Build configuration
        let mut config = HadolintConfig::default();

        // Apply ignored rules
        for rule in &args.ignore {
            config = config.ignore(rule.as_str());
        }

        // Apply threshold
        if let Some(threshold) = &args.threshold {
            config = config.with_threshold(Self::parse_threshold(threshold));
        }

        // Determine source, filename, and lint
        let (result, filename) = if let Some(content) = &args.content {
            // Lint inline content
            (lint(content, &config), "<inline>".to_string())
        } else if let Some(dockerfile) = &args.dockerfile {
            // Lint file
            let path = self.project_path.join(dockerfile);
            (lint_file(&path, &config), dockerfile.clone())
        } else {
            // Default: look for Dockerfile in project root
            let path = self.project_path.join("Dockerfile");
            if path.exists() {
                (lint_file(&path, &config), "Dockerfile".to_string())
            } else {
                return Err(HadolintError(
                    "No Dockerfile specified and no Dockerfile found in project root".to_string(),
                ));
            }
        };

        // Check for parse errors
        if !result.parse_errors.is_empty() {
            log::warn!("Dockerfile parse errors: {:?}", result.parse_errors);
        }

        Ok(Self::format_result(&result, &filename))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::fs;

    /// Helper to collect all issues from action_plan
    fn collect_all_issues(parsed: &serde_json::Value) -> Vec<serde_json::Value> {
        let mut all = Vec::new();
        for priority in ["critical", "high", "medium", "low"] {
            if let Some(arr) = parsed["action_plan"][priority].as_array() {
                all.extend(arr.clone());
            }
        }
        all
    }

    #[tokio::test]
    async fn test_hadolint_inline_content() {
        let tool = HadolintTool::new(temp_dir());
        let args = HadolintArgs {
            dockerfile: None,
            content: Some("FROM ubuntu:latest\nRUN sudo apt-get update".to_string()),
            ignore: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Should detect DL3007 (latest tag) and DL3004 (sudo)
        assert!(!parsed["success"].as_bool().unwrap_or(true));
        assert!(parsed["summary"]["total"].as_u64().unwrap_or(0) >= 2);

        // Check new fields exist
        assert!(parsed["decision_context"].is_string());
        assert!(parsed["action_plan"].is_object());

        // Check issues have fix recommendations
        let issues = collect_all_issues(&parsed);
        assert!(issues.iter().all(|i| i["fix"].is_string() && !i["fix"].as_str().unwrap().is_empty()));
    }

    #[tokio::test]
    async fn test_hadolint_ignore_rules() {
        let tool = HadolintTool::new(temp_dir());
        let args = HadolintArgs {
            dockerfile: None,
            content: Some("FROM ubuntu:latest".to_string()),
            ignore: vec!["DL3007".to_string()],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // DL3007 should be ignored
        let all_issues = collect_all_issues(&parsed);
        assert!(!all_issues.iter().any(|f| f["code"] == "DL3007"));
    }

    #[tokio::test]
    async fn test_hadolint_threshold() {
        let tool = HadolintTool::new(temp_dir());
        let args = HadolintArgs {
            dockerfile: None,
            content: Some("FROM ubuntu\nMAINTAINER test".to_string()),
            ignore: vec![],
            threshold: Some("error".to_string()),
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // DL4000 (MAINTAINER deprecated) is Error, DL3006 (untagged) is Warning
        // With error threshold, only errors should show
        let all_issues = collect_all_issues(&parsed);
        assert!(all_issues.iter().all(|f| f["severity"] == "error"));
    }

    #[tokio::test]
    async fn test_hadolint_file() {
        let temp = temp_dir().join("hadolint_test");
        fs::create_dir_all(&temp).unwrap();
        let dockerfile = temp.join("Dockerfile");
        fs::write(&dockerfile, "FROM node:18-alpine\nWORKDIR /app\nCOPY . .\nCMD [\"node\", \"app.js\"]").unwrap();

        let tool = HadolintTool::new(temp.clone());
        let args = HadolintArgs {
            dockerfile: Some("Dockerfile".to_string()),
            content: None,
            ignore: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // This is a well-formed Dockerfile, should have few/no errors
        assert!(parsed["success"].as_bool().unwrap_or(false));
        assert_eq!(parsed["file"], "Dockerfile");

        // Cleanup
        fs::remove_dir_all(&temp).ok();
    }

    #[tokio::test]
    async fn test_hadolint_valid_dockerfile() {
        let tool = HadolintTool::new(temp_dir());
        let dockerfile = r#"
FROM node:18-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production
COPY . .
RUN npm run build

FROM node:18-alpine
WORKDIR /app
COPY --from=builder /app/dist ./dist
USER node
EXPOSE 3000
CMD ["node", "dist/index.js"]
"#;

        let args = HadolintArgs {
            dockerfile: None,
            content: Some(dockerfile.to_string()),
            ignore: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Well-structured Dockerfile should pass (no errors)
        assert!(parsed["success"].as_bool().unwrap_or(false));
        // Should have decision context
        assert!(parsed["decision_context"].is_string());
        // Should not have critical or high priority issues
        assert_eq!(parsed["summary"]["by_priority"]["critical"].as_u64().unwrap_or(99), 0);
        assert_eq!(parsed["summary"]["by_priority"]["high"].as_u64().unwrap_or(99), 0);
    }

    #[tokio::test]
    async fn test_hadolint_priority_categorization() {
        let tool = HadolintTool::new(temp_dir());
        let args = HadolintArgs {
            dockerfile: None,
            content: Some("FROM ubuntu\nRUN sudo apt-get update\nMAINTAINER test".to_string()),
            ignore: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Check priority counts are present
        assert!(parsed["summary"]["by_priority"]["critical"].is_number());
        assert!(parsed["summary"]["by_priority"]["high"].is_number());
        assert!(parsed["summary"]["by_priority"]["medium"].is_number());

        // Check category counts
        assert!(parsed["summary"]["by_category"].is_object());

        // DL3004 (sudo) should be high priority security
        let all_issues = collect_all_issues(&parsed);
        let sudo_issue = all_issues.iter().find(|i| i["code"] == "DL3004");
        assert!(sudo_issue.is_some());
        assert_eq!(sudo_issue.unwrap()["category"], "security");
    }

    #[tokio::test]
    async fn test_hadolint_quick_fixes() {
        let tool = HadolintTool::new(temp_dir());
        let args = HadolintArgs {
            dockerfile: None,
            content: Some("FROM ubuntu\nRUN sudo rm -rf /".to_string()),
            ignore: vec![],
            threshold: None,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Should have quick_fixes for high priority issues
        if parsed["summary"]["by_priority"]["high"].as_u64().unwrap_or(0) > 0
            || parsed["summary"]["by_priority"]["critical"].as_u64().unwrap_or(0) > 0 {
            assert!(parsed["quick_fixes"].is_array());
        }
    }
}
