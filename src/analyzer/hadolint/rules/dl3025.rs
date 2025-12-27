//! DL3025: Use arguments JSON notation for CMD and ENTRYPOINT arguments
//!
//! Using exec form (JSON notation) for CMD and ENTRYPOINT ensures proper
//! signal handling and avoids shell processing issues.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3025",
        Severity::Warning,
        "Use arguments JSON notation for CMD and ENTRYPOINT arguments",
        |instr, _shell| match instr {
            Instruction::Cmd(args) | Instruction::Entrypoint(args) => args.is_exec_form(),
            _ => true,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::parser::instruction::Arguments;
    use crate::analyzer::hadolint::rules::{Rule, RuleState};

    #[test]
    fn test_exec_form() {
        let rule = rule();
        let mut state = RuleState::new();

        let args = Arguments::List(vec!["node".to_string(), "app.js".to_string()]);
        let instr = Instruction::Cmd(args);
        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_shell_form() {
        let rule = rule();
        let mut state = RuleState::new();

        let args = Arguments::Text("node app.js".to_string());
        let instr = Instruction::Cmd(args);
        rule.check(&mut state, 1, &instr, None);
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3025");
    }

    #[test]
    fn test_entrypoint_exec() {
        let rule = rule();
        let mut state = RuleState::new();

        let args = Arguments::List(vec!["./entrypoint.sh".to_string()]);
        let instr = Instruction::Entrypoint(args);
        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_entrypoint_shell() {
        let rule = rule();
        let mut state = RuleState::new();

        let args = Arguments::Text("./entrypoint.sh".to_string());
        let instr = Instruction::Entrypoint(args);
        rule.check(&mut state, 1, &instr, None);
        assert_eq!(state.failures.len(), 1);
    }
}
