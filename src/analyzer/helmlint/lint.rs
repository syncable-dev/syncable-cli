//! Main linting orchestration for helmlint.
//!
//! This module ties together parsing, rules, and pragmas to provide
//! the main linting API.

use std::collections::HashSet;
use std::path::Path;

use crate::analyzer::helmlint::config::HelmlintConfig;
use crate::analyzer::helmlint::parser::chart::parse_chart_yaml;
use crate::analyzer::helmlint::parser::helpers::{parse_helpers, ParsedHelpers};
use crate::analyzer::helmlint::parser::template::parse_template;
use crate::analyzer::helmlint::parser::values::parse_values_yaml;
use crate::analyzer::helmlint::pragma::{extract_template_pragmas, extract_yaml_pragmas, PragmaState};
use crate::analyzer::helmlint::rules::{all_rules, LintContext};
use crate::analyzer::helmlint::types::{CheckFailure, Severity};

/// Result of linting a Helm chart.
#[derive(Debug, Clone)]
pub struct LintResult {
    /// Path to the chart root.
    pub chart_path: String,
    /// Rule violations found.
    pub failures: Vec<CheckFailure>,
    /// Parse errors (if any).
    pub parse_errors: Vec<String>,
    /// Number of files checked.
    pub files_checked: usize,
    /// Number of errors.
    pub error_count: usize,
    /// Number of warnings.
    pub warning_count: usize,
}

