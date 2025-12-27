//! DL3011: Valid UNIX ports range from 0 to 65535
//!
//! EXPOSE instruction must use valid port numbers.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3011",
        Severity::Error,
        "Valid UNIX ports range from 0 to 65535.",
        |instr, _shell| {
            match instr {
                Instruction::Expose(ports) => {
                    // All ports must be valid (0-65535)
                    // The parser already validates this as u16, so this should always pass
                    // But we check anyway for safety
                    ports.iter().all(|p| p.number <= 65535)
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
    fn test_valid_port() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nEXPOSE 8080");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3011"));
    }

    #[test]
    fn test_valid_multiple_ports() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nEXPOSE 80 443 8080");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3011"));
    }

    #[test]
    fn test_max_valid_port() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nEXPOSE 65535");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3011"));
    }

    #[test]
    fn test_min_valid_port() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nEXPOSE 0");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3011"));
    }
}
