//! HL2xxx - Values Validation Rules
//!
//! Rules for validating values.yaml configuration.

use crate::analyzer::helmlint::rules::{LintContext, Rule};
use crate::analyzer::helmlint::types::{CheckFailure, RuleCategory, Severity};

/// Get all HL2xxx rules.
pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(HL2002),
        Box::new(HL2003),
        Box::new(HL2004),
        Box::new(HL2005),
        Box::new(HL2007),
        Box::new(HL2008),
    ]
}

/// HL2002: Value referenced in template but not defined
pub struct HL2002;

impl Rule for HL2002 {
    fn code(&self) -> &'static str {
        "HL2002"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "undefined-value"
    }

    fn description(&self) -> &'static str {
        "Value is referenced in template but not defined in values.yaml"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        // Skip if no values file
        let values = match ctx.values {
            Some(v) => v,
            None => return failures,
        };

        // Check each template reference
        for ref_path in &ctx.template_value_refs {
            // Check if base path exists (allow nested access to undefined)
            let base_path = ref_path.split('.').next().unwrap_or(ref_path);
            if !values.has_path(base_path) && !values.has_path(ref_path) {
                // Check if any parent path exists
                let mut found_parent = false;
                let parts: Vec<&str> = ref_path.split('.').collect();
                for i in 1..parts.len() {
                    let partial = parts[..i].join(".");
                    if values.has_path(&partial) {
                        found_parent = true;
                        break;
                    }
                }

                if !found_parent {
                    failures.push(CheckFailure::new(
                        "HL2002",
                        Severity::Warning,
                        format!(
                            "Value '.Values.{}' is referenced but not defined in values.yaml",
                            ref_path
                        ),
                        "values.yaml",
                        1,
                        RuleCategory::Values,
                    ));
                }
            }
        }

        failures
    }
}

/// HL2003: Value defined but never used
pub struct HL2003;

impl Rule for HL2003 {
    fn code(&self) -> &'static str {
        "HL2003"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn name(&self) -> &'static str {
        "unused-value"
    }

    fn description(&self) -> &'static str {
        "Value is defined in values.yaml but never used in templates"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        let values = match ctx.values {
            Some(v) => v,
            None => return failures,
        };

        // Check each defined value
        for path in &values.defined_paths {
            // Skip if any template references this path or a child path
            let is_used = ctx
                .template_value_refs
                .iter()
                .any(|ref_path| ref_path == path || ref_path.starts_with(&format!("{}.", path)));

            // Also skip if a parent path is referenced (e.g., toYaml .Values.config)
            let parent_is_used = ctx
                .template_value_refs
                .iter()
                .any(|ref_path| path.starts_with(&format!("{}.", ref_path)));

            if !is_used && !parent_is_used {
                let line = values.line_for_path(path).unwrap_or(1);
                failures.push(CheckFailure::new(
                    "HL2003",
                    Severity::Info,
                    format!("Value '{}' is defined but never used in templates", path),
                    "values.yaml",
                    line,
                    RuleCategory::Values,
                ));
            }
        }

        failures
    }
}

/// HL2004: Sensitive value not marked as secret
pub struct HL2004;

impl Rule for HL2004 {
    fn code(&self) -> &'static str {
        "HL2004"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "sensitive-value-exposed"
    }

    fn description(&self) -> &'static str {
        "Sensitive value should be handled as a Kubernetes Secret"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        let values = match ctx.values {
            Some(v) => v,
            None => return failures,
        };

        for path in values.sensitive_paths() {
            // Check if the value has a non-empty default
            if let Some(value) = values.get(path) {
                let has_hardcoded_value = match value {
                    serde_yaml::Value::String(s) => !s.is_empty() && !s.starts_with("$"),
                    _ => false,
                };

                if has_hardcoded_value {
                    let line = values.line_for_path(path).unwrap_or(1);
                    failures.push(CheckFailure::new(
                        "HL2004",
                        Severity::Warning,
                        format!(
                            "Sensitive value '{}' has a hardcoded default. Consider using a Secret reference",
                            path
                        ),
                        "values.yaml",
                        line,
                        RuleCategory::Values,
                    ));
                }
            }
        }

        failures
    }
}

