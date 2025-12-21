//! DL3047: wget vs curl consistency
//!
//! Avoid using both wget and curl in the same Dockerfile.
//! Pick one to reduce image size.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{custom_rule, CustomRule, RuleState};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> CustomRule<impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync> {
    custom_rule(
        "DL3047",
        Severity::Info,
        "Avoid using both `wget` and `curl` since they serve the same purpose.",
        |state, line, instr, shell| {
            match instr {
                Instruction::From(_) => {
                    // Reset tracking for new stage
                    state.data.set_bool("seen_wget", false);
                    state.data.set_bool("seen_curl", false);
                    state.data.set_bool("reported_dl3047", false);
                }
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        let uses_wget = shell.using_program("wget");
                        let uses_curl = shell.using_program("curl");

                        if uses_wget {
                            state.data.set_bool("seen_wget", true);
                        }
                        if uses_curl {
                            state.data.set_bool("seen_curl", true);
                        }

                        // Report if both are now seen and not already reported
                        let seen_both = state.data.get_bool("seen_wget") && state.data.get_bool("seen_curl");
                        let already_reported = state.data.get_bool("reported_dl3047");

                        if seen_both && !already_reported {
                            state.add_failure(
                                "DL3047",
                                Severity::Info,
                                "Avoid using both `wget` and `curl` since they serve the same purpose.",
                                line,
                            );
                            state.data.set_bool("reported_dl3047", true);
                        }
                    }
                }
                _ => {}
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::lint::{lint, LintResult};
    use crate::analyzer::hadolint::config::HadolintConfig;

    fn lint_dockerfile(content: &str) -> LintResult {
        lint(content, &HadolintConfig::default())
    }

    #[test]
    fn test_wget_only() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN wget https://example.com/file");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3047"));
    }

    #[test]
    fn test_curl_only() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN curl -O https://example.com/file");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3047"));
    }

    #[test]
    fn test_both_wget_and_curl() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nRUN wget https://example.com/file1\nRUN curl -O https://example.com/file2"
        );
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3047"));
    }

    #[test]
    fn test_both_in_same_run() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nRUN wget https://a.com/f && curl -O https://b.com/g"
        );
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3047"));
    }

    #[test]
    fn test_different_stages() {
        // Different stages should track separately
        let result = lint_dockerfile(
            "FROM ubuntu:20.04 AS stage1\nRUN wget https://a.com/f\nFROM ubuntu:20.04 AS stage2\nRUN curl https://b.com/g"
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3047"));
    }
}
