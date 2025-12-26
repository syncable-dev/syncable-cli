//! DL3045: COPY to a relative destination without WORKDIR set
//!
//! COPY to a relative path requires WORKDIR to be set to ensure
//! predictable behavior.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{CustomRule, RuleState, custom_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule()
-> CustomRule<impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync> {
    custom_rule(
        "DL3045",
        Severity::Warning,
        "`COPY` to a relative destination without `WORKDIR` set.",
        |state, line, instr, _shell| {
            match instr {
                Instruction::From(base) => {
                    // Track current stage
                    let stage_name = base
                        .alias
                        .as_ref()
                        .map(|a| a.as_str().to_string())
                        .unwrap_or_else(|| base.image.name.clone());
                    state.data.set_string("current_stage", &stage_name);

                    // Check if parent stage had WORKDIR set
                    let parent_had_workdir = state
                        .data
                        .set_contains("stages_with_workdir", &base.image.name);
                    if parent_had_workdir {
                        state.data.insert_to_set("stages_with_workdir", &stage_name);
                    }
                }
                Instruction::Workdir(_) => {
                    // Mark current stage as having WORKDIR set
                    let stage = state
                        .data
                        .get_string("current_stage")
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "__none__".to_string());
                    state.data.insert_to_set("stages_with_workdir", &stage);
                }
                Instruction::Copy(args, _) => {
                    let dest = &args.dest;

                    // Check if current stage has WORKDIR set
                    let has_workdir = state
                        .data
                        .get_string("current_stage")
                        .map(|s| state.data.set_contains("stages_with_workdir", s))
                        .unwrap_or_else(|| {
                            state.data.set_contains("stages_with_workdir", "__none__")
                        });

                    // Skip check if WORKDIR is set
                    if has_workdir {
                        return;
                    }

                    // Check if destination is absolute
                    let trimmed = dest.trim_matches(|c| c == '"' || c == '\'');

                    // Absolute paths are OK
                    if trimmed.starts_with('/') {
                        return;
                    }

                    // Windows absolute paths are OK
                    if is_windows_absolute(trimmed) {
                        return;
                    }

                    // Variable references are OK
                    if trimmed.starts_with('$') {
                        return;
                    }

                    // Relative path without WORKDIR
                    state.add_failure(
                        "DL3045",
                        Severity::Warning,
                        "`COPY` to a relative destination without `WORKDIR` set.",
                        line,
                    );
                }
                _ => {}
            }
        },
    )
}

/// Check if path is a Windows absolute path.
fn is_windows_absolute(path: &str) -> bool {
    let chars: Vec<char> = path.chars().collect();
    chars.len() >= 2 && chars[0].is_ascii_alphabetic() && chars[1] == ':'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::parser::instruction::{BaseImage, CopyArgs, CopyFlags};
    use crate::analyzer::hadolint::rules::Rule;

    #[test]
    fn test_absolute_dest() {
        let rule = rule();
        let mut state = RuleState::new();

        let from = Instruction::From(BaseImage::new("ubuntu"));
        let copy = Instruction::Copy(
            CopyArgs::new(vec!["app.js".to_string()], "/app/"),
            CopyFlags::default(),
        );

        rule.check(&mut state, 1, &from, None);
        rule.check(&mut state, 2, &copy, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_relative_dest_without_workdir() {
        let rule = rule();
        let mut state = RuleState::new();

        let from = Instruction::From(BaseImage::new("ubuntu"));
        let copy = Instruction::Copy(
            CopyArgs::new(vec!["app.js".to_string()], "app/"),
            CopyFlags::default(),
        );

        rule.check(&mut state, 1, &from, None);
        rule.check(&mut state, 2, &copy, None);
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3045");
    }

    #[test]
    fn test_relative_dest_with_workdir() {
        let rule = rule();
        let mut state = RuleState::new();

        let from = Instruction::From(BaseImage::new("ubuntu"));
        let workdir = Instruction::Workdir("/app".to_string());
        let copy = Instruction::Copy(
            CopyArgs::new(vec!["app.js".to_string()], "."),
            CopyFlags::default(),
        );

        rule.check(&mut state, 1, &from, None);
        rule.check(&mut state, 2, &workdir, None);
        rule.check(&mut state, 3, &copy, None);
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_variable_dest() {
        let rule = rule();
        let mut state = RuleState::new();

        let from = Instruction::From(BaseImage::new("ubuntu"));
        let copy = Instruction::Copy(
            CopyArgs::new(vec!["app.js".to_string()], "$APP_DIR"),
            CopyFlags::default(),
        );

        rule.check(&mut state, 1, &from, None);
        rule.check(&mut state, 2, &copy, None);
        assert!(state.failures.is_empty());
    }
}
