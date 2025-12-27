//! DL3019: Use --no-cache for apk add
//!
//! Use `apk add --no-cache` to avoid caching the index locally.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3019",
        Severity::Info,
        "Use the `--no-cache` switch to avoid the need to use `--update` and remove `/var/cache/apk/*`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            if cmd.name == "apk" && cmd.has_any_arg(&["add"]) {
                                // Must have --no-cache
                                !cmd.has_any_flag(&["no-cache"])
                            } else {
                                false
                            }
                        })
                    } else {
                        true
                    }
                }
                _ => true,
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
    fn test_apk_add_without_no_cache() {
        let result = lint_dockerfile("FROM alpine:3.18\nRUN apk add nginx=1.24.0");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3019"));
    }

    #[test]
    fn test_apk_add_with_no_cache() {
        let result = lint_dockerfile("FROM alpine:3.18\nRUN apk add --no-cache nginx=1.24.0");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3019"));
    }

    #[test]
    fn test_apk_update() {
        let result = lint_dockerfile("FROM alpine:3.18\nRUN apk update");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3019"));
    }
}
