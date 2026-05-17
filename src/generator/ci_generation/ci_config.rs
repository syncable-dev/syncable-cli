//! CI-22 — `.syncable.ci.toml` Project-Level Config
//!
//! Parses the optional `[ci]` block from `.syncable.toml` (or a standalone
//! `.syncable.ci.toml`).  Every field carries `#[serde(default)]` so partial
//! configs are always valid — only the keys present in the file are applied.
//!
//! Priority order (lowest → highest):
//!   detected value < config file < CLI flags
//!
//! `merge_config_into_context()` applies the config-file layer; CLI flags are
//! handled in `handle_generate_ci()` after this call.

use std::path::Path;

use serde::Deserialize;

use crate::cli::{CiFormat, CiPlatform};
use crate::generator::ci_generation::context::CiContext;

// ── Config struct ─────────────────────────────────────────────────────────────

/// Represents the `[ci]` section of `.syncable.toml` / `.syncable.ci.toml`.
///
/// All fields are `Option<T>` so that absent keys are distinguishable from
/// explicit `""` values, and `Default` gives every field `None` which the
/// merge function treats as "not set — keep the detected value".
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct CiConfig {
    /// Override the detected platform.
    pub platform: Option<String>,
    /// Override the effective CI format.
    pub format: Option<String>,
    /// Override the detected default branch.
    pub default_branch: Option<String>,
    /// Additional branches appended to push/PR triggers.
    pub extra_branches: Option<Vec<String>>,
    /// Override the detected test invocation command.
    pub test_command: Option<String>,
    /// Override the detected build command.
    pub build_command: Option<String>,
    /// Step names to omit from the generated pipeline (e.g. `["lint"]`).
    pub skip_steps: Option<Vec<String>>,
    /// Custom prefix for secrets/env variable names (e.g. `"MYAPP"`).
    pub env_prefix: Option<String>,
}

/// Wraps `CiConfig` when parsing from a full `.syncable.toml` that uses a
/// `[ci]` table header.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct SyncableToml {
    ci: CiConfig,
}

// ── File discovery ────────────────────────────────────────────────────────────

/// Attempts to load CI config from the project root.
///
/// Look-up order:
///   1. `.syncable.ci.toml`  — dedicated file, takes precedence
///   2. `.syncable.toml`     — shared config, reads the `[ci]` table
///
/// Returns `None` when neither file exists (not an error — just unconfigured).
pub fn load_ci_config(project_root: &Path) -> crate::Result<Option<CiConfig>> {
    // 1. Dedicated file
    let dedicated = project_root.join(".syncable.ci.toml");
    if dedicated.exists() {
        let raw = std::fs::read_to_string(&dedicated)?;
        let cfg: CiConfig = toml::from_str(&raw)
            .map_err(|e| crate::error::IaCGeneratorError::Config(
                crate::error::ConfigError::ParsingFailed(e.to_string())
            ))?;
        return Ok(Some(cfg));
    }

    // 2. Shared file with [ci] table
    let shared = project_root.join(".syncable.toml");
    if shared.exists() {
        let raw = std::fs::read_to_string(&shared)?;
        let wrapper: SyncableToml = toml::from_str(&raw)
            .map_err(|e| crate::error::IaCGeneratorError::Config(
                crate::error::ConfigError::ParsingFailed(e.to_string())
            ))?;
        // Only return Some when at least one field was explicitly set
        let cfg = wrapper.ci;
        if cfg.platform.is_some()
            || cfg.format.is_some()
            || cfg.default_branch.is_some()
            || cfg.extra_branches.is_some()
            || cfg.test_command.is_some()
            || cfg.build_command.is_some()
            || cfg.skip_steps.is_some()
            || cfg.env_prefix.is_some()
        {
            return Ok(Some(cfg));
        }
    }

    Ok(None)
}

// ── Merge ─────────────────────────────────────────────────────────────────────