/// HL2005: Port number out of valid range
pub struct HL2005;

impl Rule for HL2005 {
    fn code(&self) -> &'static str {
        "HL2005"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "invalid-port"
    }

    fn description(&self) -> &'static str {
        "Port number must be between 1 and 65535"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        let values = match ctx.values {
            Some(v) => v,
            None => return failures,
        };

        // Look for common port patterns
        let port_patterns = [
            "port",
            "containerPort",
            "targetPort",
            "hostPort",
            "nodePort",
        ];

        for path in &values.defined_paths {
            let lower_path = path.to_lowercase();
            let is_port_field = port_patterns.iter().any(|p| lower_path.ends_with(p));

            if is_port_field {
                if let Some(value) = values.get(path) {
                    if let Some(port) = extract_port_number(value) {
                        if !(1..=65535).contains(&port) {
                            let line = values.line_for_path(path).unwrap_or(1);
                            failures.push(CheckFailure::new(
                                "HL2005",
                                Severity::Error,
                                format!(
                                    "Invalid port number {} at '{}'. Must be between 1 and 65535",
                                    port, path
                                ),
                                "values.yaml",
                                line,
                                RuleCategory::Values,
                            ));
                        }
                    }
                }
            }
        }

        failures
    }
}

/// HL2007: Image tag is 'latest'
pub struct HL2007;

impl Rule for HL2007 {
    fn code(&self) -> &'static str {
        "HL2007"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "image-tag-latest"
    }

    fn description(&self) -> &'static str {
        "Using 'latest' tag is prone to unexpected changes"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        let values = match ctx.values {
            Some(v) => v,
            None => return failures,
        };

        // Look for image.tag or similar patterns
        for path in &values.defined_paths {
            let lower_path = path.to_lowercase();
            if lower_path.ends_with(".tag") || lower_path.ends_with("imagetag") {
                if let Some(serde_yaml::Value::String(tag)) = values.get(path) {
                    if tag == "latest" {
                        let line = values.line_for_path(path).unwrap_or(1);
                        failures.push(CheckFailure::new(
                            "HL2007",
                            Severity::Warning,
                            format!(
                                "Image tag at '{}' is 'latest'. Pin to a specific version for reproducibility",
                                path
                            ),
                            "values.yaml",
                            line,
                            RuleCategory::Values,
                        ));
                    }
                }
            }
        }

        failures
    }
}

/// HL2008: Replica count is zero
pub struct HL2008;

impl Rule for HL2008 {
    fn code(&self) -> &'static str {
        "HL2008"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "zero-replicas"
    }

    fn description(&self) -> &'static str {
        "Replica count is zero which means no pods will be created"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        let values = match ctx.values {
            Some(v) => v,
            None => return failures,
        };

        for path in &values.defined_paths {
            let lower_path = path.to_lowercase();
            if lower_path.ends_with("replicacount") || lower_path.ends_with("replicas") {
                if let Some(value) = values.get(path) {
                    if let Some(count) = extract_number(value) {
                        if count == 0 {
                            let line = values.line_for_path(path).unwrap_or(1);
                            failures.push(CheckFailure::new(
                                "HL2008",
                                Severity::Warning,
                                format!(
                                    "Replica count at '{}' is 0. No pods will be created by default",
                                    path
                                ),
                                "values.yaml",
                                line,
                                RuleCategory::Values,
                            ));
                        }
                    }
                }
            }
        }

        failures
    }
}

/// Extract a port number from a YAML value.
fn extract_port_number(value: &serde_yaml::Value) -> Option<i64> {
    match value {
        serde_yaml::Value::Number(n) => n.as_i64(),
        serde_yaml::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Extract a number from a YAML value.
fn extract_number(value: &serde_yaml::Value) -> Option<i64> {
    match value {
        serde_yaml::Value::Number(n) => n.as_i64(),
        serde_yaml::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_port_number() {
        assert_eq!(
            extract_port_number(&serde_yaml::Value::Number(80.into())),
            Some(80)
        );
        assert_eq!(
            extract_port_number(&serde_yaml::Value::String("8080".to_string())),
            Some(8080)
        );
        assert_eq!(extract_port_number(&serde_yaml::Value::Bool(true)), None);
    }
}
