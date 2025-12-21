//! DL3050: Superfluous label present
//!
//! Some labels are redundant or should use OCI annotation keys.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3050",
        Severity::Info,
        "Superfluous label present.",
        |instr, _shell| {
            match instr {
                Instruction::Label(pairs) => {
                    // Check for deprecated/superfluous labels that should use OCI keys
                    let deprecated_labels = [
                        "description",
                        "version",
                        "build-date",
                        "vcs-url",
                        "vcs-ref",
                        "vendor",
                        "name",
                        "url",
                        "documentation",
                        "source",
                        "licenses",
                        "title",
                        "revision",
                        "created",
                    ];

                    !pairs.iter().any(|(key, _)| {
                        let key_lower = key.to_lowercase();
                        deprecated_labels.contains(&key_lower.as_str())
                    })
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
    fn test_deprecated_description() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nLABEL description=\"Test image\"");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3050"));
    }

    #[test]
    fn test_oci_description() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nLABEL org.opencontainers.image.description=\"Test image\"");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3050"));
    }
}
