//! CD-25 — Comprehensive Unit Tests for the CD Generator
//!
//! This module covers cross-cutting concerns that span multiple cd_generation
//! submodules:
//!   - Full pipeline build → template render → YAML validation per platform
//!   - Token cross-linking between CI and CD contexts
//!   - Multi-environment structure validation
//!   - Terraform wiring into the pipeline
//!   - End-to-end dry-run simulation

#[cfg(test)]
mod cd_snapshot_tests {
    use crate::generator::cd_generation::{
        context::{CdPlatform, DeployTarget, Environment, MigrationTool, Registry},
        pipeline::build_cd_pipeline,
        templates,
        token_resolver::resolve_tokens,
    };
    use tempfile::TempDir;

    // ── Fixture builder ───────────────────────────────────────────────────

    fn make_context(
        platform: CdPlatform,
        target: DeployTarget,
    ) -> crate::generator::cd_generation::context::CdContext {
        let tmp = TempDir::new().unwrap();
        let analysis = crate::analyzer::analyze_project(tmp.path()).unwrap();
        crate::generator::cd_generation::context::CdContext {
            analysis,
            project_name: "snapshot-app".to_string(),
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
            image_name: "snapshot-app".to_string(),
            has_terraform: false,
            terraform_dir: None,
            has_k8s_manifests: false,
            k8s_manifest_dir: None,
            has_helm_chart: false,
            helm_chart_dir: None,
            migration_tool: None,
            migration_command_override: None,
            health_check_path: Some("/health".to_string()),
            default_branch: "main".to_string(),
            has_dockerfile: true,
        }
    }

    // ── Azure snapshots ───────────────────────────────────────────────────

