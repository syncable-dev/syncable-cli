//! Pragma handling for inline rule disabling.
//!
//! Supports comment-based rule disabling similar to ESLint:
//! - `# dclint-disable` - Disable all rules for the rest of the file
//! - `# dclint-disable rule-name` - Disable specific rule(s) globally
//! - `# dclint-disable-next-line` - Disable all rules for the next line
//! - `# dclint-disable-next-line rule-name` - Disable specific rule(s) for next line
//! - `# dclint-disable-file` - Disable all rules for the entire file

use std::collections::{HashMap, HashSet};

use crate::analyzer::dclint::types::RuleCode;

/// Tracks which rules are disabled at which lines.
#[derive(Debug, Clone, Default)]
pub struct PragmaState {
    /// Rules disabled for the entire file (global).
    pub global_disabled: HashSet<String>,
    /// Whether all rules are disabled globally.
    pub all_disabled: bool,
    /// Rules disabled for specific lines.
    pub line_disabled: HashMap<u32, HashSet<String>>,
    /// Lines where all rules are disabled.
    pub all_disabled_lines: HashSet<u32>,
}

impl PragmaState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a rule is ignored at a specific line.
    pub fn is_ignored(&self, code: &RuleCode, line: u32) -> bool {
        // Check global disables
        if self.all_disabled {
            return true;
        }
        if self.global_disabled.contains(code.as_str()) || self.global_disabled.contains("*") {
            return true;
        }

        // Check line-specific disables
        if self.all_disabled_lines.contains(&line) {
            return true;
        }
        if let Some(rules) = self.line_disabled.get(&line)
            && (rules.contains("*") || rules.contains(code.as_str()))
        {
            return true;
        }

        false
    }

    /// Add a globally disabled rule.
    pub fn disable_global(&mut self, rule: impl Into<String>) {
        let rule = rule.into();
        if rule == "*" {
            self.all_disabled = true;
        } else {
            self.global_disabled.insert(rule);
        }
    }

    /// Disable rules for a specific line.
    pub fn disable_line(&mut self, line: u32, rules: Vec<String>) {
        if rules.is_empty() || rules.iter().any(|r| r == "*") {
            self.all_disabled_lines.insert(line);
        } else {
            self.line_disabled.entry(line).or_default().extend(rules);
        }
    }
}

/// Extract pragmas from source content.
pub fn extract_pragmas(source: &str) -> PragmaState {
    let mut state = PragmaState::new();
    let lines: Vec<&str> = source.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        let line_num = (idx + 1) as u32;
        let trimmed = line.trim();

        // Skip non-comment lines
        if !trimmed.starts_with('#') {
            continue;
        }

        let comment = trimmed.trim_start_matches('#').trim();

        // Check for disable-file (applies to entire file)
        if let Some(rest) = comment.strip_prefix("dclint-disable-file") {
            let rules = parse_rule_list(rest);
            if rules.is_empty() {
                state.all_disabled = true;
            } else {
                for rule in rules {
                    state.disable_global(rule);
                }
            }
            continue;
        }

        // Check for disable-next-line
        if let Some(rest) = comment.strip_prefix("dclint-disable-next-line") {
            let rules = parse_rule_list(rest);
            let next_line = line_num + 1;

            if rules.is_empty() {
                state.all_disabled_lines.insert(next_line);
            } else {
                state.disable_line(next_line, rules);
            }
            continue;
        }

        // Check for global disable (at first content line, affects rest of file)
        if comment.starts_with("dclint-disable") && !comment.starts_with("dclint-disable-") {
            let rules = parse_rule_list(&comment["dclint-disable".len()..]);
            if rules.is_empty() {
                state.all_disabled = true;
            } else {
                for rule in rules {
                    state.disable_global(rule);
                }
            }
            continue;
        }
    }

    state
}

/// Parse a comma-separated list of rule names.
fn parse_rule_list(s: &str) -> Vec<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return vec![];
    }

    trimmed
        .split(',')
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty())
        .collect()
}

