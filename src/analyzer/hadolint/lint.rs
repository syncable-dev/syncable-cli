//! Main linting orchestration for hadolint-rs.
//!
//! This module ties together parsing, rules, and pragmas to provide
//! the main linting API.

use crate::analyzer::hadolint::config::HadolintConfig;
use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::parser::{InstructionPos, parse_dockerfile};
use crate::analyzer::hadolint::pragma::{PragmaState, extract_pragmas};
use crate::analyzer::hadolint::rules::{RuleState, all_rules};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::{CheckFailure, Severity};

use std::path::Path;

/// Result of linting a Dockerfile.
#[derive(Debug, Clone)]
pub struct LintResult {
    /// Rule violations found.
    pub failures: Vec<CheckFailure>,
    /// Parse errors (if any).
    pub parse_errors: Vec<String>,
}

impl LintResult {
    /// Create a new empty result.
    pub fn new() -> Self {
        Self {
            failures: Vec::new(),
            parse_errors: Vec::new(),
        }
    }

    /// Check if there are any failures.
    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    /// Check if there are any errors (failure with Error severity).
    pub fn has_errors(&self) -> bool {
        self.failures.iter().any(|f| f.severity == Severity::Error)
    }

    /// Check if there are any warnings (failure with Warning severity).
    pub fn has_warnings(&self) -> bool {
        self.failures
            .iter()
            .any(|f| f.severity == Severity::Warning)
    }

    /// Get the maximum severity in the results.
    pub fn max_severity(&self) -> Option<Severity> {
        self.failures.iter().map(|f| f.severity).max()
    }

    /// Check if the results should cause a non-zero exit.
    pub fn should_fail(&self, config: &HadolintConfig) -> bool {
        if config.no_fail {
            return false;
        }

        if let Some(max) = self.max_severity() {
            max >= config.failure_threshold
        } else {
            false
        }
    }

    /// Filter failures by severity threshold.
    pub fn filter_by_threshold(&mut self, threshold: Severity) {
        self.failures.retain(|f| f.severity >= threshold);
    }

    /// Sort failures by line number.
    pub fn sort(&mut self) {
        self.failures.sort();
    }
}

impl Default for LintResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Lint a Dockerfile string.
pub fn lint(content: &str, config: &HadolintConfig) -> LintResult {
    let mut result = LintResult::new();

    // Parse Dockerfile
    let instructions = match parse_dockerfile(content) {
        Ok(instrs) => instrs,
        Err(err) => {
            result.parse_errors.push(err.to_string());
            return result;
        }
    };

    // Extract pragmas
    let pragmas = if config.disable_ignore_pragma {
        PragmaState::new()
    } else {
        extract_pragmas(&instructions)
    };

    // Run rules
    let failures = run_rules(&instructions, config, &pragmas);

    // Filter by config
    result.failures = failures
        .into_iter()
        .filter(|f| {
            // Apply config severity overrides
            let effective_severity = config.effective_severity(&f.code, f.severity);

            // Filter by threshold
            effective_severity >= config.failure_threshold
        })
        .filter(|f| !config.is_rule_ignored(&f.code))
        .filter(|f| !pragmas.is_ignored(&f.code, f.line))
        .map(|mut f| {
            // Apply severity overrides
            f.severity = config.effective_severity(&f.code, f.severity);
            f
        })
        .collect();

    // Sort by line number
    result.sort();

    result
}

/// Lint a Dockerfile from a file path.
pub fn lint_file(path: &Path, config: &HadolintConfig) -> LintResult {
    match std::fs::read_to_string(path) {
        Ok(content) => lint(&content, config),
        Err(err) => {
            let mut result = LintResult::new();
            result
                .parse_errors
                .push(format!("Failed to read file: {}", err));
            result
        }
    }
}

