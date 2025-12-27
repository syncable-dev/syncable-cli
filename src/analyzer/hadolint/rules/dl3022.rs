//! DL3022: COPY --from should reference a previously defined FROM alias
//!
//! When using multi-stage builds, COPY --from should reference a stage
//! that was previously defined.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{CustomRule, RuleState, custom_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule()
-> CustomRule<impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync> {
    custom_rule(
        "DL3022",
        Severity::Warning,
        "`COPY --from` should reference a previously defined `FROM` alias.",
        |state, line, instr, _shell| {
            match instr {
                Instruction::From(base) => {
                    // Track stage aliases
                    if let Some(alias) = &base.alias {
                        state.data.insert_to_set("stage_aliases", alias.as_str());
                    }
                    // Track stage index
                    let stage_count = state.data.get_int("stage_count");
                    state.data.set_int("stage_count", stage_count + 1);
                }
                Instruction::Copy(_, flags) => {
                    if let Some(from) = &flags.from {
                        // Check if it's a stage reference
                        // It's valid if:
                        // 1. It's a known alias
                        // 2. It's a numeric index less than current stage count
                        // 3. It's an external image reference

                        let is_known_alias = state.data.set_contains("stage_aliases", from);
                        let is_numeric_index = from
                            .parse::<i64>()
                            .ok()
                            .map(|n| n < state.data.get_int("stage_count"))
                            .unwrap_or(false);

                        // If it looks like an image name (contains / or :), allow it
                        let is_external_image = from.contains('/') || from.contains(':');

                        if !is_known_alias && !is_numeric_index && !is_external_image {
                            state.add_failure(
                                "DL3022",
                                Severity::Warning,
                                format!("`COPY --from={}` references an undefined stage.", from),
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
    use crate::analyzer::hadolint::config::HadolintConfig;
    use crate::analyzer::hadolint::lint::{LintResult, lint};

    fn lint_dockerfile(content: &str) -> LintResult {
        lint(content, &HadolintConfig::default())
    }

    #[test]
    fn test_copy_from_valid_alias() {
        let result = lint_dockerfile(
            "FROM node:18 AS builder\nRUN npm ci\nFROM node:18-alpine\nCOPY --from=builder /app /app",
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3022"));
    }

    #[test]
    fn test_copy_from_invalid_alias() {
        let result =
            lint_dockerfile("FROM node:18\nFROM node:18-alpine\nCOPY --from=nonexistent /app /app");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3022"));
    }

    #[test]
    fn test_copy_from_numeric_index() {
        let result = lint_dockerfile(
            "FROM node:18\nRUN npm ci\nFROM node:18-alpine\nCOPY --from=0 /app /app",
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3022"));
    }

    #[test]
    fn test_copy_from_external_image() {
        let result = lint_dockerfile(
            "FROM node:18\nCOPY --from=nginx:latest /etc/nginx/nginx.conf /etc/nginx/",
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3022"));
    }

    #[test]
    fn test_copy_without_from() {
        let result = lint_dockerfile("FROM node:18\nCOPY package.json /app/");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3022"));
    }
}
