//! DCL001: no-build-and-image
//!
//! Service cannot have both `build` and `image` fields (unless `pull_policy` is set).

use crate::analyzer::dclint::rules::{LintContext, Rule, SimpleRule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL001";
const NAME: &str = "no-build-and-image";
const DESCRIPTION: &str = "Each service must use either `build` or `image`, not both.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/no-build-and-image-rule.md";

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

    for (service_name, service) in &ctx.compose.services {
        // Check if service has both build and image
        let has_build = service.build.is_some();
        let has_image = service.image.is_some();
        let has_pull_policy = service.pull_policy.is_some();

        // Having both is only allowed if pull_policy is set
        if has_build && has_image && !has_pull_policy {
            let line = service
                .build_pos
                .map(|p| p.line)
                .or(service.position.line.into())
                .unwrap_or(1);

            let message = format!(
                "Service \"{}\" is using both \"build\" and \"image\". Use one of them, but not both.",
                service_name
            );

            failures.push(
                make_failure(
                    &CODE.into(),
                    NAME,
                    Severity::Error,
                    RuleCategory::BestPractice,
                    message,
                    line,
                    1,
                    false,
                )
                .with_data("serviceName", service_name.clone()),
            );
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
    fn test_no_violation_image_only() {
        let yaml = r#"
services:
  web:
    image: nginx:latest
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_no_violation_build_only() {
        let yaml = r#"
services:
  web:
    build: .
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_build_and_image() {
        let yaml = r#"
services:
  web:
    build: .
    image: myapp:latest
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("web"));
        assert!(failures[0].message.contains("build"));
        assert!(failures[0].message.contains("image"));
    }

    #[test]
    fn test_no_violation_with_pull_policy() {
        let yaml = r#"
services:
  web:
    build: .
    image: myapp:latest
    pull_policy: build
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_multiple_services() {
        let yaml = r#"
services:
  web:
    build: .
    image: myapp:latest
  db:
    image: postgres:15
  api:
    build: ./api
    image: myapi:v1
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 2);
    }
}
