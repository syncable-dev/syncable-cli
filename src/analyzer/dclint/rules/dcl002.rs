//! DCL002: no-duplicate-container-names
//!
//! Container names must be unique across all services.

use std::collections::HashMap;

use crate::analyzer::dclint::rules::{LintContext, Rule, SimpleRule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL002";
const NAME: &str = "no-duplicate-container-names";
const DESCRIPTION: &str = "Container names must be unique across all services.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/no-duplicate-container-names-rule.md";

pub fn rule() -> impl Rule {
    SimpleRule::new(
        CODE,
        NAME,
        Severity::Error,
        RuleCategory::BestPractice,
        DESCRIPTION,
        URL,
        check,
    )
}

fn check(ctx: &LintContext) -> Vec<CheckFailure> {
    let mut failures = Vec::new();
    let mut container_names: HashMap<String, Vec<(String, u32)>> = HashMap::new();

    // Collect all container names with their service names and positions
    for (service_name, service) in &ctx.compose.services {
        if let Some(container_name) = &service.container_name {
            let line = service
                .container_name_pos
                .map(|p| p.line)
                .unwrap_or(service.position.line);

            container_names
                .entry(container_name.clone())
                .or_default()
                .push((service_name.clone(), line));
        }
    }

    // Report duplicates
    for (container_name, services) in container_names {
        if services.len() > 1 {
            for (service_name, line) in &services {
                let other_services: Vec<&str> = services
                    .iter()
                    .filter(|(name, _)| name != service_name)
                    .map(|(name, _)| name.as_str())
                    .collect();

                let message = format!(
                    "Container name \"{}\" is used by multiple services: \"{}\" and \"{}\".",
                    container_name,
                    service_name,
                    other_services.join("\", \"")
                );

                failures.push(
                    make_failure(
                        &CODE.into(),
                        NAME,
                        Severity::Error,
                        RuleCategory::BestPractice,
                        message,
                        *line,
                        1,
                        false,
                    )
                    .with_data("containerName", container_name.clone())
                    .with_data("serviceName", service_name.clone()),
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
    fn test_no_violation_unique_names() {
        let yaml = r#"
services:
  web:
    image: nginx
    container_name: my-web
  db:
    image: postgres
    container_name: my-db
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_no_violation_no_container_names() {
        let yaml = r#"
services:
  web:
    image: nginx
  db:
    image: postgres
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_duplicate_names() {
        let yaml = r#"
services:
  web:
    image: nginx
    container_name: my-container
  api:
    image: node
    container_name: my-container
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 2); // One failure per service with duplicate
        assert!(failures[0].message.contains("my-container"));
    }

    #[test]
    fn test_violation_multiple_duplicates() {
        let yaml = r#"
services:
  web:
    image: nginx
    container_name: shared-name
  api:
    image: node
    container_name: shared-name
  worker:
    image: worker
    container_name: shared-name
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 3); // One failure per service
    }
}
