//! CD Manifest Writer — CD-22
//!
//! Serialises both the resolved and unresolved token inventories, plus
//! environment metadata, to `cd-manifest.toml`.
//!
//! The manifest file serves two purposes:
//!   1. **Agent fill phase** — the LLM agent reads `[unresolved]` entries
//!      and patches them with real values.
//!   2. **Interactive prompts** — the wizard presents `[unresolved]` entries
//!      to the human developer for manual input.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

use crate::error::{GeneratorError, IaCGeneratorError};

use super::schema::{EnvironmentConfig, UnresolvedToken};
use super::token_resolver::ResolvedTokenMap;

// ── Manifest structure ────────────────────────────────────────────────────────

/// A single unresolved token entry in the TOML manifest.
#[derive(Debug, Serialize)]
struct UnresolvedEntry {
    #[serde(rename = "type")]
    token_type: String,
    hint: String,
}

/// A single environment entry in the TOML manifest.
#[derive(Debug, Serialize)]
struct EnvironmentEntry {
    requires_approval: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    app_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replicas: Option<u32>,
}

/// Top-level manifest structure serialised to TOML.
#[derive(Debug, Serialize)]
struct CdManifest {
    resolved: HashMap<String, String>,
    unresolved: HashMap<String, UnresolvedEntry>,
    environments: HashMap<String, EnvironmentEntry>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Writes `cd-manifest.toml` containing resolved tokens, unresolved tokens,
/// and environment configuration.
pub fn write_cd_manifest(
    resolved: &ResolvedTokenMap,
    unresolved: &[UnresolvedToken],
    environments: &[EnvironmentConfig],
    dest: &Path,
) -> crate::Result<()> {
    let manifest = CdManifest {
        resolved: resolved.clone(),
        unresolved: unresolved
            .iter()
            .map(|u| {
                (
                    u.name.clone(),
                    UnresolvedEntry {
                        token_type: u.token_type.clone(),
                        hint: u.hint.clone(),
                    },
                )
            })
            .collect(),
        environments: environments
            .iter()
            .map(|e| {
                (
                    e.name.clone(),
                    EnvironmentEntry {
                        requires_approval: e.requires_approval,
                        branch_filter: e.branch_filter.clone(),
                        app_url: e.app_url.clone(),
                        namespace: e.namespace.clone(),
                        replicas: e.replicas,
                    },
                )
            })
            .collect(),
    };

    let content = toml::to_string_pretty(&manifest).map_err(|e| {
        IaCGeneratorError::Generation(GeneratorError::InvalidContext(e.to_string()))
    })?;

    std::fs::write(dest, content)?;
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_resolved() -> ResolvedTokenMap {
        let mut map = HashMap::new();
        map.insert("PROJECT_NAME".to_string(), "my-app".to_string());
        map.insert("IMAGE_NAME".to_string(), "my-app".to_string());
        map.insert("REGISTRY_URL".to_string(), "ghcr.io".to_string());
        map.insert("DEFAULT_BRANCH".to_string(), "main".to_string());
        map
    }

    fn sample_unresolved() -> Vec<UnresolvedToken> {
        vec![
            UnresolvedToken::new("APP_URL", "Public URL of your application", "url"),
            UnresolvedToken::new("GCP_REGION", "GCP region for deployment", "string"),
        ]
    }

    fn sample_environments() -> Vec<EnvironmentConfig> {
        vec![
            EnvironmentConfig {
                name: "staging".to_string(),
                branch_filter: Some("develop".to_string()),
                requires_approval: false,
                app_url: None,
                namespace: Some("staging".to_string()),
                replicas: Some(1),
            },
            EnvironmentConfig {
                name: "production".to_string(),
                branch_filter: Some("main".to_string()),
                requires_approval: true,
                app_url: Some("https://my-app.example.com".to_string()),
                namespace: Some("prod".to_string()),
                replicas: Some(3),
            },
        ]
    }

    #[test]
    fn write_manifest_produces_valid_toml() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("cd-manifest.toml");

        write_cd_manifest(&sample_resolved(), &sample_unresolved(), &sample_environments(), &dest)
            .expect("write_cd_manifest failed");

        let content = std::fs::read_to_string(&dest).unwrap();
        // Should parse back as valid TOML.
        let _: toml::Value = toml::from_str(&content).expect("output is valid TOML");
    }

    #[test]
    fn manifest_contains_resolved_section() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("cd-manifest.toml");

        write_cd_manifest(&sample_resolved(), &sample_unresolved(), &sample_environments(), &dest)
            .unwrap();

        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(content.contains("[resolved]"));
        assert!(content.contains("PROJECT_NAME"));
        assert!(content.contains("my-app"));
        assert!(content.contains("ghcr.io"));
    }

    #[test]
    fn manifest_contains_unresolved_section() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("cd-manifest.toml");

        write_cd_manifest(&sample_resolved(), &sample_unresolved(), &sample_environments(), &dest)
            .unwrap();

        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(content.contains("[unresolved.APP_URL]"));
        assert!(content.contains("[unresolved.GCP_REGION]"));
        assert!(content.contains("Public URL"));
        assert!(content.contains(r#"type = "url""#));
    }

    #[test]
    fn manifest_contains_environments_section() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("cd-manifest.toml");

        write_cd_manifest(&sample_resolved(), &sample_unresolved(), &sample_environments(), &dest)
            .unwrap();

        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(content.contains("[environments.staging]") || content.contains("[environments.production]"));
        assert!(content.contains("requires_approval = true"));
        assert!(content.contains("replicas = 3"));
    }

    #[test]
    fn manifest_empty_unresolved() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("cd-manifest.toml");

        write_cd_manifest(&sample_resolved(), &[], &sample_environments(), &dest).unwrap();

        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(content.contains("[resolved]"));
        // Unresolved section should be empty map.
        assert!(content.contains("[unresolved]"));
    }

    #[test]
    fn manifest_single_environment_no_optional_fields() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("cd-manifest.toml");

        let envs = vec![EnvironmentConfig {
            name: "production".to_string(),
            branch_filter: None,
            requires_approval: false,
            app_url: None,
            namespace: None,
            replicas: None,
        }];

        write_cd_manifest(&sample_resolved(), &[], &envs, &dest).unwrap();

        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(content.contains("[environments.production]"));
        assert!(content.contains("requires_approval = false"));
        // Optional fields should not appear.
        assert!(!content.contains("app_url"));
        assert!(!content.contains("namespace"));
        assert!(!content.contains("replicas"));
    }

    #[test]
    fn manifest_file_is_written_to_disk() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("subdir").join("cd-manifest.toml");

        // Parent dir doesn't exist yet — write should handle this?
        // Actually std::fs::write requires parent to exist. Let's create it.
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();

        write_cd_manifest(&sample_resolved(), &sample_unresolved(), &sample_environments(), &dest)
            .unwrap();

        assert!(dest.exists());
        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(!content.is_empty());
    }
}
