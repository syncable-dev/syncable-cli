//! Helper template parser.
//!
//! Parses _helpers.tpl files to extract defined template helpers.

use std::collections::HashSet;
use std::path::Path;

use crate::analyzer::helmlint::parser::template::{parse_template, ParsedTemplate, TemplateToken};

/// A helper template definition.
#[derive(Debug, Clone)]
pub struct HelperDefinition {
    /// The name of the helper (e.g., "mychart.fullname").
    pub name: String,
    /// The line number where the helper is defined.
    pub line: u32,
    /// The content of the helper definition.
    pub content: String,
    /// Documentation comment (if any).
    pub doc_comment: Option<String>,
}

/// Parsed helpers file.
#[derive(Debug, Clone)]
pub struct ParsedHelpers {
    /// Path to the helpers file.
    pub path: String,
    /// All defined helpers.
    pub helpers: Vec<HelperDefinition>,
    /// All helper names for quick lookup.
    pub helper_names: HashSet<String>,
    /// The underlying template parse result.
    pub template: ParsedTemplate,
}

impl ParsedHelpers {
    /// Check if a helper is defined.
    pub fn has_helper(&self, name: &str) -> bool {
        self.helper_names.contains(name)
    }

    /// Get a helper by name.
    pub fn get_helper(&self, name: &str) -> Option<&HelperDefinition> {
        self.helpers.iter().find(|h| h.name == name)
    }

    /// Get all helper names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.helper_names.iter().map(|s| s.as_str())
    }
}

/// Parse a helpers file.
pub fn parse_helpers(content: &str, path: &str) -> ParsedHelpers {
    let template = parse_template(content, path);
    let mut helpers = Vec::new();
    let mut helper_names = HashSet::new();

    // Track the previous comment for documentation
    let mut last_comment: Option<(String, u32)> = None;

    // Look for define blocks
    let mut i = 0;
    while i < template.tokens.len() {
        let token = &template.tokens[i];

        match token {
            TemplateToken::Comment { content, line } => {
                // Save comment as potential documentation
                last_comment = Some((content.clone(), *line));
            }
            TemplateToken::Action { content, line, .. } => {
                let trimmed = content.trim();
                if trimmed.starts_with("define ") {
                    // Extract helper name
                    if let Some(name) = extract_define_name(trimmed) {
                        // Collect the helper content until we hit the matching end
                        let mut helper_content = String::new();
                        let mut depth = 1;
                        let mut j = i + 1;

                        while j < template.tokens.len() && depth > 0 {
                            match &template.tokens[j] {
                                TemplateToken::Action {
                                    content: inner_content,
                                    ..
                                } => {
                                    let inner_trimmed = inner_content.trim();
                                    if inner_trimmed.starts_with("define ")
                                        || inner_trimmed.starts_with("if ")
                                        || inner_trimmed.starts_with("range ")
                                        || inner_trimmed.starts_with("with ")
                                        || inner_trimmed.starts_with("block ")
                                    {
                                        depth += 1;
                                    } else if inner_trimmed == "end" {
                                        depth -= 1;
                                        if depth == 0 {
                                            break;
                                        }
                                    }
                                    if depth > 0 {
                                        helper_content
                                            .push_str(&format!("{{{{ {} }}}}", inner_content));
                                    }
                                }
                                TemplateToken::Text {
                                    content: text_content,
                                    ..
                                } => {
                                    helper_content.push_str(text_content);
                                }
                                TemplateToken::Comment {
                                    content: comment_content,
                                    ..
                                } => {
                                    helper_content
                                        .push_str(&format!("{{{{/* {} */}}}}", comment_content));
                                }
                            }
                            j += 1;
                        }

                        // Check if previous comment is documentation (within a few lines)
                        // The comment line is the starting line of the comment, which may be
                        // several lines before the define if it's a multi-line comment
                        let doc_comment = last_comment
                            .take()
                            .filter(|(_, comment_line)| *line > *comment_line && *line - *comment_line <= 5)
                            .map(|(c, _)| c);

                        helpers.push(HelperDefinition {
                            name: name.clone(),
                            line: *line,
                            content: helper_content.trim().to_string(),
                            doc_comment,
                        });
                        helper_names.insert(name);
                    }
                }

                // Clear comment if this isn't immediately after a comment
                if !content.trim().starts_with("define ") {
                    last_comment = None;
                }
            }
            TemplateToken::Text { .. } => {
                // Only clear comment if there's non-whitespace text
                if !token.content().trim().is_empty() {
                    last_comment = None;
                }
            }
        }
        i += 1;
    }

    ParsedHelpers {
        path: path.to_string(),
        helpers,
        helper_names,
        template,
    }
}

