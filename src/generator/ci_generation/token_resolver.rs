//! Placeholder Token Resolution Engine — CI-15
//!
//! Two-pass strategy:
//!   1. **Deterministic pass** — replaces `{{TOKEN_NAME}}` in String fields
//!      when the value can be derived unambiguously from `CiContext`.
//!   2. **Placeholder pass** — any remaining `{{TOKEN_NAME}}` pattern becomes
//!      an `UnresolvedToken` in `pipeline.unresolved_tokens`.
//!
//! `write_manifest` serialises both maps to `ci-manifest.toml` for the agent
//! fill phase and interactive prompts.

use std::collections::HashMap;
use std::path::Path;

use regex::Regex;
use serde::Serialize;

use crate::error::{GeneratorError, IaCGeneratorError};
use crate::generator::ci_generation::{
    context::CiContext,
    schema::{CiPipeline, UnresolvedToken},
};

/// A map from `TOKEN_NAME` to its resolved value.
pub type ResolvedTokenMap = HashMap<String, String>;

/// Runs the two-pass resolution engine on `pipeline` in place.
///
/// Returns the map of resolved tokens; callers pass this to `write_manifest`.
pub fn resolve_tokens(ctx: &CiContext, pipeline: &mut CiPipeline) -> ResolvedTokenMap {
    let resolved = build_resolved_map(ctx);
    // Compile once; reused across every field visit.
    let re = Regex::new(r"\{\{([A-Z][A-Z0-9_]*)\}\}").expect("static regex is valid");
    apply_to_pipeline(pipeline, &resolved, &re);
    resolved
}

