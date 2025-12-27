//! DCL013: service-ports-alphabetical-order
//!
//! Service ports should be sorted alphabetically/numerically.

use crate::analyzer::dclint::rules::{FixableRule, LintContext, Rule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL013";
const NAME: &str = "service-ports-alphabetical-order";
const DESCRIPTION: &str = "Service ports should be sorted numerically.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/service-ports-alphabetical-order-rule.md";

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

fn check(ctx: &LintContext) -> Vec<CheckFailure> {
    let mut failures = Vec::new();

    for (service_name, service) in &ctx.compose.services {
        if service.ports.len() > 1 {
            let port_strs: Vec<String> = service.ports.iter().map(|p| p.raw.clone()).collect();
            let mut sorted_ports = port_strs.clone();
            sorted_ports.sort();

            if port_strs != sorted_ports {
                let line = service
                    .ports_pos
                    .map(|p| p.line)
                    .unwrap_or(service.position.line);

                let message = format!(
                    "Ports in service \"{}\" are not in alphabetical order. Expected: [{}], got: [{}].",
                    service_name,
                    sorted_ports.join(", "),
                    port_strs.join(", ")
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
                    .with_data("serviceName", service_name.clone()),
                );
            }
        }
    }

    failures
}

fn fix(source: &str) -> Option<String> {
    let mut result = String::new();
    let mut modified = false;
    let mut in_ports_section = false;
    let mut ports_indent = 0;
    let mut _service_indent = 0;
    let mut ports: Vec<(String, String)> = Vec::new(); // (raw, full line)

    for line in source.lines() {
        let trimmed = line.trim();
        let indent = line.len() - line.trim_start().len();

        // Track service indent level
        if !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && !trimmed.starts_with('-')
            && trimmed.ends_with(':')
            && indent == 2
        {
            _service_indent = indent;
        }

        // Track if we're in a ports section
        if trimmed.starts_with("ports:") {
            in_ports_section = true;
            ports_indent = indent;
            ports.clear();
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Exit ports section when indent decreases
        if in_ports_section
            && !trimmed.is_empty()
            && indent <= ports_indent
            && !trimmed.starts_with('-')
        {
            // Sort and output ports
            let mut sorted_ports = ports.clone();
            sorted_ports.sort_by(|a, b| a.0.cmp(&b.0));

            if ports.iter().map(|(r, _)| r.clone()).collect::<Vec<_>>()
                != sorted_ports
                    .iter()
                    .map(|(r, _)| r.clone())
                    .collect::<Vec<_>>()
            {
                modified = true;
                for (_, full_line) in &sorted_ports {
                    result.push_str(full_line);
                    result.push('\n');
                }
            } else {
                for (_, full_line) in &ports {
                    result.push_str(full_line);
                    result.push('\n');
                }
            }

            ports.clear();
            in_ports_section = false;
        }

        // Collect port entries
        if in_ports_section && trimmed.starts_with('-') {
            let port_value = trimmed.trim_start_matches('-').trim();
            let raw = port_value.trim_matches('"').trim_matches('\'').to_string();
            ports.push((raw, line.to_string()));
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    // Handle case where ports section is at the end
    if in_ports_section && !ports.is_empty() {
        let mut sorted_ports = ports.clone();
        sorted_ports.sort_by(|a, b| a.0.cmp(&b.0));

        if ports.iter().map(|(r, _)| r.clone()).collect::<Vec<_>>()
            != sorted_ports
                .iter()
                .map(|(r, _)| r.clone())
                .collect::<Vec<_>>()
        {
            modified = true;
            for (_, full_line) in &sorted_ports {
                result.push_str(full_line);
                result.push('\n');
            }
        } else {
            for (_, full_line) in &ports {
                result.push_str(full_line);
                result.push('\n');
            }
        }
    }

    if modified {
        if !source.ends_with('\n') {
            result.pop();
        }
        Some(result)
    } else {
        None
    }
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
    fn test_no_violation_sorted() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "3000:3000"
      - "8080:80"
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_unsorted() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "8080:80"
      - "3000:3000"
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("alphabetical"));
    }

    #[test]
    fn test_no_violation_single_port() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "8080:80"
"#;
        assert!(check_yaml(yaml).is_empty());
    }
}
