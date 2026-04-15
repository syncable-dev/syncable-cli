//! CD-02 — `CdContext` and `collect_cd_context` entry point.
//!
//! Captures everything needed to build a CD pipeline skeleton. The context
//! collector calls the existing `ProjectAnalysis` and enriches it with
//! deployment-specific detection: Terraform directories, K8s manifests,
//! Helm charts, database migration tools, and health check paths.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::analyzer::{analyze_project, ProjectAnalysis};

// ── Platform & target enums ──────────────────────────────────────────────────

/// Cloud platform for CD deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum CdPlatform {
    Azure,
    Gcp,
    Hetzner,
}

/// Concrete deployment target within a platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum DeployTarget {
    /// Azure App Service (PaaS)
    AppService,
    /// Azure Kubernetes Service
    Aks,
    /// Azure Container Apps
    ContainerApps,
    /// Google Cloud Run (serverless containers)
    CloudRun,
    /// Google Kubernetes Engine
    Gke,
    /// Hetzner VPS via SSH + Docker Compose
    Vps,
    /// Hetzner-managed Kubernetes (hcloud)
    HetznerK8s,
    /// Coolify self-hosted PaaS on Hetzner
    Coolify,
}

impl std::fmt::Display for DeployTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::AppService => "app-service",
            Self::Aks => "aks",
            Self::ContainerApps => "container-apps",
            Self::CloudRun => "cloud-run",
            Self::Gke => "gke",
            Self::Vps => "vps",
            Self::HetznerK8s => "hetzner-k8s",
            Self::Coolify => "coolify",
        };
        write!(f, "{}", s)
    }
}

/// Container registry type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Registry {
    /// Azure Container Registry
    Acr,
    /// Google Artifact Registry
    Gar,
    /// GitHub Container Registry
    Ghcr,
    /// User-provided custom registry URL
    Custom(String),
}

impl std::fmt::Display for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Acr => write!(f, "acr"),
            Self::Gar => write!(f, "gar"),
            Self::Ghcr => write!(f, "ghcr"),
            Self::Custom(url) => write!(f, "custom({})", url),
        }
    }
}

/// Database migration tool detected in the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MigrationTool {
    Flyway,
    Liquibase,
    Alembic,
    DjangoMigrations,
    Prisma,
    Sqlx,
    Diesel,
}

impl std::fmt::Display for MigrationTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Flyway => "flyway",
            Self::Liquibase => "liquibase",
            Self::Alembic => "alembic",
            Self::DjangoMigrations => "django",
            Self::Prisma => "prisma",
            Self::Sqlx => "sqlx",
            Self::Diesel => "diesel",
        };
        write!(f, "{}", s)
    }
}

/// A deployment environment (dev, staging, production).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Environment {
    pub name: String,
    /// Whether this environment requires manual approval before deploy.
    pub requires_approval: bool,
}

// ── Primary struct ────────────────────────────────────────────────────────────

/// Enriched snapshot of a project consumed by all CD generators.
#[derive(Debug, Clone, Serialize)]
pub struct CdContext {
    /// Raw analyzer output; available to generators that need additional fields.
    pub analysis: ProjectAnalysis,
    /// Human-readable project name (from Cargo.toml, package.json, or dir name).
    pub project_name: String,
    /// Target cloud platform.
    pub platform: CdPlatform,
    /// Concrete deployment target within the platform.
    pub deploy_target: DeployTarget,
    /// Ordered list of deployment environments.
    pub environments: Vec<Environment>,
    /// Container registry to push images to.
    pub registry: Registry,
    /// Docker image name (without registry prefix or tag).
    pub image_name: String,
    /// Whether a Terraform directory was detected.
    pub has_terraform: bool,
    /// Path to the Terraform directory, if detected.
    pub terraform_dir: Option<PathBuf>,
    /// Whether Kubernetes manifest files were detected.
    pub has_k8s_manifests: bool,
    /// Path to the Kubernetes manifest directory, if detected.
    pub k8s_manifest_dir: Option<PathBuf>,
    /// Whether a Helm chart was detected.
    pub has_helm_chart: bool,
    /// Path to the Helm chart directory, if detected.
    pub helm_chart_dir: Option<PathBuf>,
    /// Database migration tool detected, if any.
    pub migration_tool: Option<MigrationTool>,
    /// Custom migration command from `.syncable.cd.toml`, overrides the
    /// tool-derived default when set.
    pub migration_command_override: Option<String>,
    /// Health check endpoint path (e.g. `/health`, `/healthz`).
    pub health_check_path: Option<String>,
    /// Default git branch name.
    pub default_branch: String,
    /// Whether the project has a Dockerfile.
    pub has_dockerfile: bool,
}

// ── Detection helpers ─────────────────────────────────────────────────────────

