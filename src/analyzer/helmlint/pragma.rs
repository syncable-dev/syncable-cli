//! Pragma support for inline rule ignoring.
//!
//! Supports comment-based rule ignoring in Helm templates and YAML files:
//! - `# helmlint-ignore HL1001,HL1002` - ignore specific rules for next line
//! - `# helmlint-ignore-file` - ignore all rules for entire file
//! - `# helmlint-ignore-file HL1001` - ignore specific rule for entire file
//! - `{{/* helmlint-ignore HL1001 */}}` - template comment format

use std::collections::{HashMap, HashSet};

use crate::analyzer::helmlint::types::RuleCode;

/// State for pragma processing.
#[derive(Debug, Clone, Default)]
pub struct PragmaState {
    /// Rules ignored for the entire file.
    pub file_ignores: HashSet<String>,
    /// Rules ignored for specific lines (line -> set of rule codes).
    pub line_ignores: HashMap<u32, HashSet<String>>,
    /// Whether the entire file is ignored.
    pub file_disabled: bool,
}

impl PragmaState {
    /// Create a new empty pragma state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a rule is ignored for a specific line.
    pub fn is_ignored(&self, code: &RuleCode, line: u32) -> bool {
        if self.file_disabled {
            return true;
        }

        if self.file_ignores.contains(code.as_str()) {
            return true;
        }

        // Check if the rule is ignored for this specific line
        if let Some(ignores) = self.line_ignores.get(&line) {
            if ignores.contains(code.as_str()) {
                return true;
            }
        }

        // Check if previous line has an ignore pragma for this line
        if line > 1 {
            if let Some(ignores) = self.line_ignores.get(&(line - 1)) {
                if ignores.contains(code.as_str()) {
                    return true;
                }
            }
        }

        false
    }

    /// Add a file-level ignore for a rule.
    pub fn add_file_ignore(&mut self, code: impl Into<String>) {
        self.file_ignores.insert(code.into());
    }

    /// Add a line-level ignore for a rule.
    pub fn add_line_ignore(&mut self, line: u32, code: impl Into<String>) {
        self.line_ignores
            .entry(line)
            .or_default()
            .insert(code.into());
    }

    /// Set the file as completely disabled.
    pub fn disable_file(&mut self) {
        self.file_disabled = true;
    }
}

/// Extract pragmas from YAML content (values.yaml, Chart.yaml).
pub fn extract_yaml_pragmas(content: &str) -> PragmaState {
    let mut state = PragmaState::new();

    for (line_num, line) in content.lines().enumerate() {
        let line_number = (line_num + 1) as u32;
        let trimmed = line.trim();

        // Check for YAML comments
        if let Some(comment) = trimmed.strip_prefix('#') {
            process_comment(comment.trim(), line_number, &mut state);
        }
    }

    state
}

/// Extract pragmas from template content.
pub fn extract_template_pragmas(content: &str) -> PragmaState {
    let mut state = PragmaState::new();

    // Process YAML-style comments
    for (line_num, line) in content.lines().enumerate() {
        let line_number = (line_num + 1) as u32;
        let trimmed = line.trim();

        // Check for YAML comments (outside of templates)
        if let Some(comment) = trimmed.strip_prefix('#') {
            // Make sure it's not inside a template action
            if !line.contains("{{") || line.find('#') < line.find("{{") {
                process_comment(comment.trim(), line_number, &mut state);
            }
        }
    }

    // Process template comments {{/* ... */}}
    let mut line_num: u32 = 1;
    let mut i = 0;
    let chars: Vec<char> = content.chars().collect();

    while i < chars.len() {
        if chars[i] == '\n' {
            line_num += 1;
            i += 1;
            continue;
        }

        // Look for template comment start
        if i + 4 < chars.len()
            && chars[i] == '{'
            && chars[i + 1] == '{'
            && (chars[i + 2] == '/' || (chars[i + 2] == '-' && i + 5 < chars.len() && chars[i + 3] == '/'))
        {
            let _comment_start = i;
            let comment_line = line_num;

            // Skip to comment content
            i += 2;
            if chars[i] == '-' {
                i += 1;
            }
            i += 2; // skip /*

            // Find comment end
            let mut comment_content = String::new();
            while i + 3 < chars.len() {
                if chars[i] == '\n' {
                    line_num += 1;
                }
                if chars[i] == '*' && chars[i + 1] == '/' {
                    i += 2;
                    // Skip optional trim marker and closing braces
                    if i < chars.len() && chars[i] == '-' {
                        i += 1;
                    }
                    if i + 1 < chars.len() && chars[i] == '}' && chars[i + 1] == '}' {
                        i += 2;
                    }
                    break;
                }
                comment_content.push(chars[i]);
                i += 1;
            }

            // Process the comment
            process_comment(&comment_content.trim(), comment_line, &mut state);
            continue;
        }

        i += 1;
    }

    state
}

