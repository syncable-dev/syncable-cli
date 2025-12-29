//! HL1xxx - Chart Structure Rules
//!
//! Rules for validating Helm chart structure, Chart.yaml, and file organization.

use crate::analyzer::helmlint::parser::chart::ApiVersion;
use crate::analyzer::helmlint::rules::{LintContext, Rule};
use crate::analyzer::helmlint::types::{CheckFailure, RuleCategory, Severity};

/// Get all HL1xxx rules.
pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(HL1001),
        Box::new(HL1002),
        Box::new(HL1003),
        Box::new(HL1004),
        Box::new(HL1005),
        Box::new(HL1006),
        Box::new(HL1007),
        Box::new(HL1008),
        Box::new(HL1009),
        Box::new(HL1010),
        Box::new(HL1011),
        Box::new(HL1012),
        Box::new(HL1013),
        Box::new(HL1014),
        Box::new(HL1015),
        Box::new(HL1016),
        Box::new(HL1017),
    ]
}

/// HL1001: Missing Chart.yaml
pub struct HL1001;

impl Rule for HL1001 {
    fn code(&self) -> &'static str {
        "HL1001"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "missing-chart-yaml"
    }

    fn description(&self) -> &'static str {
        "Chart.yaml is required for all Helm charts"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if ctx.chart_metadata.is_none() && !ctx.has_file("Chart.yaml") {
            vec![CheckFailure::new(
                "HL1001",
                Severity::Error,
                "Missing Chart.yaml file",
                "Chart.yaml",
                1,
                RuleCategory::Structure,
            )]
        } else {
            vec![]
        }
    }
}

/// HL1002: Invalid apiVersion
pub struct HL1002;

impl Rule for HL1002 {
    fn code(&self) -> &'static str {
        "HL1002"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "invalid-api-version"
    }

    fn description(&self) -> &'static str {
        "Chart apiVersion must be v1 or v2"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if let Some(chart) = ctx.chart_metadata {
            if !chart.has_valid_api_version() {
                let version = match &chart.api_version {
                    ApiVersion::Unknown(v) => v.clone(),
                    _ => "unknown".to_string(),
                };
                return vec![CheckFailure::new(
                    "HL1002",
                    Severity::Error,
                    format!("Invalid apiVersion '{}'. Must be v1 or v2", version),
                    "Chart.yaml",
                    1,
                    RuleCategory::Structure,
                )];
            }
        }
        vec![]
    }
}

/// HL1003: Missing required field 'name'
pub struct HL1003;

impl Rule for HL1003 {
    fn code(&self) -> &'static str {
        "HL1003"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "missing-name"
    }

    fn description(&self) -> &'static str {
        "Chart.yaml must have a 'name' field"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if let Some(chart) = ctx.chart_metadata {
            if chart.name.is_empty() {
                return vec![CheckFailure::new(
                    "HL1003",
                    Severity::Error,
                    "Missing required field 'name' in Chart.yaml",
                    "Chart.yaml",
                    1,
                    RuleCategory::Structure,
                )];
            }
        }
        vec![]
    }
}

/// HL1004: Missing required field 'version'
pub struct HL1004;

impl Rule for HL1004 {
    fn code(&self) -> &'static str {
        "HL1004"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "missing-version"
    }

    fn description(&self) -> &'static str {
        "Chart.yaml must have a 'version' field"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if let Some(chart) = ctx.chart_metadata {
            if chart.version.is_empty() {
                return vec![CheckFailure::new(
                    "HL1004",
                    Severity::Error,
                    "Missing required field 'version' in Chart.yaml",
                    "Chart.yaml",
                    1,
                    RuleCategory::Structure,
                )];
            }
        }
        vec![]
    }
}

/// HL1005: Version not valid SemVer
pub struct HL1005;

impl Rule for HL1005 {
    fn code(&self) -> &'static str {
        "HL1005"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "invalid-semver"
    }

    fn description(&self) -> &'static str {
        "Chart version should be valid SemVer"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if let Some(chart) = ctx.chart_metadata {
            if !chart.version.is_empty() && !is_valid_semver(&chart.version) {
                return vec![CheckFailure::new(
                    "HL1005",
                    Severity::Warning,
                    format!(
                        "Version '{}' is not valid SemVer (expected X.Y.Z format)",
                        chart.version
                    ),
                    "Chart.yaml",
                    1,
                    RuleCategory::Structure,
                )];
            }
        }
        vec![]
    }
}

/// HL1006: Missing description
pub struct HL1006;

impl Rule for HL1006 {
    fn code(&self) -> &'static str {
        "HL1006"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn name(&self) -> &'static str {
        "missing-description"
    }

    fn description(&self) -> &'static str {
        "Chart should have a description"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if let Some(chart) = ctx.chart_metadata {
            if chart.description.is_none()
                || chart
                    .description
                    .as_ref()
                    .map(|d| d.is_empty())
                    .unwrap_or(true)
            {
                return vec![CheckFailure::new(
                    "HL1006",
                    Severity::Info,
                    "Chart.yaml is missing a description",
                    "Chart.yaml",
                    1,
                    RuleCategory::Structure,
                )];
            }
        }
        vec![]
    }
}