/// Applies `config` onto `ctx`, overwriting only the fields the config file
/// explicitly set.  CLI flags are applied *after* this call and will win over
/// both detected values and config-file values.
pub fn merge_config_into_context(config: &CiConfig, ctx: &mut CiContext) {
    if let Some(branch) = &config.default_branch {
        ctx.default_branch = branch.clone();
    }

    if let Some(cmd) = &config.test_command {
        // The test command lives inside the nested TestStep once the pipeline
        // is built, but CiContext doesn't own that struct yet — store it in a
        // dedicated field so the pipeline builder can pick it up.
        ctx.config_test_command = Some(cmd.clone());
    }

    if let Some(cmd) = &config.build_command {
        ctx.build_command = Some(cmd.clone());
    }

    if let Some(prefix) = &config.env_prefix {
        ctx.env_prefix = Some(prefix.clone());
    }

    if let Some(skip) = &config.skip_steps {
        ctx.skip_steps = skip.clone();
    }

    if let Some(extra) = &config.extra_branches {
        ctx.extra_branches = extra.clone();
    }

    // platform / format overrides: convert string → enum, ignore unknown values
    if let Some(p) = &config.platform {
        if let Ok(platform) = parse_platform(p) {
            ctx.platform = platform;
        }
    }

    if let Some(f) = &config.format {
        if let Ok(format) = parse_format(f) {
            ctx.format = format;
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_platform(s: &str) -> Result<CiPlatform, ()> {
    match s.to_lowercase().as_str() {
        "azure" => Ok(CiPlatform::Azure),
        "gcp" => Ok(CiPlatform::Gcp),
        "hetzner" => Ok(CiPlatform::Hetzner),
        _ => Err(()),
    }
}

fn parse_format(s: &str) -> Result<CiFormat, ()> {
    match s.to_lowercase().replace('-', "_").as_str() {
        "github_actions" | "githubactions" => Ok(CiFormat::GithubActions),
        "azure_pipelines" | "azurepipelines" => Ok(CiFormat::AzurePipelines),
        "cloud_build" | "cloudbuild" => Ok(CiFormat::CloudBuild),
        _ => Err(()),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_config(toml_str: &str) -> CiConfig {
        toml::from_str(toml_str).expect("should parse")
    }

    #[test]
    fn test_empty_toml_parses_to_all_none() {
        let cfg = parse_config("");
        assert!(cfg.platform.is_none());
        assert!(cfg.default_branch.is_none());
        assert!(cfg.test_command.is_none());
        assert!(cfg.build_command.is_none());
        assert!(cfg.skip_steps.is_none());
        assert!(cfg.env_prefix.is_none());
        assert!(cfg.extra_branches.is_none());
        assert!(cfg.format.is_none());
    }

    #[test]
    fn test_partial_toml_parses() {
        let cfg = parse_config(r#"
            platform = "gcp"
            default_branch = "main"
        "#);
        assert_eq!(cfg.platform.as_deref(), Some("gcp"));
        assert_eq!(cfg.default_branch.as_deref(), Some("main"));
        assert!(cfg.test_command.is_none());
    }

    #[test]
    fn test_full_toml_parses() {
        let cfg = parse_config(r#"
            platform = "azure"
            format = "azure-pipelines"
            default_branch = "main"
            extra_branches = ["develop", "release/*"]
            test_command = "npm run test:ci"
            build_command = "npm run build"
            skip_steps = ["lint"]
            env_prefix = "MYAPP"
        "#);
        assert_eq!(cfg.platform.as_deref(), Some("azure"));
        assert_eq!(cfg.format.as_deref(), Some("azure-pipelines"));
        assert_eq!(cfg.default_branch.as_deref(), Some("main"));
        let expected_branches: Vec<String> = vec!["develop".to_string(), "release/*".to_string()];
        assert_eq!(cfg.extra_branches.as_deref(), Some(expected_branches.as_slice()));
        assert_eq!(cfg.test_command.as_deref(), Some("npm run test:ci"));
        assert_eq!(cfg.build_command.as_deref(), Some("npm run build"));
        let expected_skip: Vec<String> = vec!["lint".to_string()];
        assert_eq!(cfg.skip_steps.as_deref(), Some(expected_skip.as_slice()));
        assert_eq!(cfg.env_prefix.as_deref(), Some("MYAPP"));
    }

    #[test]
    fn test_syncable_toml_wrapper_parses() {
        let raw = r#"
            [ci]
            platform = "gcp"
            test_command = "pytest"
        "#;
        let wrapper: SyncableToml = toml::from_str(raw).expect("should parse");
        assert_eq!(wrapper.ci.platform.as_deref(), Some("gcp"));
        assert_eq!(wrapper.ci.test_command.as_deref(), Some("pytest"));
    }

    #[test]
    fn test_syncable_toml_no_ci_section_gives_empty() {
        let raw = r#"
            [other_section]
            key = "value"
        "#;
        let wrapper: SyncableToml = toml::from_str(raw).expect("should parse");
        assert!(wrapper.ci.platform.is_none());
    }

    // ── merge tests ────────────────────────────────────────────────────────

    fn make_context() -> CiContext {
        use crate::generator::ci_generation::test_helpers::make_minimal_context;
        make_minimal_context()
    }

    #[test]
    fn test_merge_default_branch() {
        let cfg = parse_config(r#"default_branch = "develop""#);
        let mut ctx = make_context();
        merge_config_into_context(&cfg, &mut ctx);
        assert_eq!(ctx.default_branch, "develop");
    }

    #[test]
    fn test_merge_does_not_overwrite_when_field_absent() {
        let cfg = parse_config("");
        let mut ctx = make_context();
        let original_branch = ctx.default_branch.clone();
        merge_config_into_context(&cfg, &mut ctx);
        assert_eq!(ctx.default_branch, original_branch);
    }

    #[test]
    fn test_merge_build_command() {
        let cfg = parse_config(r#"build_command = "cargo build --release""#);
        let mut ctx = make_context();
        merge_config_into_context(&cfg, &mut ctx);
        assert_eq!(ctx.build_command.as_deref(), Some("cargo build --release"));
    }

    #[test]
    fn test_merge_test_command_stored_in_config_field() {
        let cfg = parse_config(r#"test_command = "npx jest --ci""#);
        let mut ctx = make_context();
        merge_config_into_context(&cfg, &mut ctx);
        assert_eq!(ctx.config_test_command.as_deref(), Some("npx jest --ci"));
    }

    #[test]
    fn test_merge_skip_steps() {
        let cfg = parse_config(r#"skip_steps = ["lint", "build"]"#);
        let mut ctx = make_context();
        merge_config_into_context(&cfg, &mut ctx);
        assert_eq!(ctx.skip_steps, vec!["lint", "build"]);
    }

    #[test]
    fn test_merge_platform_string_to_enum() {
        let cfg = parse_config(r#"platform = "gcp""#);
        let mut ctx = make_context();
        merge_config_into_context(&cfg, &mut ctx);
        assert!(matches!(ctx.platform, CiPlatform::Gcp));
    }

    #[test]
    fn test_merge_unknown_platform_ignored() {
        let cfg = parse_config(r#"platform = "unknown-cloud""#);
        let mut ctx = make_context();
        let original_platform = ctx.platform.clone();
        merge_config_into_context(&cfg, &mut ctx);
        // platform unchanged because we can't parse it
        assert_eq!(
            std::mem::discriminant(&ctx.platform),
            std::mem::discriminant(&original_platform)
        );
    }

    #[test]
    fn test_merge_format_normalises_hyphens() {
        let cfg = parse_config(r#"format = "github-actions""#);
        let mut ctx = make_context();
        merge_config_into_context(&cfg, &mut ctx);
        assert!(matches!(ctx.format, CiFormat::GithubActions));
    }

    #[test]
    fn test_merge_extra_branches() {
        let cfg = parse_config(r#"extra_branches = ["develop"]"#);
        let mut ctx = make_context();
        merge_config_into_context(&cfg, &mut ctx);
        assert_eq!(ctx.extra_branches, vec!["develop"]);
    }

    #[test]
    fn test_deserialize_env_prefix_and_platform() {
        let raw = r#"
            platform = "hetzner"
            env_prefix = "APP"
        "#;
        let cfg: CiConfig = toml::from_str(raw).unwrap();
        assert_eq!(cfg.platform.as_deref(), Some("hetzner"));
        assert_eq!(cfg.env_prefix.as_deref(), Some("APP"));
    }
}