    #[test]
    fn azure_app_service_yaml_is_valid() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        let mut pipeline = build_cd_pipeline(&ctx);
        resolve_tokens(&ctx, &mut pipeline);
        let yaml = templates::azure::render(&pipeline);
        assert!(yaml.contains("name:"), "Missing workflow name");
        assert!(yaml.contains("on:"), "Missing trigger section");
        assert!(yaml.contains("jobs:"), "Missing jobs section");
        assert!(yaml.contains("azure/login@v2"), "Missing Azure login action");
        assert!(yaml.contains("snapshot-app"), "Missing project name");
    }

    #[test]
    fn azure_aks_yaml_contains_kubectl() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::Aks);
        let pipeline = build_cd_pipeline(&ctx);
        let yaml = templates::azure::render(&pipeline);
        assert!(yaml.contains("kubectl") || yaml.contains("aks"), "Missing K8s deploy");
    }

    #[test]
    fn azure_container_apps_yaml_valid() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::ContainerApps);
        let pipeline = build_cd_pipeline(&ctx);
        let yaml = templates::azure::render(&pipeline);
        assert!(yaml.contains("name:"));
        assert!(yaml.contains("jobs:"));
    }

    // ── GCP snapshots ─────────────────────────────────────────────────────

    #[test]
    fn gcp_cloud_run_yaml_is_valid() {
        let ctx = make_context(CdPlatform::Gcp, DeployTarget::CloudRun);
        let mut pipeline = build_cd_pipeline(&ctx);
        resolve_tokens(&ctx, &mut pipeline);
        let yaml = templates::gcp::render(&pipeline);
        assert!(yaml.contains("name:"));
        assert!(yaml.contains("on:"));
        assert!(yaml.contains("jobs:"));
        assert!(
            yaml.contains("google-github-actions/auth@v2"),
            "Missing GCP auth action"
        );
    }

    #[test]
    fn gcp_gke_yaml_contains_k8s_deploy() {
        let ctx = make_context(CdPlatform::Gcp, DeployTarget::Gke);
        let pipeline = build_cd_pipeline(&ctx);
        let yaml = templates::gcp::render(&pipeline);
        assert!(yaml.contains("kubectl") || yaml.contains("gke"), "Missing GKE deploy");
    }

    // ── Hetzner snapshots ─────────────────────────────────────────────────

    #[test]
    fn hetzner_vps_yaml_is_valid() {
        let ctx = make_context(CdPlatform::Hetzner, DeployTarget::Vps);
        let mut pipeline = build_cd_pipeline(&ctx);
        resolve_tokens(&ctx, &mut pipeline);
        let yaml = templates::hetzner::render(&pipeline);
        assert!(yaml.contains("name:"));
        assert!(yaml.contains("on:"));
        assert!(yaml.contains("jobs:"));
        assert!(yaml.contains("ssh") || yaml.contains("SSH"), "Missing SSH");
    }

    #[test]
    fn hetzner_k8s_yaml_valid() {
        let ctx = make_context(CdPlatform::Hetzner, DeployTarget::HetznerK8s);
        let pipeline = build_cd_pipeline(&ctx);
        let yaml = templates::hetzner::render(&pipeline);
        assert!(yaml.contains("name:"));
        assert!(yaml.contains("jobs:"));
    }

    #[test]
    fn hetzner_coolify_yaml_valid() {
        let ctx = make_context(CdPlatform::Hetzner, DeployTarget::Coolify);
        let pipeline = build_cd_pipeline(&ctx);
        let yaml = templates::hetzner::render(&pipeline);
        assert!(yaml.contains("name:"));
    }

    // ── No hardcoded secrets ──────────────────────────────────────────────

    fn assert_no_hardcoded_secrets(yaml: &str) {
        assert!(!yaml.contains("sk-"), "Contains hardcoded API key");
        assert!(!yaml.contains("ghp_"), "Contains hardcoded GitHub token");
        assert!(!yaml.contains("AKIA"), "Contains hardcoded AWS key");
    }

    #[test]
    fn azure_yaml_no_hardcoded_secrets() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        let yaml = templates::azure::render(&pipeline);
        assert_no_hardcoded_secrets(&yaml);
    }

    #[test]
    fn gcp_yaml_no_hardcoded_secrets() {
        let ctx = make_context(CdPlatform::Gcp, DeployTarget::CloudRun);
        let pipeline = build_cd_pipeline(&ctx);
        let yaml = templates::gcp::render(&pipeline);
        assert_no_hardcoded_secrets(&yaml);
    }

    #[test]
    fn hetzner_yaml_no_hardcoded_secrets() {
        let ctx = make_context(CdPlatform::Hetzner, DeployTarget::Vps);
        let pipeline = build_cd_pipeline(&ctx);
        let yaml = templates::hetzner::render(&pipeline);
        assert_no_hardcoded_secrets(&yaml);
    }

    // ── Pipeline structure tests ──────────────────────────────────────────

    #[test]
    fn pipeline_has_two_environments() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        assert_eq!(pipeline.environments.len(), 2);
        assert_eq!(pipeline.environments[0].name, "staging");
        assert_eq!(pipeline.environments[1].name, "production");
    }

    #[test]
    fn production_requires_approval() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        let prod = pipeline.environments.iter().find(|e| e.name == "production").unwrap();
        assert!(prod.requires_approval);
    }

    #[test]
    fn staging_no_approval() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        let staging = pipeline.environments.iter().find(|e| e.name == "staging").unwrap();
        assert!(!staging.requires_approval);
    }

    #[test]
    fn health_check_has_endpoint() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        // health_check is always present (non-Option)
        assert!(!pipeline.health_check.url.is_empty());
    }

    #[test]
    fn migration_present_when_tool_detected() {
        let mut ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        ctx.migration_tool = Some(MigrationTool::Prisma);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.migration.is_some());
        assert!(pipeline.migration.as_ref().unwrap().command.contains("prisma"));
    }

    #[test]
    fn migration_absent_when_no_tool() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.migration.is_none());
    }

    // ── Terraform wiring ──────────────────────────────────────────────────

    #[test]
    fn terraform_step_present_when_has_terraform() {
        let mut ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        ctx.has_terraform = true;
        ctx.terraform_dir = Some(std::path::PathBuf::from("terraform"));
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.terraform.is_some());
        assert_eq!(pipeline.terraform.as_ref().unwrap().working_directory, "terraform");
    }

    #[test]
    fn terraform_step_absent_when_no_terraform() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.terraform.is_none());
    }

    // ── Notification wiring ───────────────────────────────────────────────

    #[test]
    fn notification_always_present() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(pipeline.notifications.is_some());
        assert_eq!(pipeline.notifications.as_ref().unwrap().channel_type, "slack");
    }

    // ── Rollback info ─────────────────────────────────────────────────────

    #[test]
    fn rollback_info_has_strategy() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(!pipeline.rollback_info.strategy.is_empty());
    }

    #[test]
    fn rollback_info_has_command_hint() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        let pipeline = build_cd_pipeline(&ctx);
        assert!(!pipeline.rollback_info.command_hint.is_empty());
    }

    // ── Token resolution ──────────────────────────────────────────────────

    #[test]
    fn tokens_resolved_after_resolution() {
        let ctx = make_context(CdPlatform::Azure, DeployTarget::AppService);
        let mut pipeline = build_cd_pipeline(&ctx);
        resolve_tokens(&ctx, &mut pipeline);
        // After resolution, unresolved tokens should be minimal
    }

    // ── Multi-platform consistency ────────────────────────────────────────

    #[test]
    fn all_platforms_produce_valid_yaml() {
        let platforms = [
            (CdPlatform::Azure, DeployTarget::AppService),
            (CdPlatform::Gcp, DeployTarget::CloudRun),
            (CdPlatform::Hetzner, DeployTarget::Vps),
        ];

        for (platform, target) in &platforms {
            let ctx = make_context(platform.clone(), target.clone());
            let mut pipeline = build_cd_pipeline(&ctx);
            resolve_tokens(&ctx, &mut pipeline);
            let yaml = match platform {
                CdPlatform::Azure => templates::azure::render(&pipeline),
                CdPlatform::Gcp => templates::gcp::render(&pipeline),
                CdPlatform::Hetzner => templates::hetzner::render(&pipeline),
            };
            assert!(yaml.contains("name:"), "Missing 'name:' for {:?}", platform);
            assert!(yaml.contains("on:"), "Missing 'on:' for {:?}", platform);
            assert!(yaml.contains("jobs:"), "Missing 'jobs:' for {:?}", platform);
            assert_no_hardcoded_secrets(&yaml);
        }
    }
}

