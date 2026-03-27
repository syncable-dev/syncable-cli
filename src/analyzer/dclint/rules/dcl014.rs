//! DCL014: services-alphabetical-order
//!
//! Services should be sorted alphabetically.

use crate::analyzer::dclint::rules::{FixableRule, LintContext, Rule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL014";
const NAME: &str = "services-alphabetical-order";
const DESCRIPTION: &str = "Services should be defined in alphabetical order.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/services-alphabetical-order-rule.md";

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

    let service_names: Vec<String> = ctx.compose.services.keys().cloned().collect();

    if service_names.len() > 1 {
        let mut sorted_names = service_names.clone();
        sorted_names.sort();

        // Check if they're already sorted
        let current_order: Vec<String> = {
            // We need to get the actual order from the source
            // The HashMap doesn't preserve order, so we check against sorted
            let mut names: Vec<_> = ctx.compose.services.keys().cloned().collect();
            names.sort_by_key(|name| {
                ctx.compose
                    .services
                    .get(name)
                    .map(|s| s.position.line)
                    .unwrap_or(u32::MAX)
            });
            names
        };

        if current_order != sorted_names {
            let line = ctx.compose.services_pos.map(|p| p.line).unwrap_or(1);

            let message = format!(
                "Services are not in alphabetical order. Expected: [{}], got: [{}].",
                sorted_names.join(", "),
                current_order.join(", ")
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
                .with_data("expected", sorted_names.join(", "))
                .with_data("actual", current_order.join(", ")),
            );
        }
    }

    failures
}

fn fix(_source: &str) -> Option<String> {
    // Full service reordering requires proper YAML AST manipulation
    // This is complex and would need yaml-rust2's Document API for proper handling
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
    fn test_no_violation_sorted() {
        let yaml = r#"
services:
  api:
    image: api
  db:
    image: postgres
  web:
    image: nginx
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_unsorted() {
        let yaml = r#"
services:
  web:
    image: nginx
  api:
    image: api
  db:
    image: postgres
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("alphabetical"));
    }

    #[test]
    fn test_no_violation_single_service() {
        let yaml = r#"
services:
  web:
    image: nginx
"#;
        assert!(check_yaml(yaml).is_empty());
    }
}
