//! DCL003: no-duplicate-exported-ports
//!
//! Exported host ports must be unique across all services.

use std::collections::HashMap;

use crate::analyzer::dclint::rules::{LintContext, Rule, SimpleRule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL003";
const NAME: &str = "no-duplicate-exported-ports";
const DESCRIPTION: &str = "Exported host ports must be unique across all services.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/no-duplicate-exported-ports-rule.md";

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
    // Map from exported port to list of (service_name, port_raw, line)
    let mut exported_ports: HashMap<String, Vec<(String, String, u32)>> = HashMap::new();

    for (service_name, service) in &ctx.compose.services {
        for port in &service.ports {
            // Only check ports with a host port binding
            if let Some(host_port) = port.host_port {
                let key = if let Some(ip) = &port.host_ip {
                    format!("{}:{}", ip, host_port)
                } else {
                    // Unbound ports conflict with any other unbound port on same port number
                    host_port.to_string()
                };

                exported_ports.entry(key).or_default().push((
                    service_name.clone(),
                    port.raw.clone(),
                    port.position.line,
                ));
            }
        }
    }

    // Report duplicates
    for (exported_port, usages) in exported_ports {
        if usages.len() > 1 {
            for (service_name, port_raw, line) in &usages {
                let other_services: Vec<&str> = usages
                    .iter()
                    .filter(|(name, _, _)| name != service_name)
                    .map(|(name, _, _)| name.as_str())
                    .collect();

                let message = format!(
                    "Port \"{}\" is exported by multiple services: \"{}\" and \"{}\".",
                    exported_port,
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
                    .with_data("exportedPort", exported_port.clone())
                    .with_data("serviceName", service_name.clone())
                    .with_data("portMapping", port_raw.clone()),
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
    fn test_no_violation_unique_ports() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "8080:80"
  api:
    image: node
    ports:
      - "3000:3000"
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_no_violation_same_container_port_different_host() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "8080:80"
  api:
    image: nginx
    ports:
      - "8081:80"
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_no_violation_container_only_ports() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - 80
  api:
    image: node
    ports:
      - 80
"#;
        // Container-only ports (no host binding) are not exported
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_duplicate_host_ports() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "8080:80"
  api:
    image: node
    ports:
      - "8080:3000"
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 2); // One per service
        assert!(failures[0].message.contains("8080"));
    }

    #[test]
    fn test_no_violation_different_interfaces() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "127.0.0.1:8080:80"
  api:
    image: node
    ports:
      - "192.168.1.1:8080:3000"
"#;
        // Different interfaces are technically different bindings
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_same_interface_same_port() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "127.0.0.1:8080:80"
  api:
    image: node
    ports:
      - "127.0.0.1:8080:3000"
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 2);
        assert!(failures[0].message.contains("127.0.0.1:8080"));
    }

    #[test]
    fn test_multiple_duplicates() {
        let yaml = r#"
services:
  web1:
    image: nginx
    ports:
      - "8080:80"
  web2:
    image: nginx
    ports:
      - "8080:80"
  web3:
    image: nginx
    ports:
      - "8080:80"
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 3); // One per service
    }
}
