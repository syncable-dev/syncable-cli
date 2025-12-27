//! Pragma parsing for inline rule ignores.
//!
//! Hadolint supports inline pragmas to ignore rules:
//! - `# hadolint ignore=DL3008,DL3009` - Ignore for next instruction
//! - `# hadolint global ignore=DL3008` - Ignore for entire file
//! - `# hadolint shell=/bin/bash` - Set shell for ShellCheck

use crate::analyzer::hadolint::types::RuleCode;
use std::collections::{HashMap, HashSet};

/// Parsed pragma state for a Dockerfile.
#[derive(Debug, Clone, Default)]
pub struct PragmaState {
    /// Per-line ignored rules: line -> set of ignored codes.
    pub ignored: HashMap<u32, HashSet<RuleCode>>,
    /// Globally ignored rules.
    pub global_ignored: HashSet<RuleCode>,
    /// Shell override (if specified).
    pub shell: Option<String>,
}

impl PragmaState {
    /// Create a new empty pragma state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a rule should be ignored on a specific line.
    pub fn is_ignored(&self, code: &RuleCode, line: u32) -> bool {
        // Check global ignores
        if self.global_ignored.contains(code) {
            return true;
        }

        // Check line-specific ignores (check previous line, as pragma applies to next line)
        if let Some(ignored) = self.ignored.get(&line) {
            if ignored.contains(code) {
                return true;
            }
        }

        // Also check if the pragma was on the line before
        if line > 0 {
            if let Some(ignored) = self.ignored.get(&(line - 1)) {
                if ignored.contains(code) {
                    return true;
                }
            }
        }

        false
    }
}

/// Parse pragma from a comment string.
/// Returns the pragma type and any associated data.
pub fn parse_pragma(comment: &str) -> Option<Pragma> {
    let comment = comment.trim();

    // Look for hadolint pragma
    let pragma_start = comment.find("hadolint")?;
    let pragma_content = &comment[pragma_start + "hadolint".len()..].trim();

    // Parse global ignore
    if pragma_content.starts_with("global") {
        let rest = &pragma_content["global".len()..].trim();
        if let Some(codes) = parse_ignore_list(rest) {
            return Some(Pragma::GlobalIgnore(codes));
        }
    }

    // Parse ignore
    if let Some(codes) = parse_ignore_list(pragma_content) {
        return Some(Pragma::Ignore(codes));
    }

    // Parse shell
    if pragma_content.starts_with("shell=") {
        let shell = &pragma_content["shell=".len()..].trim();
        return Some(Pragma::Shell(shell.to_string()));
    }

    None
}

/// Parse an ignore list from a pragma string.
fn parse_ignore_list(s: &str) -> Option<Vec<RuleCode>> {
    let s = s.trim();

    // Look for ignore= pattern
    if !s.starts_with("ignore=") && !s.starts_with("ignore =") {
        return None;
    }

    // Find the = sign and get the codes
    let eq_pos = s.find('=')?;
    let codes_str = &s[eq_pos + 1..].trim();

    // Split by comma and parse codes
    let codes: Vec<RuleCode> = codes_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| RuleCode::new(s))
        .collect();

    if codes.is_empty() { None } else { Some(codes) }
}

/// Parsed pragma types.
#[derive(Debug, Clone)]
pub enum Pragma {
    /// Ignore rules for the next instruction.
    Ignore(Vec<RuleCode>),
    /// Ignore rules globally for the entire file.
    GlobalIgnore(Vec<RuleCode>),
    /// Set shell for ShellCheck analysis.
    Shell(String),
}

/// Extract pragma state from Dockerfile instructions.
pub fn extract_pragmas(
    instructions: &[crate::analyzer::hadolint::parser::InstructionPos],
) -> PragmaState {
    let mut state = PragmaState::new();

    for instr in instructions {
        if let crate::analyzer::hadolint::parser::instruction::Instruction::Comment(comment) =
            &instr.instruction
        {
            if let Some(pragma) = parse_pragma(comment) {
                match pragma {
                    Pragma::Ignore(codes) => {
                        // Ignore applies to the next line
                        let entry = state.ignored.entry(instr.line_number).or_default();
                        for code in codes {
                            entry.insert(code);
                        }
                    }
                    Pragma::GlobalIgnore(codes) => {
                        for code in codes {
                            state.global_ignored.insert(code);
                        }
                    }
                    Pragma::Shell(shell) => {
                        state.shell = Some(shell);
                    }
                }
            }
        }
    }

    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ignore() {
        let pragma = parse_pragma("# hadolint ignore=DL3008,DL3009").unwrap();
        match pragma {
            Pragma::Ignore(codes) => {
                assert_eq!(codes.len(), 2);
                assert_eq!(codes[0].as_str(), "DL3008");
                assert_eq!(codes[1].as_str(), "DL3009");
            }
            _ => panic!("Expected Ignore pragma"),
        }
    }

    #[test]
    fn test_parse_global_ignore() {
        let pragma = parse_pragma("# hadolint global ignore=DL3008").unwrap();
        match pragma {
            Pragma::GlobalIgnore(codes) => {
                assert_eq!(codes.len(), 1);
                assert_eq!(codes[0].as_str(), "DL3008");
            }
            _ => panic!("Expected GlobalIgnore pragma"),
        }
    }

    #[test]
    fn test_parse_shell() {
        let pragma = parse_pragma("# hadolint shell=/bin/bash").unwrap();
        match pragma {
            Pragma::Shell(shell) => {
                assert_eq!(shell, "/bin/bash");
            }
            _ => panic!("Expected Shell pragma"),
        }
    }

    #[test]
    fn test_no_pragma() {
        assert!(parse_pragma("# This is a regular comment").is_none());
    }

    #[test]
    fn test_pragma_state_is_ignored() {
        let mut state = PragmaState::new();

        // Add line-specific ignore
        let mut codes = HashSet::new();
        codes.insert(RuleCode::new("DL3008"));
        state.ignored.insert(5, codes);

        // Add global ignore
        state.global_ignored.insert(RuleCode::new("DL3009"));

        // Test line-specific (pragma on line 5 affects line 6)
        assert!(state.is_ignored(&RuleCode::new("DL3008"), 6));
        assert!(!state.is_ignored(&RuleCode::new("DL3008"), 10));

        // Test global
        assert!(state.is_ignored(&RuleCode::new("DL3009"), 1));
        assert!(state.is_ignored(&RuleCode::new("DL3009"), 100));

        // Test non-ignored
        assert!(!state.is_ignored(&RuleCode::new("DL3010"), 1));
    }
}