/// Writes the resolved and unresolved token inventories to `ci-manifest.toml`.
pub fn write_manifest(
    resolved: &ResolvedTokenMap,
    unresolved: &[UnresolvedToken],
    dest: &Path,
) -> crate::Result<()> {
    #[derive(Serialize)]
    struct Entry {
        #[serde(rename = "type")]
        token_type: String,
        hint: String,
    }

    #[derive(Serialize)]
    struct Manifest {
        resolved: HashMap<String, String>,
        unresolved: HashMap<String, Entry>,
    }

    let manifest = Manifest {
        resolved: resolved.clone(),
        unresolved: unresolved
            .iter()
            .map(|u| {
                (
                    u.name.clone(),
                    Entry { token_type: u.token_type.clone(), hint: u.hint.clone() },
                )
            })
            .collect(),
    };

    let content = toml::to_string_pretty(&manifest)
        .map_err(|e| IaCGeneratorError::Generation(GeneratorError::InvalidContext(e.to_string())))?;

    std::fs::write(dest, content)?;
    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Builds the deterministic token map from `ctx`.
fn build_resolved_map(ctx: &CiContext) -> ResolvedTokenMap {
    let mut map = HashMap::new();
    map.insert("PROJECT_NAME".to_string(), ctx.project_name.clone());
    if let Some(version) = ctx.runtime_versions.get(&ctx.primary_language) {
        map.insert("RUNTIME_VERSION".to_string(), version.clone());
    }
    map
}

/// Visits every String field in `pipeline` that may carry a `{{TOKEN}}` and
/// applies both resolution passes.
fn apply_to_pipeline(pipeline: &mut CiPipeline, resolved: &ResolvedTokenMap, re: &Regex) {
    let acc = &mut pipeline.unresolved_tokens;

    resolve_str(&mut pipeline.project_name, resolved, re, acc);

    resolve_str(&mut pipeline.runtime.version, resolved, re, acc);

    if let Some(cache) = &mut pipeline.cache {
        for path in &mut cache.paths {
            resolve_str(path, resolved, re, acc);
        }
        resolve_str(&mut cache.key, resolved, re, acc);
        for key in &mut cache.restore_keys {
            resolve_str(key, resolved, re, acc);
        }
    }

    resolve_str(&mut pipeline.install.command, resolved, re, acc);

    if let Some(lint) = &mut pipeline.lint {
        resolve_str(&mut lint.command, resolved, re, acc);
    }

    resolve_str(&mut pipeline.test.command, resolved, re, acc);

    if let Some(build) = &mut pipeline.build {
        resolve_str(&mut build.command, resolved, re, acc);
    }

    if let Some(docker) = &mut pipeline.docker_build {
        resolve_str(&mut docker.image_tag, resolved, re, acc);
    }

    if let Some(scan) = &mut pipeline.image_scan {
        resolve_str(&mut scan.image_ref, resolved, re, acc);
    }

    if let Some(artifact) = &mut pipeline.upload_artifact {
        resolve_str(&mut artifact.name, resolved, re, acc);
        resolve_str(&mut artifact.path, resolved, re, acc);
    }
}

/// Resolves known tokens and collects unknown ones from a single String field.
fn resolve_str(
    field: &mut String,
    resolved: &ResolvedTokenMap,
    re: &Regex,
    acc: &mut Vec<UnresolvedToken>,
) {
    for (name, value) in resolved {
        let placeholder = format!("{{{{{}}}}}", name);
        if field.contains(&placeholder) {
            *field = field.replace(&placeholder, value);
        }
    }

    let snapshot = field.clone();
    for cap in re.captures_iter(&snapshot) {
        let name = cap[1].to_string();
        if !acc.iter().any(|u| u.name == name) {
            acc.push(UnresolvedToken::new(&name, "Provide a value for this token", "string"));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{CiFormat, CiPlatform};
    use crate::generator::ci_generation::{
        context::CiContext,
        schema::{
            CiPipeline, InstallStep, SecretScanStep, TestStep, TriggerConfig,
        },
        test_helpers::make_base_ctx,
    };
    use tempfile::TempDir;

    fn make_pipeline(project_name: &str) -> CiPipeline {
        CiPipeline {
            project_name: project_name.to_string(),
            platform: CiPlatform::Gcp,
            format: CiFormat::GithubActions,
            triggers: TriggerConfig {
                push_branches: vec!["main".to_string()],
                pr_branches: vec!["main".to_string()],
                tag_pattern: None,
                scheduled: None,
            },
            runtime: crate::generator::ci_generation::schema::RuntimeStep {
                action: "actions/setup-node@v4".to_string(),
                version: "20".to_string(),
            },
            cache: None,
            install: InstallStep { command: "npm ci".to_string() },
            lint: None,
            test: TestStep {
                command: "npm test".to_string(),
                coverage_flag: None,
                coverage_report_path: None,
            },
            build: None,
            docker_build: None,
            image_scan: None,
            secret_scan: SecretScanStep,
            upload_artifact: None,
            unresolved_tokens: vec![],
        }
    }

    fn ctx_with_name(root: &std::path::Path, name: &str) -> CiContext {
        CiContext { project_name: name.to_string(), ..make_base_ctx(root, "") }
    }

    // ── Deterministic pass ────────────────────────────────────────────────────

    #[test]
    fn test_project_name_token_is_replaced() {
        let dir = TempDir::new().unwrap();
        let ctx = ctx_with_name(dir.path(), "my-app");
        let mut pipeline = make_pipeline("{{PROJECT_NAME}}");

        let resolved = resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(pipeline.project_name, "my-app");
        assert_eq!(resolved.get("PROJECT_NAME").map(|s| s.as_str()), Some("my-app"));
    }

    #[test]
    fn test_runtime_version_token_is_replaced() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_base_ctx(dir.path(), "Node.js");
        ctx.runtime_versions.insert("Node.js".to_string(), "20".to_string());

        let mut pipeline = make_pipeline("proj");
        pipeline.runtime.version = "{{RUNTIME_VERSION}}".to_string();

        resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(pipeline.runtime.version, "20");
        assert!(pipeline.unresolved_tokens.is_empty());
    }

    #[test]
    fn test_no_version_in_context_leaves_token_unresolved() {
        let dir = TempDir::new().unwrap();
        let ctx = make_base_ctx(dir.path(), "Node.js"); // no runtime_versions

        let mut pipeline = make_pipeline("proj");
        pipeline.runtime.version = "{{RUNTIME_VERSION}}".to_string();

        resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(pipeline.runtime.version, "{{RUNTIME_VERSION}}");
        assert_eq!(pipeline.unresolved_tokens.len(), 1);
        assert_eq!(pipeline.unresolved_tokens[0].name, "RUNTIME_VERSION");
    }

    // ── Placeholder pass ──────────────────────────────────────────────────────

    #[test]
    fn test_unknown_token_becomes_unresolved_entry() {
        let dir = TempDir::new().unwrap();
        let ctx = make_base_ctx(dir.path(), "");

        let mut pipeline = make_pipeline("proj");
        pipeline.docker_build = Some(crate::generator::ci_generation::schema::DockerBuildStep {
            image_tag: "{{REGISTRY_URL}}/my-app:latest".to_string(),
            push: true,
            qemu: false,
        });

        resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(pipeline.unresolved_tokens.len(), 1);
        assert_eq!(pipeline.unresolved_tokens[0].name, "REGISTRY_URL");
        assert_eq!(
            pipeline.unresolved_tokens[0].placeholder,
            "{{REGISTRY_URL}}"
        );
    }

    #[test]
    fn test_duplicate_tokens_deduplicated() {
        let dir = TempDir::new().unwrap();
        let ctx = make_base_ctx(dir.path(), "");

        let mut pipeline = make_pipeline("proj");
        pipeline.docker_build = Some(crate::generator::ci_generation::schema::DockerBuildStep {
            image_tag: "{{REGISTRY_URL}}/app:tag".to_string(),
            push: true,
            qemu: false,
        });
        pipeline.image_scan = Some(crate::generator::ci_generation::schema::ImageScanStep {
            image_ref: "{{REGISTRY_URL}}/app:tag".to_string(),
            fail_on_severity: "HIGH".to_string(),
        });

        resolve_tokens(&ctx, &mut pipeline);

        let registry_tokens: Vec<_> = pipeline
            .unresolved_tokens
            .iter()
            .filter(|u| u.name == "REGISTRY_URL")
            .collect();
        assert_eq!(registry_tokens.len(), 1, "REGISTRY_URL should not be duplicated");
    }

    // ── Manifest writing ──────────────────────────────────────────────────────

    #[test]
    fn test_write_manifest_produces_valid_toml() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("ci-manifest.toml");

        let mut resolved = ResolvedTokenMap::new();
        resolved.insert("PROJECT_NAME".to_string(), "my-app".to_string());

        let unresolved = vec![UnresolvedToken::new("REGISTRY_URL", "Container registry", "url")];

        write_manifest(&resolved, &unresolved, &dest).expect("write_manifest failed");

        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(content.contains("PROJECT_NAME"));
        assert!(content.contains("my-app"));
        assert!(content.contains("REGISTRY_URL"));
    }

    #[test]
    fn test_write_manifest_empty_unresolved() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("ci-manifest.toml");

        let mut resolved = ResolvedTokenMap::new();
        resolved.insert("PROJECT_NAME".to_string(), "clean-app".to_string());

        write_manifest(&resolved, &[], &dest).expect("write_manifest failed");

        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(content.contains("clean-app"));
    }
}
