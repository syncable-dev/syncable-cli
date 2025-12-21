//! DL1001: Please refrain from using inline ignore pragmas
//!
//! This is a meta-rule that warns when inline ignore pragmas are used.
//! It's disabled by default but can be enabled for strict linting.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL1001",
        Severity::Info,
        "Please refrain from using inline ignore pragmas `# hadolint ignore=...`.",
        |instr, _shell| {
            match instr {
                Instruction::Comment(comment) => {
                    // Check if it's a hadolint ignore pragma
                    let lower = comment.to_lowercase();
                    !lower.contains("hadolint") || !lower.contains("ignore")
                }
                _ => true,
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::rules::{Rule, RuleState};

    #[test]
    fn test_ignore_pragma() {
        let rule = rule();
        let mut state = RuleState::new();
        let instr = Instruction::Comment("hadolint ignore=DL3008".to_string());
        rule.check(&mut state, 1, &instr, None);
        assert_eq!(state.failures.len(), 1);
    }

    #[test]
    fn test_regular_comment() {
        let rule = rule();
        let mut state = RuleState::new();
        let instr = Instruction::Comment("This is a regular comment".to_string());
        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }
}
