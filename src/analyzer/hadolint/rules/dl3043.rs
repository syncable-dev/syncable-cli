//! DL3043: ONBUILD ONBUILD is not allowed
//!
//! Nested ONBUILD instructions are not allowed.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3043",
        Severity::Error,
        "`ONBUILD` combined with `ONBUILD` is not allowed.",
        |instr, _shell| match instr {
            Instruction::OnBuild(inner) => !matches!(inner.as_ref(), Instruction::OnBuild(_)),
            _ => true,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::parser::instruction::{Arguments, RunArgs, RunFlags};
    use crate::analyzer::hadolint::rules::{Rule, RuleState};

    #[test]
    fn test_nested_onbuild() {
        let rule = rule();
        let mut state = RuleState::new();

        // ONBUILD ONBUILD RUN echo hello
        let inner_run = Instruction::Run(RunArgs {
            arguments: Arguments::Text("echo hello".to_string()),
            flags: RunFlags::default(),
        });
        let inner_onbuild = Instruction::OnBuild(Box::new(inner_run));
        let instr = Instruction::OnBuild(Box::new(inner_onbuild));

        rule.check(&mut state, 1, &instr, None);
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3043");
    }

    #[test]
    fn test_valid_onbuild() {
        let rule = rule();
        let mut state = RuleState::new();

        // ONBUILD RUN echo hello
        let inner = Instruction::Run(RunArgs {
            arguments: Arguments::Text("echo hello".to_string()),
            flags: RunFlags::default(),
        });
        let instr = Instruction::OnBuild(Box::new(inner));

        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }
}
