//! DL3017: Do not use apk upgrade
//!
//! Using apk upgrade in a Dockerfile is not recommended
//! as it can lead to unpredictable builds.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3017",
        Severity::Warning,
        "Do not use `apk upgrade`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            cmd.name == "apk" && cmd.has_any_arg(&["upgrade"])
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
    use crate::analyzer::hadolint::lint::{lint, LintResult};
    use crate::analyzer::hadolint::config::HadolintConfig;

    fn lint_dockerfile(content: &str) -> LintResult {
        lint(content, &HadolintConfig::default())
    }

    #[test]
    fn test_apk_upgrade() {
        let result = lint_dockerfile("FROM alpine:3.18\nRUN apk upgrade");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3017"));
    }

    #[test]
    fn test_apk_update() {
        let result = lint_dockerfile("FROM alpine:3.18\nRUN apk update");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3017"));
    }

    #[test]
    fn test_apk_add() {
        let result = lint_dockerfile("FROM alpine:3.18\nRUN apk add --no-cache curl=8.0.0");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3017"));
    }
}
