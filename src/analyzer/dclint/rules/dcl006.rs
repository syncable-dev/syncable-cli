//! DCL006: no-version-field
//!
//! The `version` field is deprecated and should be removed.

use crate::analyzer::dclint::rules::{FixableRule, LintContext, Rule, make_failure};
use crate::analyzer::dclint::types::{CheckFailure, RuleCategory, Severity};

const CODE: &str = "DCL006";
const NAME: &str = "no-version-field";
const DESCRIPTION: &str = "The `version` field is deprecated in Docker Compose.";
const URL: &str = "https://github.com/zavoloklom/docker-compose-linter/blob/main/docs/rules/no-version-field-rule.md";

pub fn rule() -> impl Rule {
    FixableRule::new(
        CODE,
        NAME,
        Severity::Info,
        RuleCategory::Style,
        DESCRIPTION,
        URL,
        check,
        fix,
    )
}

fn check(ctx: &LintContext) -> Vec<CheckFailure> {
    let mut failures = Vec::new();

    if ctx.compose.version.is_some() {
        let line = ctx.compose.version_pos.map(|p| p.line).unwrap_or(1);

        let message = "The `version` field is obsolete and should be removed. Docker Compose now infers the version from the file structure.".to_string();

        failures.push(
            make_failure(
                &CODE.into(),
                NAME,
                Severity::Info,
                RuleCategory::Style,
                message,
                line,
                1,
                true,
            )
            .with_data("version", ctx.compose.version.clone().unwrap_or_default()),
        );
    }

    failures
}

fn fix(source: &str) -> Option<String> {
    let mut result = Vec::new();
    let mut modified = false;
    let mut skip_next_empty = false;

    for line in source.lines() {
        let trimmed = line.trim();

        // Skip version line
        if trimmed.starts_with("version:") {
            modified = true;
            skip_next_empty = true;
            continue;
        }

        // Skip empty line after version
        if skip_next_empty && trimmed.is_empty() {
            skip_next_empty = false;
            continue;
        }
        skip_next_empty = false;

        result.push(line);
    }

    if modified {
        let mut output = result.join("\n");
        if source.ends_with('\n') {
            output.push('\n');
        }
        Some(output)
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
    fn test_no_violation_no_version() {
        let yaml = r#"
services:
  web:
    image: nginx
"#;
        assert!(check_yaml(yaml).is_empty());
    }

    #[test]
    fn test_violation_has_version() {
        let yaml = r#"
version: "3.8"
services:
  web:
    image: nginx
"#;
        let failures = check_yaml(yaml);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("obsolete"));
    }

    #[test]
    fn test_fix_removes_version() {
        let yaml = r#"version: "3.8"

services:
  web:
    image: nginx
"#;
        let fixed = fix(yaml).unwrap();
        assert!(!fixed.contains("version"));
        assert!(fixed.contains("services"));
    }

    #[test]
    fn test_fix_no_change_when_no_version() {
        let yaml = r#"services:
  web:
    image: nginx
"#;
        assert!(fix(yaml).is_none());
    }
}
