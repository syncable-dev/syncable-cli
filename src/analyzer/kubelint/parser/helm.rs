//! Helm chart rendering for Kubernetes manifests.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::parser::yaml;
use std::path::Path;
use std::process::Command;

/// Render a Helm chart to Kubernetes objects.
///
/// This function shells out to the `helm template` command to render
/// the chart and then parses the resulting YAML.
pub fn render_helm_chart(
    chart_path: &Path,
    values: Option<&Path>,
) -> Result<Vec<Object>, HelmError> {
    // Check if helm binary is available
    if !is_helm_available() {
        return Err(HelmError::HelmNotFound);
    }

    // Build helm template command
    let mut cmd = Command::new("helm");
    cmd.arg("template")
        .arg("release-name") // Use a default release name for linting
        .arg(chart_path);

    // Add values file if provided
    if let Some(values_path) = values {
        cmd.arg("-f").arg(values_path);
    }

    // Execute helm template
    let output = cmd
        .output()
        .map_err(|e| HelmError::RenderError(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(HelmError::RenderError(stderr.to_string()));
    }

    // Parse the rendered YAML
    let yaml_content = String::from_utf8_lossy(&output.stdout);
    yaml::parse_yaml_with_path(&yaml_content, chart_path)
        .map_err(|e| HelmError::RenderError(e.to_string()))
}

/// Render a Helm chart with custom values.
pub fn render_helm_chart_with_values(
    chart_path: &Path,
    values_files: &[&Path],
    set_values: &[(&str, &str)],
) -> Result<Vec<Object>, HelmError> {
    if !is_helm_available() {
        return Err(HelmError::HelmNotFound);
    }

    let mut cmd = Command::new("helm");
    cmd.arg("template").arg("release-name").arg(chart_path);

    // Add all values files
    for values_path in values_files {
        cmd.arg("-f").arg(values_path);
    }

    // Add --set values
    for (key, value) in set_values {
        cmd.arg("--set").arg(format!("{}={}", key, value));
    }

    let output = cmd
        .output()
        .map_err(|e| HelmError::RenderError(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(HelmError::RenderError(stderr.to_string()));
    }

    let yaml_content = String::from_utf8_lossy(&output.stdout);
    yaml::parse_yaml_with_path(&yaml_content, chart_path)
        .map_err(|e| HelmError::RenderError(e.to_string()))
}

/// Check if a directory is a Helm chart.
pub fn is_helm_chart(path: &Path) -> bool {
    path.join("Chart.yaml").exists() || path.join("Chart.yml").exists()
}

/// Check if helm binary is available in PATH.
pub fn is_helm_available() -> bool {
    Command::new("helm")
        .arg("version")
        .arg("--short")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get Helm version if available.
pub fn helm_version() -> Option<String> {
    Command::new("helm")
        .arg("version")
        .arg("--short")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Helm rendering errors.
#[derive(Debug, Clone)]
pub enum HelmError {
    /// Helm binary not found.
    HelmNotFound,
    /// Chart validation error.
    ChartError(String),
    /// Rendering error.
    RenderError(String),
}

impl std::fmt::Display for HelmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HelmNotFound => write!(f, "helm binary not found in PATH"),
            Self::ChartError(msg) => write!(f, "Chart error: {}", msg),
            Self::RenderError(msg) => write!(f, "Render error: {}", msg),
        }
    }
}

impl std::error::Error for HelmError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_helm_chart_detection() {
        // This test checks the detection logic without requiring actual files
        let temp_dir = std::env::temp_dir();
        assert!(!is_helm_chart(&temp_dir)); // temp dir is not a Helm chart
    }

    #[test]
    fn test_helm_availability() {
        // Just verify the function runs without panicking
        let _available = is_helm_available();
    }
}
