//! DL3012: Multiple HEALTHCHECK instructions
//!
//! Only one HEALTHCHECK instruction is allowed per stage.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{custom_rule, CustomRule, RuleState};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> CustomRule<impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync> {
    custom_rule(
        "DL3012",
        Severity::Error,
        "Multiple `HEALTHCHECK` instructions.",
        |state, line, instr, _shell| {
            match instr {
                Instruction::From(_) => {
                    // Reset healthcheck count for new stage
                    state.data.set_int("healthcheck_count", 0);
                }
                Instruction::Healthcheck(_) => {
                    let count = state.data.get_int("healthcheck_count");
                    if count > 0 {
                        state.add_failure(
                            "DL3012",
                            Severity::Error,
                            "Multiple `HEALTHCHECK` instructions.",
                            line,
                        );
                    }
                    state.data.set_int("healthcheck_count", count + 1);
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
    fn test_single_healthcheck() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nHEALTHCHECK CMD curl -f http://localhost/ || exit 1"
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3012"));
    }

    #[test]
    fn test_multiple_healthchecks() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nHEALTHCHECK CMD curl http://localhost/\nHEALTHCHECK CMD wget http://localhost/"
        );
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3012"));
    }

    #[test]
    fn test_healthcheck_different_stages() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04 AS builder\nHEALTHCHECK CMD curl http://localhost/\nFROM ubuntu:20.04\nHEALTHCHECK CMD wget http://localhost/"
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3012"));
    }

    #[test]
    fn test_no_healthcheck() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN echo hello");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3012"));
    }
}
