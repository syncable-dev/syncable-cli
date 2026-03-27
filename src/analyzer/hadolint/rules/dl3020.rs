//! DL3020: Use COPY instead of ADD for files/dirs
//!
//! ADD has special behaviors (URL download, tar extraction) that make it
//! less predictable. Use COPY for simply copying files.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3020",
        Severity::Error,
        "Use COPY instead of ADD for files and folders",
        |instr, _shell| {
            match instr {
                Instruction::Add(args, _) => {
                    // ADD is OK for URLs and archives
                    args.has_url() || args.has_archive()
                }
                _ => true,
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::parser::instruction::{AddArgs, AddFlags};
    use crate::analyzer::hadolint::rules::{Rule, RuleState};

    #[test]
    fn test_add_file() {
        let rule = rule();
        let mut state = RuleState::new();

        let args = AddArgs::new(vec!["app.js".to_string()], "/app/");
        let instr = Instruction::Add(args, AddFlags::default());
        rule.check(&mut state, 1, &instr, None);
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3020");
    }

    #[test]
    fn test_add_url() {
        let rule = rule();
        let mut state = RuleState::new();

        let args = AddArgs::new(vec!["https://example.com/file.tar.gz".to_string()], "/app/");
        let instr = Instruction::Add(args, AddFlags::default());
        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_add_archive() {
        let rule = rule();
        let mut state = RuleState::new();

        let args = AddArgs::new(vec!["app.tar.gz".to_string()], "/app/");
        let instr = Instruction::Add(args, AddFlags::default());
        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_copy_ok() {
        let rule = rule();
        let mut state = RuleState::new();

        // COPY is always OK
        let instr = Instruction::Workdir("/app".to_string()); // Different instruction
        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }
}
