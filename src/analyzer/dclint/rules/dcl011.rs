//! DCL011: service-image-require-explicit-tag
//!
//! Service images should have explicit version tags.

use crate::analyzer::dclint::rules::{LintContext, Rule, SimpleRule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL011";
const NAME: &str = "service-image-require-explicit-tag";
const DESCRIPTION: &str = "Service images should have explicit version tags.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/service-image-require-explicit-tag-rule.md";

pub fn rule() -> impl Rule {
    SimpleRule::new(
        CODE,
        NAME,
        Severity::Warning,
        RuleCategory::BestPractice,
        DESCRIPTION,
        URL,
        check,
    )
}

fn check(ctx: &LintContext) -> Vec<CheckFailure> {
    let mut failures = Vec::new();

    for (service_name, service) in &ctx.compose.services {
        if let Some(image) = &service.image {
            // Check if image has a tag
            let has_tag = image.contains(':');
            let is_latest = image.ends_with(":latest");
            let is_digest = image.contains('@'); // sha256 digest

            if !has_tag && !is_digest {
                let line = service
                    .image_pos
                    .map(|p| p.line)
                    .unwrap_or(service.position.line);

                let message = format!(
                    "Image \"{}\" in service \"{}\" does not have an explicit tag. Use a specific version tag for reproducible builds.",
                    image, service_name
                );

                failures.push(
                    make_failure(
                        &CODE.into(),
                        NAME,
                        Severity::Warning,
                        RuleCategory::BestPractice,
                        message,
                        line,
                        1,
                        false,
                    )
                    .with_data("serviceName", service_name.clone())
                    .with_data("image", image.clone()),
                );
            } else if is_latest {
                let line = service
                    .image_pos
                    .map(|p| p.line)
                    .unwrap_or(service.position.line);

                let message = format!(
                    "Image \"{}\" in service \"{}\" uses the `latest` tag. Use a specific version tag for reproducible builds.",
                    image, service_name
                );

                failures.push(
                    make_failure(
                        &CODE.into(),
                        NAME,
                        Severity::Warning,
                        RuleCategory::BestPractice,
                        message,
                        line,
                        1,
                        false,
                    )
                    .with_data("serviceName", service_name.clone())
                    .with_data("image", image.clone()),
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
    fn test_no_violation_explicit_tag() {
        let yaml = r#"
services:
  web:
    image: nginx:1.25
  db:
    image: postgres:15-alpine
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_no_violation_digest() {
        let yaml = r#"
services:
  web:
    image: nginx@sha256:abc123def456
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_no_tag() {
        let yaml = r#"
services:
  web:
    image: nginx
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("nginx"));
        assert!(failures[0].message.contains("explicit tag"));
    }

    #[test]
    fn test_violation_latest_tag() {
        let yaml = r#"
services:
  web:
    image: nginx:latest
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("latest"));
    }

    #[test]
    fn test_no_violation_no_image() {
        let yaml = r#"
services:
  web:
    build: .
"#;
        // Services with only build and no image are fine
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_multiple_violations() {
        let yaml = r#"
services:
  web:
    image: nginx
  db:
    image: postgres:latest
  cache:
    image: redis:7
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 2); // nginx (no tag) and postgres:latest
    }
}
