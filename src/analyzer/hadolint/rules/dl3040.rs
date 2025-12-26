//! DL3040: dnf clean all missing after dnf install
//!
//! Clean up dnf cache after installing packages.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3040",
        Severity::Warning,
        "`dnf clean all` missing after dnf install.",
        |instr, shell| match instr {
            Instruction::Run(_) => {
                if let Some(shell) = shell {
                    let has_install =
                        shell.any_command(|cmd| cmd.name == "dnf" && cmd.has_any_arg(&["install"]));

                    if !has_install {
                        return true;
                    }

                    let has_clean = shell.any_command(|cmd| {
                        (cmd.name == "dnf" && cmd.has_any_arg(&["clean"]))
                            || (cmd.name == "rm"
                                && cmd.arguments.iter().any(|a| a.contains("/var/cache/dnf")))
                    });

                    has_clean
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
    fn test_dnf_without_clean() {
        let result = lint_dockerfile("FROM fedora:latest\nRUN dnf install -y nginx");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3040"));
    }

    #[test]
    fn test_dnf_with_clean() {
        let result =
            lint_dockerfile("FROM fedora:latest\nRUN dnf install -y nginx && dnf clean all");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3040"));
    }
}
