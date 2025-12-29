//! Go template parser for Helm templates.
//!
//! Tokenizes Go templates for static analysis without full evaluation.

use std::collections::HashSet;
use std::path::Path;

/// A token in a Go template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateToken {
    /// Raw text outside of template delimiters
    Text {
        content: String,
        line: u32,
    },
    /// Template action: {{ ... }}
    Action {
        content: String,
        line: u32,
        trim_left: bool,
        trim_right: bool,
    },
    /// Template comment: {{/* ... */}}
    Comment {
        content: String,
        line: u32,
    },
}

impl TemplateToken {
    /// Get the line number of this token.
    pub fn line(&self) -> u32 {
        match self {
            Self::Text { line, .. } => *line,
            Self::Action { line, .. } => *line,
            Self::Comment { line, .. } => *line,
        }
    }

    /// Check if this is an action token.
    pub fn is_action(&self) -> bool {
        matches!(self, Self::Action { .. })
    }

    /// Get the content of the token.
    pub fn content(&self) -> &str {
        match self {
            Self::Text { content, .. } => content,
            Self::Action { content, .. } => content,
            Self::Comment { content, .. } => content,
        }
    }
}

/// Control structure type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlStructure {
    If,
    Else,
    ElseIf,
    Range,
    With,
    Define,
    Block,
    Template,
    End,
}

impl ControlStructure {
    /// Parse from action content.
    pub fn parse(content: &str) -> Option<Self> {
        let trimmed = content.trim();
        let first_word = trimmed.split_whitespace().next()?;

        match first_word {
            "if" => Some(Self::If),
            "else" => {
                if trimmed.starts_with("else if") {
                    Some(Self::ElseIf)
                } else {
                    Some(Self::Else)
                }
            }
            "range" => Some(Self::Range),
            "with" => Some(Self::With),
            "define" => Some(Self::Define),
            "block" => Some(Self::Block),
            "template" => Some(Self::Template),
            "end" => Some(Self::End),
            _ => None,
        }
    }

    /// Check if this starts a block (needs matching end).
    pub fn starts_block(&self) -> bool {
        matches!(
            self,
            Self::If | Self::Range | Self::With | Self::Define | Self::Block
        )
    }

    /// Check if this ends a block.
    pub fn ends_block(&self) -> bool {
        matches!(self, Self::End)
    }
}

/// A parsed Go template with analysis data.
#[derive(Debug, Clone)]
pub struct ParsedTemplate {
    /// The original file path.
    pub path: String,
    /// All tokens in the template.
    pub tokens: Vec<TemplateToken>,
    /// All variables referenced (e.g., ".Values.image", ".Release.Name").
    pub variables_used: HashSet<String>,
    /// All functions called (e.g., "include", "tpl", "default").
    pub functions_called: HashSet<String>,
    /// Defined template names (from define/block).
    pub defined_templates: HashSet<String>,
    /// Referenced template names (from template/include).
    pub referenced_templates: HashSet<String>,
    /// Control structure stack tracking.
    pub unclosed_blocks: Vec<(ControlStructure, u32)>,
    /// Parse errors encountered.
    pub errors: Vec<TemplateParseError>,
}

impl ParsedTemplate {
    /// Get all .Values references.
    pub fn values_references(&self) -> Vec<&str> {
        self.variables_used
            .iter()
            .filter(|v| v.starts_with(".Values."))
            .map(|s| s.as_str())
            .collect()
    }

    /// Get all .Release references.
    pub fn release_references(&self) -> Vec<&str> {
        self.variables_used
            .iter()
            .filter(|v| v.starts_with(".Release."))
            .map(|s| s.as_str())
            .collect()
    }

    /// Check if the template has unclosed blocks.
    pub fn has_unclosed_blocks(&self) -> bool {
        !self.unclosed_blocks.is_empty()
    }

    /// Check if a function is called.
    pub fn calls_function(&self, name: &str) -> bool {
        self.functions_called.contains(name)
    }

