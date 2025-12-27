//! DL3058: Label `org.opencontainers.image.url` is not a valid URL
//!
//! The url label should contain a valid URL.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3058",
        Severity::Warning,
        "Label `org.opencontainers.image.url` is not a valid URL.",
        |instr, _shell| match instr {
            Instruction::Label(pairs) => {
                for (key, value) in pairs {
                    if key == "org.opencontainers.image.url"
                        && !is_valid_url(value)
                    {
                        return false;
                    }
                }
                true
            }
            _ => true,
        },
    )
}

fn is_valid_url(url: &str) -> bool {
    if url.is_empty() {
        return false;
    }

    // Basic URL validation - must start with http:// or https://
    url.starts_with("http://") || url.starts_with("https://")
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
    fn test_valid_url() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nLABEL org.opencontainers.image.url=\"https://example.com\"",
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3058"));
    }

    #[test]
    fn test_invalid_url() {
        let result =
            lint_dockerfile("FROM ubuntu:20.04\nLABEL org.opencontainers.image.url=\"not-a-url\"");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3058"));
    }
}
