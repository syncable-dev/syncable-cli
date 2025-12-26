//! DCL012: service-keys-order
//!
//! Service keys should be in a standard order.

use crate::analyzer::dclint::rules::{FixableRule, LintContext, Rule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL012";
const NAME: &str = "service-keys-order";
const DESCRIPTION: &str = "Service keys should follow a standard ordering convention.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/service-keys-order-rule.md";

// Standard key order for services
const KEY_ORDER: &[&str] = &[
    "image",
    "build",
    "container_name",
    "hostname",
    "restart",
    "depends_on",
    "links",
    "ports",
    "expose",
    "volumes",
    "volumes_from",
    "environment",
    "env_file",
    "secrets",
    "configs",
    "labels",
    "logging",
    "network_mode",
    "networks",
    "extra_hosts",
    "dns",
    "dns_search",
    "healthcheck",
    "deploy",
    "command",
    "entrypoint",
    "working_dir",
    "user",
    "privileged",
    "cap_add",
    "cap_drop",
    "security_opt",
    "tmpfs",
    "stdin_open",
    "tty",
    "ulimits",
    "sysctls",
    "extends",
    "profiles",
];

pub fn rule() -> impl Rule {
    FixableRule::new(
        CODE,
        NAME,
        Severity::Style,
        RuleCategory::Style,
        DESCRIPTION,
        URL,
        check,
        fix,
    )
}

fn get_key_order(key: &str) -> usize {
    KEY_ORDER
        .iter()
        .position(|&k| k == key)
        .unwrap_or(KEY_ORDER.len())
}

fn check(ctx: &LintContext) -> Vec<CheckFailure> {
    let mut failures = Vec::new();

    for (service_name, service) in &ctx.compose.services {
        if service.keys.len() > 1 {
            // Check if keys are in the expected order
            let mut sorted_keys = service.keys.clone();
            sorted_keys.sort_by_key(|k| get_key_order(k));

            if service.keys != sorted_keys {
                let line = service.position.line;

                // Find the first out-of-order key
                let mut first_wrong = None;
                for (i, key) in service.keys.iter().enumerate() {
                    if i < sorted_keys.len() && key != &sorted_keys[i] {
                        first_wrong = Some(key.clone());
                        break;
                    }
                }

                let message = format!(
                    "Service \"{}\" has keys in non-standard order. Consider reordering for consistency.",
                    service_name
                );

                failures.push(
                    make_failure(
                        &CODE.into(),
                        NAME,
                        Severity::Style,
                        RuleCategory::Style,
                        message,
                        line,
                        1,
                        true,
                    )
                    .with_data("serviceName", service_name.clone())
                    .with_data("firstWrongKey", first_wrong.unwrap_or_default()),
                );
            }
        }
    }

    failures
}

fn fix(_source: &str) -> Option<String> {
    // Full YAML key reordering requires proper YAML AST manipulation
    // This is a placeholder - a full implementation would need yaml-rust2's Document API
    None
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
    fn test_no_violation_correct_order() {
        let yaml = r#"
services:
  web:
    image: nginx
    container_name: web
    ports:
      - "80:80"
    environment:
      - DEBUG=true
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_wrong_order() {
        let yaml = r#"
services:
  web:
    environment:
      - DEBUG=true
    image: nginx
    ports:
      - "80:80"
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("non-standard order"));
    }

    #[test]
    fn test_no_violation_single_key() {
        let yaml = r#"
services:
  web:
    image: nginx
"#;
        assert!(check_yaml(yaml).is_empty());
    }
}
