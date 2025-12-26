//! YAML parser for Docker Compose files.
//!
//! Provides parsing of docker-compose.yaml files with position tracking
//! for accurate error reporting.

pub mod compose;

pub use compose::{
    ComposeFile, ParseError, Position, Service, ServiceBuild, ServicePort, ServiceVolume,
    parse_compose, parse_compose_with_positions,
};

use yaml_rust2::{Yaml, YamlLoader};

/// Parse a YAML string and return the document.
pub fn parse_yaml(content: &str) -> Result<Yaml, ParseError> {
    let docs =
        YamlLoader::load_from_str(content).map_err(|e| ParseError::YamlError(e.to_string()))?;

    docs.into_iter().next().ok_or(ParseError::EmptyDocument)
}

/// Find the line number for a given path in the source YAML.
///
/// This function searches the raw source for the key to determine its position.
pub fn find_line_for_key(source: &str, path: &[&str]) -> Option<u32> {
    if path.is_empty() {
        return Some(1);
    }

    let lines: Vec<&str> = source.lines().collect();
    let mut current_indent = 0;
    let mut path_idx = 0;

    for (line_num, line) in lines.iter().enumerate() {
        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }

        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();

        // Check if this line starts with the current path element as a key
        let target_key = path[path_idx];
        let key_pattern = format!("{}:", target_key);

        if trimmed.starts_with(&key_pattern) || trimmed == target_key {
            if path_idx == 0 || indent > current_indent {
                path_idx += 1;
                current_indent = indent;

                if path_idx == path.len() {
                    return Some((line_num + 1) as u32); // 1-indexed
                }
            }
        }
    }

    None
}

/// Find the line number for a service key.
pub fn find_line_for_service(source: &str, service_name: &str) -> Option<u32> {
    find_line_for_key(source, &["services", service_name])
}

/// Find the line number for a key within a service.
pub fn find_line_for_service_key(source: &str, service_name: &str, key: &str) -> Option<u32> {
    find_line_for_key(source, &["services", service_name, key])
}

/// Find the column for a value on a given line.
pub fn find_column_for_value(source: &str, line: u32, key: &str) -> u32 {
    let lines: Vec<&str> = source.lines().collect();
    if let Some(line_content) = lines.get((line - 1) as usize) {
        if let Some(pos) = line_content.find(':') {
            // Column after the colon and any whitespace
            let after_colon = &line_content[pos + 1..];
            let leading_ws = after_colon.len() - after_colon.trim_start().len();
            return (pos + 2 + leading_ws) as u32;
        }
        // If no colon, look for the key position
        if let Some(pos) = line_content.find(key) {
            return (pos + 1) as u32;
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_line_for_key() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "80:80"
  db:
    image: postgres
"#;
        assert_eq!(find_line_for_key(yaml, &["services"]), Some(2));
        assert_eq!(find_line_for_key(yaml, &["services", "web"]), Some(3));
        assert_eq!(
            find_line_for_key(yaml, &["services", "web", "image"]),
            Some(4)
        );
        assert_eq!(find_line_for_key(yaml, &["services", "db"]), Some(7));
    }

    #[test]
    fn test_find_line_for_service() {
        let yaml = r#"
services:
  web:
    image: nginx
  db:
    image: postgres
"#;
        assert_eq!(find_line_for_service(yaml, "web"), Some(3));
        assert_eq!(find_line_for_service(yaml, "db"), Some(5));
        assert_eq!(find_line_for_service(yaml, "nonexistent"), None);
    }
}
