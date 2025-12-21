//! DL3030: Use the --yes switch to avoid prompts for zypper install
//!
//! zypper install should use --non-interactive or -n to avoid prompts.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3030",
        Severity::Warning,
        "Use the `--non-interactive` switch to avoid prompts during `zypper` install.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            if cmd.name == "zypper" && cmd.has_any_arg(&["install", "in"]) {
                                !cmd.has_any_flag(&["n", "non-interactive", "no-confirm", "y"])
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
    use crate::analyzer::hadolint::lint::{lint, LintResult};
    use crate::analyzer::hadolint::config::HadolintConfig;

    fn lint_dockerfile(content: &str) -> LintResult {
        lint(content, &HadolintConfig::default())
    }

    #[test]
    fn test_zypper_without_flag() {
        let result = lint_dockerfile("FROM opensuse:latest\nRUN zypper install nginx");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3030"));
    }

    #[test]
    fn test_zypper_with_n() {
        let result = lint_dockerfile("FROM opensuse:latest\nRUN zypper -n install nginx");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3030"));
    }

    #[test]
    fn test_zypper_with_non_interactive() {
        let result = lint_dockerfile("FROM opensuse:latest\nRUN zypper --non-interactive install nginx");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3030"));
    }
}
