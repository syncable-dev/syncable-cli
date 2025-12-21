//! DL3005: Do not use apt-get upgrade or dist-upgrade
//!
//! Using apt-get upgrade or dist-upgrade in a Dockerfile is not recommended
//! as it can lead to unpredictable builds.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3005",
        Severity::Warning,
        "Do not use `apt-get upgrade` or `dist-upgrade`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            cmd.name == "apt-get" && cmd.has_any_arg(&["upgrade", "dist-upgrade"])
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
    fn test_apt_get_upgrade() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get update && apt-get upgrade");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3005"));
    }

    #[test]
    fn test_apt_get_dist_upgrade() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get dist-upgrade");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3005"));
    }

    #[test]
    fn test_apt_get_update() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get update");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3005"));
    }

    #[test]
    fn test_apt_get_install() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get install -y nginx");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3005"));
    }
}
