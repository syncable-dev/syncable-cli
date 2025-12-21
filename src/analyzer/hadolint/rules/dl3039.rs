//! DL3039: Do not use dnf update
//!
//! Using dnf update in a Dockerfile is not recommended.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3039",
        Severity::Warning,
        "Do not use `dnf update`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            cmd.name == "dnf" && cmd.has_any_arg(&["update", "upgrade"])
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
    fn test_dnf_update() {
        let result = lint_dockerfile("FROM fedora:latest\nRUN dnf update -y");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3039"));
    }

    #[test]
    fn test_dnf_install() {
        let result = lint_dockerfile("FROM fedora:latest\nRUN dnf install -y nginx");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3039"));
    }
}
