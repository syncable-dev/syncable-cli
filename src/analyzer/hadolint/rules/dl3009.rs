//! DL3009: Delete the apt-get lists after installing something
//!
//! After installing packages with apt-get, the package lists should be
//! removed to reduce image size.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3009",
        Severity::Info,
        "Delete the apt-get lists after installing something.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        // Check if apt-get install is used
                        let has_apt_install = shell.any_command(|cmd| {
                            cmd.name == "apt-get" && cmd.has_any_arg(&["install"])
                        });

                        if !has_apt_install {
                            return true;
                        }

                        // Check if lists are cleaned
                        let has_cleanup = shell.any_command(|cmd| {
                            // rm -rf /var/lib/apt/lists/*
                            (cmd.name == "rm" && cmd.arguments.iter().any(|arg| {
                                arg.contains("/var/lib/apt/lists")
                            }))
                            // Or apt-get clean
                            || (cmd.name == "apt-get" && cmd.has_any_arg(&["clean", "autoclean"]))
                        });

                        has_cleanup
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
    fn test_apt_get_without_cleanup() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get update && apt-get install -y nginx");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3009"));
    }

    #[test]
    fn test_apt_get_with_rm_cleanup() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nRUN apt-get update && apt-get install -y nginx && rm -rf /var/lib/apt/lists/*"
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3009"));
    }

    #[test]
    fn test_apt_get_with_clean() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nRUN apt-get update && apt-get install -y nginx && apt-get clean"
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3009"));
    }

    #[test]
    fn test_no_apt_get() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN echo hello");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3009"));
    }
}
