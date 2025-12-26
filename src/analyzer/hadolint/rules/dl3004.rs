//! DL3004: Do not use sudo
//!
//! Using sudo in Dockerfiles is unnecessary since containers run as root
//! by default, and using it indicates a misunderstanding of Docker.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3004",
        Severity::Error,
        "Do not use sudo as it leads to unpredictable behavior. Use a tool like gosu to enforce root",
        |instr, shell| match instr {
            Instruction::Run(_) => {
                if let Some(shell) = shell {
                    !shell.any_command(|cmd| cmd.name == "sudo")
                } else {
                    true
                }
            }
            _ => true,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::parser::instruction::RunArgs;
    use crate::analyzer::hadolint::rules::{Rule, RuleState};

    #[test]
    fn test_no_sudo() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("apt-get update"));
        let shell = ParsedShell::parse("apt-get update");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_with_sudo() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("sudo apt-get update"));
        let shell = ParsedShell::parse("sudo apt-get update");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3004");
    }
}
