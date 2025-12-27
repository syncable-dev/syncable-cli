//! DL3006: Always tag the version of an image explicitly
//!
//! Images should be tagged to ensure reproducible builds.
//! Using untagged images may result in different versions being pulled.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{CustomRule, RuleState, custom_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule()
-> CustomRule<impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync> {
    custom_rule(
        "DL3006",
        Severity::Warning,
        "Always tag the version of an image explicitly",
        |state, line, instr, _shell| {
            if let Instruction::From(base) = instr {
                // Remember stage aliases
                if let Some(alias) = &base.alias {
                    state.data.insert_to_set("aliases", alias.as_str());
                }

                // Check if image needs a tag
                let image_name = &base.image.name;

                // Skip check for:
                // 1. scratch image
                // 2. images with tags
                // 3. images with digests
                // 4. variable references
                // 5. references to previous build stages

                if base.is_scratch() {
                    return;
                }

                if base.has_version() {
                    return;
                }

                if base.is_variable() {
                    return;
                }

                // Check if it's a reference to a previous stage
                if state.data.set_contains("aliases", image_name) {
                    return;
                }

                // Image doesn't have a tag
                state.add_failure(
                    "DL3006",
                    Severity::Warning,
                    "Always tag the version of an image explicitly",
                    line,
                );
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::parser::instruction::{BaseImage, ImageAlias};
    use crate::analyzer::hadolint::rules::Rule;

    #[test]
    fn test_tagged_image() {
        let rule = rule();
        let mut state = RuleState::new();

        let mut base = BaseImage::new("ubuntu");
        base.tag = Some("20.04".to_string());
        let instr = Instruction::From(base);

        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_untagged_image() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::From(BaseImage::new("ubuntu"));
        rule.check(&mut state, 1, &instr, None);
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3006");
    }

    #[test]
    fn test_scratch_image() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::From(BaseImage::new("scratch"));
        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_stage_reference() {
        let rule = rule();
        let mut state = RuleState::new();

        // First stage with alias
        let mut base1 = BaseImage::new("node");
        base1.tag = Some("18".to_string());
        base1.alias = Some(ImageAlias::new("builder"));
        let instr1 = Instruction::From(base1);
        rule.check(&mut state, 1, &instr1, None);

        // Second stage referencing first
        let instr2 = Instruction::From(BaseImage::new("builder"));
        rule.check(&mut state, 10, &instr2, None);

        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_variable_image() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::From(BaseImage::new("${BASE_IMAGE}"));
        rule.check(&mut state, 1, &instr, None);
        assert!(state.failures.is_empty());
    }
}