    /// Check if the template uses lookup (requires K8s cluster).
    pub fn uses_lookup(&self) -> bool {
        self.functions_called.contains("lookup")
    }

    /// Check if the template uses tpl (dynamic template execution).
    pub fn uses_tpl(&self) -> bool {
        self.functions_called.contains("tpl")
    }
}

/// Parse error for templates.
#[derive(Debug, Clone)]
pub struct TemplateParseError {
    pub message: String,
    pub line: u32,
}

impl std::fmt::Display for TemplateParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// Parse a Go template file.
pub fn parse_template(content: &str, path: &str) -> ParsedTemplate {
    let mut tokens = Vec::new();
    let mut variables_used = HashSet::new();
    let mut functions_called = HashSet::new();
    let mut defined_templates = HashSet::new();
    let mut referenced_templates = HashSet::new();
    let mut errors = Vec::new();
    let mut block_stack: Vec<(ControlStructure, u32)> = Vec::new();

    let mut line_num: u32 = 1;
    let mut chars = content.chars().peekable();
    let mut current_text = String::new();
    let mut text_start_line = 1;

    while let Some(c) = chars.next() {
        if c == '\n' {
            current_text.push(c);
            line_num += 1;
            continue;
        }

        if c == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second {

            // Save any pending text
            if !current_text.is_empty() {
                tokens.push(TemplateToken::Text {
                    content: std::mem::take(&mut current_text),
                    line: text_start_line,
                });
            }

            let action_start_line = line_num;

            // Check for trim marker or comment
            let trim_left = chars.peek() == Some(&'-');
            if trim_left {
                chars.next();
            }

            let is_comment = chars.peek() == Some(&'/');

            // Collect action content
            let mut action_content = String::new();
            let mut found_end = false;
            let mut trim_right = false;

            while let Some(c) = chars.next() {
                if c == '\n' {
                    line_num += 1;
                    action_content.push(c);
                } else if c == '-' && chars.peek() == Some(&'}') {
                    trim_right = true;
                    chars.next(); // consume }
                    if chars.peek() == Some(&'}') {
                        chars.next(); // consume second }
                        found_end = true;
                        break;
                    }
                } else if c == '}' && chars.peek() == Some(&'}') {
                    chars.next(); // consume second }
                    found_end = true;
                    break;
                } else {
                    action_content.push(c);
                }
            }

            if !found_end {
                errors.push(TemplateParseError {
                    message: "Unclosed template action".to_string(),
                    line: action_start_line,
                });
            }

            // Process the action
            let trimmed_content = action_content.trim();

            if is_comment {
                // Remove /* and */ from comment
                let comment = trimmed_content
                    .trim_start_matches('/')
                    .trim_start_matches('*')
                    .trim_end_matches('*')
                    .trim_end_matches('/')
                    .trim();
                tokens.push(TemplateToken::Comment {
                    content: comment.to_string(),
                    line: action_start_line,
                });
            } else {
                tokens.push(TemplateToken::Action {
                    content: trimmed_content.to_string(),
                    line: action_start_line,
                    trim_left,
                    trim_right,
                });

                // Analyze the action content
                analyze_action(
                    trimmed_content,
                    action_start_line,
                    &mut variables_used,
                    &mut functions_called,
                    &mut defined_templates,
                    &mut referenced_templates,
                    &mut block_stack,
                );
            }

            text_start_line = line_num;
        } else {
            if current_text.is_empty() {
                text_start_line = line_num;
            }
            current_text.push(c);
        }
    }

    // Save any remaining text
    if !current_text.is_empty() {
        tokens.push(TemplateToken::Text {
            content: current_text,
            line: text_start_line,
        });
    }

    // Report unclosed blocks
    for (structure, line) in &block_stack {
        errors.push(TemplateParseError {
            message: format!("Unclosed {:?} block", structure),
            line: *line,
        });
    }

    ParsedTemplate {
        path: path.to_string(),
        tokens,
        variables_used,
        functions_called,
        defined_templates,
        referenced_templates,
        unclosed_blocks: block_stack,
        errors,
    }
}

