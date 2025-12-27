//! DL4003: Multiple CMD instructions
//!
//! Only one CMD instruction should be present. If multiple are present,
//! only the last one takes effect.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{CustomRule, RuleState, custom_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule()
-> CustomRule<impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync> {
    custom_rule(
        "DL4003",
        Severity::Warning,
        "Multiple `CMD` instructions found. If you list more than one `CMD` then only the last `CMD` will take effect",
        |state, line, instr, _shell| {
            match instr {
                Instruction::From(_) => {
                    // Reset count for each stage
                    state.data.set_int("cmd_count", 0);
                }
                Instruction::Cmd(_) => {
                    let count = state.data.get_int("cmd_count") + 1;
                    state.data.set_int("cmd_count", count);

                    if count > 1 {
                        state.add_failure(
                            "DL4003",
                            Severity::Warning,
                            "Multiple `CMD` instructions found. If you list more than one `CMD` then only the last `CMD` will take effect",
                            line,
                        );
                    }
                }
                _ => {}
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::parser::instruction::{Arguments, BaseImage};
    use crate::analyzer::hadolint::rules::Rule;

    #[test]
    fn test_single_cmd() {
        let rule = rule();
        let mut state = RuleState::new();

        let from = Instruction::From(BaseImage::new("ubuntu"));
        let cmd = Instruction::Cmd(Arguments::List(vec!["node".to_string()]));

        rule.check(&mut state, 1, &from, None);
        rule.check(&mut state, 2, &cmd, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_multiple_cmds() {
        let rule = rule();
        let mut state = RuleState::new();

        let from = Instruction::From(BaseImage::new("ubuntu"));
        let cmd1 = Instruction::Cmd(Arguments::List(vec!["node".to_string()]));
        let cmd2 = Instruction::Cmd(Arguments::List(vec!["npm".to_string()]));

        rule.check(&mut state, 1, &from, None);
        rule.check(&mut state, 2, &cmd1, None);
        rule.check(&mut state, 3, &cmd2, None);
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL4003");
    }

    #[test]
    fn test_multiple_stages_ok() {
        let rule = rule();
        let mut state = RuleState::new();

        let from1 = Instruction::From(BaseImage::new("node"));
        let cmd1 = Instruction::Cmd(Arguments::List(vec!["npm".to_string()]));
        let from2 = Instruction::From(BaseImage::new("alpine"));
        let cmd2 = Instruction::Cmd(Arguments::List(vec!["node".to_string()]));

        rule.check(&mut state, 1, &from1, None);
        rule.check(&mut state, 2, &cmd1, None);
        rule.check(&mut state, 3, &from2, None);
        rule.check(&mut state, 4, &cmd2, None);
        assert!(state.failures.is_empty());
    }
}
