//! Chart.yaml parser.
//!
//! Parses Helm chart metadata from Chart.yaml files.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Helm Chart API version.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ApiVersion {
    /// Helm 2 style charts
    V1,
    /// Helm 3 style charts
    #[default]
    V2,
    /// Unknown/invalid version
    Unknown(String),
}

impl<'de> Deserialize<'de> for ApiVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "v1" => ApiVersion::V1,
            "v2" => ApiVersion::V2,
            other => ApiVersion::Unknown(other.to_string()),
        })
    }
}

impl Serialize for ApiVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ApiVersion::V1 => serializer.serialize_str("v1"),
            ApiVersion::V2 => serializer.serialize_str("v2"),
            ApiVersion::Unknown(s) => serializer.serialize_str(s),
        }
    }
}

/// Chart type.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChartType {
    /// Standard application chart
    #[default]
    Application,
    /// Library chart (no templates rendered directly)
    Library,
}

/// Chart maintainer information.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Maintainer {
    /// Maintainer name
    pub name: String,
    /// Maintainer email
    pub email: Option<String>,
    /// Maintainer URL
    pub url: Option<String>,
}

/// Chart dependency.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Dependency {
    /// Dependency chart name
    pub name: String,
    /// Version constraint (SemVer)
    pub version: Option<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// Condition for enabling
    pub condition: Option<String>,
    /// Tags for enabling
    pub tags: Option<Vec<String>>,
    /// Import values configuration
    #[serde(rename = "import-values")]
    pub import_values: Option<Vec<serde_yaml::Value>>,
    /// Alias for the dependency
    pub alias: Option<String>,
}

/// Parsed Chart.yaml metadata.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChartMetadata {
    /// The chart API version (v1 or v2)
    #[serde(rename = "apiVersion")]
    pub api_version: ApiVersion,

    /// The name of the chart
    pub name: String,

    /// A SemVer 2 version
    pub version: String,

    /// Kubernetes version constraint
    #[serde(rename = "kubeVersion")]
    pub kube_version: Option<String>,

    /// A single-sentence description of this project
    pub description: Option<String>,

    /// The type of the chart (application or library)
    #[serde(rename = "type")]
    pub chart_type: Option<ChartType>,

    /// A list of keywords about this project
    #[serde(default)]
    pub keywords: Vec<String>,

    /// The URL of this projects home page
    pub home: Option<String>,

    /// A list of URLs to source code for this project
    #[serde(default)]
    pub sources: Vec<String>,

    /// A list of chart dependencies
    #[serde(default)]
    pub dependencies: Vec<Dependency>,

    /// A list of maintainers
    #[serde(default)]
    pub maintainers: Vec<Maintainer>,

    /// A URL to an SVG or PNG image to be used as an icon
    pub icon: Option<String>,

    /// The version of the app that this contains
    #[serde(rename = "appVersion")]
    pub app_version: Option<String>,

    /// Whether this chart is deprecated
    pub deprecated: Option<bool>,

    /// Annotations
    #[serde(default)]
    pub annotations: HashMap<String, String>,
}

impl ChartMetadata {
    /// Check if the chart has valid API version.
    pub fn has_valid_api_version(&self) -> bool {
        matches!(self.api_version, ApiVersion::V1 | ApiVersion::V2)
    }

    /// Check if this is a v2 (Helm 3) chart.
    pub fn is_v2(&self) -> bool {
        matches!(self.api_version, ApiVersion::V2)
    }

    /// Check if this is a library chart.
    pub fn is_library(&self) -> bool {
        matches!(self.chart_type, Some(ChartType::Library))
    }

    /// Check if the chart is marked as deprecated.
    pub fn is_deprecated(&self) -> bool {
        self.deprecated.unwrap_or(false)
    }

    /// Get dependency names.
    pub fn dependency_names(&self) -> Vec<&str> {
        self.dependencies.iter().map(|d| d.name.as_str()).collect()
    }

