//! DL3023: COPY --from cannot reference its own FROM alias
//!
//! A COPY instruction cannot reference the current stage as the source.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{custom_rule, CustomRule, RuleState};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> CustomRule<impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync> {
    custom_rule(
        "DL3023",
        Severity::Error,
        "`COPY --from` cannot reference its own `FROM` alias.",
        |state, line, instr, _shell| {
            match instr {
                Instruction::From(base) => {
                    // Track current stage alias
                    if let Some(alias) = &base.alias {
                        state.data.set_string("current_stage", alias.as_str());
                    } else {
                        state.data.strings.remove("current_stage");
                    }
                    // Track current stage index
                    let stage_count = state.data.get_int("stage_count");
                    state.data.set_int("current_stage_index", stage_count);
                    state.data.set_int("stage_count", stage_count + 1);
                }
                Instruction::Copy(_, flags) => {
                    if let Some(from) = &flags.from {
                        // Check if referencing current stage
                        let is_current_alias = state.data.get_string("current_stage")
                            .map(|s| s == from)
                            .unwrap_or(false);

                        let is_current_index = from.parse::<i64>().ok()
                            .map(|n| n == state.data.get_int("current_stage_index"))
                            .unwrap_or(false);

                        if is_current_alias || is_current_index {
                            state.add_failure(
                                "DL3023",
                                Severity::Error,
                                "`COPY --from` cannot reference its own `FROM` alias.",
                                line,
                            );
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
    fn test_copy_from_same_stage() {
        let result = lint_dockerfile(
            "FROM node:18 AS builder\nCOPY --from=builder /app /app"
        );
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3023"));
    }

    #[test]
    fn test_copy_from_same_index() {
        let result = lint_dockerfile(
            "FROM node:18\nCOPY --from=0 /app /app"
        );
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3023"));
    }

    #[test]
    fn test_copy_from_different_stage() {
        let result = lint_dockerfile(
            "FROM node:18 AS builder\nRUN npm ci\nFROM node:18-alpine\nCOPY --from=builder /app /app"
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3023"));
    }

    #[test]
    fn test_copy_without_from() {
        let result = lint_dockerfile("FROM node:18 AS builder\nCOPY package.json /app/");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3023"));
    }
}