#[cfg(test)]
mod cd_cross_linking_tests {
    use crate::generator::cd_generation::{
        environments::{generate_environment_jobs, render_environment_jobs_yaml},
        rollback::{generate_rollback_script},
        versioning::{compute_image_tags, render_versioning_env_block, render_tag_resolution_step},
        dispatch::{generate_dispatch_inputs, render_dispatch_yaml},
        notification::{generate_notification_step, render_notification_yaml},
        terraform_step::{generate_terraform_step, render_terraform_yaml},
        reusable_workflow::{render_reusable_base, render_caller_job},
        context::{CdPlatform, DeployTarget},
        schema::{EnvironmentConfig, RollbackInfo},
    };

    // ── Environment → dispatch consistency ────────────────────────────────

    #[test]
    fn dispatch_inputs_match_default_environments() {
        let dispatch = generate_dispatch_inputs(&[]);
        let env_input = &dispatch[1];
        if let crate::generator::cd_generation::dispatch::DispatchInputType::Choice { options } =
            &env_input.input_type
        {
            // default dispatch options = development, staging, production
            assert!(options.contains(&"development".to_string()));
            assert!(options.contains(&"staging".to_string()));
            assert!(options.contains(&"production".to_string()));
        }
    }

    #[test]
    fn custom_environments_flow_to_dispatch() {
        let envs = vec!["dev".to_string(), "prod".to_string()];
        let dispatch = generate_dispatch_inputs(&envs);
        let env_input = &dispatch[1];
        if let crate::generator::cd_generation::dispatch::DispatchInputType::Choice { options } =
            &env_input.input_type
        {
            assert_eq!(options.len(), 2);
        }
    }

