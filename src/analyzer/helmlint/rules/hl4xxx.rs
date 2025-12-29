//! HL4xxx - Security Rules
//!
//! Rules for validating container and Kubernetes security settings.

use crate::analyzer::helmlint::parser::template::TemplateToken;
use crate::analyzer::helmlint::rules::{LintContext, Rule};
use crate::analyzer::helmlint::types::{CheckFailure, RuleCategory, Severity};

/// Get all HL4xxx rules.
pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(HL4001),
        Box::new(HL4002),
        Box::new(HL4003),
        Box::new(HL4004),
        Box::new(HL4005),
        Box::new(HL4006),
        Box::new(HL4011),
        Box::new(HL4012),
    ]
}

/// HL4001: Container running as root
pub struct HL4001;

impl Rule for HL4001 {
    fn code(&self) -> &'static str {
        "HL4001"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "container-runs-as-root"
    }

    fn description(&self) -> &'static str {
        "Container may run as root user"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        // Check values.yaml for runAsNonRoot settings
        if let Some(values) = ctx.values {
            // Look for securityContext settings
            let has_run_as_non_root = values
                .defined_paths
                .iter()
                .any(|p| p.to_lowercase().contains("runasnonroot"));

            let has_run_as_user = values
                .defined_paths
                .iter()
                .any(|p| p.to_lowercase().contains("runasuser"));

            if !has_run_as_non_root && !has_run_as_user {
                failures.push(CheckFailure::new(
                    "HL4001",
                    Severity::Warning,
                    "No runAsNonRoot or runAsUser setting found. Container may run as root",
                    "values.yaml",
                    1,
                    RuleCategory::Security,
                ));
            }
        }

        // Check templates for hardcoded security contexts
        for template in ctx.templates {
            let content = template
                .tokens
                .iter()
                .filter_map(|t| match t {
                    TemplateToken::Text { content, .. } => Some(content.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");

            // Check for runAsUser: 0 (root)
            if content.contains("runAsUser: 0") || content.contains("runAsUser:0") {
                failures.push(CheckFailure::new(
                    "HL4001",
                    Severity::Warning,
                    "Container is configured to run as root (runAsUser: 0)",
                    &template.path,
                    1,
                    RuleCategory::Security,
                ));
            }
        }

        failures
    }
}

/// HL4002: Privileged container
pub struct HL4002;

impl Rule for HL4002 {
    fn code(&self) -> &'static str {
        "HL4002"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "privileged-container"
    }

    fn description(&self) -> &'static str {
        "Container runs in privileged mode"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        // Check values.yaml
        if let Some(values) = ctx.values {
            for path in &values.defined_paths {
                if path.to_lowercase().contains("privileged") {
                    if let Some(value) = values.get(path) {
                        if is_truthy(value) {
                            let line = values.line_for_path(path).unwrap_or(1);
                            failures.push(CheckFailure::new(
                                "HL4002",
                                Severity::Error,
                                format!("Privileged mode enabled at '{}'", path),
                                "values.yaml",
                                line,
                                RuleCategory::Security,
                            ));
                        }
                    }
                }
            }
        }

        // Check templates for hardcoded privileged: true
        for template in ctx.templates {
            for token in &template.tokens {
                if let TemplateToken::Text { content, line } = token {
                    if content.contains("privileged: true") {
                        failures.push(CheckFailure::new(
                            "HL4002",
                            Severity::Error,
                            "Container is configured with privileged: true",
                            &template.path,
                            *line,
                            RuleCategory::Security,
                        ));
                    }
                }
            }
        }

        failures
    }
}

/// HL4003: HostPath volume mount
pub struct HL4003;

impl Rule for HL4003 {
    fn code(&self) -> &'static str {
        "HL4003"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "hostpath-volume"
    }

    fn description(&self) -> &'static str {
        "Using hostPath volumes can expose host filesystem"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        for template in ctx.templates {
            for token in &template.tokens {
                if let TemplateToken::Text { content, line } = token {
                    if content.contains("hostPath:") {
                        failures.push(CheckFailure::new(
                            "HL4003",
                            Severity::Warning,
                            "Using hostPath volume mount. This can expose the host filesystem to the container",
                            &template.path,
                            *line,
                            RuleCategory::Security,
                        ));
                    }
                }
            }
        }

        failures
    }
}

/// HL4004: HostNetwork enabled
pub struct HL4004;

impl Rule for HL4004 {
    fn code(&self) -> &'static str {
        "HL4004"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "host-network"
    }

    fn description(&self) -> &'static str {
        "Using host network can bypass network policies"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        // Check values.yaml
        if let Some(values) = ctx.values {
            for path in &values.defined_paths {
                if path.to_lowercase().contains("hostnetwork") {
                    if let Some(value) = values.get(path) {
                        if is_truthy(value) {
                            let line = values.line_for_path(path).unwrap_or(1);
                            failures.push(CheckFailure::new(
                                "HL4004",
                                Severity::Warning,
                                format!("Host network enabled at '{}'", path),
                                "values.yaml",
                                line,
                                RuleCategory::Security,
                            ));
                        }
                    }
                }
            }
        }

        // Check templates
        for template in ctx.templates {
            for token in &template.tokens {
                if let TemplateToken::Text { content, line } = token {
                    if content.contains("hostNetwork: true") {
                        failures.push(CheckFailure::new(
                            "HL4004",
                            Severity::Warning,
                            "Pod uses host network. This bypasses network policies",
                            &template.path,
                            *line,
                            RuleCategory::Security,
                        ));
                    }
                }
            }
        }

        failures
    }
}

/// HL4005: HostPID enabled
pub struct HL4005;

