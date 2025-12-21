//! DL3015: Avoid additional packages by specifying --no-install-recommends
//!
//! apt-get install should use --no-install-recommends to avoid
//! installing unnecessary packages.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3015",
        Severity::Info,
        "Avoid additional packages by specifying `--no-install-recommends`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        // Check all apt-get install commands
                        !shell.any_command(|cmd| {
                            if cmd.name == "apt-get" && cmd.has_any_arg(&["install"]) {
                                // Must have --no-install-recommends
                                !cmd.has_any_flag(&["no-install-recommends"])
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
    fn test_apt_get_without_no_install_recommends() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get install -y nginx");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3015"));
    }

    #[test]
    fn test_apt_get_with_no_install_recommends() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get install -y --no-install-recommends nginx");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3015"));
    }

    #[test]
    fn test_apt_get_update_no_flag_needed() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get update");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3015"));
    }
}