/// Detect the project name from the analysis metadata.
fn detect_project_name(analysis: &ProjectAnalysis) -> String {
    analysis
        .project_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".to_string())
}

/// Returns the upstream default branch via `git symbolic-ref`; falls back to `"main"`.
fn detect_default_branch(path: &Path) -> String {
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .current_dir(path)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let raw = String::from_utf8_lossy(&out.stdout);
            raw.trim()
                .rsplit('/')
                .next()
                .unwrap_or("main")
                .to_string()
        }
        _ => "main".to_string(),
    }
}

/// Detect a Terraform directory at the project root.
fn detect_terraform(root: &Path) -> Option<PathBuf> {
    // Check common Terraform directory names
    let candidates = ["terraform", "tf", "infra", "infrastructure"];
    for name in &candidates {
        let dir = root.join(name);
        if dir.is_dir() && has_tf_files(&dir) {
            return Some(dir);
        }
    }
    // Check root for main.tf
    if root.join("main.tf").exists() {
        return Some(root.to_path_buf());
    }
    None
}

/// Returns true if the directory contains `.tf` files.
fn has_tf_files(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "tf")
                .unwrap_or(false)
        })
}

/// Detect Kubernetes manifest directories.
fn detect_k8s_manifests(root: &Path) -> Option<PathBuf> {
    let candidates = ["k8s", "kubernetes", "manifests", "deploy", "kube"];
    for name in &candidates {
        let dir = root.join(name);
        if dir.is_dir() && has_k8s_yamls(&dir) {
            return Some(dir);
        }
    }
    None
}

/// Returns true if the directory contains YAML files with `apiVersion:` and `kind:`.
fn has_k8s_yamls(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| {
            let p = e.path();
            let is_yaml = p
                .extension()
                .map(|ext| ext == "yml" || ext == "yaml")
                .unwrap_or(false);
            if !is_yaml {
                return false;
            }
            std::fs::read_to_string(&p)
                .map(|content| content.contains("apiVersion:") && content.contains("kind:"))
                .unwrap_or(false)
        })
}

/// Detect a Helm chart directory.
fn detect_helm_chart(root: &Path) -> Option<PathBuf> {
    // Chart.yaml at root
    if root.join("Chart.yaml").exists() {
        return Some(root.to_path_buf());
    }
    // Common chart subdirectories
    let candidates = ["chart", "helm", "charts"];
    for name in &candidates {
        let dir = root.join(name);
        if dir.join("Chart.yaml").exists() {
            return Some(dir);
        }
    }
    None
}

/// Detect database migration tool from project file markers.
fn detect_migration_tool(root: &Path) -> Option<MigrationTool> {
    // Prisma — schema.prisma + migrations directory
    if root.join("prisma").join("schema.prisma").exists()
        || root.join("schema.prisma").exists()
    {
        return Some(MigrationTool::Prisma);
    }
    // Diesel — diesel.toml
    if root.join("diesel.toml").exists() {
        return Some(MigrationTool::Diesel);
    }
    // sqlx — sqlx-data.json or .sqlx directory
    if root.join("sqlx-data.json").exists() || root.join(".sqlx").is_dir() {
        return Some(MigrationTool::Sqlx);
    }
    // Alembic — alembic.ini
    if root.join("alembic.ini").exists() {
        return Some(MigrationTool::Alembic);
    }
    // Django — manage.py (with migrations directory somewhere)
    if root.join("manage.py").exists() {
        return Some(MigrationTool::DjangoMigrations);
    }
    // Flyway — flyway.conf or db/migration directory
    if root.join("flyway.conf").exists()
        || root.join("db").join("migration").is_dir()
    {
        return Some(MigrationTool::Flyway);
    }
    // Liquibase — liquibase.properties
    if root.join("liquibase.properties").exists() {
        return Some(MigrationTool::Liquibase);
    }
    None
}

/// Detect health check endpoint by scanning for common route patterns in source files.
fn detect_health_check_path(root: &Path) -> Option<String> {
    // Check common locations for health endpoint definitions
    let src_dirs = ["src", "app", "server", "api", "lib"];

    for dir_name in &src_dirs {
        let dir = root.join(dir_name);
        if !dir.is_dir() {
            continue;
        }
        if let Some(path) = scan_dir_for_health_route(&dir, 0) {
            return Some(path);
        }
    }
    // Also check root-level files (e.g. main.py, app.py, server.js)
    scan_dir_for_health_route(root, 0)
}

/// Recursively scan source files for health endpoint route definitions.
fn scan_dir_for_health_route(dir: &Path, depth: usize) -> Option<String> {
    if depth > 3 {
        return None;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return None,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.')
                && name != "node_modules"
                && name != "target"
                && let Some(found) = scan_dir_for_health_route(&path, depth + 1)
            {
                return Some(found);
            }
        } else if path.is_file()
            && let Some(found) = check_file_for_health_route(&path)
        {
            return Some(found);
        }
    }
    None
}

