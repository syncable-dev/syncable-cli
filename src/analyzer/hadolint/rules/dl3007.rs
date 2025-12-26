//! DL3007: Using latest is prone to errors
//!
//! Using the :latest tag can lead to inconsistent builds and should be avoided.
//! Use specific version tags instead.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3007",
        Severity::Warning,
        "Using latest is prone to errors if the image will ever update. Pin the version explicitly to a release tag",
        |instr, _shell| {
            match instr {
                Instruction::From(base) => {
                    // Check if tag is "latest"
                    match &base.tag {
                        Some(tag) => tag != "latest",
                        None => true, // No tag is handled by DL3006
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
    use crate::analyzer::hadolint::parser::instruction::BaseImage;
    use crate::analyzer::hadolint::rules::{Rule, RuleState};

    #[test]
    fn test_specific_tag() {
        let rule = rule();
        let mut state = RuleState::new();

        let mut base = BaseImage::new("ubuntu");
        base.tag = Some("20.04".to_string());
        let instr = Instruction::From(base);

        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_latest_tag() {
        let rule = rule();
        let mut state = RuleState::new();

        let mut base = BaseImage::new("ubuntu");
        base.tag = Some("latest".to_string());
        let instr = Instruction::From(base);

        rule.check(&mut state, 1, &instr, None);
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3007");
    }

    #[test]
    fn test_no_tag() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::From(BaseImage::new("ubuntu"));
        rule.check(&mut state, 1, &instr, None);
        // No tag is OK here (handled by DL3006)
        assert!(state.failures.is_empty());
    }
}
