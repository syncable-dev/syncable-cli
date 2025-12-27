//! DL3053: Label `org.opencontainers.image.title` is empty
//!
//! The title label should not be empty.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3053",
        Severity::Warning,
        "Label `org.opencontainers.image.title` is empty.",
        |instr, _shell| match instr {
            Instruction::Label(pairs) => {
                for (key, value) in pairs {
                    if key == "org.opencontainers.image.title" && value.trim().is_empty() {
                        return false;
                    }
                }
                true
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
    fn test_valid_title() {
        let result =
            lint_dockerfile("FROM ubuntu:20.04\nLABEL org.opencontainers.image.title=\"My App\"");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3053"));
    }

    #[test]
    fn test_empty_title() {
        let result =
            lint_dockerfile("FROM ubuntu:20.04\nLABEL org.opencontainers.image.title=\"\"");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3053"));
    }
}