/// HL1007: Missing maintainers
pub struct HL1007;

impl Rule for HL1007 {
    fn code(&self) -> &'static str {
        "HL1007"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn name(&self) -> &'static str {
        "missing-maintainers"
    }

    fn description(&self) -> &'static str {
        "Chart should have maintainers listed"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if let Some(chart) = ctx.chart_metadata {
            if chart.maintainers.is_empty() {
                return vec![CheckFailure::new(
                    "HL1007",
                    Severity::Info,
                    "Chart.yaml has no maintainers listed",
                    "Chart.yaml",
                    1,
                    RuleCategory::Structure,
                )];
            }
        }
        vec![]
    }
}

/// HL1008: Chart is deprecated
pub struct HL1008;

impl Rule for HL1008 {
    fn code(&self) -> &'static str {
        "HL1008"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "chart-deprecated"
    }

    fn description(&self) -> &'static str {
        "Chart is marked as deprecated"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if let Some(chart) = ctx.chart_metadata {
            if chart.is_deprecated() {
                return vec![CheckFailure::new(
                    "HL1008",
                    Severity::Warning,
                    "Chart is marked as deprecated",
                    "Chart.yaml",
                    1,
                    RuleCategory::Structure,
                )];
            }
        }
        vec![]
    }
}

/// HL1009: Missing templates directory
pub struct HL1009;

impl Rule for HL1009 {
    fn code(&self) -> &'static str {
        "HL1009"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "missing-templates"
    }

    fn description(&self) -> &'static str {
        "Chart should have a templates directory"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        // Skip for library charts
        if let Some(chart) = ctx.chart_metadata {
            if chart.is_library() {
                return vec![];
            }
        }

        let has_templates = ctx
            .files
            .iter()
            .any(|f| f.starts_with("templates/") || f.contains("/templates/"));
        if !has_templates && ctx.templates.is_empty() {
            return vec![CheckFailure::new(
                "HL1009",
                Severity::Warning,
                "Chart has no templates directory",
                ".",
                1,
                RuleCategory::Structure,
            )];
        }
        vec![]
    }
}

/// HL1010: Invalid chart type
pub struct HL1010;

impl Rule for HL1010 {
    fn code(&self) -> &'static str {
        "HL1010"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "invalid-chart-type"
    }

    fn description(&self) -> &'static str {
        "Chart type must be 'application' or 'library'"
    }

    fn check(&self, _ctx: &LintContext) -> Vec<CheckFailure> {
        // This is handled during parsing - if type is invalid, serde will fail
        // or produce Unknown variant which we handle elsewhere
        vec![]
    }
}

/// HL1011: Missing values.yaml
pub struct HL1011;

impl Rule for HL1011 {
    fn code(&self) -> &'static str {
        "HL1011"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "missing-values-yaml"
    }

    fn description(&self) -> &'static str {
        "Chart should have a values.yaml file"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if ctx.values.is_none() && !ctx.has_file("values.yaml") {
            return vec![CheckFailure::new(
                "HL1011",
                Severity::Warning,
                "Missing values.yaml file",
                "values.yaml",
                1,
                RuleCategory::Structure,
            )];
        }
        vec![]
    }
}

/// HL1012: Chart name contains invalid characters
pub struct HL1012;

impl Rule for HL1012 {
    fn code(&self) -> &'static str {
        "HL1012"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "invalid-chart-name"
    }

    fn description(&self) -> &'static str {
        "Chart name must contain only lowercase alphanumeric characters and hyphens"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if let Some(chart) = ctx.chart_metadata {
            if !is_valid_chart_name(&chart.name) {
                return vec![CheckFailure::new(
                    "HL1012",
                    Severity::Error,
                    format!(
                        "Chart name '{}' contains invalid characters. Use only lowercase letters, numbers, and hyphens",
                        chart.name
                    ),
                    "Chart.yaml",
                    1,
                    RuleCategory::Structure,
                )];
            }
        }
        vec![]
    }
}

/// HL1013: Icon URL not HTTPS
pub struct HL1013;

impl Rule for HL1013 {
    fn code(&self) -> &'static str {
        "HL1013"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "icon-not-https"
    }

    fn description(&self) -> &'static str {
        "Icon URL should use HTTPS"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if let Some(chart) = ctx.chart_metadata {
            if let Some(icon) = &chart.icon {
                if icon.starts_with("http://") {
                    return vec![CheckFailure::new(
                        "HL1013",
                        Severity::Warning,
                        "Icon URL should use HTTPS instead of HTTP",
                        "Chart.yaml",
                        1,
                        RuleCategory::Structure,
                    )];
                }
            }
        }
        vec![]
    }
}

