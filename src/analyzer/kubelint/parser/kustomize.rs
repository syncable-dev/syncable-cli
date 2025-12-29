//! Kustomize support for Kubernetes manifests.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::parser::yaml;
use std::path::Path;
use std::process::Command;

/// Render a Kustomize directory to Kubernetes objects.
///
/// This function shells out to `kustomize build` (or `kubectl kustomize`)
/// to render the directory and then parses the resulting YAML.
pub fn render_kustomize(dir: &Path) -> Result<Vec<Object>, KustomizeError> {
    // Try kustomize binary first, fall back to kubectl kustomize
    let output = if is_kustomize_available() {
        let mut cmd = Command::new("kustomize");
        cmd.arg("build").arg(dir);
        cmd.output()
            .map_err(|e| KustomizeError::BuildError(e.to_string()))?
    } else if is_kubectl_kustomize_available() {
        let mut cmd = Command::new("kubectl");
        cmd.arg("kustomize").arg(dir);
        cmd.output()
            .map_err(|e| KustomizeError::BuildError(e.to_string()))?
    } else {
        return Err(KustomizeError::KustomizeNotFound);
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(KustomizeError::BuildError(stderr.to_string()));
    }

    // Parse the rendered YAML
    let yaml_content = String::from_utf8_lossy(&output.stdout);
    yaml::parse_yaml_with_path(&yaml_content, dir)
        .map_err(|e| KustomizeError::BuildError(e.to_string()))
}

/// Render Kustomize with specific options.
pub fn render_kustomize_with_options(
    dir: &Path,
    enable_helm: bool,
    load_restrictors: LoadRestrictors,
) -> Result<Vec<Object>, KustomizeError> {
    if !is_kustomize_available() && !is_kubectl_kustomize_available() {
        return Err(KustomizeError::KustomizeNotFound);
    }

    let output = if is_kustomize_available() {
        let mut cmd = Command::new("kustomize");
        cmd.arg("build").arg(dir);

        if enable_helm {
            cmd.arg("--enable-helm");
        }

        match load_restrictors {
            LoadRestrictors::None => {
                cmd.arg("--load-restrictor=none");
            }
            LoadRestrictors::RootOnly => {
                // Default behavior, no flag needed
            }
        }

        cmd.output()
            .map_err(|e| KustomizeError::BuildError(e.to_string()))?
    } else {
        // kubectl kustomize has limited options
        let mut cmd = Command::new("kubectl");
        cmd.arg("kustomize").arg(dir);

        if enable_helm {
            cmd.arg("--enable-helm");
        }

        cmd.output()
            .map_err(|e| KustomizeError::BuildError(e.to_string()))?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(KustomizeError::BuildError(stderr.to_string()));
    }

    let yaml_content = String::from_utf8_lossy(&output.stdout);
    yaml::parse_yaml_with_path(&yaml_content, dir)
        .map_err(|e| KustomizeError::BuildError(e.to_string()))
}

/// Load restrictor options for kustomize.
#[derive(Debug, Clone, Copy, Default)]
pub enum LoadRestrictors {
    /// No restrictions (can load from anywhere).
    None,
    /// Only load from root directory (default).
    #[default]
    RootOnly,
}

/// Check if a directory is a Kustomize directory.
pub fn is_kustomize_dir(path: &Path) -> bool {
    path.join("kustomization.yaml").exists()
        || path.join("kustomization.yml").exists()
        || path.join("Kustomization").exists()
}

/// Check if kustomize binary is available in PATH.
pub fn is_kustomize_available() -> bool {
    Command::new("kustomize")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if kubectl kustomize is available.
pub fn is_kubectl_kustomize_available() -> bool {
    Command::new("kubectl")
        .arg("kustomize")
        .arg("--help")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get kustomize version if available.
pub fn kustomize_version() -> Option<String> {
    // Try kustomize binary first
    if let Some(version) = Command::new("kustomize")
        .arg("version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    {
        return Some(version);
    }

    // Fall back to kubectl version
    Command::new("kubectl")
        .arg("version")
        .arg("--client")
        .arg("-o")
        .arg("json")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            let output = String::from_utf8_lossy(&o.stdout);
            format!("kubectl ({})", output.lines().next().unwrap_or("unknown"))
        })
}

/// Kustomize errors.
#[derive(Debug, Clone)]
pub enum KustomizeError {
    /// kustomize binary not found.
    KustomizeNotFound,
    /// Build error.
    BuildError(String),
}

impl std::fmt::Display for KustomizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KustomizeNotFound => {
                write!(
                    f,
                    "kustomize binary not found in PATH (tried 'kustomize' and 'kubectl kustomize')"
                )
            }
            Self::BuildError(msg) => write!(f, "Build error: {}", msg),
        }
    }
}

impl std::error::Error for KustomizeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_kustomize_dir_detection() {
        let temp_dir = std::env::temp_dir();
        assert!(!is_kustomize_dir(&temp_dir)); // temp dir is not a Kustomize dir
    }

    #[test]
    fn test_kustomize_availability() {
        // Just verify the function runs without panicking
        let _available = is_kustomize_available();
        let _kubectl_available = is_kubectl_kustomize_available();
    }
}
