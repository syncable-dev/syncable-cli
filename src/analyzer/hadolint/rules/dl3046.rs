//! DL3046: useradd without -l flag may result in large layers
//!
//! When adding a user with useradd, use the -l flag to avoid creating
//! large layers due to /var/log/lastlog growing.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3046",
        Severity::Warning,
        "`useradd` without flag `-l` and target UID not within `/etc/login.defs` may result in excessively large image.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            if cmd.name == "useradd" {
                                // Check if -l or --no-log-init flag is present
                                // Also check combined flags like -lm
                                let has_l_flag = cmd.arguments.iter().any(|a| {
                                    a == "-l" || a == "--no-log-init" ||
                                    (a.starts_with('-') && !a.starts_with("--") && a.contains('l'))
                                });
                                !has_l_flag
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
    fn test_useradd_without_l() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN useradd -m myuser");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3046"));
    }

    #[test]
    fn test_useradd_with_l() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN useradd -l -m myuser");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3046"));
    }

    #[test]
    fn test_useradd_with_no_log_init() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN useradd --no-log-init -m myuser");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3046"));
    }
}
