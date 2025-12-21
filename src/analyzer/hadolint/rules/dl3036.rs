//! DL3036: zypper clean missing after zypper install
//!
//! Clean up zypper cache after installing packages.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3036",
        Severity::Warning,
        "`zypper clean` missing after zypper install.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        let has_install = shell.any_command(|cmd| {
                            cmd.name == "zypper" && cmd.has_any_arg(&["install", "in"])
                        });

                        if !has_install {
                            return true;
                        }

                        let has_clean = shell.any_command(|cmd| {
                            (cmd.name == "zypper" && cmd.has_any_arg(&["clean", "cc"]))
                            || (cmd.name == "rm" && cmd.arguments.iter().any(|a| a.contains("/var/cache/zypp")))
                        });

                        has_clean
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
    fn test_zypper_without_clean() {
        let result = lint_dockerfile("FROM opensuse:latest\nRUN zypper -n install nginx");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3036"));
    }

    #[test]
    fn test_zypper_with_clean() {
        let result = lint_dockerfile("FROM opensuse:latest\nRUN zypper -n install nginx && zypper clean");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3036"));
    }
}
