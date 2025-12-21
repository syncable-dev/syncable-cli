//! DL3059: Multiple consecutive RUN instructions
//!
//! Combine consecutive RUN instructions to reduce the number of layers.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{custom_rule, CustomRule, RuleState};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> CustomRule<impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync> {
    custom_rule(
        "DL3059",
        Severity::Info,
        "Multiple consecutive `RUN` instructions. Consider consolidation.",
        |state, line, instr, _shell| {
            match instr {
                Instruction::From(_) => {
                    // Reset tracking for new stage
                    state.data.set_int("consecutive_runs", 0);
                    state.data.set_int("last_run_line", 0);
                }
                Instruction::Run(_) => {
                    let consecutive = state.data.get_int("consecutive_runs");
                    state.data.set_int("consecutive_runs", consecutive + 1);
                    state.data.set_int("last_run_line", line as i64);

                    // Report on the second consecutive RUN
                    if consecutive >= 1 {
                        state.add_failure(
                            "DL3059",
                            Severity::Info,
                            "Multiple consecutive `RUN` instructions. Consider consolidation.",
                            line,
                        );
                    }
                }
                // Other instructions reset the counter
                _ => {
                    state.data.set_int("consecutive_runs", 0);
                }
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
    fn test_consecutive_runs() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nRUN apt-get update\nRUN apt-get install -y nginx"
        );
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3059"));
    }

    #[test]
    fn test_single_run() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nRUN apt-get update && apt-get install -y nginx"
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3059"));
    }

    #[test]
    fn test_runs_separated_by_other() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nRUN apt-get update\nENV DEBIAN_FRONTEND=noninteractive\nRUN apt-get install -y nginx"
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3059"));
    }

    #[test]
    fn test_three_consecutive_runs() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nRUN echo 1\nRUN echo 2\nRUN echo 3"
        );
        // Should report on 2nd and 3rd RUN
        let count = result.failures.iter().filter(|f| f.code.as_str() == "DL3059").count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_different_stages() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04 AS stage1\nRUN echo 1\nFROM ubuntu:20.04 AS stage2\nRUN echo 2"
        );
        // Different stages, no consecutive RUNs
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3059"));
    }
}
