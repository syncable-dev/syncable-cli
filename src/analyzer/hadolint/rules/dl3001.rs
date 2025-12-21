//! DL3001: Don't use invalid commands in RUN
//!
//! Commands like ssh, vim, shutdown, service, ps, free, top, kill, and mount
//! are not appropriate for Dockerfile RUN instructions.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

/// Invalid commands that shouldn't be used in Dockerfiles.
const INVALID_COMMANDS: &[&str] = &[
    "ssh",
    "vim",
    "shutdown",
    "service",
    "ps",
    "free",
    "top",
    "kill",
    "mount",
    "ifconfig",
    "nano",
];

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3001",
        Severity::Info,
        "For some bash commands it makes no sense running them in a Docker container like ssh, vim, shutdown, service, ps, free, top, kill, mount, ifconfig",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| INVALID_COMMANDS.contains(&cmd.name.as_str()))
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
    fn test_valid_command() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("apt-get update"));
        let shell = ParsedShell::parse("apt-get update");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_invalid_ssh() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("ssh user@host"));
        let shell = ParsedShell::parse("ssh user@host");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3001");
    }

    #[test]
    fn test_invalid_vim() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("vim /etc/config"));
        let shell = ParsedShell::parse("vim /etc/config");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert_eq!(state.failures.len(), 1);
    }
}
