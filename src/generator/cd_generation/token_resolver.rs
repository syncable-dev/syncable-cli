//! CD Token Resolution Engine — adapted from CI-15 for CD tokens.
//!
//! Two-pass strategy identical to the CI resolver:
//!   1. **Deterministic pass** — replaces `{{TOKEN_NAME}}` in String fields
//!      when the value can be derived unambiguously from `CdContext`.
//!   2. **Placeholder pass** — any remaining `{{TOKEN_NAME}}` pattern becomes
//!      an `UnresolvedToken` in `pipeline.unresolved_tokens`.

use std::collections::HashMap;

use regex::Regex;

use super::context::{CdContext, Registry};
use super::schema::{CdPipeline, UnresolvedToken};

/// A map from `TOKEN_NAME` to its resolved value.
pub type ResolvedTokenMap = HashMap<String, String>;

/// Runs the two-pass resolution engine on `pipeline` in place.
///
/// Returns the map of deterministically resolved tokens; callers pass this
/// to the manifest writer.
pub fn resolve_tokens(ctx: &CdContext, pipeline: &mut CdPipeline) -> ResolvedTokenMap {
    let resolved = build_resolved_map(ctx);
    let re = Regex::new(r"\{\{([A-Z][A-Z0-9_]*)\}\}").expect("static regex is valid");
    apply_to_pipeline(pipeline, &resolved, &re);
    resolved
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Builds the deterministic token map from `CdContext`.
fn build_resolved_map(ctx: &CdContext) -> ResolvedTokenMap {
    let mut map = HashMap::new();

    map.insert("PROJECT_NAME".to_string(), ctx.project_name.clone());
    map.insert("IMAGE_NAME".to_string(), ctx.image_name.clone());
    map.insert("DEFAULT_BRANCH".to_string(), ctx.default_branch.clone());

    // Registry URL is deterministic for known registries.
    match &ctx.registry {
        Registry::Ghcr => {
            map.insert("REGISTRY_URL".to_string(), "ghcr.io".to_string());
        }
        Registry::Acr | Registry::Gar | Registry::Custom(_) => {
            // These remain as placeholders — user must supply.
        }
    }

    // Health check path if detected.
    if let Some(hp) = &ctx.health_check_path {
        map.insert("HEALTH_CHECK_PATH".to_string(), hp.clone());
    }

    // Terraform directory if detected.
    if let Some(tf_dir) = &ctx.terraform_dir {
        map.insert(
            "TERRAFORM_DIR".to_string(),
            tf_dir.to_string_lossy().into_owned(),
        );
    }

    // K8s manifest directory if detected.
    if let Some(k8s_dir) = &ctx.k8s_manifest_dir {
        map.insert(
            "K8S_MANIFEST_DIR".to_string(),
            k8s_dir.to_string_lossy().into_owned(),
        );
    }

    // Helm chart directory if detected.
    if let Some(helm_dir) = &ctx.helm_chart_dir {
        map.insert(
            "HELM_CHART_DIR".to_string(),
            helm_dir.to_string_lossy().into_owned(),
        );
    }

    map
}

/// Visits every `String` field in the `CdPipeline` that may carry a `{{TOKEN}}`
/// and applies both resolution passes.
fn apply_to_pipeline(pipeline: &mut CdPipeline, resolved: &ResolvedTokenMap, re: &Regex) {
    let acc = &mut pipeline.unresolved_tokens;

    // Top-level fields.
    resolve_str(&mut pipeline.project_name, resolved, re, acc);
    resolve_str(&mut pipeline.image_name, resolved, re, acc);
    resolve_str(&mut pipeline.default_branch, resolved, re, acc);

    // Auth step.
    if let Some(action) = &mut pipeline.auth.action {
        resolve_str(action, resolved, re, acc);
    }
    resolve_str(&mut pipeline.auth.method, resolved, re, acc);
    for s in &mut pipeline.auth.required_secrets {
        resolve_str(s, resolved, re, acc);
    }

    // Registry step.
    resolve_str(&mut pipeline.registry.registry_url, resolved, re, acc);

    // Docker build + push step.
    resolve_str(&mut pipeline.docker_build_push.image_tag, resolved, re, acc);
    resolve_str(&mut pipeline.docker_build_push.context, resolved, re, acc);
    resolve_str(&mut pipeline.docker_build_push.dockerfile, resolved, re, acc);
    for arg in &mut pipeline.docker_build_push.build_args {
        resolve_str(arg, resolved, re, acc);
    }

    // Migration step.
    if let Some(mig) = &mut pipeline.migration {
        resolve_str(&mut mig.command, resolved, re, acc);
    }

    // Terraform step.
    if let Some(tf) = &mut pipeline.terraform {
        resolve_str(&mut tf.working_directory, resolved, re, acc);
        resolve_str(&mut tf.version, resolved, re, acc);
        for bc in &mut tf.backend_config {
            resolve_str(bc, resolved, re, acc);
        }
    }

    // Deploy step.
    resolve_str(&mut pipeline.deploy.command, resolved, re, acc);
    for arg in &mut pipeline.deploy.args {
        resolve_str(arg, resolved, re, acc);
    }

    // Health check step.
    resolve_str(&mut pipeline.health_check.url, resolved, re, acc);

    // Rollback info.
    resolve_str(&mut pipeline.rollback_info.command_hint, resolved, re, acc);

    // Notification step.
    if let Some(notify) = &mut pipeline.notifications {
        resolve_str(&mut notify.webhook_secret, resolved, re, acc);
    }

    // Environment configs.
    for env in &mut pipeline.environments {
        if let Some(url) = &mut env.app_url {
            resolve_str(url, resolved, re, acc);
        }
        if let Some(ns) = &mut env.namespace {
            resolve_str(ns, resolved, re, acc);
        }
    }
}

/// Resolves known tokens and collects unknown ones from a single `String` field.
fn resolve_str(
    field: &mut String,
    resolved: &ResolvedTokenMap,
    re: &Regex,
    acc: &mut Vec<UnresolvedToken>,
) {
    // Pass 1: replace deterministic tokens.
    for (name, value) in resolved {
        let placeholder = format!("{{{{{}}}}}", name);
        if field.contains(&placeholder) {
            *field = field.replace(&placeholder, value);
        }
    }

    // Pass 2: collect remaining placeholders as unresolved.
    let snapshot = field.clone();
    for cap in re.captures_iter(&snapshot) {
        let name = cap[1].to_string();
        if !acc.iter().any(|u| u.name == name) {
            acc.push(UnresolvedToken::new(
                &name,
                "Provide a value for this token",
                "string",
            ));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::context::*;
    use super::super::schema::*;
    use crate::analyzer::{AnalysisMetadata, ProjectAnalysis};
    use std::path::PathBuf;

    /// Build a minimal `ProjectAnalysis` for testing.
    #[allow(deprecated)]
    fn stub_analysis() -> ProjectAnalysis {
        ProjectAnalysis {
            project_root: PathBuf::from("/tmp/test-app"),
            languages: vec![],
            technologies: vec![],
            frameworks: vec![],
            dependencies: Default::default(),
            entry_points: vec![],
            ports: vec![],
            health_endpoints: vec![],
            environment_variables: vec![],
            project_type: crate::analyzer::ProjectType::Unknown,
            build_scripts: vec![],
            services: vec![],
            architecture_type: crate::analyzer::ArchitectureType::Monolithic,
            docker_analysis: None,
            infrastructure: None,
            analysis_metadata: AnalysisMetadata {
                timestamp: String::new(),
                analyzer_version: String::new(),
                analysis_duration_ms: 0,
                files_analyzed: 0,
                confidence_score: 0.0,
            },
        }
    }

    /// Build a minimal `CdContext` for testing.
    fn make_test_context() -> CdContext {
        CdContext {
            analysis: stub_analysis(),
            project_name: "test-app".to_string(),
            platform: CdPlatform::Gcp,
            deploy_target: DeployTarget::CloudRun,
            environments: vec![Environment {
                name: "production".to_string(),
                requires_approval: false,
            }],
            registry: Registry::Ghcr,
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

    /// Build a minimal `CdPipeline` for testing, with placeholders.
    fn make_test_pipeline() -> CdPipeline {
        CdPipeline {
            project_name: "{{PROJECT_NAME}}".to_string(),
            platform: CdPlatform::Gcp,
            deploy_target: DeployTarget::CloudRun,
            environments: vec![EnvironmentConfig {
                name: "production".to_string(),
                branch_filter: Some("main".to_string()),
                requires_approval: false,
                app_url: Some("https://{{APP_URL}}".to_string()),
                namespace: None,
                replicas: None,
            }],
            auth: AuthStep {
                action: Some("google-github-actions/auth@v2".to_string()),
                method: "workload-identity".to_string(),
                required_secrets: vec![
                    "GCP_WORKLOAD_IDENTITY_PROVIDER".to_string(),
                    "GCP_SERVICE_ACCOUNT".to_string(),
                ],
            },
            registry: RegistryStep {
                registry: Registry::Ghcr,
                login_action: Some("docker/login-action@v3".to_string()),
                registry_url: "{{REGISTRY_URL}}".to_string(),
            },
            docker_build_push: DockerBuildPushStep {
                image_tag: "{{REGISTRY_URL}}/{{IMAGE_NAME}}:sha".to_string(),
                context: ".".to_string(),
                dockerfile: "Dockerfile".to_string(),
                push: true,
                buildx: false,
                build_args: vec![],
            },
            migration: None,
            terraform: None,
            deploy: DeployStep {
                strategy: "rolling".to_string(),
                command: "gcloud run deploy {{PROJECT_NAME}}".to_string(),
                args: vec!["--region={{GCP_REGION}}".to_string()],
                target: DeployTarget::CloudRun,
            },
            health_check: HealthCheckStep {
                url: "https://{{APP_URL}}/{{HEALTH_CHECK_PATH}}".to_string(),
                retries: 5,
                interval_secs: 10,
                expected_status: 200,
            },
            rollback_info: RollbackInfo {
                strategy: "redeploy-previous".to_string(),
                command_hint: "gcloud run services update-traffic --to-revisions=LATEST=100"
                    .to_string(),
            },
            notifications: None,
            unresolved_tokens: vec![],
            default_branch: "{{DEFAULT_BRANCH}}".to_string(),
            image_name: "{{IMAGE_NAME}}".to_string(),
        }
    }

    // ── Deterministic pass tests ──────────────────────────────────────────────

    #[test]
    fn project_name_token_resolved() {
        let ctx = make_test_context();
        let mut pipeline = make_test_pipeline();

        resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(pipeline.project_name, "test-app");
    }

    #[test]
    fn image_name_token_resolved() {
        let ctx = make_test_context();
        let mut pipeline = make_test_pipeline();

        resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(pipeline.image_name, "test-app");
    }

    #[test]
    fn default_branch_token_resolved() {
        let ctx = make_test_context();
        let mut pipeline = make_test_pipeline();

        resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(pipeline.default_branch, "main");
    }

    #[test]
    fn registry_url_resolved_for_ghcr() {
        let ctx = make_test_context();
        let mut pipeline = make_test_pipeline();

        resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(pipeline.registry.registry_url, "ghcr.io");
        assert_eq!(
            pipeline.docker_build_push.image_tag,
            "ghcr.io/test-app:sha"
        );
    }

    #[test]
    fn health_check_path_resolved() {
        let ctx = make_test_context();
        let mut pipeline = make_test_pipeline();

        resolve_tokens(&ctx, &mut pipeline);

        // The health check URL should have HEALTH_CHECK_PATH replaced,
        // but APP_URL remains unresolved.
        assert!(pipeline.health_check.url.contains("/health"));
    }

    #[test]
    fn deploy_command_resolved() {
        let ctx = make_test_context();
        let mut pipeline = make_test_pipeline();

        resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(pipeline.deploy.command, "gcloud run deploy test-app");
    }

    // ── Placeholder pass tests ────────────────────────────────────────────────

    #[test]
    fn unknown_token_becomes_unresolved() {
        let ctx = make_test_context();
        let mut pipeline = make_test_pipeline();

        resolve_tokens(&ctx, &mut pipeline);

        let names: Vec<&str> = pipeline
            .unresolved_tokens
            .iter()
            .map(|u| u.name.as_str())
            .collect();
        assert!(names.contains(&"GCP_REGION"), "GCP_REGION should be unresolved");
        assert!(names.contains(&"APP_URL"), "APP_URL should be unresolved");
    }

    #[test]
    fn duplicate_tokens_deduplicated() {
        let ctx = make_test_context();
        let mut pipeline = make_test_pipeline();

        resolve_tokens(&ctx, &mut pipeline);

        let app_url_count = pipeline
            .unresolved_tokens
            .iter()
            .filter(|u| u.name == "APP_URL")
            .count();
        assert_eq!(app_url_count, 1, "APP_URL should appear exactly once");
    }

    #[test]
    fn acr_registry_url_stays_unresolved() {
        let mut ctx = make_test_context();
        ctx.registry = Registry::Acr;
        let mut pipeline = make_test_pipeline();
        pipeline.registry.registry_url = "{{ACR_LOGIN_SERVER}}".to_string();

        resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(pipeline.registry.registry_url, "{{ACR_LOGIN_SERVER}}");
        let names: Vec<&str> = pipeline
            .unresolved_tokens
            .iter()
            .map(|u| u.name.as_str())
            .collect();
        assert!(names.contains(&"ACR_LOGIN_SERVER"));
    }

    #[test]
    fn terraform_dir_resolved_when_present() {
        let mut ctx = make_test_context();
        ctx.has_terraform = true;
        ctx.terraform_dir = Some(PathBuf::from("infra/terraform"));

        let mut pipeline = make_test_pipeline();
        pipeline.terraform = Some(TerraformStep {
            working_directory: "{{TERRAFORM_DIR}}".to_string(),
            version: "{{TERRAFORM_VERSION}}".to_string(),
            backend_config: vec![],
            auto_approve: false,
        });

        let resolved = resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(
            pipeline.terraform.as_ref().unwrap().working_directory,
            "infra/terraform"
        );
        assert!(resolved.contains_key("TERRAFORM_DIR"));
        // TERRAFORM_VERSION is still unresolved.
        let names: Vec<&str> = pipeline
            .unresolved_tokens
            .iter()
            .map(|u| u.name.as_str())
            .collect();
        assert!(names.contains(&"TERRAFORM_VERSION"));
    }

    #[test]
    fn resolved_map_contains_expected_keys() {
        let ctx = make_test_context();
        let mut pipeline = make_test_pipeline();

        let resolved = resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(resolved.get("PROJECT_NAME").map(|s| s.as_str()), Some("test-app"));
        assert_eq!(resolved.get("IMAGE_NAME").map(|s| s.as_str()), Some("test-app"));
        assert_eq!(resolved.get("DEFAULT_BRANCH").map(|s| s.as_str()), Some("main"));
        assert_eq!(resolved.get("REGISTRY_URL").map(|s| s.as_str()), Some("ghcr.io"));
        assert_eq!(resolved.get("HEALTH_CHECK_PATH").map(|s| s.as_str()), Some("/health"));
    }

    #[test]
    fn migration_command_tokens_resolved() {
        let mut ctx = make_test_context();
        ctx.project_name = "mydb".to_string();
        let mut pipeline = make_test_pipeline();
        pipeline.migration = Some(MigrationStep {
            tool: MigrationTool::Prisma,
            command: "npx prisma migrate deploy --schema={{PROJECT_NAME}}/prisma/schema.prisma"
                .to_string(),
            via_ssh: false,
        });

        resolve_tokens(&ctx, &mut pipeline);

        assert_eq!(
            pipeline.migration.as_ref().unwrap().command,
            "npx prisma migrate deploy --schema=mydb/prisma/schema.prisma"
        );
    }

    #[test]
    fn environment_app_url_resolved_when_deterministic() {
        let mut ctx = make_test_context();
        // Make REGISTRY_URL deterministic (GHCR).
        ctx.registry = Registry::Ghcr;
        let mut pipeline = make_test_pipeline();
        // APP_URL is not deterministic — should stay unresolved.
        pipeline.environments[0].app_url = Some("https://{{APP_URL}}/home".to_string());

        resolve_tokens(&ctx, &mut pipeline);

        // APP_URL stays as placeholder.
        assert!(pipeline.environments[0]
            .app_url
            .as_ref()
            .unwrap()
            .contains("{{APP_URL}}"));
    }
}
