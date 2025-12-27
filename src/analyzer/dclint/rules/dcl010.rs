//! DCL010: service-dependencies-alphabetical-order
//!
//! Service dependencies should be sorted alphabetically.

use crate::analyzer::dclint::rules::{FixableRule, LintContext, Rule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL010";
const NAME: &str = "service-dependencies-alphabetical-order";
const DESCRIPTION: &str = "Service dependencies should be sorted alphabetically.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/service-dependencies-alphabetical-order-rule.md";

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
        if service.depends_on.len() > 1 {
            let mut sorted = service.depends_on.clone();
            sorted.sort();

            if service.depends_on != sorted {
                let line = service
                    .depends_on_pos
                    .map(|p| p.line)
                    .unwrap_or(service.position.line);

                let message = format!(
                    "Dependencies in service \"{}\" are not in alphabetical order. Expected: [{}], got: [{}].",
                    service_name,
                    sorted.join(", "),
                    service.depends_on.join(", ")
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
                    .with_data("expected", sorted.join(", "))
                    .with_data("actual", service.depends_on.join(", ")),
                );
            }
        }
    }

    failures
}

fn fix(source: &str) -> Option<String> {
    // This is a simplified fix that works for array-style depends_on
    // A full implementation would need proper YAML manipulation
    let mut result = String::new();
    let mut modified = false;
    let mut in_depends_on = false;
    let mut depends_on_indent = 0;
    let mut deps: Vec<String> = Vec::new();
    let mut _deps_start_line = 0;
    let mut collected_lines: Vec<String> = Vec::new();

    for (idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        let indent = line.len() - line.trim_start().len();

        // Track if we're in a depends_on section
        if trimmed.starts_with("depends_on:") {
            in_depends_on = true;
            depends_on_indent = indent;
            _deps_start_line = idx;
            deps.clear();
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Collect dependencies
        if in_depends_on && trimmed.starts_with('-') && indent > depends_on_indent {
            let dep = trimmed.trim_start_matches('-').trim().to_string();
            deps.push(dep);
            collected_lines.push(line.to_string());
            continue;
        }

        // Exit depends_on section
        if in_depends_on && (!trimmed.starts_with('-') || indent <= depends_on_indent) {
            // Sort and output deps
            let mut sorted_deps = deps.clone();
            sorted_deps.sort();

            if deps != sorted_deps {
                modified = true;
                for dep in &sorted_deps {
                    result.push_str(&" ".repeat(depends_on_indent + 2));
                    result.push_str("- ");
                    result.push_str(dep);
                    result.push('\n');
                }
            } else {
                for dep_line in &collected_lines {
                    result.push_str(dep_line);
                    result.push('\n');
                }
            }

            deps.clear();
            collected_lines.clear();
            in_depends_on = false;
        }

        result.push_str(line);
        result.push('\n');
    }

    // Handle case where depends_on is at the end of file
    if in_depends_on && !deps.is_empty() {
        let mut sorted_deps = deps.clone();
        sorted_deps.sort();

        if deps != sorted_deps {
            modified = true;
            for dep in &sorted_deps {
                result.push_str(&" ".repeat(depends_on_indent + 2));
                result.push_str("- ");
                result.push_str(dep);
                result.push('\n');
            }
        } else {
            for dep_line in &collected_lines {
                result.push_str(dep_line);
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
    depends_on:
      - cache
      - db
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_no_violation_single_dep() {
        let yaml = r#"
services:
  web:
    image: nginx
    depends_on:
      - db
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_unsorted() {
        let yaml = r#"
services:
  web:
    image: nginx
    depends_on:
      - db
      - cache
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("alphabetical"));
    }

    #[test]
    fn test_violation_multiple_unsorted() {
        let yaml = r#"
services:
  web:
    image: nginx
    depends_on:
      - redis
      - db
      - cache
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
    }
}
