//! DL4000: MAINTAINER is deprecated
//!
//! The MAINTAINER instruction is deprecated. Use LABEL instead.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL4000",
        Severity::Error,
        "MAINTAINER is deprecated",
        |instr, _shell| !matches!(instr, Instruction::Maintainer(_)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::rules::{Rule, RuleState};

    #[test]
    fn test_no_maintainer() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::User("node".to_string());
        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_with_maintainer() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Maintainer("John Doe <john@example.com>".to_string());
        rule.check(&mut state, 1, &instr, None);
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL4000");
    }
}