/// Check a single file for common health endpoint patterns.
fn check_file_for_health_route(path: &Path) -> Option<String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "rs" | "py" | "js" | "ts" | "go" | "java" | "kt" => {}
        _ => return None,
    }
    let content = std::fs::read_to_string(path).ok()?;
    // Check for common health route patterns
    let patterns = [
        "/healthz",
        "/health",
        "/api/health",
        "/api/healthz",
        "/_health",
    ];
    for pattern in &patterns {
        if content.contains(pattern) {
            return Some(pattern.to_string());
        }
    }
    None
}

/// Default registry for a platform.
fn default_registry(platform: &CdPlatform) -> Registry {
    match platform {
        CdPlatform::Azure => Registry::Acr,
        CdPlatform::Gcp => Registry::Gar,
        CdPlatform::Hetzner => Registry::Ghcr,
    }
}

/// Default deploy target for a platform.
fn default_deploy_target(platform: &CdPlatform) -> DeployTarget {
    match platform {
        CdPlatform::Azure => DeployTarget::AppService,
        CdPlatform::Gcp => DeployTarget::CloudRun,
        CdPlatform::Hetzner => DeployTarget::Vps,
    }
}

/// Default environments when none are specified.
fn default_environments() -> Vec<Environment> {
    vec![
        Environment {
            name: "staging".to_string(),
            requires_approval: false,
        },
        Environment {
            name: "production".to_string(),
            requires_approval: true,
        },
    ]
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Runs the project analyzer and assembles a `CdContext` for the given path.
///
/// The `deploy_target` and `environments` parameters are optional; sensible
/// defaults are provided based on the chosen platform.
pub fn collect_cd_context(
    path: &Path,
    platform: CdPlatform,
    deploy_target: Option<DeployTarget>,
    environments: Option<Vec<Environment>>,
    registry_override: Option<Registry>,
    image_name_override: Option<String>,
) -> crate::Result<CdContext> {
    let analysis = analyze_project(path)?;

    let project_name = detect_project_name(&analysis);
    let default_branch = detect_default_branch(path);
    let has_dockerfile = analysis.docker_analysis.is_some();

    let root = &analysis.project_root;

    // Detect infrastructure
    let terraform_dir = detect_terraform(root);
    let has_terraform = terraform_dir.is_some();

    let k8s_manifest_dir = detect_k8s_manifests(root);
    let has_k8s_manifests = k8s_manifest_dir.is_some();

    let helm_chart_dir = detect_helm_chart(root);
    let has_helm_chart = helm_chart_dir.is_some();

    // Detect migration tool
    let migration_tool = detect_migration_tool(root);

    // Detect health check path
    let health_check_path = detect_health_check_path(root);

    // Resolve defaults
    let deploy_target = deploy_target.unwrap_or_else(|| default_deploy_target(&platform));
    let environments = environments.unwrap_or_else(default_environments);
    let registry = registry_override.unwrap_or_else(|| default_registry(&platform));
    let image_name = image_name_override.unwrap_or_else(|| project_name.clone());

    Ok(CdContext {
        analysis,
        project_name,
        platform,
        deploy_target,
        environments,
        registry,
        image_name,
        has_terraform,
        terraform_dir,
        has_k8s_manifests,
        k8s_manifest_dir,
        has_helm_chart,
        helm_chart_dir,
        migration_tool,
        migration_command_override: None,
        health_check_path,
        default_branch,
        has_dockerfile,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_registry_per_platform() {
        assert_eq!(default_registry(&CdPlatform::Azure), Registry::Acr);
        assert_eq!(default_registry(&CdPlatform::Gcp), Registry::Gar);
        assert_eq!(default_registry(&CdPlatform::Hetzner), Registry::Ghcr);
    }

    #[test]
    fn test_default_deploy_target_per_platform() {
        assert_eq!(default_deploy_target(&CdPlatform::Azure), DeployTarget::AppService);
        assert_eq!(default_deploy_target(&CdPlatform::Gcp), DeployTarget::CloudRun);
        assert_eq!(default_deploy_target(&CdPlatform::Hetzner), DeployTarget::Vps);
    }

    #[test]
    fn test_default_environments() {
        let envs = default_environments();
        assert_eq!(envs.len(), 2);
        assert_eq!(envs[0].name, "staging");
        assert!(!envs[0].requires_approval);
        assert_eq!(envs[1].name, "production");
        assert!(envs[1].requires_approval);
    }

    #[test]
    fn test_detect_terraform_main_tf_at_root() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("main.tf"), "resource {}").unwrap();

        let result = detect_terraform(dir.path());
        assert_eq!(result, Some(dir.path().to_path_buf()));
    }

    #[test]
    fn test_detect_terraform_in_subdir() {
        let dir = TempDir::new().unwrap();
        let tf_dir = dir.path().join("terraform");
        std::fs::create_dir(&tf_dir).unwrap();
        std::fs::write(tf_dir.join("main.tf"), "resource {}").unwrap();

        let result = detect_terraform(dir.path());
        assert_eq!(result, Some(tf_dir));
    }

    #[test]
    fn test_detect_terraform_none() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_terraform(dir.path()), None);
    }

    #[test]
    fn test_detect_k8s_manifests() {
        let dir = TempDir::new().unwrap();
        let k8s_dir = dir.path().join("k8s");
        std::fs::create_dir(&k8s_dir).unwrap();
        std::fs::write(
            k8s_dir.join("deployment.yaml"),
            "apiVersion: apps/v1\nkind: Deployment\n",
        )
        .unwrap();

        let result = detect_k8s_manifests(dir.path());
        assert_eq!(result, Some(k8s_dir));
    }

    #[test]
    fn test_detect_k8s_manifests_none() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_k8s_manifests(dir.path()), None);
    }

    #[test]
    fn test_detect_helm_chart_at_root() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("Chart.yaml"), "name: my-chart").unwrap();

        let result = detect_helm_chart(dir.path());
        assert_eq!(result, Some(dir.path().to_path_buf()));
    }

    #[test]
    fn test_detect_helm_chart_in_subdir() {
        let dir = TempDir::new().unwrap();
        let chart_dir = dir.path().join("chart");
        std::fs::create_dir(&chart_dir).unwrap();
        std::fs::write(chart_dir.join("Chart.yaml"), "name: my-chart").unwrap();

        let result = detect_helm_chart(dir.path());
        assert_eq!(result, Some(chart_dir));
    }

    #[test]
    fn test_detect_migration_prisma() {
        let dir = TempDir::new().unwrap();
        let prisma_dir = dir.path().join("prisma");
        std::fs::create_dir(&prisma_dir).unwrap();
        std::fs::write(prisma_dir.join("schema.prisma"), "model User {}").unwrap();

        assert_eq!(detect_migration_tool(dir.path()), Some(MigrationTool::Prisma));
    }

    #[test]
    fn test_detect_migration_diesel() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("diesel.toml"), "[print_schema]").unwrap();

        assert_eq!(detect_migration_tool(dir.path()), Some(MigrationTool::Diesel));
    }

    #[test]
    fn test_detect_migration_alembic() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("alembic.ini"), "[alembic]").unwrap();

        assert_eq!(detect_migration_tool(dir.path()), Some(MigrationTool::Alembic));
    }

    #[test]
    fn test_detect_migration_none() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_migration_tool(dir.path()), None);
    }

    #[test]
    fn test_detect_health_check_in_source() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(
            src.join("main.rs"),
            r#"fn main() { router.get("/health", health_handler); }"#,
        )
        .unwrap();

        let result = detect_health_check_path(dir.path());
        assert_eq!(result, Some("/health".to_string()));
    }

    #[test]
    fn test_detect_health_check_healthz() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(
            src.join("app.py"),
            r#"@app.get("/healthz") def healthz(): return "ok""#,
        )
        .unwrap();

        let result = detect_health_check_path(dir.path());
        // /healthz is checked before /health
        assert_eq!(result, Some("/healthz".to_string()));
    }

    #[test]
    fn test_detect_health_check_none() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("main.rs"), "fn main() {}").unwrap();

        assert_eq!(detect_health_check_path(dir.path()), None);
    }

    #[test]
    fn test_deploy_target_display() {
        assert_eq!(DeployTarget::AppService.to_string(), "app-service");
        assert_eq!(DeployTarget::CloudRun.to_string(), "cloud-run");
        assert_eq!(DeployTarget::Vps.to_string(), "vps");
        assert_eq!(DeployTarget::HetznerK8s.to_string(), "hetzner-k8s");
    }

    #[test]
    fn test_registry_display() {
        assert_eq!(Registry::Acr.to_string(), "acr");
        assert_eq!(Registry::Gar.to_string(), "gar");
        assert_eq!(Registry::Ghcr.to_string(), "ghcr");
        assert_eq!(
            Registry::Custom("my.registry.io".to_string()).to_string(),
            "custom(my.registry.io)"
        );
    }

    #[test]
    fn test_migration_tool_display() {
        assert_eq!(MigrationTool::Prisma.to_string(), "prisma");
        assert_eq!(MigrationTool::Diesel.to_string(), "diesel");
        assert_eq!(MigrationTool::Flyway.to_string(), "flyway");
    }
}
