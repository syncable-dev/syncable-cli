//! DL3057: HEALTHCHECK instruction missing
//!
//! Images should have a HEALTHCHECK instruction to allow the container orchestrator
//! to monitor the health of the container.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{CheckFailure, RuleState, VeryCustomRule, very_custom_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> VeryCustomRule<
    impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync,
    impl Fn(RuleState) -> Vec<CheckFailure> + Send + Sync,
> {
    very_custom_rule(
        "DL3057",
        Severity::Info,
        "HEALTHCHECK instruction missing.",
        // Step function
        |state, _line, instr, _shell| {
            if matches!(instr, Instruction::Healthcheck(_)) {
                state.data.set_bool("has_healthcheck", true);
            }
            // Track if we have any real instructions (not just FROM)
            if !matches!(instr, Instruction::From(_) | Instruction::Comment(_)) {
                state.data.set_bool("has_instructions", true);
            }
        },
        // Finalize function - add failure if no healthcheck found
        |state| {
            // Only report if there are actual instructions beyond FROM
            if !state.data.get_bool("has_healthcheck") && state.data.get_bool("has_instructions") {
                let mut failures = state.failures;
                failures.push(CheckFailure::new(
                    "DL3057",
                    Severity::Info,
                    "HEALTHCHECK instruction missing.",
                    1,
                ));
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
    fn test_missing_healthcheck() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN echo hello");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3057"));
    }

    #[test]
    fn test_has_healthcheck() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nHEALTHCHECK CMD curl -f http://localhost/ || exit 1",
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3057"));
    }

    #[test]
    fn test_healthcheck_none() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nHEALTHCHECK NONE");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3057"));
    }
}