impl Rule for HL4005 {
    fn code(&self) -> &'static str {
        "HL4005"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "host-pid"
    }

    fn description(&self) -> &'static str {
        "Using host PID namespace can expose host processes"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        for template in ctx.templates {
            for token in &template.tokens {
                if let TemplateToken::Text { content, line } = token {
                    if content.contains("hostPID: true") {
                        failures.push(CheckFailure::new(
                            "HL4005",
                            Severity::Warning,
                            "Pod uses host PID namespace. This can expose host processes",
                            &template.path,
                            *line,
                            RuleCategory::Security,
                        ));
                    }
                }
            }
        }

        failures
    }
}

/// HL4006: Missing securityContext
pub struct HL4006;

impl Rule for HL4006 {
    fn code(&self) -> &'static str {
        "HL4006"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn name(&self) -> &'static str {
        "missing-security-context"
    }

    fn description(&self) -> &'static str {
        "Container or pod is missing securityContext"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        // Check if values.yaml has any security context settings
        if let Some(values) = ctx.values {
            let has_security_context = values
                .defined_paths
                .iter()
                .any(|p| p.to_lowercase().contains("securitycontext"));

            if !has_security_context {
                failures.push(CheckFailure::new(
                    "HL4006",
                    Severity::Info,
                    "No securityContext configuration found in values.yaml",
                    "values.yaml",
                    1,
                    RuleCategory::Security,
                ));
            }
        }

        failures
    }
}

/// HL4011: Secret in environment variable
pub struct HL4011;

impl Rule for HL4011 {
    fn code(&self) -> &'static str {
        "HL4011"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "secret-in-env"
    }

    fn description(&self) -> &'static str {
        "Sensitive value passed via environment variable instead of mounted secret"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        // Look for environment variables with sensitive names and direct values
        let sensitive_patterns = [
            "PASSWORD",
            "SECRET",
            "TOKEN",
            "API_KEY",
            "APIKEY",
            "PRIVATE_KEY",
            "CREDENTIALS",
        ];

        for template in ctx.templates {
            for token in &template.tokens {
                if let TemplateToken::Text { content, line } = token {
                    // Check if this looks like an env definition with a sensitive name
                    for pattern in &sensitive_patterns {
                        let search = format!("name: {}", pattern);
                        let search_lower = format!("name: {}", pattern.to_lowercase());
                        if (content.contains(&search) || content.contains(&search_lower))
                            && content.contains("value:")
                            && !content.contains("valueFrom:")
                            && !content.contains("secretKeyRef:")
                        {
                            failures.push(CheckFailure::new(
                                "HL4011",
                                Severity::Warning,
                                format!(
                                    "Environment variable matching '{}' should use secretKeyRef instead of direct value",
                                    pattern
                                ),
                                &template.path,
                                *line,
                                RuleCategory::Security,
                            ));
                        }
                    }
                }
            }
        }

        failures
    }
}

/// HL4012: Hardcoded credentials detected
pub struct HL4012;

impl Rule for HL4012 {
    fn code(&self) -> &'static str {
        "HL4012"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "hardcoded-credentials"
    }

    fn description(&self) -> &'static str {
        "Hardcoded credentials or secrets detected in templates"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        // Credential types to check for
        let credential_types = [
            ("password:", "password"),
            ("secret:", "secret"),
            ("apikey:", "API key"),
            ("token:", "token"),
        ];

        for template in ctx.templates {
            for token in &template.tokens {
                if let TemplateToken::Text { content, line } = token {
                    let lower_content = content.to_lowercase();

                    for (pattern, cred_type) in &credential_types {
                        // Check for patterns that look like credentials
                        if lower_content.contains(pattern) {
                            // Make sure it's not using a template variable
                            let has_template_var = content.contains("{{") && content.contains("}}");
                            let is_empty = content.contains("\"\"") || content.contains("''");

                            if !has_template_var && !is_empty {
                                // Additional check: line should have an actual value
                                let parts: Vec<&str> = content.split(':').collect();
                                if parts.len() >= 2 {
                                    let value_part = parts[1].trim();
                                    if !value_part.is_empty()
                                        && !value_part.starts_with('{')
                                        && !value_part.starts_with('$')
                                        && value_part != "\"\""
                                        && value_part != "''"
                                    {
                                        failures.push(CheckFailure::new(
                                            "HL4012",
                                            Severity::Error,
                                            format!(
                                                "Possible hardcoded {} detected. Use Secrets instead",
                                                cred_type
                                            ),
                                            &template.path,
                                            *line,
                                            RuleCategory::Security,
                                        ));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        failures
    }
}

/// Check if a YAML value is truthy.
fn is_truthy(value: &serde_yaml::Value) -> bool {
    match value {
        serde_yaml::Value::Bool(b) => *b,
        serde_yaml::Value::String(s) => {
            let lower = s.to_lowercase();
            lower == "true" || lower == "yes" || lower == "1"
        }
        serde_yaml::Value::Number(n) => n.as_i64().map(|i| i != 0).unwrap_or(false),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_truthy() {
        assert!(is_truthy(&serde_yaml::Value::Bool(true)));
        assert!(!is_truthy(&serde_yaml::Value::Bool(false)));
        assert!(is_truthy(&serde_yaml::Value::String("true".to_string())));
        assert!(is_truthy(&serde_yaml::Value::String("yes".to_string())));
        assert!(!is_truthy(&serde_yaml::Value::String("false".to_string())));
        assert!(is_truthy(&serde_yaml::Value::Number(1.into())));
        assert!(!is_truthy(&serde_yaml::Value::Number(0.into())));
    }

    #[test]
    fn test_rules_exist() {
        let all_rules = rules();
        assert!(!all_rules.is_empty());
    }
}