impl LintResult {
    /// Create a new empty result.
    pub fn new(chart_path: impl Into<String>) -> Self {
        Self {
            chart_path: chart_path.into(),
            failures: Vec::new(),
            parse_errors: Vec::new(),
            files_checked: 0,
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Update counts based on failures.
    fn update_counts(&mut self) {
        self.error_count = self
            .failures
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count();
        self.warning_count = self
            .failures
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count();
    }

    /// Check if there are any failures.
    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Check if there are any warnings.
    pub fn has_warnings(&self) -> bool {
        self.warning_count > 0
    }

    /// Get the maximum severity in the results.
    pub fn max_severity(&self) -> Option<Severity> {
        self.failures.iter().map(|f| f.severity).max()
    }

    /// Check if the results should cause a non-zero exit.
    pub fn should_fail(&self, config: &HelmlintConfig) -> bool {
        if config.no_fail {
            return false;
        }

        if let Some(max) = self.max_severity() {
            max >= config.failure_threshold
        } else {
            false
        }
    }

    /// Sort failures by file and line number.
    pub fn sort(&mut self) {
        self.failures.sort();
    }
}

/// Lint a Helm chart directory.
pub fn lint_chart(path: &Path, config: &HelmlintConfig) -> LintResult {
    let chart_path_str = path.display().to_string();
    let mut result = LintResult::new(&chart_path_str);

    // Validate path
    if !path.exists() {
        result
            .parse_errors
            .push(format!("Chart path does not exist: {}", chart_path_str));
        return result;
    }

    if !path.is_dir() {
        result
            .parse_errors
            .push(format!("Chart path is not a directory: {}", chart_path_str));
        return result;
    }

    // Collect all files
    let files = collect_chart_files(path);
    result.files_checked = files.len();

    // Parse Chart.yaml
    let chart_yaml_path = path.join("Chart.yaml");
    let chart_metadata = if chart_yaml_path.exists() {
        match std::fs::read_to_string(&chart_yaml_path) {
            Ok(content) => match parse_chart_yaml(&content) {
                Ok(metadata) => Some(metadata),
                Err(e) => {
                    result.parse_errors.push(format!("Chart.yaml: {}", e));
                    None
                }
            },
            Err(e) => {
                result.parse_errors.push(format!("Failed to read Chart.yaml: {}", e));
                None
            }
        }
    } else {
        None
    };

    // Parse values.yaml
    let values_yaml_path = path.join("values.yaml");
    let values = if values_yaml_path.exists() {
        match std::fs::read_to_string(&values_yaml_path) {
            Ok(content) => match parse_values_yaml(&content) {
                Ok(v) => Some(v),
                Err(e) => {
                    result.parse_errors.push(format!("values.yaml: {}", e));
                    None
                }
            },
            Err(e) => {
                result.parse_errors.push(format!("Failed to read values.yaml: {}", e));
                None
            }
        }
    } else {
        None
    };

    // Parse templates
    let templates_dir = path.join("templates");
    let mut templates = Vec::new();
    let mut helpers: Option<ParsedHelpers> = None;

    if templates_dir.exists() && templates_dir.is_dir() {
        for entry in walkdir::WalkDir::new(&templates_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            if file_path.is_file() {
                let relative_path = file_path
                    .strip_prefix(path)
                    .unwrap_or(file_path)
                    .display()
                    .to_string();

                // Skip excluded files
                if config.is_excluded(&relative_path) {
                    continue;
                }

                let extension = file_path.extension().and_then(|e| e.to_str());
                match extension {
                    Some("yaml") | Some("yml") | Some("tpl") | Some("txt") => {
                        match std::fs::read_to_string(file_path) {
                            Ok(content) => {
                                let parsed = parse_template(&content, &relative_path);

                                // Check if this is the helpers file
                                if relative_path.contains("_helpers") {
                                    helpers = Some(parse_helpers(&content, &relative_path));
                                }

                                templates.push(parsed);
                            }
                            Err(e) => {
                                result.parse_errors.push(format!(
                                    "Failed to read {}: {}",
                                    relative_path, e
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Collect pragmas from all files
    let mut all_pragmas = PragmaState::new();

    // Chart.yaml pragmas
    if let Ok(content) = std::fs::read_to_string(&chart_yaml_path) {
        let pragmas = extract_yaml_pragmas(&content);
        merge_pragmas(&mut all_pragmas, pragmas);
    }

    // values.yaml pragmas
    if let Ok(content) = std::fs::read_to_string(&values_yaml_path) {
        let pragmas = extract_yaml_pragmas(&content);
        merge_pragmas(&mut all_pragmas, pragmas);
    }

    // Template pragmas
    for template in &templates {
        let content = template
            .tokens
            .iter()
            .map(|t| t.content())
            .collect::<Vec<_>>()
            .join("");
        let pragmas = extract_template_pragmas(&content);
        merge_pragmas(&mut all_pragmas, pragmas);
    }

    // Build lint context
    let ctx = LintContext::new(
        path,
        chart_metadata.as_ref(),
        values.as_ref(),
        helpers.as_ref(),
        &templates,
        &files,
    );

    // Run all rules
    let rules = all_rules();
    let mut all_failures = Vec::new();

    for rule in rules {
        // Skip ignored rules
        if config.is_rule_ignored(rule.code()) {
            continue;
        }

        let failures = rule.check(&ctx);
        all_failures.extend(failures);
    }

    // Filter by config and pragmas
    result.failures = all_failures
        .into_iter()
        .filter(|f| {
            // Apply config severity overrides
            let effective_severity = config.effective_severity(f.code.as_str(), f.severity);
            config.should_report(effective_severity)
        })
        .filter(|f| !config.is_rule_ignored(f.code.as_str()))
        .filter(|f| {
            if config.disable_ignore_pragma {
                true
            } else {
                !all_pragmas.is_ignored(&f.code, f.line)
            }
        })
        .filter(|f| {
            if config.fixable_only {
                f.fixable
            } else {
                true
            }
        })
        .map(|mut f| {
            // Apply severity overrides
            f.severity = config.effective_severity(f.code.as_str(), f.severity);
            f
        })
        .collect();

    // Sort and update counts
    result.sort();
    result.update_counts();

    result
}

/// Lint a single Helm chart file (Chart.yaml only).
pub fn lint_chart_file(path: &Path, config: &HelmlintConfig) -> LintResult {
    // Find chart root from the file
    let chart_root = path.parent().unwrap_or(path);
    lint_chart(chart_root, config)
}

/// Collect all files in the chart directory.
fn collect_chart_files(path: &Path) -> HashSet<String> {
    let mut files = HashSet::new();

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().is_file() {
            if let Ok(relative) = entry.path().strip_prefix(path) {
                files.insert(relative.display().to_string());
            }
        }
    }

    files
}

/// Merge pragmas from one state into another.
fn merge_pragmas(target: &mut PragmaState, source: PragmaState) {
    if source.file_disabled {
        target.file_disabled = true;
    }

    for code in source.file_ignores {
        target.file_ignores.insert(code);
    }

    for (line, codes) in source.line_ignores {
        target.line_ignores.entry(line).or_default().extend(codes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_chart(dir: &Path) {
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

    #[test]
    fn test_lint_valid_chart() {
        let temp_dir = TempDir::new().unwrap();
        create_test_chart(temp_dir.path());

        let config = HelmlintConfig::default();
        let result = lint_chart(temp_dir.path(), &config);

        assert!(result.parse_errors.is_empty());
    }

    #[test]
    fn test_lint_nonexistent_path() {
        let config = HelmlintConfig::default();
        let result = lint_chart(Path::new("/nonexistent/path"), &config);

        assert!(!result.parse_errors.is_empty());
    }

    #[test]
    fn test_lint_with_ignored_rules() {
        let temp_dir = TempDir::new().unwrap();
        create_test_chart(temp_dir.path());

        let config = HelmlintConfig::default()
            .ignore("HL1007")  // Missing maintainers
            .ignore("HL5001"); // Missing resource limits

        let result = lint_chart(temp_dir.path(), &config);

        assert!(!result.failures.iter().any(|f| f.code.as_str() == "HL1007"));
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "HL5001"));
    }

    #[test]
    fn test_result_counts() {
        let mut result = LintResult::new("test");
        result.failures.push(CheckFailure::new(
            "HL1001",
            Severity::Error,
            "test",
            "Chart.yaml",
            1,
            crate::analyzer::helmlint::types::RuleCategory::Structure,
        ));
        result.failures.push(CheckFailure::new(
            "HL1002",
            Severity::Warning,
            "test",
            "Chart.yaml",
            2,
            crate::analyzer::helmlint::types::RuleCategory::Structure,
        ));
        result.update_counts();

        assert_eq!(result.error_count, 1);
        assert_eq!(result.warning_count, 1);
        assert!(result.has_errors());
        assert!(result.has_warnings());
    }
}
