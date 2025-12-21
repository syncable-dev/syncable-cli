//! DL3061: Invalid image name in FROM
//!
//! The image name in FROM should be valid.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3061",
        Severity::Error,
        "Invalid image name in `FROM`.",
        |instr, _shell| {
            match instr {
                Instruction::From(base_image) => {
                    is_valid_image_name(&base_image.image.name)
                }
                _ => true,
            }
        },
    )
}

fn is_valid_image_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Allow scratch as a special case
    if name == "scratch" {
        return true;
    }

    // Allow variable expansion
    if name.starts_with('$') {
        return true;
    }

    // Image name can have:
    // - Registry prefix: registry.example.com/
    // - Namespace: namespace/
    // - Name: imagename

    // Basic validation: should contain only valid chars
    let valid_chars = |c: char| {
        c.is_ascii_lowercase()
            || c.is_ascii_digit()
            || c == '-'
            || c == '_'
            || c == '.'
            || c == '/'
            || c == ':'
    };

    name.chars().all(valid_chars)
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
    fn test_valid_image() {
        let result = lint_dockerfile("FROM ubuntu:20.04");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3061"));
    }

    #[test]
    fn test_valid_registry_image() {
        let result = lint_dockerfile("FROM registry.example.com/myimage:latest");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3061"));
    }

    #[test]
    fn test_scratch() {
        let result = lint_dockerfile("FROM scratch");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3061"));
    }

    #[test]
    fn test_variable_image() {
        let result = lint_dockerfile("ARG BASE=ubuntu\nFROM $BASE");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3061"));
    }
}
