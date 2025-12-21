//! DL3031: Do not use yum update
//!
//! Using yum update in a Dockerfile is not recommended.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3031",
        Severity::Warning,
        "Do not use `yum update`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            cmd.name == "yum" && cmd.has_any_arg(&["update", "upgrade"])
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
    fn test_yum_update() {
        let result = lint_dockerfile("FROM centos:7\nRUN yum update -y");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3031"));
    }

    #[test]
    fn test_yum_install() {
        let result = lint_dockerfile("FROM centos:7\nRUN yum install -y nginx-1.20.0");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3031"));
    }
}
