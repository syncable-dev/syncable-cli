//! DL4001: Either use wget or curl, but not both
//!
//! When downloading files, use either wget or curl consistently, not both.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{CheckFailure, RuleState, VeryCustomRule, very_custom_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> VeryCustomRule<
    impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync,
    impl Fn(RuleState) -> Vec<CheckFailure> + Send + Sync,
> {
    very_custom_rule(
        "DL4001",
        Severity::Warning,
        "Either use `wget` or `curl`, but not both.",
        |state, line, instr, shell| {
            if let Instruction::Run(_) = instr {
                if let Some(shell) = shell {
                    if shell.any_command(|cmd| cmd.name == "wget") {
                        // Store wget lines as comma-separated string
                        let existing = state
                            .data
                            .get_string("wget_lines")
                            .unwrap_or("")
                            .to_string();
                        let new = if existing.is_empty() {
                            line.to_string()
                        } else {
                            format!("{},{}", existing, line)
                        };
                        state.data.set_string("wget_lines", new);
                    }
                    if shell.any_command(|cmd| cmd.name == "curl") {
                        let existing = state
                            .data
                            .get_string("curl_lines")
                            .unwrap_or("")
                            .to_string();
                        let new = if existing.is_empty() {
                            line.to_string()
                        } else {
                            format!("{},{}", existing, line)
                        };
                        state.data.set_string("curl_lines", new);
                    }
                }
            }
        },
        |state| {
            let wget_lines = state.data.get_string("wget_lines").unwrap_or("");
            let curl_lines = state.data.get_string("curl_lines").unwrap_or("");

            // If both wget and curl are used, report failures
            if !wget_lines.is_empty() && !curl_lines.is_empty() {
                let mut failures = state.failures;
                for line in wget_lines.split(',').filter_map(|s| s.parse::<u32>().ok()) {
                    failures.push(CheckFailure::new(
                        "DL4001",
                        Severity::Warning,
                        "Either use `wget` or `curl`, but not both.",
                        line,
                    ));
                }
                for line in curl_lines.split(',').filter_map(|s| s.parse::<u32>().ok()) {
                    failures.push(CheckFailure::new(
                        "DL4001",
                        Severity::Warning,
                        "Either use `wget` or `curl`, but not both.",
                        line,
                    ));
                }
                failures
            } else {
                state.failures
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::config::HadolintConfig;
    use crate::analyzer::hadolint::lint::{LintResult, lint};

    fn lint_dockerfile(content: &str) -> LintResult {
        lint(content, &HadolintConfig::default())
    }

    #[test]
    fn test_only_wget() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN wget http://example.com/file");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL4001"));
    }

    #[test]
    fn test_only_curl() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN curl http://example.com/file");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL4001"));
    }

    #[test]
    fn test_both_wget_and_curl() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nRUN wget http://example.com/file\nRUN curl http://example.com/other",
        );
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL4001"));
    }
}