/// Parse a template from a file.
pub fn parse_template_file(path: &Path) -> Result<ParsedTemplate, std::io::Error> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_template(&content, &path.display().to_string()))
}

/// Analyze a template action for variables, functions, and control structures.
fn analyze_action(
    content: &str,
    line: u32,
    variables: &mut HashSet<String>,
    functions: &mut HashSet<String>,
    defined: &mut HashSet<String>,
    referenced: &mut HashSet<String>,
    block_stack: &mut Vec<(ControlStructure, u32)>,
) {
    let trimmed = content.trim();

    // Handle control structures
    if let Some(structure) = ControlStructure::parse(trimmed) {
        match &structure {
            ControlStructure::Define | ControlStructure::Block => {
                // Extract template name
                if let Some(name) = extract_template_name(trimmed) {
                    defined.insert(name);
                }
                block_stack.push((structure, line));
            }
            ControlStructure::Template => {
                // Extract referenced template name
                if let Some(name) = extract_template_name(trimmed) {
                    referenced.insert(name);
                }
            }
            ControlStructure::End => {
                block_stack.pop();
            }
            s if s.starts_block() => {
                block_stack.push((structure, line));
            }
            _ => {}
        }
    }

    // Extract variables (things starting with .)
    extract_variables(trimmed, variables);

    // Extract function calls
    extract_functions(trimmed, functions, referenced);
}

/// Extract variable references from action content.
fn extract_variables(content: &str, variables: &mut HashSet<String>) {
    let mut chars = content.chars().peekable();
    let mut current_var = String::new();
    let mut in_var = false;

    while let Some(c) = chars.next() {
        if c == '.' && !in_var {
            // Start of a variable reference
            in_var = true;
            current_var.push(c);
        } else if in_var {
            if c.is_alphanumeric() || c == '_' || c == '.' {
                current_var.push(c);
            } else {
                // End of variable
                if !current_var.is_empty() && current_var.len() > 1 {
                    variables.insert(std::mem::take(&mut current_var));
                }
                current_var.clear();
                in_var = false;
            }
        }
    }

    // Don't forget the last variable
    if !current_var.is_empty() && current_var.len() > 1 {
        variables.insert(current_var);
    }
}

/// Extract function calls from action content.
fn extract_functions(content: &str, functions: &mut HashSet<String>, referenced: &mut HashSet<String>) {
    // Common Helm/Sprig functions to detect
    let known_functions = [
        "include", "tpl", "lookup", "required", "default", "empty", "coalesce",
        "toYaml", "toJson", "fromYaml", "fromJson", "indent", "nindent",
        "trim", "trimAll", "trimPrefix", "trimSuffix", "quote", "squote",
        "upper", "lower", "title", "untitle", "substr", "replace", "trunc",
        "list", "dict", "get", "set", "unset", "hasKey", "keys", "values",
        "merge", "mergeOverwrite", "append", "prepend", "concat", "first", "last",
        "printf", "print", "println", "fail", "kindOf", "typeOf", "deepEqual",
        "b64enc", "b64dec", "sha256sum", "randAlphaNum", "randAlpha",
        "now", "date", "dateModify", "toDate", "env", "expandenv",
    ];

    for func in known_functions {
        if content.contains(func) {
            functions.insert(func.to_string());
        }
    }

    // Extract include/template references
    if content.contains("include") || content.contains("template") {
        // Try to extract the template name from include "name" or template "name"
        let parts: Vec<&str> = content.split('"').collect();
        if parts.len() >= 2 {
            let name = parts[1].trim();
            if !name.is_empty() {
                referenced.insert(name.to_string());
            }
        }
    }
}

