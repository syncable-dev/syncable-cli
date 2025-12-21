//! DL3042: Avoid use of cache directory with pip
//!
//! Use --no-cache-dir with pip install to reduce image size.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3042",
        Severity::Warning,
        "Avoid use of cache directory with pip. Use `pip install --no-cache-dir <package>`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            if shell.is_pip_install(cmd) {
                                // Must have --no-cache-dir
                                !cmd.has_any_flag(&["no-cache-dir"])
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
    fn test_pip_install_without_no_cache() {
        let result = lint_dockerfile("FROM python:3.11\nRUN pip install flask");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3042"));
    }

    #[test]
    fn test_pip_install_with_no_cache() {
        let result = lint_dockerfile("FROM python:3.11\nRUN pip install --no-cache-dir flask");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3042"));
    }

    #[test]
    fn test_pip3_install_without_no_cache() {
        let result = lint_dockerfile("FROM python:3.11\nRUN pip3 install flask");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3042"));
    }

    #[test]
    fn test_pip3_install_with_no_cache() {
        let result = lint_dockerfile("FROM python:3.11\nRUN pip3 install --no-cache-dir flask");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3042"));
    }

    #[test]
    fn test_python_m_pip_without_no_cache() {
        let result = lint_dockerfile("FROM python:3.11\nRUN python -m pip install flask");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3042"));
    }

    #[test]
    fn test_pip_freeze() {
        // pip freeze doesn't need --no-cache-dir
        let result = lint_dockerfile("FROM python:3.11\nRUN pip freeze > requirements.txt");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3042"));
    }
}
