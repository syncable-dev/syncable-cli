//! DL3054: Label `org.opencontainers.image.description` is empty
//!
//! The description label should not be empty.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3054",
        Severity::Warning,
        "Label `org.opencontainers.image.description` is empty.",
        |instr, _shell| {
            match instr {
                Instruction::Label(pairs) => {
                    for (key, value) in pairs {
                        if key == "org.opencontainers.image.description" && value.trim().is_empty() {
                            return false;
                        }
                    }
                    true
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
    fn test_valid_description() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nLABEL org.opencontainers.image.description=\"A description\"");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3054"));
    }

    #[test]
    fn test_empty_description() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nLABEL org.opencontainers.image.description=\"\"");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3054"));
    }
}