/// Run all enabled rules on the instructions.
fn run_rules(
    instructions: &[InstructionPos],
    config: &HadolintConfig,
    pragmas: &PragmaState,
) -> Vec<CheckFailure> {
    let rules = all_rules();
    let mut all_failures = Vec::new();

    for rule in rules {
        // Skip ignored rules
        if config.is_rule_ignored(rule.code()) {
            continue;
        }

        let mut state = RuleState::new();

        // Process each instruction
        for instr in instructions {
            // Parse shell if this is a RUN instruction
            let shell = match &instr.instruction {
                Instruction::Run(args) => Some(ParsedShell::from_run_args(args)),
                _ => None,
            };

            // Check the instruction
            rule.check(
                &mut state,
                instr.line_number,
                &instr.instruction,
                shell.as_ref(),
            );

            // Also check ONBUILD contents
            if let Instruction::OnBuild(inner) = &instr.instruction {
                let inner_shell = match inner.as_ref() {
                    Instruction::Run(args) => Some(ParsedShell::from_run_args(args)),
                    _ => None,
                };
                rule.check(
                    &mut state,
                    instr.line_number,
                    inner.as_ref(),
                    inner_shell.as_ref(),
                );
            }
        }

        // Finalize the rule
        let failures = rule.finalize(state);
        all_failures.extend(failures);
    }

    all_failures
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lint_empty() {
        let result = lint("", &HadolintConfig::default());
        assert!(result.failures.is_empty());
    }

    #[test]
    fn test_lint_valid_dockerfile() {
        let dockerfile = r#"
FROM ubuntu:20.04
WORKDIR /app
COPY . .
CMD ["./app"]
"#;
        let result = lint(dockerfile, &HadolintConfig::default());
        // Should have no DL3000 (WORKDIR is absolute)
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3000"));
    }

    #[test]
    fn test_lint_relative_workdir() {
        let dockerfile = r#"
FROM ubuntu:20.04
WORKDIR app
"#;
        let result = lint(dockerfile, &HadolintConfig::default());
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3000"));
    }

    #[test]
    fn test_lint_maintainer() {
        let dockerfile = r#"
FROM ubuntu:20.04
MAINTAINER John Doe <john@example.com>
"#;
        let result = lint(dockerfile, &HadolintConfig::default());
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL4000"));
    }

    #[test]
    fn test_lint_untagged_image() {
        let dockerfile = "FROM ubuntu\n";
        let result = lint(dockerfile, &HadolintConfig::default());
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3006"));
    }

    #[test]
    fn test_lint_latest_tag() {
        let dockerfile = "FROM ubuntu:latest\n";
        let result = lint(dockerfile, &HadolintConfig::default());
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3007"));
    }

    #[test]
    fn test_lint_ignore_pragma() {
        let dockerfile = r#"
# hadolint ignore=DL3006
FROM ubuntu
"#;
        let result = lint(dockerfile, &HadolintConfig::default());
        // DL3006 should be ignored
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3006"));
    }

    #[test]
    fn test_lint_config_ignore() {
        let dockerfile = "FROM ubuntu\n";
        let config = HadolintConfig::default().ignore("DL3006");
        let result = lint(dockerfile, &config);
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3006"));
    }

    #[test]
    fn test_lint_threshold() {
        let dockerfile = r#"
FROM ubuntu
MAINTAINER John
"#;
        let mut config = HadolintConfig::default();
        config.failure_threshold = Severity::Error;
        let result = lint(dockerfile, &config);
        // DL3006 (warning) should be filtered out
        // DL4000 (error) should remain
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3006"));
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL4000"));
    }

    #[test]
    fn test_should_fail() {
        let dockerfile = "FROM ubuntu:latest\n";
        let config = HadolintConfig::default().with_threshold(Severity::Warning);
        let result = lint(dockerfile, &config);

        // DL3007 is a warning, should trigger failure with Warning threshold
        assert!(result.should_fail(&config));

        // With no_fail, should not fail
        let mut no_fail_config = config.clone();
        no_fail_config.no_fail = true;
        assert!(!result.should_fail(&no_fail_config));
    }

    #[test]
    fn test_lint_sudo() {
        let dockerfile = r#"
FROM ubuntu:20.04
RUN sudo apt-get update
"#;
        let result = lint(dockerfile, &HadolintConfig::default());
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3004"));
    }

    #[test]
    fn test_lint_cd() {
        let dockerfile = r#"
FROM ubuntu:20.04
RUN cd /app && npm install
"#;
        let result = lint(dockerfile, &HadolintConfig::default());
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3003"));
    }

    #[test]
    fn test_lint_shell_form_cmd() {
        let dockerfile = r#"
FROM ubuntu:20.04
CMD node app.js
"#;
        let result = lint(dockerfile, &HadolintConfig::default());
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3025"));
    }

    #[test]
    fn test_lint_exec_form_cmd() {
        let dockerfile = r#"
FROM ubuntu:20.04
CMD ["node", "app.js"]
"#;
        let result = lint(dockerfile, &HadolintConfig::default());
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3025"));
    }

    #[test]
    fn test_lint_error_dockerfile() {
        // Comprehensive test Dockerfile with many intentional errors
        let dockerfile = r#"
# Test Dockerfile with maximum hadolint errors
MAINTAINER bad@example.com

FROM ubuntu:latest

LABEL maintainer="test@test.com" \
      description="" \
      org.opencontainers.image.created="not-a-date" \
      org.opencontainers.image.licenses="INVALID" \
      org.opencontainers.image.title="" \
      org.opencontainers.image.description="" \
      org.opencontainers.image.documentation="not-url" \
      org.opencontainers.image.source="not-url" \
      org.opencontainers.image.url="not-url"

ENV FOO=bar BAR=$FOO

COPY package.json app/

WORKDIR relative/path

RUN apt update
RUN apt-get upgrade
RUN apt-get install curl wget nginx

RUN sudo useradd -m testuser

RUN cd /app && echo "hello"

RUN pip install flask requests

RUN npm install -g express

RUN gem install rails

FROM alpine:latest AS alpine-stage
RUN apk upgrade
RUN apk add nginx

FROM centos:latest AS centos-stage
RUN yum update -y
RUN yum install -y httpd

FROM fedora:latest AS fedora-stage
RUN dnf update
RUN dnf install nginx

FROM ubuntu:latest AS builder
FROM debian:latest AS builder

ADD https://example.com/file.txt /app/
ADD localfile.txt /app/

COPY --from=nonexistent /app /app

EXPOSE 99999

RUN ln -s /bin/bash /bin/sh

RUN curl http://example.com | grep pattern

RUN wget http://example.com/file1
RUN curl http://example.com/file2

ENTRYPOINT /bin/bash start.sh

CMD echo "first"
CMD echo "second"

ENTRYPOINT ["python"]
ENTRYPOINT ["node"]

HEALTHCHECK CMD curl localhost
HEALTHCHECK CMD wget localhost

USER root
"#;
        let result = lint(dockerfile, &HadolintConfig::default());

        // Collect unique rule codes triggered
        let mut triggered_rules: Vec<&str> =
            result.failures.iter().map(|f| f.code.as_str()).collect();
        triggered_rules.sort();
        triggered_rules.dedup();

        // Print summary for debugging
        println!("\n=== HADOLINT ERROR DOCKERFILE TEST ===");
        println!("Total violations: {}", result.failures.len());
        println!("Unique rules triggered: {}", triggered_rules.len());
        println!("\nRules triggered:");
        for rule in &triggered_rules {
            let count = result
                .failures
                .iter()
                .filter(|f| f.code.as_str() == *rule)
                .count();
            println!("  {} ({}x)", rule, count);
        }

        // Verify we catch many rules
        assert!(
            triggered_rules.len() >= 30,
            "Expected at least 30 different rules, got {}",
            triggered_rules.len()
        );

        // Verify some key rules are triggered
        assert!(triggered_rules.contains(&"DL3000"), "DL3000 not triggered");
        assert!(triggered_rules.contains(&"DL3004"), "DL3004 not triggered");
        assert!(triggered_rules.contains(&"DL3007"), "DL3007 not triggered");
        assert!(triggered_rules.contains(&"DL3027"), "DL3027 not triggered");
        assert!(triggered_rules.contains(&"DL4000"), "DL4000 not triggered");
        assert!(triggered_rules.contains(&"DL4003"), "DL4003 not triggered");
        assert!(triggered_rules.contains(&"DL4004"), "DL4004 not triggered");
    }
}
