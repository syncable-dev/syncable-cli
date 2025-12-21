//! DL3049: Label `maintainer` is deprecated
//!
//! The maintainer label is deprecated. Use org.opencontainers.image.authors instead.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3049",
        Severity::Info,
        "Label `maintainer` is deprecated, use `org.opencontainers.image.authors` instead.",
        |instr, _shell| {
            match instr {
                Instruction::Label(pairs) => {
                    !pairs.iter().any(|(key, _)| key.to_lowercase() == "maintainer")
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
    fn test_maintainer_label() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nLABEL maintainer=\"test@test.com\"");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3049"));
    }

    #[test]
    fn test_oci_authors_label() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nLABEL org.opencontainers.image.authors=\"test@test.com\"");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3049"));
    }
}