    /// Check for duplicate dependency names.
    pub fn has_duplicate_dependencies(&self) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        let mut duplicates = Vec::new();
        for dep in &self.dependencies {
            let name = dep.alias.as_ref().unwrap_or(&dep.name);
            if !seen.insert(name.as_str()) {
                duplicates.push(name.as_str());
            }
        }
        duplicates
    }
}

/// Parse error for Chart.yaml.
#[derive(Debug)]
pub struct ChartParseError {
    pub message: String,
    pub line: Option<u32>,
}

impl std::fmt::Display for ChartParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line {
            write!(f, "line {}: {}", line, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ChartParseError {}

/// Parse Chart.yaml content.
pub fn parse_chart_yaml(content: &str) -> Result<ChartMetadata, ChartParseError> {
    serde_yaml::from_str(content).map_err(|e| {
        let line = e.location().map(|l| l.line() as u32);
        ChartParseError {
            message: e.to_string(),
            line,
        }
    })
}

/// Parse Chart.yaml from a file path.
pub fn parse_chart_yaml_file(path: &Path) -> Result<ChartMetadata, ChartParseError> {
    let content = std::fs::read_to_string(path).map_err(|e| ChartParseError {
        message: format!("Failed to read file: {}", e),
        line: None,
    })?;
    parse_chart_yaml(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_chart() {
        let yaml = r#"
apiVersion: v2
name: test-chart
version: 0.1.0
"#;
        let chart = parse_chart_yaml(yaml).unwrap();
        assert_eq!(chart.name, "test-chart");
        assert_eq!(chart.version, "0.1.0");
        assert!(chart.is_v2());
    }

    #[test]
    fn test_parse_full_chart() {
        let yaml = r#"
apiVersion: v2
name: my-app
version: 1.2.3
kubeVersion: ">=1.19.0"
description: A sample application
type: application
keywords:
  - app
  - example
home: https://example.com
sources:
  - https://github.com/example/my-app
maintainers:
  - name: John Doe
    email: john@example.com
icon: https://example.com/icon.png
appVersion: "2.0.0"
dependencies:
  - name: postgresql
    version: "~11.0"
    repository: https://charts.bitnami.com/bitnami
annotations:
  category: backend
"#;
        let chart = parse_chart_yaml(yaml).unwrap();
        assert_eq!(chart.name, "my-app");
        assert_eq!(chart.version, "1.2.3");
        assert_eq!(chart.kube_version, Some(">=1.19.0".to_string()));
        assert_eq!(chart.description, Some("A sample application".to_string()));
        assert!(!chart.is_library());
        assert_eq!(chart.keywords.len(), 2);
        assert_eq!(chart.maintainers.len(), 1);
        assert_eq!(chart.dependencies.len(), 1);
    }

    #[test]
    fn test_parse_library_chart() {
        let yaml = r#"
apiVersion: v2
name: common
version: 1.0.0
type: library
"#;
        let chart = parse_chart_yaml(yaml).unwrap();
        assert!(chart.is_library());
    }

    #[test]
    fn test_parse_v1_chart() {
        let yaml = r#"
apiVersion: v1
name: legacy-chart
version: 1.0.0
"#;
        let chart = parse_chart_yaml(yaml).unwrap();
        assert!(!chart.is_v2());
        assert!(chart.has_valid_api_version());
    }

    #[test]
    fn test_deprecated_chart() {
        let yaml = r#"
apiVersion: v2
name: old-chart
version: 1.0.0
deprecated: true
"#;
        let chart = parse_chart_yaml(yaml).unwrap();
        assert!(chart.is_deprecated());
    }

    #[test]
    fn test_duplicate_dependencies() {
        let yaml = r#"
apiVersion: v2
name: test
version: 1.0.0
dependencies:
  - name: redis
    version: "1.0.0"
    repository: https://charts.bitnami.com/bitnami
  - name: redis
    version: "2.0.0"
    repository: https://charts.bitnami.com/bitnami
"#;
        let chart = parse_chart_yaml(yaml).unwrap();
        let duplicates = chart.has_duplicate_dependencies();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0], "redis");
    }

    #[test]
    fn test_parse_error() {
        let yaml = "invalid: [yaml";
        let result = parse_chart_yaml(yaml);
        assert!(result.is_err());
    }
}
