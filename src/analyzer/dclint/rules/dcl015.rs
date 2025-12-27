//! DCL015: top-level-properties-order
//!
//! Top-level properties should be in a standard order.

use crate::analyzer::dclint::rules::{FixableRule, LintContext, Rule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL015";
const NAME: &str = "top-level-properties-order";
const DESCRIPTION: &str = "Top-level properties should follow a standard ordering convention.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/top-level-properties-order-rule.md";

// Standard top-level key order
const KEY_ORDER: &[&str] = &[
    "version", // Deprecated but may exist
    "name", "services", "networks", "volumes", "configs", "secrets",
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

    if ctx.compose.top_level_keys.len() > 1 {
        let mut sorted_keys = ctx.compose.top_level_keys.clone();
        sorted_keys.sort_by_key(|k| get_key_order(k));

        if ctx.compose.top_level_keys != sorted_keys {
            let message = format!(
                "Top-level properties are not in standard order. Expected: [{}], got: [{}].",
                sorted_keys.join(", "),
                ctx.compose.top_level_keys.join(", ")
            );

            failures.push(
                make_failure(
                    &CODE.into(),
                    NAME,
                    Severity::Style,
                    RuleCategory::Style,
                    message,
                    1,
                    1,
                    true,
                )
                .with_data("expected", sorted_keys.join(", "))
                .with_data("actual", ctx.compose.top_level_keys.join(", ")),
            );
        }
    }

    failures
}

fn fix(_source: &str) -> Option<String> {
    // Full reordering requires proper YAML AST manipulation
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
name: myproject
services:
  web:
    image: nginx
networks:
  default:
volumes:
  data:
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_wrong_order() {
        let yaml = r#"
services:
  web:
    image: nginx
name: myproject
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("standard order"));
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

    #[test]
    fn test_violation_volumes_before_services() {
        let yaml = r#"
volumes:
  data:
services:
  web:
    image: nginx
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
    }
}