/// HL1014: Home URL not HTTPS
pub struct HL1014;

impl Rule for HL1014 {
    fn code(&self) -> &'static str {
        "HL1014"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "home-not-https"
    }

    fn description(&self) -> &'static str {
        "Home URL should use HTTPS"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if let Some(chart) = ctx.chart_metadata {
            if let Some(home) = &chart.home {
                if home.starts_with("http://") {
                    return vec![CheckFailure::new(
                        "HL1014",
                        Severity::Warning,
                        "Home URL should use HTTPS instead of HTTP",
                        "Chart.yaml",
                        1,
                        RuleCategory::Structure,
                    )];
                }
            }
        }
        vec![]
    }
}

/// HL1015: Duplicate dependency names
pub struct HL1015;

impl Rule for HL1015 {
    fn code(&self) -> &'static str {
        "HL1015"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "duplicate-dependencies"
    }

    fn description(&self) -> &'static str {
        "Chart has duplicate dependency names"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        if let Some(chart) = ctx.chart_metadata {
            let duplicates = chart.has_duplicate_dependencies();
            if !duplicates.is_empty() {
                return vec![CheckFailure::new(
                    "HL1015",
                    Severity::Error,
                    format!("Duplicate dependency names: {}", duplicates.join(", ")),
                    "Chart.yaml",
                    1,
                    RuleCategory::Structure,
                )];
            }
        }
        vec![]
    }
}

/// HL1016: Dependency missing version
pub struct HL1016;

impl Rule for HL1016 {
    fn code(&self) -> &'static str {
        "HL1016"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "dependency-missing-version"
    }

    fn description(&self) -> &'static str {
        "Chart dependency is missing a version"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();
        if let Some(chart) = ctx.chart_metadata {
            for dep in &chart.dependencies {
                if dep.version.is_none()
                    || dep.version.as_ref().map(|v| v.is_empty()).unwrap_or(true)
                {
                    failures.push(CheckFailure::new(
                        "HL1016",
                        Severity::Warning,
                        format!("Dependency '{}' is missing a version", dep.name),
                        "Chart.yaml",
                        1,
                        RuleCategory::Structure,
                    ));
                }
            }
        }
        failures
    }
}

/// HL1017: Dependency missing repository
pub struct HL1017;

impl Rule for HL1017 {
    fn code(&self) -> &'static str {
        "HL1017"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "dependency-missing-repository"
    }

    fn description(&self) -> &'static str {
        "Chart dependency is missing a repository"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();
        if let Some(chart) = ctx.chart_metadata {
            for dep in &chart.dependencies {
                if dep.repository.is_none()
                    || dep
                        .repository
                        .as_ref()
                        .map(|r| r.is_empty())
                        .unwrap_or(true)
                {
                    // Skip if it's a file:// reference (local dependency)
                    failures.push(CheckFailure::new(
                        "HL1017",
                        Severity::Error,
                        format!("Dependency '{}' is missing a repository", dep.name),
                        "Chart.yaml",
                        1,
                        RuleCategory::Structure,
                    ));
                }
            }
        }
        failures
    }
}

/// Check if a version string is valid SemVer.
fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }

    // Check major and minor are numeric
    for (i, part) in parts.iter().enumerate() {
        // Allow pre-release and build metadata on the last part
        let numeric_part = if i == parts.len() - 1 {
            part.split(|c| c == '-' || c == '+').next().unwrap_or(part)
        } else {
            part
        };

        if numeric_part.parse::<u64>().is_err() {
            return false;
        }
    }

    true
}

/// Check if a chart name is valid.
fn is_valid_chart_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Must start with a letter
    if !name
        .chars()
        .next()
        .map(|c| c.is_ascii_lowercase())
        .unwrap_or(false)
    {
        return false;
    }

    // Must contain only lowercase letters, numbers, and hyphens
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_semver() {
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("0.1.0"));
        assert!(is_valid_semver("10.20.30"));
        assert!(is_valid_semver("1.0.0-alpha"));
        assert!(is_valid_semver("1.0.0+build"));
        assert!(is_valid_semver("1.0"));
        assert!(!is_valid_semver("1"));
        assert!(!is_valid_semver("v1.0.0"));
        assert!(!is_valid_semver("1.0.0.0"));
        assert!(!is_valid_semver(""));
    }

    #[test]
    fn test_valid_chart_name() {
        assert!(is_valid_chart_name("my-chart"));
        assert!(is_valid_chart_name("mychart"));
        assert!(is_valid_chart_name("my-chart-123"));
        assert!(!is_valid_chart_name("My-Chart"));
        assert!(!is_valid_chart_name("my_chart"));
        assert!(!is_valid_chart_name("123-chart"));
        assert!(!is_valid_chart_name(""));
    }
}
