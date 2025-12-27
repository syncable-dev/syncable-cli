//! DL3034: Non-interactive switch missing from zypper command
//!
//! zypper commands should use -n or --non-interactive.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3034",
        Severity::Warning,
        "Non-interactive switch missing from `zypper` command: `-n`.",
        |instr, shell| match instr {
            Instruction::Run(_) => {
                if let Some(shell) = shell {
                    !shell.any_command(|cmd| {
                        if cmd.name == "zypper" {
                            !cmd.has_any_flag(&["n", "non-interactive"])
                        } else {
                            false
                        }
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
    fn test_zypper_without_n() {
        let result = lint_dockerfile("FROM opensuse:latest\nRUN zypper refresh");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3034"));
    }

    #[test]
    fn test_zypper_with_n() {
        let result = lint_dockerfile("FROM opensuse:latest\nRUN zypper -n refresh");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3034"));
    }
}
