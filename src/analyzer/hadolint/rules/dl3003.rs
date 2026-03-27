//! DL3003: Use WORKDIR to switch to a directory
//!
//! Don't use `cd` in RUN instructions. Use WORKDIR instead to change
//! the working directory.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3003",
        Severity::Warning,
        "Use WORKDIR to switch to a directory",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        // Check if cd is used as a command
                        !shell.any_command(|cmd| cmd.name == "cd")
                    } else {
                        true
                    }
                }
                _ => true,
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::parser::instruction::RunArgs;
    use crate::analyzer::hadolint::rules::{Rule, RuleState};

    #[test]
    fn test_no_cd() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("apt-get update"));
        let shell = ParsedShell::parse("apt-get update");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_with_cd() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("cd /app && npm install"));
        let shell = ParsedShell::parse("cd /app && npm install");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3003");
    }
}