/// Process a comment for pragma directives.
fn process_comment(comment: &str, line: u32, state: &mut PragmaState) {
    let lower = comment.to_lowercase();

    // Check for file-level disable
    if lower.starts_with("helmlint-ignore-file") || lower.starts_with("helmlint-disable-file") {
        let rest = comment
            .strip_prefix("helmlint-ignore-file")
            .or_else(|| comment.strip_prefix("helmlint-disable-file"))
            .unwrap_or("")
            .trim();

        if rest.is_empty() {
            state.disable_file();
        } else {
            // Parse specific rules to ignore for the file
            for code in parse_rule_list(rest) {
                state.add_file_ignore(code);
            }
        }
        return;
    }

    // Check for line-level ignore
    if lower.starts_with("helmlint-ignore") || lower.starts_with("helmlint-disable") {
        let rest = comment
            .strip_prefix("helmlint-ignore")
            .or_else(|| comment.strip_prefix("helmlint-disable"))
            .unwrap_or("")
            .trim();

        if rest.is_empty() {
            // Ignore all rules for next line - we'll use a special marker
            state.add_line_ignore(line, "*");
        } else {
            for code in parse_rule_list(rest) {
                state.add_line_ignore(line, code);
            }
        }
    }
}

/// Parse a comma-separated list of rule codes.
fn parse_rule_list(input: &str) -> Vec<String> {
    input
        .split(|c| c == ',' || c == ' ')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && s.starts_with("HL"))
        .map(|s| s.to_string())
        .collect()
}

/// Check if content starts with a file-level disable comment.
pub fn starts_with_disable_file_comment(content: &str) -> bool {
    for line in content.lines().take(10) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(comment) = trimmed.strip_prefix('#') {
            let comment_lower = comment.trim().to_lowercase();
            if comment_lower.starts_with("helmlint-ignore-file")
                || comment_lower.starts_with("helmlint-disable-file")
            {
                // Check if it's a full file disable (no specific rules)
                let rest = comment
                    .trim()
                    .strip_prefix("helmlint-ignore-file")
                    .or_else(|| comment.trim().strip_prefix("helmlint-disable-file"))
                    .unwrap_or("")
                    .trim();
                if rest.is_empty() {
                    return true;
                }
            }
        }
        // Only check the first non-empty, non-comment-only lines
        if !trimmed.starts_with('#') {
            break;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_pragma_ignore() {
        let content = r#"
# helmlint-ignore HL1001
name: test-chart
version: 1.0.0
"#;
        let state = extract_yaml_pragmas(content);
        assert!(state.is_ignored(&RuleCode::new("HL1001"), 3));
        assert!(!state.is_ignored(&RuleCode::new("HL1002"), 3));
    }

    #[test]
    fn test_yaml_pragma_file_ignore() {
        let content = r#"
# helmlint-ignore-file HL1001,HL1002
name: test-chart
"#;
        let state = extract_yaml_pragmas(content);
        assert!(state.is_ignored(&RuleCode::new("HL1001"), 3));
        assert!(state.is_ignored(&RuleCode::new("HL1002"), 10));
        assert!(!state.is_ignored(&RuleCode::new("HL1003"), 3));
    }

    #[test]
    fn test_yaml_pragma_disable_file() {
        let content = r#"
# helmlint-ignore-file
name: test-chart
"#;
        let state = extract_yaml_pragmas(content);
        assert!(state.file_disabled);
        assert!(state.is_ignored(&RuleCode::new("HL1001"), 3));
        assert!(state.is_ignored(&RuleCode::new("HL9999"), 100));
    }

    #[test]
    fn test_template_pragma() {
        let content = r#"
{{/* helmlint-ignore HL3001 */}}
{{ .Values.name }}
"#;
        let state = extract_template_pragmas(content);
        assert!(state.is_ignored(&RuleCode::new("HL3001"), 3));
    }

    #[test]
    fn test_template_pragma_file_ignore() {
        let content = r#"
{{/* helmlint-ignore-file HL3001 */}}
apiVersion: v1
kind: ConfigMap
"#;
        let state = extract_template_pragmas(content);
        assert!(state.is_ignored(&RuleCode::new("HL3001"), 3));
        assert!(state.is_ignored(&RuleCode::new("HL3001"), 4));
    }

    #[test]
    fn test_multiple_rules() {
        let content = r#"
# helmlint-ignore HL1001, HL1002, HL1003
apiVersion: v2
"#;
        let state = extract_yaml_pragmas(content);
        assert!(state.is_ignored(&RuleCode::new("HL1001"), 3));
        assert!(state.is_ignored(&RuleCode::new("HL1002"), 3));
        assert!(state.is_ignored(&RuleCode::new("HL1003"), 3));
    }

    #[test]
    fn test_starts_with_disable_file() {
        let content = r#"# helmlint-ignore-file
apiVersion: v2
"#;
        assert!(starts_with_disable_file_comment(content));

        let content_with_rules = r#"# helmlint-ignore-file HL1001
apiVersion: v2
"#;
        assert!(!starts_with_disable_file_comment(content_with_rules));

        let content_normal = r#"apiVersion: v2
name: test
"#;
        assert!(!starts_with_disable_file_comment(content_normal));
    }

    #[test]
    fn test_disable_alias() {
        let content = r#"
# helmlint-disable HL1001
apiVersion: v2
"#;
        let state = extract_yaml_pragmas(content);
        assert!(state.is_ignored(&RuleCode::new("HL1001"), 3));
    }
}
