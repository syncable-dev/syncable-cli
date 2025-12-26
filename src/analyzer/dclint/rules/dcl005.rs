//! DCL005: no-unbound-port-interfaces
//!
//! Ports should bind to a specific interface (not 0.0.0.0).

use crate::analyzer::dclint::rules::{LintContext, Rule, SimpleRule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL005";
const NAME: &str = "no-unbound-port-interfaces";
const DESCRIPTION: &str = "Ports should bind to a specific interface for security.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/no-unbound-port-interfaces-rule.md";

pub fn rule() -> impl Rule {
    SimpleRule::new(
        CODE,
        NAME,
        Severity::Warning,
        RuleCategory::Security,
        DESCRIPTION,
        URL,
        check,
    )
}

fn check(ctx: &LintContext) -> Vec<CheckFailure> {
    let mut failures = Vec::new();

    for (service_name, service) in &ctx.compose.services {
        for port in &service.ports {
            // Check if port has a host port but no explicit interface
            if port.host_port.is_some() && !port.has_explicit_interface() {
                let message = format!(
                    "Port \"{}\" in service \"{}\" does not specify a host interface. Consider binding to 127.0.0.1 for local-only access.",
                    port.raw, service_name
                );

                failures.push(
                    make_failure(
                        &CODE.into(),
                        NAME,
                        Severity::Warning,
                        RuleCategory::Security,
                        message,
                        port.position.line,
                        port.position.column,
                        false,
                    )
                    .with_data("serviceName", service_name.clone())
                    .with_data("port", port.raw.clone()),
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
    fn test_no_violation_explicit_interface() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "127.0.0.1:8080:80"
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_no_violation_container_only() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - 80
"#;
        // Container-only ports don't bind to host
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_unbound_port() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "8080:80"
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("8080:80"));
        assert!(failures[0].message.contains("127.0.0.1"));
    }

    #[test]
    fn test_multiple_violations() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "8080:80"
      - "127.0.0.1:8443:443"
      - "3000:3000"
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 2); // 8080 and 3000, not 8443
    }
}
