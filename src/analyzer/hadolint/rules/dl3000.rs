//! DL3000: Use absolute WORKDIR
//!
//! WORKDIR should use an absolute path to avoid confusion about the
//! starting directory.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3000",
        Severity::Error,
        "Use absolute WORKDIR",
        |instr, _shell| {
            match instr {
                Instruction::Workdir(path) => {
                    // Allow absolute paths and variables
                    path.starts_with('/') || path.starts_with('$') || is_windows_absolute(path)
                }
                _ => true,
            }
        },
    )
}

/// Check if path is a Windows absolute path.
fn is_windows_absolute(path: &str) -> bool {
    let chars: Vec<char> = path.chars().collect();
    chars.len() >= 2 && chars[0].is_ascii_alphabetic() && chars[1] == ':'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::rules::{Rule, RuleState};

    #[test]
    fn test_absolute_path() {
        let rule = rule();
        let mut state = RuleState::new();

        // Good: absolute path
        let instr = Instruction::Workdir("/app".to_string());
        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_relative_path() {
        let rule = rule();
        let mut state = RuleState::new();

        // Bad: relative path
        let instr = Instruction::Workdir("app".to_string());
        rule.check(&mut state, 1, &instr, None);
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3000");
    }

    #[test]
    fn test_variable_path() {
        let rule = rule();
        let mut state = RuleState::new();

        // Good: variable
        let instr = Instruction::Workdir("$APP_DIR".to_string());
        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }
}
