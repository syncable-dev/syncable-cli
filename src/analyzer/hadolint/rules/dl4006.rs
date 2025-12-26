//! DL4006: Set the SHELL option -o pipefail before RUN with a pipe in it
//!
//! If a pipe is used in RUN, the shell option pipefail should be set
//! to ensure the entire pipeline fails if any command fails.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL4006",
        Severity::Warning,
        "Set the SHELL option -o pipefail before RUN with a pipe in it. If you are using /bin/sh in an alpine image or if your shell is symlinked to busybox then consider explicitly setting your SHELL to /bin/ash, or disable this check",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        // If there are pipes, this rule fails
                        // (should have set pipefail)
                        // In a real implementation, we'd track if SHELL with pipefail was set
                        !shell.has_pipes
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
    fn test_no_pipe() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("apt-get update"));
        let shell = ParsedShell::parse("apt-get update");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_with_pipe() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("cat file | grep pattern"));
        let shell = ParsedShell::parse("cat file | grep pattern");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL4006");
    }
}
