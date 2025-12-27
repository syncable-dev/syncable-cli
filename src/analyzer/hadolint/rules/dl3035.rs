//! DL3035: Do not use zypper update
//!
//! Using zypper update in a Dockerfile is not recommended.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3035",
        Severity::Warning,
        "Do not use `zypper update`.",
        |instr, shell| match instr {
            Instruction::Run(_) => {
                if let Some(shell) = shell {
                    !shell.any_command(|cmd| {
                        cmd.name == "zypper" && cmd.has_any_arg(&["update", "up"])
                    })
                } else {
                    true
                }
            }
            _ => true,
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
    fn test_zypper_update() {
        let result = lint_dockerfile("FROM opensuse:latest\nRUN zypper -n update");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3035"));
    }

    #[test]
    fn test_zypper_install() {
        let result = lint_dockerfile("FROM opensuse:latest\nRUN zypper -n install nginx");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3035"));
    }
}
