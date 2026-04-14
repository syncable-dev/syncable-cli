//! CD Pipeline Builder
//!
//! Assembles a `CdPipeline` intermediate representation from a `CdContext`.
//! This mirrors the CI pattern: `collect_context → build_pipeline → resolve_tokens → render`.
//!
//! The builder calls platform-specific generators (auth, registry, deploy,
//! migration, health_check) and converts their outputs into schema types.

use super::auth_azure;
use super::auth_gcp;
use super::auth_hetzner;
use super::context::{CdContext, CdPlatform, DeployTarget};
use super::deploy_azure;
use super::deploy_gcp;
use super::deploy_hetzner;
use super::health_check;
use super::migration;
use super::notification;
use super::registry;
use super::schema::{
    CdPipeline, DockerBuildPushStep, EnvironmentConfig,
};
use super::terraform_step;

// ── Public API ────────────────────────────────────────────────────────────────

/// Assembles a complete `CdPipeline` from the given project context.
///
/// The resulting pipeline can be fed to `token_resolver::resolve_tokens` and
/// then to one of the template renderers (azure/gcp/hetzner).
pub fn build_cd_pipeline(ctx: &CdContext) -> CdPipeline {
    // ── Auth step ─────────────────────────────────────────────────────────
    let auth = match ctx.platform {
        CdPlatform::Azure => {
            let cfg = auth_azure::generate_azure_auth();
            auth_azure::to_auth_step(&cfg)
        }
        CdPlatform::Gcp => {
            let cfg = auth_gcp::generate_gcp_auth();
            auth_gcp::to_auth_step(&cfg)
        }
        CdPlatform::Hetzner => {
            let cfg = auth_hetzner::generate_hetzner_auth(&ctx.deploy_target);
            auth_hetzner::to_auth_step(&cfg)
        }
    };

    // ── Registry step ─────────────────────────────────────────────────────
    let reg_cfg = registry::generate_registry_config(&ctx.registry);
    let registry_step = registry::to_registry_step(&reg_cfg);

    // ── Image tag ─────────────────────────────────────────────────────────
    let image_tag = registry::build_image_tag(&reg_cfg, &ctx.image_name);

    // ── Docker build+push step ────────────────────────────────────────────
    let docker_build_push = DockerBuildPushStep {
        image_tag: image_tag.clone(),
        context: ".".to_string(),
        dockerfile: "Dockerfile".to_string(),
        push: true,
        buildx: true,
        build_args: vec![],
    };

    // ── Deploy step ───────────────────────────────────────────────────────
    let deploy = match ctx.platform {
        CdPlatform::Azure => {
            deploy_azure::generate_azure_deploy(&ctx.deploy_target, &image_tag)
        }
        CdPlatform::Gcp => {
            deploy_gcp::generate_gcp_deploy(&ctx.deploy_target, &image_tag)
        }
        CdPlatform::Hetzner => {
            deploy_hetzner::generate_hetzner_deploy(&ctx.deploy_target, &image_tag)
        }
    };

    // ── Rollback info ─────────────────────────────────────────────────────
    let rollback_info = match ctx.platform {
        CdPlatform::Azure => deploy_azure::azure_rollback_info(&ctx.deploy_target),
        CdPlatform::Gcp => deploy_gcp::gcp_rollback_info(&ctx.deploy_target),
        CdPlatform::Hetzner => deploy_hetzner::hetzner_rollback_info(&ctx.deploy_target),
    };

    // ── Migration step ────────────────────────────────────────────────────
    let via_ssh = ctx.deploy_target == DeployTarget::Vps;
    let migration_step =
        migration::generate_migration_step(ctx.migration_tool.as_ref(), via_ssh);

    // ── Health check step ─────────────────────────────────────────────────
    let health_check_step = health_check::generate_health_check(
        &ctx.deploy_target,
        ctx.health_check_path.as_deref(),
    );

    // ── Environment configs ───────────────────────────────────────────────
    let environments: Vec<EnvironmentConfig> = ctx
        .environments
        .iter()
        .map(|env| EnvironmentConfig {
            name: env.name.clone(),
            branch_filter: default_branch_filter(&env.name, &ctx.default_branch),
            requires_approval: env.requires_approval,
            app_url: None,
            namespace: default_namespace(&env.name, &ctx.deploy_target),
            replicas: default_replicas(&env.name),
        })
        .collect();

    // ── Terraform step (CD-16) ─────────────────────────────────────────
    let terraform = if ctx.has_terraform {
        let tf_dir = ctx
            .terraform_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "terraform".to_string());
        Some(terraform_step::generate_terraform_step(&tf_dir, false))
    } else {
        None
    };

    // ── Notification step (CD-21) ────────────────────────────────────────
    let notifications = Some(notification::generate_notification_step(
        "SLACK_WEBHOOK_URL",
        true,
        true,
    ));

    CdPipeline {
        project_name: ctx.project_name.clone(),
        platform: ctx.platform.clone(),
        deploy_target: ctx.deploy_target.clone(),
        environments,
        auth,
        registry: registry_step,
        docker_build_push,
        migration: migration_step,
        terraform,
        deploy,
        health_check: health_check_step,
        rollback_info,
        notifications,
        unresolved_tokens: vec![],
        default_branch: ctx.default_branch.clone(),
        image_name: ctx.image_name.clone(),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns a branch filter for common environment names.
fn default_branch_filter(env_name: &str, default_branch: &str) -> Option<String> {
    match env_name {
        "production" | "prod" => Some(default_branch.to_string()),
        "staging" | "stage" => Some("develop".to_string()),
        "dev" | "development" => Some("develop".to_string()),
        _ => None,
    }
}

/// Returns a Kubernetes namespace when the target is a k8s-based target.
fn default_namespace(env_name: &str, target: &DeployTarget) -> Option<String> {
    match target {
        DeployTarget::Aks | DeployTarget::Gke | DeployTarget::HetznerK8s => {
            Some(env_name.to_string())
        }
        _ => None,
    }
}

/// Returns default replica counts per environment.
fn default_replicas(env_name: &str) -> Option<u32> {
    match env_name {
        "production" | "prod" => Some(2),
        "staging" | "stage" => Some(1),
        "dev" | "development" => Some(1),
        _ => None,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::cd_generation::context::{
        CdPlatform, DeployTarget, Environment, Registry,
    };
    use tempfile::TempDir;

    fn sample_context(platform: CdPlatform, target: DeployTarget) -> CdContext {
        let tmp = TempDir::new().unwrap();
        let analysis = crate::analyzer::analyze_project(tmp.path()).unwrap();
        CdContext {
            analysis,
            project_name: "test-app".to_string(),
            platform: platform.clone(),
            deploy_target: target,
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
            registry: match platform {
                CdPlatform::Azure => Registry::Acr,
                CdPlatform::Gcp => Registry::Gar,
                CdPlatform::Hetzner => Registry::Ghcr,
            },
            image_name: "test-app".to_string(),
            has_terraform: false,
            terraform_dir: None,
            has_k8s_manifests: false,
            k8s_manifest_dir: None,
            has_helm_chart: false,
            helm_chart_dir: None,
            migration_tool: None,
            health_check_path: Some("/health".to_string()),
            default_branch: "main".to_string(),
            has_dockerfile: true,
        }
    }

    // ── Azure ─────────────────────────────────────────────────────────────

    #[test]
    fn azure_app_service_pipeline_has_oidc_auth() {
        let ctx = sample_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        assert_eq!(pipeline.auth.method, "oidc");
        assert!(pipeline.auth.action.as_deref() == Some("azure/login@v2"));
    }

    #[test]
    fn azure_pipeline_uses_acr_registry() {
        let ctx = sample_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        assert_eq!(pipeline.registry.registry, Registry::Acr);
    }

    #[test]
    fn azure_aks_deploy_step_uses_k8s_deploy_action() {
        let ctx = sample_context(CdPlatform::Azure, DeployTarget::Aks);
        let pipeline = build_cd_pipeline(&ctx);
        assert_eq!(pipeline.deploy.command, "azure/k8s-deploy@v5");
    }

    #[test]
    fn azure_pipeline_has_two_environments() {
        let ctx = sample_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        assert_eq!(pipeline.environments.len(), 2);
        assert_eq!(pipeline.environments[0].name, "staging");
        assert_eq!(pipeline.environments[1].name, "production");
    }

    #[test]
    fn production_env_has_branch_filter_main() {
        let ctx = sample_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        let prod = &pipeline.environments[1];
        assert_eq!(prod.branch_filter.as_deref(), Some("main"));
        assert!(prod.requires_approval);
    }

    #[test]
    fn aks_environments_have_namespace() {
        let ctx = sample_context(CdPlatform::Azure, DeployTarget::Aks);
        let pipeline = build_cd_pipeline(&ctx);
        assert_eq!(
            pipeline.environments[0].namespace.as_deref(),
            Some("staging")
        );
        assert_eq!(
            pipeline.environments[1].namespace.as_deref(),
            Some("production")
        );
    }

    // ── GCP ───────────────────────────────────────────────────────────────

    #[test]
    fn gcp_cloud_run_pipeline_has_wif_auth() {
        let ctx = sample_context(CdPlatform::Gcp, DeployTarget::CloudRun);
        let pipeline = build_cd_pipeline(&ctx);
        assert_eq!(pipeline.auth.method, "workload-identity");
    }

    #[test]
    fn gcp_pipeline_uses_gar_registry() {
        let ctx = sample_context(CdPlatform::Gcp, DeployTarget::CloudRun);
        let pipeline = build_cd_pipeline(&ctx);
        assert_eq!(pipeline.registry.registry, Registry::Gar);
    }

    #[test]
    fn gcp_gke_deploy_uses_kubectl() {
        let ctx = sample_context(CdPlatform::Gcp, DeployTarget::Gke);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.deploy.command.contains("kubectl"));
    }

    // ── Hetzner ───────────────────────────────────────────────────────────

    #[test]
    fn hetzner_vps_pipeline_has_ssh_auth() {
        let ctx = sample_context(CdPlatform::Hetzner, DeployTarget::Vps);
        let pipeline = build_cd_pipeline(&ctx);
        assert_eq!(pipeline.auth.method, "ssh");
    }

    #[test]
    fn hetzner_pipeline_uses_ghcr() {
        let ctx = sample_context(CdPlatform::Hetzner, DeployTarget::Vps);
        let pipeline = build_cd_pipeline(&ctx);
        assert_eq!(pipeline.registry.registry, Registry::Ghcr);
    }

    #[test]
    fn hetzner_vps_deploy_uses_ssh() {
        let ctx = sample_context(CdPlatform::Hetzner, DeployTarget::Vps);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.deploy.command.contains("ssh"));
    }

    // ── Migration ─────────────────────────────────────────────────────────

    #[test]
    fn pipeline_without_migration_tool_has_no_migration_step() {
        let ctx = sample_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.migration.is_none());
    }

    #[test]
    fn pipeline_with_migration_tool_has_migration_step() {
        use crate::generator::cd_generation::context::MigrationTool;
        let mut ctx = sample_context(CdPlatform::Azure, DeployTarget::AppService);
        ctx.migration_tool = Some(MigrationTool::Prisma);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.migration.is_some());
        assert!(pipeline.migration.unwrap().command.contains("prisma"));
    }

    #[test]
    fn hetzner_vps_migration_is_via_ssh() {
        use crate::generator::cd_generation::context::MigrationTool;
        let mut ctx = sample_context(CdPlatform::Hetzner, DeployTarget::Vps);
        ctx.migration_tool = Some(MigrationTool::Alembic);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.migration.as_ref().unwrap().via_ssh);
    }

    // ── Docker build ──────────────────────────────────────────────────────

    #[test]
    fn docker_build_push_defaults_to_buildx() {
        let ctx = sample_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.docker_build_push.buildx);
        assert!(pipeline.docker_build_push.push);
        assert_eq!(pipeline.docker_build_push.context, ".");
    }

    #[test]
    fn image_tag_contains_registry_and_image_name() {
        let ctx = sample_context(CdPlatform::Hetzner, DeployTarget::Vps);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.docker_build_push.image_tag.contains("ghcr.io"));
        assert!(pipeline.docker_build_push.image_tag.contains("test-app"));
    }

    // ── Health check ──────────────────────────────────────────────────────

    #[test]
    fn health_check_uses_detected_path() {
        let ctx = sample_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.health_check.url.contains("/health"));
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    #[test]
    fn default_branch_filter_production_uses_main() {
        assert_eq!(
            default_branch_filter("production", "main"),
            Some("main".to_string())
        );
    }

    #[test]
    fn default_branch_filter_staging_uses_develop() {
        assert_eq!(
            default_branch_filter("staging", "main"),
            Some("develop".to_string())
        );
    }

    #[test]
    fn default_branch_filter_unknown_returns_none() {
        assert_eq!(default_branch_filter("custom-env", "main"), None);
    }

    #[test]
    fn default_namespace_for_k8s_targets() {
        assert_eq!(
            default_namespace("staging", &DeployTarget::Aks),
            Some("staging".to_string())
        );
        assert_eq!(default_namespace("prod", &DeployTarget::AppService), None);
    }

    #[test]
    fn default_replicas_production_is_two() {
        assert_eq!(default_replicas("production"), Some(2));
        assert_eq!(default_replicas("staging"), Some(1));
        assert_eq!(default_replicas("custom"), None);
    }
}
