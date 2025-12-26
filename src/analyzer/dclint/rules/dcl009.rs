//! DCL009: service-container-name-regex
//!
//! Container names must match a specified regex pattern.

use regex::Regex;

use crate::analyzer::dclint::rules::{LintContext, Rule, SimpleRule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL009";
const NAME: &str = "service-container-name-regex";
const DESCRIPTION: &str = "Container names must follow the naming convention.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/service-container-name-regex-rule.md";

// Default pattern: lowercase letters, numbers, hyphens, underscores
const DEFAULT_PATTERN: &str = r"^[a-z][a-z0-9_-]*$";

pub fn rule() -> impl Rule {
    SimpleRule::new(
        CODE,
        NAME,
        Severity::Warning,
        RuleCategory::Style,
        DESCRIPTION,
        URL,
        check,
    )
}

fn check(ctx: &LintContext) -> Vec<CheckFailure> {
    let mut failures = Vec::new();

    // Use default pattern (in a real implementation, this could be configurable)
    let pattern = Regex::new(DEFAULT_PATTERN).expect("Invalid default pattern");

    for (service_name, service) in &ctx.compose.services {
        if let Some(container_name) = &service.container_name {
            if !pattern.is_match(container_name) {
                let line = service
                    .container_name_pos
                    .map(|p| p.line)
                    .unwrap_or(service.position.line);

                let message = format!(
                    "Container name \"{}\" in service \"{}\" does not match the required pattern: {}",
                    container_name, service_name, DEFAULT_PATTERN
                );

                failures.push(
                    make_failure(
                        &CODE.into(),
                        NAME,
                        Severity::Warning,
                        RuleCategory::Style,
                        message,
                        line,
                        1,
                        false,
                    )
                    .with_data("serviceName", service_name.clone())
                    .with_data("containerName", container_name.clone())
                    .with_data("pattern", DEFAULT_PATTERN.to_string()),
                );
            }
        }
    }

    failures
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::dclint::parser::parse_compose;

    fn check_yaml(yaml: &str) -> Vec<CheckFailure> {
        let compose = parse_compose(yaml).unwrap();
        let ctx = LintContext::new(&compose, yaml, "docker-compose.yml");
        check(&ctx)
    }

    #[test]
    fn test_no_violation_valid_name() {
        let yaml = r#"
services:
  web:
    image: nginx
    container_name: my-web-container
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_no_violation_no_container_name() {
        let yaml = r#"
services:
  web:
    image: nginx
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_uppercase() {
        let yaml = r#"
services:
  web:
    image: nginx
    container_name: MyContainer
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("MyContainer"));
    }

    #[test]
    fn test_violation_starts_with_number() {
        let yaml = r#"
services:
  web:
    image: nginx
    container_name: 123container
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
    }

    #[test]
    fn test_valid_names() {
        let valid_names = ["web", "my-app", "app_v1", "a123", "web-api-v2"];

        for name in valid_names {
            let yaml = format!(
                r#"
services:
  web:
    image: nginx
    container_name: {}
"#,
                name
            );
            assert!(
                check_yaml(&yaml).is_empty(),
                "Name '{}' should be valid",
                name
            );
        }
    }
}
