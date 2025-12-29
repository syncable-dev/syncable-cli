//! Values.yaml parser.
//!
//! Parses Helm values files with position tracking for error reporting.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_yaml::Value;

/// Parsed values file with metadata.
#[derive(Debug, Clone)]
pub struct ValuesFile {
    /// The parsed YAML values.
    pub values: Value,
    /// Map of value paths to their line numbers.
    pub line_map: HashMap<String, u32>,
    /// All defined value paths.
    pub defined_paths: HashSet<String>,
}

impl ValuesFile {
    /// Create a new empty values file.
    pub fn empty() -> Self {
        Self {
            values: Value::Mapping(serde_yaml::Mapping::new()),
            line_map: HashMap::new(),
            defined_paths: HashSet::new(),
        }
    }

    /// Get a value by path (e.g., "image.repository").
    pub fn get(&self, path: &str) -> Option<&Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &self.values;

        for part in parts {
            match current {
                Value::Mapping(map) => {
                    current = map.get(Value::String(part.to_string()))?;
                }
                _ => return None,
            }
        }

        Some(current)
    }

    /// Check if a path is defined.
    pub fn has_path(&self, path: &str) -> bool {
        self.defined_paths.contains(path)
    }

    /// Get the line number for a path.
    pub fn line_for_path(&self, path: &str) -> Option<u32> {
        self.line_map.get(path).copied()
    }

    /// Get all paths that match a pattern (simple prefix matching).
    pub fn paths_with_prefix(&self, prefix: &str) -> Vec<&str> {
        self.defined_paths
            .iter()
            .filter(|p| p.starts_with(prefix))
            .map(|s| s.as_str())
            .collect()
    }

    /// Check if a value is a sensitive field (common patterns).
    pub fn is_sensitive_path(path: &str) -> bool {
        let lower = path.to_lowercase();
        lower.contains("password")
            || lower.contains("secret")
            || lower.contains("token")
            || lower.contains("key")
            || lower.contains("credential")
            || lower.contains("apikey")
            || lower.contains("api_key")
            || lower.ends_with(".auth")
    }

    /// Get all sensitive paths.
    pub fn sensitive_paths(&self) -> Vec<&str> {
        self.defined_paths
            .iter()
            .filter(|p| Self::is_sensitive_path(p))
            .map(|s| s.as_str())
            .collect()
    }
}

/// Parse error for values.yaml.
#[derive(Debug)]
pub struct ValuesParseError {
    pub message: String,
    pub line: Option<u32>,
}

