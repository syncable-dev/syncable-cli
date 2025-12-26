//! DL3038: Use the -y switch to avoid prompts for dnf install
//!
//! dnf install should use -y to avoid prompts.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3038",
        Severity::Warning,
        "Use the `-y` switch to avoid prompts during `dnf install`.",
        |instr, shell| match instr {
            Instruction::Run(_) => {
                if let Some(shell) = shell {
                    !shell.any_command(|cmd| {
                        if cmd.name == "dnf" && cmd.has_any_arg(&["install"]) {
                            !cmd.has_any_flag(&["y", "yes", "assumeyes"])
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
    fn test_dnf_without_y() {
        let result = lint_dockerfile("FROM fedora:latest\nRUN dnf install nginx");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3038"));
    }

    #[test]
    fn test_dnf_with_y() {
        let result = lint_dockerfile("FROM fedora:latest\nRUN dnf install -y nginx");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3038"));
    }
}
