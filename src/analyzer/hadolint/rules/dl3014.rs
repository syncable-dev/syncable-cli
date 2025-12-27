//! DL3014: Use the -y switch to avoid manual input
//!
//! apt-get install should use -y to avoid prompts during build.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3014",
        Severity::Warning,
        "Use the `-y` switch to avoid manual input `apt-get -y install <package>`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        // Check all apt-get install commands
                        !shell.any_command(|cmd| {
                            if cmd.name == "apt-get" && cmd.has_any_arg(&["install"]) {
                                // Must have -y, --yes, or --assume-yes
                                !cmd.has_any_flag(&["y", "yes", "assume-yes"])
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
    use crate::analyzer::hadolint::config::HadolintConfig;
    use crate::analyzer::hadolint::lint::{LintResult, lint};

    fn lint_dockerfile(content: &str) -> LintResult {
        lint(content, &HadolintConfig::default())
    }

    #[test]
    fn test_apt_get_without_y() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get install nginx");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3014"));
    }

    #[test]
    fn test_apt_get_with_y() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get install -y nginx");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3014"));
    }

    #[test]
    fn test_apt_get_with_yes() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get install --yes nginx");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3014"));
    }

    #[test]
    fn test_apt_get_with_assume_yes() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get install --assume-yes nginx");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3014"));
    }

    #[test]
    fn test_apt_get_update_no_y() {
        // apt-get update doesn't need -y
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN apt-get update");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3014"));
    }
}
