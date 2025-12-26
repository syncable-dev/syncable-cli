//! DL3062: COPY --from should reference a defined stage
//!
//! When using COPY --from, the source should be a defined build stage.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{CustomRule, RuleState, custom_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule()
-> CustomRule<impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync> {
    custom_rule(
        "DL3062",
        Severity::Warning,
        "`COPY --from` should reference a defined build stage or an external image.",
        |state, line, instr, _shell| {
            match instr {
                Instruction::From(base_image) => {
                    // Track stage aliases
                    if let Some(alias) = &base_image.alias {
                        state
                            .data
                            .insert_to_set("stages", alias.as_str().to_string());
                    }
                    // Track stage count
                    let count = state.data.get_int("stage_count");
                    state.data.insert_to_set("stages", count.to_string());
                    state.data.set_int("stage_count", count + 1);
                }
                Instruction::Copy(_, flags) => {
                    if let Some(from) = &flags.from {
                        let from_str = from.as_str();

                        // It's valid if:
                        // 1. It references a defined stage alias
                        // 2. It references a stage by index
                        // 3. It's an external image (contains / or . or : for tags)

                        let is_stage_alias = state.data.set_contains("stages", from_str);
                        let is_stage_index = from_str.parse::<usize>().is_ok();
                        let is_external = from_str.contains('/')
                            || from_str.contains('.')
                            || from_str.contains(':');

                        if !is_stage_alias && !is_stage_index && !is_external {
                            state.add_failure("DL3062", Severity::Warning, "`COPY --from` should reference a defined build stage or an external image.", line);
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
    use crate::analyzer::hadolint::config::HadolintConfig;
    use crate::analyzer::hadolint::lint::{LintResult, lint};

    fn lint_dockerfile(content: &str) -> LintResult {
        lint(content, &HadolintConfig::default())
    }

    #[test]
    fn test_copy_from_defined_stage() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04 AS builder\nRUN echo hello\nFROM alpine:3.14\nCOPY --from=builder /app /app",
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3062"));
    }

    #[test]
    fn test_copy_from_stage_index() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nRUN echo hello\nFROM alpine:3.14\nCOPY --from=0 /app /app",
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3062"));
    }

    #[test]
    fn test_copy_from_external_image() {
        let result =
            lint_dockerfile("FROM ubuntu:20.04\nCOPY --from=nginx:latest /etc/nginx /etc/nginx");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3062"));
    }

    #[test]
    fn test_copy_from_undefined_stage() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nCOPY --from=nonexistent /app /app");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3062"));
    }
}
