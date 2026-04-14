//! CD-24 — `.syncable.cd.toml` Project-Level Config
//!
//! Parses the optional `[cd]` block from `.syncable.toml` (or a standalone
//! `.syncable.cd.toml`).  Every field carries `#[serde(default)]` so partial
//! configs are always valid — only the keys present in the file are applied.
//!
//! Priority order (lowest → highest):
//!   detected value < config file < CLI flags
//!
//! `merge_config_into_cd_context()` applies the config-file layer; CLI flags
//! are handled in `handle_generate_cd()` after this call.

use std::path::Path;

use serde::Deserialize;

use super::context::{CdContext, CdPlatform, DeployTarget, Environment, Registry};

// ── Config struct ─────────────────────────────────────────────────────────────

/// Represents the `[cd]` section of `.syncable.toml` / `.syncable.cd.toml`.
///
/// All fields are `Option<T>` so that absent keys are distinguishable from
/// explicit `""` values, and `Default` gives every field `None` which the
/// merge function treats as "not set — keep the detected value".
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct CdConfig {
    /// Override the detected platform (`azure`, `gcp`, `hetzner`).
    pub platform: Option<String>,
    /// Override the deploy target (e.g. `app-service`, `cloud-run`, `vps`).
    pub target: Option<String>,
    /// Environments to generate (e.g. `["staging", "production"]`).
    pub environments: Option<Vec<String>>,
    /// Override the container registry (`acr`, `gar`, `ghcr`).
    pub registry: Option<String>,
    /// Override the Docker image name.
    pub image_name: Option<String>,
    /// Override the health check path.
    pub health_check_path: Option<String>,
    /// Override the migration command.
    pub migration_command: Option<String>,
    /// Override the default branch.
    pub default_branch: Option<String>,
}

/// Wraps `CdConfig` when parsing from a full `.syncable.toml` that uses a
/// `[cd]` table header.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct SyncableToml {
    cd: CdConfig,
}

// ── File discovery ────────────────────────────────────────────────────────────