impl std::fmt::Display for ValuesParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line {
            write!(f, "line {}: {}", line, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ValuesParseError {}

/// Parse values.yaml content.
pub fn parse_values_yaml(content: &str) -> Result<ValuesFile, ValuesParseError> {
    // Parse the YAML
    let values: Value = serde_yaml::from_str(content).map_err(|e| {
        let line = e.location().map(|l| l.line() as u32);
        ValuesParseError {
            message: e.to_string(),
            line,
        }
    })?;

    // Build line map by re-parsing with position tracking
    let (line_map, defined_paths) = build_line_map(content);

    Ok(ValuesFile {
        values,
        line_map,
        defined_paths,
    })
}

/// Parse values.yaml from a file path.
pub fn parse_values_yaml_file(path: &Path) -> Result<ValuesFile, ValuesParseError> {
    let content = std::fs::read_to_string(path).map_err(|e| ValuesParseError {
        message: format!("Failed to read file: {}", e),
        line: None,
    })?;
    parse_values_yaml(&content)
}

/// Build a map of value paths to line numbers.
fn build_line_map(content: &str) -> (HashMap<String, u32>, HashSet<String>) {
    let mut line_map = HashMap::new();
    let mut defined_paths = HashSet::new();
    let mut path_stack: Vec<(String, usize)> = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line_number = (line_num + 1) as u32;
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Count indentation (spaces)
        let indent = line.len() - line.trim_start().len();

        // Pop items from stack that are at same or greater indentation
        while let Some((_, stack_indent)) = path_stack.last() {
            if indent <= *stack_indent {
                path_stack.pop();
            } else {
                break;
            }
        }

        // Check if this line defines a key
        if let Some(colon_pos) = trimmed.find(':') {
            let key = trimmed[..colon_pos].trim();

            // Skip if key contains special characters that indicate it's not a simple key
            if key.contains(' ') && !key.starts_with('"') && !key.starts_with('\'') {
                continue;
            }

            // Clean up quoted keys
            let key = key.trim_matches('"').trim_matches('\'');

            // Build the full path
            let full_path = if path_stack.is_empty() {
                key.to_string()
            } else {
                let parent_path = &path_stack.last().unwrap().0;
                format!("{}.{}", parent_path, key)
            };

            line_map.insert(full_path.clone(), line_number);
            defined_paths.insert(full_path.clone());

            // Check if this key has a nested value (no value after colon or just whitespace)
            let after_colon = trimmed[colon_pos + 1..].trim();
            if after_colon.is_empty() || after_colon.starts_with('#') {
                // This is a parent key, add to stack
                path_stack.push((full_path, indent));
            }
        }
    }

    (line_map, defined_paths)
}

/// Extract all value references from a path expression.
/// E.g., ".Values.image.repository" -> "image.repository"
pub fn extract_values_path(expr: &str) -> Option<&str> {
    let trimmed = expr.trim();
    if trimmed.starts_with(".Values.") {
        Some(&trimmed[8..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_values() {
        let yaml = r#"
replicaCount: 1
image:
  repository: nginx
  tag: "1.25"
"#;
        let values = parse_values_yaml(yaml).unwrap();
        assert!(values.has_path("replicaCount"));
        assert!(values.has_path("image"));
        assert!(values.has_path("image.repository"));
        assert!(values.has_path("image.tag"));
    }

    #[test]
    fn test_get_value() {
        let yaml = r#"
image:
  repository: nginx
  tag: "1.25"
service:
  port: 80
"#;
        let values = parse_values_yaml(yaml).unwrap();

        assert_eq!(
            values.get("image.repository"),
            Some(&Value::String("nginx".to_string()))
        );
        assert_eq!(
            values.get("service.port"),
            Some(&Value::Number(80.into()))
        );
        assert_eq!(values.get("nonexistent"), None);
    }

    #[test]
    fn test_line_numbers() {
        let yaml = r#"replicaCount: 1
image:
  repository: nginx
  tag: "1.25"
"#;
        let values = parse_values_yaml(yaml).unwrap();
        assert_eq!(values.line_for_path("replicaCount"), Some(1));
        assert_eq!(values.line_for_path("image"), Some(2));
        assert_eq!(values.line_for_path("image.repository"), Some(3));
        assert_eq!(values.line_for_path("image.tag"), Some(4));
    }

    #[test]
    fn test_sensitive_paths() {
        let yaml = r#"
database:
  password: secret123
  host: localhost
auth:
  apiKey: abc123
  token: xyz789
"#;
        let values = parse_values_yaml(yaml).unwrap();
        let sensitive = values.sensitive_paths();

        assert!(sensitive.contains(&"database.password"));
        assert!(sensitive.contains(&"auth.apiKey"));
        assert!(sensitive.contains(&"auth.token"));
        assert!(!sensitive.contains(&"database.host"));
    }

    #[test]
    fn test_extract_values_path() {
        assert_eq!(
            extract_values_path(".Values.image.repository"),
            Some("image.repository")
        );
        assert_eq!(
            extract_values_path(".Values.replicaCount"),
            Some("replicaCount")
        );
        assert_eq!(extract_values_path(".Release.Name"), None);
        assert_eq!(extract_values_path("something.else"), None);
    }

    #[test]
    fn test_paths_with_prefix() {
        let yaml = r#"
image:
  repository: nginx
  tag: "1.25"
  pullPolicy: Always
service:
  port: 80
"#;
        let values = parse_values_yaml(yaml).unwrap();
        let image_paths = values.paths_with_prefix("image.");

        assert_eq!(image_paths.len(), 3);
        assert!(image_paths.contains(&"image.repository"));
        assert!(image_paths.contains(&"image.tag"));
        assert!(image_paths.contains(&"image.pullPolicy"));
    }

    #[test]
    fn test_empty_values() {
        let values = ValuesFile::empty();
        assert!(!values.has_path("anything"));
    }

    #[test]
    fn test_parse_error() {
        let yaml = "invalid: [yaml";
        let result = parse_values_yaml(yaml);
        assert!(result.is_err());
    }
}