/// Extract global disable rules from the first comment line.
/// Returns set of disabled rule names (empty string means all disabled).
pub fn extract_global_disable_rules(source: &str) -> HashSet<String> {
    let state = extract_pragmas(source);
    let mut result = state.global_disabled;
    if state.all_disabled {
        result.insert("*".to_string());
    }
    result
}

/// Extract line-specific disable rules.
/// Returns map of line number -> set of disabled rules.
pub fn extract_line_disable_rules(source: &str) -> HashMap<u32, HashSet<String>> {
    let state = extract_pragmas(source);
    let mut result = state.line_disabled;

    // Add all-disabled lines
    for line in state.all_disabled_lines {
        result.entry(line).or_default().insert("*".to_string());
    }

    result
}

/// Check if file starts with a disable-file comment.
pub fn starts_with_disable_file_comment(source: &str) -> bool {
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            let comment = trimmed.trim_start_matches('#').trim();
            return comment.starts_with("dclint-disable-file")
                || (comment.starts_with("dclint-disable")
                    && !comment.starts_with("dclint-disable-"));
        }
        // First non-empty, non-comment line
        return false;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_global_disable() {
        let source = "# dclint-disable\nservices:\n  web:\n    image: nginx\n";
        let state = extract_pragmas(source);
        assert!(state.all_disabled);
    }

    #[test]
    fn test_extract_global_disable_specific_rules() {
        let source = "# dclint-disable DCL001, DCL002\nservices:\n  web:\n    image: nginx\n";
        let state = extract_pragmas(source);
        assert!(!state.all_disabled);
        assert!(state.global_disabled.contains("DCL001"));
        assert!(state.global_disabled.contains("DCL002"));
        assert!(!state.global_disabled.contains("DCL003"));
    }

    #[test]
    fn test_extract_disable_next_line() {
        let source = r#"
services:
  # dclint-disable-next-line DCL001
  web:
    build: .
    image: nginx
"#;
        let state = extract_pragmas(source);
        assert!(!state.all_disabled);

        // The disable comment is on line 3, so DCL001 should be disabled on line 4
        assert!(state.is_ignored(&RuleCode::new("DCL001"), 4));
        assert!(!state.is_ignored(&RuleCode::new("DCL001"), 5));
    }

    #[test]
    fn test_extract_disable_next_line_all() {
        let source = r#"
services:
  # dclint-disable-next-line
  web:
    build: .
    image: nginx
"#;
        let state = extract_pragmas(source);

        // All rules disabled on line 4
        assert!(state.all_disabled_lines.contains(&4));
        assert!(state.is_ignored(&RuleCode::new("DCL001"), 4));
        assert!(state.is_ignored(&RuleCode::new("DCL002"), 4));
    }

    #[test]
    fn test_extract_disable_file() {
        let source = "# dclint-disable-file\nservices:\n  web:\n    image: nginx\n";
        let state = extract_pragmas(source);
        assert!(state.all_disabled);
    }

    #[test]
    fn test_is_ignored() {
        let source = "# dclint-disable DCL001\nservices:\n  web:\n    image: nginx\n";
        let state = extract_pragmas(source);

        assert!(state.is_ignored(&RuleCode::new("DCL001"), 1));
        assert!(state.is_ignored(&RuleCode::new("DCL001"), 5));
        assert!(!state.is_ignored(&RuleCode::new("DCL002"), 1));
    }

    #[test]
    fn test_starts_with_disable_file_comment() {
        assert!(starts_with_disable_file_comment(
            "# dclint-disable-file\nservices:"
        ));
        assert!(starts_with_disable_file_comment(
            "# dclint-disable\nservices:"
        ));
        assert!(!starts_with_disable_file_comment("services:\n  web:"));
        assert!(!starts_with_disable_file_comment(
            "# Some other comment\nservices:"
        ));
    }

    #[test]
    fn test_parse_rule_list() {
        assert_eq!(parse_rule_list(""), Vec::<String>::new());
        assert_eq!(parse_rule_list("DCL001"), vec!["DCL001"]);
        assert_eq!(parse_rule_list("DCL001, DCL002"), vec!["DCL001", "DCL002"]);
        assert_eq!(
            parse_rule_list("  DCL001 , DCL002  "),
            vec!["DCL001", "DCL002"]
        );
    }
}