/// Extract template name from define/block/template action.
fn extract_template_name(content: &str) -> Option<String> {
    // Pattern: define "name" or template "name" or block "name"
    let parts: Vec<&str> = content.split('"').collect();
    if parts.len() >= 2 {
        let name = parts[1].trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_template() {
        let content = r#"apiVersion: v1
kind: ConfigMap
metadata:
  name: {{ .Release.Name }}-config
data:
  value: {{ .Values.config.value }}
"#;
        let parsed = parse_template(content, "configmap.yaml");
        assert!(parsed.errors.is_empty());
        assert!(parsed.variables_used.contains(".Release.Name"));
        assert!(parsed.variables_used.contains(".Values.config.value"));
    }

    #[test]
    fn test_parse_control_structures() {
        let content = r#"{{- if .Values.enabled }}
apiVersion: v1
kind: Service
{{- end }}
"#;
        let parsed = parse_template(content, "service.yaml");
        assert!(parsed.errors.is_empty());
        assert!(parsed.unclosed_blocks.is_empty());
    }

    #[test]
    fn test_unclosed_block() {
        let content = r#"{{- if .Values.enabled }}
apiVersion: v1
kind: Service
"#;
        let parsed = parse_template(content, "service.yaml");
        assert!(!parsed.errors.is_empty());
        assert!(parsed.has_unclosed_blocks());
    }

    #[test]
    fn test_detect_functions() {
        let content = r#"
{{ include "mychart.labels" . }}
{{ .Values.name | default "default-name" | quote }}
{{ toYaml .Values.config | nindent 4 }}
"#;
        let parsed = parse_template(content, "deployment.yaml");
        assert!(parsed.calls_function("include"));
        assert!(parsed.calls_function("default"));
        assert!(parsed.calls_function("quote"));
        assert!(parsed.calls_function("toYaml"));
        assert!(parsed.calls_function("nindent"));
    }

    #[test]
    fn test_detect_lookup() {
        let content = r#"
{{- $secret := lookup "v1" "Secret" .Release.Namespace "my-secret" }}
"#;
        let parsed = parse_template(content, "secret.yaml");
        assert!(parsed.uses_lookup());
    }

    #[test]
    fn test_detect_tpl() {
        let content = r#"
{{ tpl .Values.customTemplate . }}
"#;
        let parsed = parse_template(content, "custom.yaml");
        assert!(parsed.uses_tpl());
    }

    #[test]
    fn test_parse_define() {
        let content = r#"
{{- define "mychart.name" -}}
{{ .Chart.Name }}
{{- end -}}
"#;
        let parsed = parse_template(content, "_helpers.tpl");
        assert!(parsed.errors.is_empty());
        assert!(parsed.defined_templates.contains("mychart.name"));
    }

    #[test]
    fn test_parse_comment() {
        let content = r#"
{{/* This is a comment */}}
apiVersion: v1
"#;
        let parsed = parse_template(content, "test.yaml");
        let comments: Vec<_> = parsed
            .tokens
            .iter()
            .filter(|t| matches!(t, TemplateToken::Comment { .. }))
            .collect();
        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn test_values_references() {
        let content = r#"
image: {{ .Values.image.repository }}:{{ .Values.image.tag }}
replicas: {{ .Values.replicaCount }}
"#;
        let parsed = parse_template(content, "deployment.yaml");
        let refs = parsed.values_references();
        assert!(refs.contains(&".Values.image.repository"));
        assert!(refs.contains(&".Values.image.tag"));
        assert!(refs.contains(&".Values.replicaCount"));
    }

    #[test]
    fn test_unclosed_action() {
        let content = "{{ .Values.name";
        let parsed = parse_template(content, "test.yaml");
        assert!(!parsed.errors.is_empty());
        assert!(parsed.errors[0].message.contains("Unclosed"));
    }

    #[test]
    fn test_trim_markers() {
        let content = "{{- .Values.name -}}";
        let parsed = parse_template(content, "test.yaml");
        if let Some(TemplateToken::Action {
            trim_left,
            trim_right,
            ..
        }) = parsed.tokens.first()
        {
            assert!(*trim_left);
            assert!(*trim_right);
        } else {
            panic!("Expected Action token");
        }
    }
}
