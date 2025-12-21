//! DL3048: Invalid label key
//!
//! Label keys should follow the OCI annotation specification.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3048",
        Severity::Style,
        "Invalid label key.",
        |instr, _shell| {
            match instr {
                Instruction::Label(pairs) => {
                    pairs.iter().all(|(key, _)| is_valid_label_key(key))
                }
                _ => true,
            }
        },
    )
}

fn is_valid_label_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }

    // Label keys must start with a letter or number
    let first_char = key.chars().next().unwrap();
    if !first_char.is_ascii_alphanumeric() {
        return false;
    }

    // Label keys can only contain alphanumeric, -, _, .
    key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
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
    fn test_valid_label() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nLABEL maintainer=\"test@test.com\"");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3048"));
    }

    #[test]
    fn test_valid_oci_label() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nLABEL org.opencontainers.image.title=\"Test\"");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3048"));
    }

    #[test]
    fn test_invalid_label_special_char() {
        // Note: The parser may not accept labels starting with special chars,
        // so this test validates the rule itself works with the unit test approach
        use crate::analyzer::hadolint::rules::{Rule, RuleState};
        use crate::analyzer::hadolint::parser::instruction::Instruction;

        let rule = rule();
        let mut state = RuleState::new();

        // Manually test with an invalid key starting with @
        let instr = Instruction::Label(vec![("@invalid".to_string(), "test".to_string())]);
        rule.check(&mut state, 1, &instr, None);

        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3048");
    }
}