/// Attempts to load CD config from the project root.
///
/// Look-up order:
///   1. `.syncable.cd.toml`  — dedicated file, takes precedence
///   2. `.syncable.toml`     — shared config, reads the `[cd]` table
///
/// Returns `None` when neither file exists.
pub fn load_cd_config(project_root: &Path) -> crate::Result<Option<CdConfig>> {
    // 1. Dedicated file
    let dedicated = project_root.join(".syncable.cd.toml");
    if dedicated.exists() {
        let raw = std::fs::read_to_string(&dedicated)?;
        let cfg: CdConfig = toml::from_str(&raw).map_err(|e| {
            crate::error::IaCGeneratorError::Config(crate::error::ConfigError::ParsingFailed(
                e.to_string(),
            ))
        })?;
        return Ok(Some(cfg));
    }

    // 2. Shared file with [cd] table
    let shared = project_root.join(".syncable.toml");
    if shared.exists() {
        let raw = std::fs::read_to_string(&shared)?;
        let wrapper: SyncableToml = toml::from_str(&raw).map_err(|e| {
            crate::error::IaCGeneratorError::Config(crate::error::ConfigError::ParsingFailed(
                e.to_string(),
            ))
        })?;
        let cfg = wrapper.cd;
        if cfg.platform.is_some()
            || cfg.target.is_some()
            || cfg.environments.is_some()
            || cfg.registry.is_some()
            || cfg.image_name.is_some()
            || cfg.health_check_path.is_some()
            || cfg.migration_command.is_some()
            || cfg.default_branch.is_some()
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
pub fn merge_config_into_cd_context(config: &CdConfig, ctx: &mut CdContext) {
    if let Some(ref p) = config.platform
        && let Some(platform) = parse_platform(p)
    {
        ctx.platform = platform;
    }

    if let Some(ref t) = config.target
        && let Some(target) = parse_deploy_target(t)
    {
        ctx.deploy_target = target;
    }

    if let Some(ref envs) = config.environments {
        ctx.environments = envs
            .iter()
            .map(|name| Environment {
                name: name.clone(),
                requires_approval: name == "production",
            })
            .collect();
    }

    if let Some(ref r) = config.registry
        && let Some(registry) = parse_registry(r)
    {
        ctx.registry = registry;
    }

    if let Some(ref img) = config.image_name {
        ctx.image_name = img.clone();
    }

    if let Some(ref path) = config.health_check_path {
        ctx.health_check_path = Some(path.clone());
    }

    if let Some(ref branch) = config.default_branch {
        ctx.default_branch = branch.clone();
    }
}

// ── Parsers ───────────────────────────────────────────────────────────────────

fn parse_platform(s: &str) -> Option<CdPlatform> {
    match s.to_lowercase().as_str() {
        "azure" => Some(CdPlatform::Azure),
        "gcp" => Some(CdPlatform::Gcp),
        "hetzner" => Some(CdPlatform::Hetzner),
        _ => None,
    }
}

fn parse_deploy_target(s: &str) -> Option<DeployTarget> {
    match s.to_lowercase().replace('_', "-").as_str() {
        "app-service" | "appservice" => Some(DeployTarget::AppService),
        "aks" => Some(DeployTarget::Aks),
        "container-apps" | "containerapps" => Some(DeployTarget::ContainerApps),
        "cloud-run" | "cloudrun" => Some(DeployTarget::CloudRun),
        "gke" => Some(DeployTarget::Gke),
        "vps" => Some(DeployTarget::Vps),
        "hetzner-k8s" | "hetznerk8s" | "k8s" => Some(DeployTarget::HetznerK8s),
        "coolify" => Some(DeployTarget::Coolify),
        _ => None,
    }
}

fn parse_registry(s: &str) -> Option<Registry> {
    match s.to_lowercase().as_str() {
        "acr" => Some(Registry::Acr),
        "gar" => Some(Registry::Gar),
        "ghcr" => Some(Registry::Ghcr),
        _ => None,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn parse_config(toml_str: &str) -> CdConfig {
        toml::from_str(toml_str).unwrap()
    }

    #[test]
    fn parse_full_config() {
        let cfg = parse_config(
            r#"
            platform = "azure"
            target = "app-service"
            environments = ["staging", "production"]
            registry = "acr"
            image_name = "my-app"
            health_check_path = "/api/health"
            migration_command = "npm run db:migrate"
            default_branch = "main"
            "#,
        );
        assert_eq!(cfg.platform.as_deref(), Some("azure"));
        assert_eq!(cfg.target.as_deref(), Some("app-service"));
        assert_eq!(cfg.environments.as_ref().unwrap().len(), 2);
        assert_eq!(cfg.registry.as_deref(), Some("acr"));
        assert_eq!(cfg.image_name.as_deref(), Some("my-app"));
        assert_eq!(cfg.health_check_path.as_deref(), Some("/api/health"));
        assert_eq!(cfg.migration_command.as_deref(), Some("npm run db:migrate"));
        assert_eq!(cfg.default_branch.as_deref(), Some("main"));
    }

    #[test]
    fn parse_partial_config() {
        let cfg = parse_config(
            r#"
            platform = "gcp"
            "#,
        );
        assert_eq!(cfg.platform.as_deref(), Some("gcp"));
        assert!(cfg.target.is_none());
        assert!(cfg.environments.is_none());
    }

    #[test]
    fn parse_empty_config() {
        let cfg = parse_config("");
        assert!(cfg.platform.is_none());
        assert!(cfg.target.is_none());
    }

    #[test]
    fn load_config_from_dedicated_file() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".syncable.cd.toml"),
            r#"platform = "hetzner""#,
        )
        .unwrap();
        let cfg = load_cd_config(dir.path()).unwrap();
        assert!(cfg.is_some());
        assert_eq!(cfg.unwrap().platform.as_deref(), Some("hetzner"));
    }

    #[test]
    fn load_config_from_shared_file() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".syncable.toml"),
            r#"
            [cd]
            platform = "azure"
            target = "aks"
            "#,
        )
        .unwrap();
        let cfg = load_cd_config(dir.path()).unwrap();
        assert!(cfg.is_some());
        let c = cfg.unwrap();
        assert_eq!(c.platform.as_deref(), Some("azure"));
        assert_eq!(c.target.as_deref(), Some("aks"));
    }

    #[test]
    fn load_config_dedicated_takes_precedence() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".syncable.cd.toml"),
            r#"platform = "gcp""#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join(".syncable.toml"),
            r#"
            [cd]
            platform = "azure"
            "#,
        )
        .unwrap();
        let cfg = load_cd_config(dir.path()).unwrap();
        assert_eq!(cfg.unwrap().platform.as_deref(), Some("gcp"));
    }

    #[test]
    fn load_config_no_files() {
        let dir = TempDir::new().unwrap();
        let cfg = load_cd_config(dir.path()).unwrap();
        assert!(cfg.is_none());
    }

    #[test]
    fn load_config_shared_no_cd_section() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".syncable.toml"),
            r#"
            [ci]
            platform = "azure"
            "#,
        )
        .unwrap();
        let cfg = load_cd_config(dir.path()).unwrap();
        assert!(cfg.is_none());
    }

    #[test]
    fn parse_platform_variants() {
        assert_eq!(parse_platform("azure"), Some(CdPlatform::Azure));
        assert_eq!(parse_platform("Azure"), Some(CdPlatform::Azure));
        assert_eq!(parse_platform("gcp"), Some(CdPlatform::Gcp));
        assert_eq!(parse_platform("hetzner"), Some(CdPlatform::Hetzner));
        assert_eq!(parse_platform("unknown"), None);
    }

    #[test]
    fn parse_deploy_target_variants() {
        assert_eq!(
            parse_deploy_target("app-service"),
            Some(DeployTarget::AppService)
        );
        assert_eq!(
            parse_deploy_target("appservice"),
            Some(DeployTarget::AppService)
        );
        assert_eq!(parse_deploy_target("aks"), Some(DeployTarget::Aks));
        assert_eq!(
            parse_deploy_target("container-apps"),
            Some(DeployTarget::ContainerApps)
        );
        assert_eq!(
            parse_deploy_target("cloud-run"),
            Some(DeployTarget::CloudRun)
        );
        assert_eq!(parse_deploy_target("gke"), Some(DeployTarget::Gke));
        assert_eq!(parse_deploy_target("vps"), Some(DeployTarget::Vps));
        assert_eq!(
            parse_deploy_target("hetzner-k8s"),
            Some(DeployTarget::HetznerK8s)
        );
        assert_eq!(
            parse_deploy_target("coolify"),
            Some(DeployTarget::Coolify)
        );
        assert_eq!(parse_deploy_target("unknown"), None);
    }

    #[test]
    fn parse_registry_variants() {
        assert_eq!(parse_registry("acr"), Some(Registry::Acr));
        assert_eq!(parse_registry("gar"), Some(Registry::Gar));
        assert_eq!(parse_registry("ghcr"), Some(Registry::Ghcr));
        assert_eq!(parse_registry("unknown"), None);
    }

    #[test]
    fn merge_platform() {
        let cfg = parse_config(r#"platform = "gcp""#);
        let dir = TempDir::new().unwrap();
        // Create a minimal CdContext via fixture
        let mut ctx = make_test_context(dir.path());
        assert_eq!(ctx.platform, CdPlatform::Azure);
        merge_config_into_cd_context(&cfg, &mut ctx);
        assert_eq!(ctx.platform, CdPlatform::Gcp);
    }

    #[test]
    fn merge_target() {
        let cfg = parse_config(r#"target = "gke""#);
        let dir = TempDir::new().unwrap();
        let mut ctx = make_test_context(dir.path());
        merge_config_into_cd_context(&cfg, &mut ctx);
        assert_eq!(ctx.deploy_target, DeployTarget::Gke);
    }

    #[test]
    fn merge_environments() {
        let cfg = parse_config(r#"environments = ["dev", "prod"]"#);
        let dir = TempDir::new().unwrap();
        let mut ctx = make_test_context(dir.path());
        merge_config_into_cd_context(&cfg, &mut ctx);
        assert_eq!(ctx.environments.len(), 2);
        assert_eq!(ctx.environments[0].name, "dev");
        assert!(!ctx.environments[0].requires_approval);
        // "production" triggers approval
        assert!(!ctx.environments[1].requires_approval); // "prod" != "production"
    }

    #[test]
    fn merge_environments_production_approval() {
        let cfg = parse_config(r#"environments = ["staging", "production"]"#);
        let dir = TempDir::new().unwrap();
        let mut ctx = make_test_context(dir.path());
        merge_config_into_cd_context(&cfg, &mut ctx);
        assert!(!ctx.environments[0].requires_approval);
        assert!(ctx.environments[1].requires_approval);
    }

    #[test]
    fn merge_image_name() {
        let cfg = parse_config(r#"image_name = "custom-app""#);
        let dir = TempDir::new().unwrap();
        let mut ctx = make_test_context(dir.path());
        merge_config_into_cd_context(&cfg, &mut ctx);
        assert_eq!(ctx.image_name, "custom-app");
    }

    #[test]
    fn merge_health_check_path() {
        let cfg = parse_config(r#"health_check_path = "/healthz""#);
        let dir = TempDir::new().unwrap();
        let mut ctx = make_test_context(dir.path());
        merge_config_into_cd_context(&cfg, &mut ctx);
        assert_eq!(ctx.health_check_path.as_deref(), Some("/healthz"));
    }

    #[test]
    fn merge_default_branch() {
        let cfg = parse_config(r#"default_branch = "master""#);
        let dir = TempDir::new().unwrap();
        let mut ctx = make_test_context(dir.path());
        merge_config_into_cd_context(&cfg, &mut ctx);
        assert_eq!(ctx.default_branch, "master");
    }

    #[test]
    fn merge_empty_config_no_changes() {
        let cfg = parse_config("");
        let dir = TempDir::new().unwrap();
        let mut ctx = make_test_context(dir.path());
        let original_platform = ctx.platform.clone();
        merge_config_into_cd_context(&cfg, &mut ctx);
        assert_eq!(ctx.platform, original_platform);
    }

    /// Creates a minimal CdContext for merge testing.
    fn make_test_context(path: &Path) -> CdContext {
        let analysis = crate::analyzer::analyze_project(path).unwrap();

        CdContext {
            analysis,
            project_name: "test-project".to_string(),
            platform: CdPlatform::Azure,
            deploy_target: DeployTarget::AppService,
            environments: vec![
                Environment {
                    name: "staging".to_string(),
                    requires_approval: false,
                },
                Environment {
                    name: "production".to_string(),
                    requires_approval: true,
                },
            ],
            registry: Registry::Acr,
            image_name: "test-project".to_string(),
            has_terraform: false,
            terraform_dir: None,
            has_k8s_manifests: false,
            k8s_manifest_dir: None,
            has_helm_chart: false,
            helm_chart_dir: None,
            migration_tool: None,
            health_check_path: None,
            default_branch: "main".to_string(),
            has_dockerfile: false,
        }
    }
}