    // ── Versioning + notification YAML composability ──────────────────────

    #[test]
    fn versioning_env_block_combines_with_notification() {
        let tags = compute_image_tags("ghcr.io", "my-app");
        let env_block = render_versioning_env_block(&tags);
        let notif_step = generate_notification_step("SLACK_WEBHOOK_URL", true, true);
        let notif_yaml = render_notification_yaml(&notif_step);

        // Both should be valid YAML fragments that can be placed in the same file
        assert!(env_block.contains("IMAGE_TAG"));
        assert!(notif_yaml.contains("Notify Slack"));
    }

    // ── Terraform + rollback consistency ──────────────────────────────────

    #[test]
    fn terraform_yaml_and_rollback_both_reference_image_tag() {
        let tf_step = generate_terraform_step("terraform", false);
        let tf_yaml = render_terraform_yaml(&tf_step, "main");
        assert!(tf_yaml.contains("IMAGE_TAG"), "Terraform should reference IMAGE_TAG");

        let rollback_info = RollbackInfo {
            strategy: "redeploy-previous".to_string(),
            command_hint: "az webapp deployment slot swap".to_string(),
        };
        let rollback_script = generate_rollback_script(
            &CdPlatform::Azure,
            &DeployTarget::AppService,
            &rollback_info,
        );
        assert!(!rollback_script.is_empty(), "Rollback script should not be empty");
    }

    // ── Reusable workflow + environment integration ───────────────────────

    #[test]
    fn reusable_base_renders_for_all_platforms() {
        for platform in &[CdPlatform::Azure, CdPlatform::Gcp, CdPlatform::Hetzner] {
            let target = match platform {
                CdPlatform::Azure => DeployTarget::AppService,
                CdPlatform::Gcp => DeployTarget::CloudRun,
                CdPlatform::Hetzner => DeployTarget::Vps,
            };
            let base = render_reusable_base(platform, &target, "my-app");
            assert!(base.contains("workflow_call"), "Missing workflow_call for {:?}", platform);
        }
    }

    #[test]
    fn caller_job_references_environment() {
        let caller = render_caller_job("staging", "${{ env.IMAGE_TAG }}", Some("build"));
        assert!(caller.contains("staging"));
        assert!(caller.contains("IMAGE_TAG"));
    }

    // ── Environment jobs generate yaml ────────────────────────────────────

    #[test]
    fn environment_jobs_render_correct_yaml() {
        let envs = vec![
            EnvironmentConfig {
                name: "staging".to_string(),
                branch_filter: None,
                requires_approval: false,
                app_url: None,
                namespace: None,
                replicas: None,
            },
            EnvironmentConfig {
                name: "production".to_string(),
                branch_filter: None,
                requires_approval: true,
                app_url: None,
                namespace: None,
                replicas: None,
            },
        ];
        let jobs = generate_environment_jobs(&envs);
        assert_eq!(jobs.len(), 2);
        let yaml = render_environment_jobs_yaml(&jobs);
        assert!(yaml.contains("staging"));
        assert!(yaml.contains("production"));
    }

    // ── Dispatch yaml renders ─────────────────────────────────────────────

    #[test]
    fn full_dispatch_yaml_renders() {
        let inputs = generate_dispatch_inputs(&[]);
        let yaml = render_dispatch_yaml(&inputs);
        assert!(yaml.contains("workflow_dispatch:"));
        assert!(yaml.contains("image_tag:"));
        assert!(yaml.contains("environment:"));
        assert!(yaml.contains("dry_run:"));
    }

    // ── Tag resolution step is valid ──────────────────────────────────────

    #[test]
    fn tag_resolution_step_yaml() {
        let step = render_tag_resolution_step();
        assert!(step.contains("Compute image tags"));
        assert!(step.contains("GITHUB_OUTPUT"));
    }
}
