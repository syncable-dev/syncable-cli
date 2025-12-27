//! DCL007: require-project-name-field
//!
//! The `name` field should be present for explicit project naming.

use crate::analyzer::dclint::rules::{LintContext, Rule, SimpleRule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL007";
const NAME: &str = "require-project-name-field";
const DESCRIPTION: &str = "The top-level `name` field should be set for explicit project naming.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/require-project-name-field-rule.md";

pub fn rule() -> impl Rule {
    SimpleRule::new(
        CODE,
        NAME,
        Severity::Info,
        RuleCategory::BestPractice,
        DESCRIPTION,
        URL,
        check,
    )
}

fn check(ctx: &LintContext) -> Vec<CheckFailure> {
    let mut failures = Vec::new();

    if ctx.compose.name.is_none() {
        let message = "Consider adding a `name` field to explicitly set the project name instead of relying on the directory name.".to_string();

        failures.push(make_failure(
            &CODE.into(),
            NAME,
            Severity::Info,
            RuleCategory::BestPractice,
            message,
            1,
            1,
            false,
        ));
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
    fn test_no_violation_has_name() {
        let yaml = r#"
name: myproject
services:
  web:
    image: nginx
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_no_name() {
        let yaml = r#"
services:
  web:
    image: nginx
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("name"));
    }
}
