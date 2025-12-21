//! DL3026: Use only an allowed registry in the FROM image
//!
//! Restricts base images to trusted registries configured in the config file.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3026",
        Severity::Error,
        "Use only an allowed registry in the FROM image.",
        |instr, _shell| {
            // This rule requires configuration to be useful
            // By default, we allow all registries
            // The actual check is done in lint.rs with config.allowed_registries
            match instr {
                Instruction::From(_) => {
                    // Always pass by default - config-dependent rule
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
    fn test_docker_hub_default() {
        // By default, all registries are allowed
        let result = lint_dockerfile("FROM ubuntu:20.04");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3026"));
    }

    #[test]
    fn test_custom_registry_default() {
        // By default, all registries are allowed
        let result = lint_dockerfile("FROM gcr.io/my-project/my-image:latest");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3026"));
    }
}
