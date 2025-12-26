//! DL3002: Last USER should not be root
//!
//! Running as root in containers is a security risk. The last USER
//! instruction should switch to a non-root user.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{CustomRule, RuleState, custom_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule()
-> CustomRule<impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync> {
    custom_rule(
        "DL3002",
        Severity::Warning,
        "Last USER should not be root",
        |state, line, instr, _shell| {
            match instr {
                Instruction::From(_) => {
                    // Reset state for each stage
                    state.data.set_bool("is_root", true);
                    state.data.set_int("last_user_line", 0);
                }
                Instruction::User(user) => {
                    let is_root = user == "root" || user == "0" || user.starts_with("root:");
                    state.data.set_bool("is_root", is_root);
                    state.data.set_int("last_user_line", line as i64);
                }
                _ => {}
            }
        },
    )
}

/// Custom finalize implementation for DL3002.
/// This is called manually in the lint process.
pub fn finalize(state: RuleState) -> Vec<crate::analyzer::hadolint::types::CheckFailure> {
    let mut failures = state.failures;

    // Check if the last USER was root
    if state.data.get_bool("is_root") {
        let last_line = state.data.get_int("last_user_line");
        if last_line > 0 {
            failures.push(crate::analyzer::hadolint::types::CheckFailure::new(
                "DL3002",
                Severity::Warning,
                "Last USER should not be root",
                last_line as u32,
            ));
        }
    }

    failures
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::parser::instruction::BaseImage;
    use crate::analyzer::hadolint::rules::Rule;

    #[test]
    fn test_non_root_user() {
        let rule = rule();
        let mut state = RuleState::new();

        let from = Instruction::From(BaseImage::new("ubuntu"));
        let user = Instruction::User("appuser".to_string());

        rule.check(&mut state, 1, &from, None);
        rule.check(&mut state, 2, &user, None);

        let failures = finalize(state);
        assert!(failures.is_empty());
    }

    #[test]
    fn test_root_user() {
        let rule = rule();
        let mut state = RuleState::new();

        let from = Instruction::From(BaseImage::new("ubuntu"));
        let user = Instruction::User("root".to_string());

        rule.check(&mut state, 1, &from, None);
        rule.check(&mut state, 2, &user, None);

        let failures = finalize(state);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].code.as_str(), "DL3002");
    }

    #[test]
    fn test_switch_from_root() {
        let rule = rule();
        let mut state = RuleState::new();

        let from = Instruction::From(BaseImage::new("ubuntu"));
        let user1 = Instruction::User("root".to_string());
        let user2 = Instruction::User("appuser".to_string());

        rule.check(&mut state, 1, &from, None);
        rule.check(&mut state, 2, &user1, None);
        rule.check(&mut state, 3, &user2, None);

        let failures = finalize(state);
        assert!(failures.is_empty());
    }
}
