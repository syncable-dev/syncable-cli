//! DL4004: Multiple ENTRYPOINT instructions
//!
//! Only one ENTRYPOINT instruction should be present. If multiple are present,
//! only the last one takes effect.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{CustomRule, RuleState, custom_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule()
-> CustomRule<impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync> {
    custom_rule(
        "DL4004",
        Severity::Error,
        "Multiple `ENTRYPOINT` instructions found. If you list more than one `ENTRYPOINT` then only the last `ENTRYPOINT` will take effect",
        |state, line, instr, _shell| {
            match instr {
                Instruction::From(_) => {
                    // Reset count for each stage
                    state.data.set_int("entrypoint_count", 0);
                }
                Instruction::Entrypoint(_) => {
                    let count = state.data.get_int("entrypoint_count") + 1;
                    state.data.set_int("entrypoint_count", count);

                    if count > 1 {
                        state.add_failure(
                            "DL4004",
                            Severity::Error,
                            "Multiple `ENTRYPOINT` instructions found. If you list more than one `ENTRYPOINT` then only the last `ENTRYPOINT` will take effect",
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
    fn test_single_entrypoint() {
        let rule = rule();
        let mut state = RuleState::new();

        let from = Instruction::From(BaseImage::new("ubuntu"));
        let ep = Instruction::Entrypoint(Arguments::List(vec!["./entrypoint.sh".to_string()]));

        rule.check(&mut state, 1, &from, None);
        rule.check(&mut state, 2, &ep, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_multiple_entrypoints() {
        let rule = rule();
        let mut state = RuleState::new();

        let from = Instruction::From(BaseImage::new("ubuntu"));
        let ep1 = Instruction::Entrypoint(Arguments::List(vec!["./script1.sh".to_string()]));
        let ep2 = Instruction::Entrypoint(Arguments::List(vec!["./script2.sh".to_string()]));

        rule.check(&mut state, 1, &from, None);
        rule.check(&mut state, 2, &ep1, None);
        rule.check(&mut state, 3, &ep2, None);
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL4004");
    }
}