/// Parse a helpers file from disk.
pub fn parse_helpers_file(path: &Path) -> Result<ParsedHelpers, std::io::Error> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_helpers(&content, &path.display().to_string()))
}

/// Extract the name from a define action.
fn extract_define_name(content: &str) -> Option<String> {
    // Pattern: define "name"
    let parts: Vec<&str> = content.split('"').collect();
    if parts.len() >= 2 {
        let name = parts[1].trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

/// Common helper names that charts typically define.
pub const COMMON_HELPERS: &[&str] = &[
    "chart",
    "name",
    "fullname",
    "labels",
    "selectorLabels",
    "serviceAccountName",
    "image",
];

/// Check if a helper name follows the expected pattern.
pub fn is_valid_helper_name(name: &str) -> bool {
    // Should be chart.name or similar
    if name.is_empty() {
        return false;
    }

    // Allow alphanumeric, dots, hyphens, and underscores
    name.chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_helpers() {
        let content = r#"
{{/*
Get the name of the chart.
*/}}
{{- define "mychart.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "mychart.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}

{{- define "mychart.labels" -}}
app.kubernetes.io/name: {{ include "mychart.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}
"#;
        let parsed = parse_helpers(content, "_helpers.tpl");

        assert!(parsed.has_helper("mychart.name"));
        assert!(parsed.has_helper("mychart.fullname"));
        assert!(parsed.has_helper("mychart.labels"));
        assert_eq!(parsed.helpers.len(), 3);

        // Check documentation comment
        let name_helper = parsed.get_helper("mychart.name").unwrap();
        assert!(name_helper.doc_comment.is_some());
        assert!(name_helper
            .doc_comment
            .as_ref()
            .unwrap()
            .contains("Get the name"));
    }

    #[test]
    fn test_parse_empty_helpers() {
        let content = "";
        let parsed = parse_helpers(content, "_helpers.tpl");
        assert!(parsed.helpers.is_empty());
    }

    #[test]
    fn test_valid_helper_name() {
        assert!(is_valid_helper_name("mychart.name"));
        assert!(is_valid_helper_name("my-chart.full_name"));
        assert!(is_valid_helper_name("common.labels"));
        assert!(!is_valid_helper_name(""));
        assert!(!is_valid_helper_name("has space"));
        assert!(!is_valid_helper_name("has:colon"));
    }

    #[test]
    fn test_helper_content() {
        let content = r#"
{{- define "simple.helper" -}}
hello world
{{- end }}
"#;
        let parsed = parse_helpers(content, "_helpers.tpl");
        let helper = parsed.get_helper("simple.helper").unwrap();
        assert!(helper.content.contains("hello world"));
    }

    #[test]
    fn test_nested_structures() {
        let content = r#"
{{- define "mychart.conditional" -}}
{{- if .Values.enabled }}
enabled
{{- else }}
disabled
{{- end }}
{{- end }}
"#;
        let parsed = parse_helpers(content, "_helpers.tpl");
        assert!(parsed.has_helper("mychart.conditional"));
        assert!(parsed.template.errors.is_empty());
    }
}
