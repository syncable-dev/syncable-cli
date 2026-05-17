//! CD-26 — End-to-end integration tests for the CD generation subsystem.
//!
//! Tests the full pipeline: context collection → pipeline build → token
//! resolution → template rendering → YAML output validation.
//!
//! Also exercises `collect_cd_context` against language fixture directories
//! and verifies secrets-doc generation, config loading, and the combined
//! CI+CD workflow generation path.

use std::path::PathBuf;

use syncable_cli::generator::cd_generation::{
    context::{CdPlatform, DeployTarget, collect_cd_context},
    pipeline::build_cd_pipeline,
    secrets_doc::generate_cd_secrets_doc,
    templates,
    token_resolver::resolve_tokens,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns the absolute path to a CI language fixture directory.
fn fixture(lang: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ci")
        .join(lang)
}

/// Asserts YAML string contains no patterns that look like real credentials.
fn assert_no_hardcoded_secrets(yaml: &str) {
    assert!(
        !yaml
            .split_whitespace()
            .any(|w| w.starts_with("ghp_") && w.len() > 10),
        "output contains a GitHub token pattern"
    );
    assert!(
        !yaml.split_whitespace().any(|w| {
            w.starts_with("AKIA")
                && w.len() == 20
                && w[4..].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        }),
        "output contains an AWS access key pattern"
    );
    assert!(
        !yaml
            .split_whitespace()
            .any(|w| w.starts_with("sk-") && w.len() > 20),
        "output contains an API secret key pattern"
    );
}

/// Run the full pipeline (context → build → resolve → render) and return YAML.
fn render_full_pipeline(platform: CdPlatform, target: DeployTarget) -> String {
    let tmp = tempfile::TempDir::new().unwrap();
    let ctx = collect_cd_context(tmp.path(), platform.clone(), Some(target), None, None, None)
        .expect("context collection should succeed");
    let mut pipeline = build_cd_pipeline(&ctx);
    resolve_tokens(&ctx, &mut pipeline);
    match platform {
        CdPlatform::Azure => templates::azure::render(&pipeline),
        CdPlatform::Gcp => templates::gcp::render(&pipeline),
        CdPlatform::Hetzner => templates::hetzner::render(&pipeline),
    }
}

// ── Full pipeline rendering: Azure ────────────────────────────────────────────

#[test]
fn azure_app_service_full_pipeline_has_structure() {
    let yaml = render_full_pipeline(CdPlatform::Azure, DeployTarget::AppService);
    assert!(!yaml.is_empty());
    assert!(yaml.contains("name:"), "missing workflow name");
    assert!(yaml.contains("on:"), "missing trigger section");
    assert!(yaml.contains("jobs:"), "missing jobs section");
}

#[test]
fn azure_app_service_yaml_has_required_sections() {
    let yaml = render_full_pipeline(CdPlatform::Azure, DeployTarget::AppService);
    assert!(yaml.contains("name:"), "missing workflow name");
    assert!(yaml.contains("on:"), "missing trigger section");
    assert!(yaml.contains("jobs:"), "missing jobs section");
}

#[test]
fn azure_aks_full_pipeline_has_structure() {
    let yaml = render_full_pipeline(CdPlatform::Azure, DeployTarget::Aks);
    assert!(!yaml.is_empty());
    assert!(yaml.contains("name:"));
    assert!(yaml.contains("jobs:"));
}

#[test]
fn azure_container_apps_full_pipeline_has_structure() {
    let yaml = render_full_pipeline(CdPlatform::Azure, DeployTarget::ContainerApps);
    assert!(!yaml.is_empty());
    assert!(yaml.contains("name:"));
    assert!(yaml.contains("jobs:"));
}

#[test]
fn azure_yaml_has_no_hardcoded_secrets() {
    let yaml = render_full_pipeline(CdPlatform::Azure, DeployTarget::AppService);
    assert_no_hardcoded_secrets(&yaml);
}

#[test]
fn azure_yaml_contains_login_action() {
    let yaml = render_full_pipeline(CdPlatform::Azure, DeployTarget::AppService);
    assert!(
        yaml.contains("azure/login@v2"),
        "Azure pipeline must include azure/login action"
    );
}

// ── Full pipeline rendering: GCP ──────────────────────────────────────────────

#[test]
fn gcp_cloud_run_full_pipeline_has_structure() {
    let yaml = render_full_pipeline(CdPlatform::Gcp, DeployTarget::CloudRun);
    assert!(!yaml.is_empty());
    assert!(yaml.contains("name:"), "missing workflow name");
    assert!(yaml.contains("on:"), "missing trigger section");
    assert!(yaml.contains("jobs:"), "missing jobs section");
}

#[test]
fn gcp_cloud_run_yaml_has_required_sections() {
    let yaml = render_full_pipeline(CdPlatform::Gcp, DeployTarget::CloudRun);
    assert!(yaml.contains("name:"), "missing workflow name");
    assert!(yaml.contains("on:"), "missing trigger section");
    assert!(yaml.contains("jobs:"), "missing jobs section");
}

#[test]
fn gcp_gke_full_pipeline_has_structure() {
    let yaml = render_full_pipeline(CdPlatform::Gcp, DeployTarget::Gke);
    assert!(!yaml.is_empty());
    assert!(yaml.contains("name:"));
    assert!(yaml.contains("jobs:"));
}

#[test]
fn gcp_yaml_has_no_hardcoded_secrets() {
    let yaml = render_full_pipeline(CdPlatform::Gcp, DeployTarget::CloudRun);
    assert_no_hardcoded_secrets(&yaml);
}

#[test]
fn gcp_yaml_contains_auth_action() {
    let yaml = render_full_pipeline(CdPlatform::Gcp, DeployTarget::CloudRun);
    assert!(
        yaml.contains("google-github-actions/auth@v2"),
        "GCP pipeline must include google-github-actions/auth"
    );
}

// ── Full pipeline rendering: Hetzner ──────────────────────────────────────────

#[test]
fn hetzner_vps_full_pipeline_has_structure() {
    let yaml = render_full_pipeline(CdPlatform::Hetzner, DeployTarget::Vps);
    assert!(!yaml.is_empty());
    assert!(yaml.contains("name:"), "missing workflow name");
    assert!(yaml.contains("on:"), "missing trigger section");
    assert!(yaml.contains("jobs:"), "missing jobs section");
}

#[test]
fn hetzner_vps_yaml_has_required_sections() {
    let yaml = render_full_pipeline(CdPlatform::Hetzner, DeployTarget::Vps);
    assert!(yaml.contains("name:"), "missing workflow name");
    assert!(yaml.contains("on:"), "missing trigger section");
    assert!(yaml.contains("jobs:"), "missing jobs section");
}

#[test]
fn hetzner_k8s_full_pipeline_has_structure() {
    let yaml = render_full_pipeline(CdPlatform::Hetzner, DeployTarget::HetznerK8s);
    assert!(!yaml.is_empty());
    assert!(yaml.contains("name:"));
    assert!(yaml.contains("jobs:"));
}

#[test]
fn hetzner_coolify_full_pipeline_has_structure() {
    let yaml = render_full_pipeline(CdPlatform::Hetzner, DeployTarget::Coolify);
    assert!(!yaml.is_empty());
    assert!(yaml.contains("name:"));
}

#[test]
fn hetzner_yaml_has_no_hardcoded_secrets() {
    let yaml = render_full_pipeline(CdPlatform::Hetzner, DeployTarget::Vps);
    assert_no_hardcoded_secrets(&yaml);
}

#[test]
fn hetzner_yaml_contains_ssh_reference() {
    let yaml = render_full_pipeline(CdPlatform::Hetzner, DeployTarget::Vps);
    assert!(
        yaml.contains("ssh") || yaml.contains("SSH"),
        "Hetzner VPS pipeline must reference SSH"
    );
}

// ── Secrets doc generation ────────────────────────────────────────────────────

#[test]
fn secrets_doc_for_azure_yaml_contains_credentials() {
    let yaml = render_full_pipeline(CdPlatform::Azure, DeployTarget::AppService);
    let doc = generate_cd_secrets_doc(&yaml, &CdPlatform::Azure);
    assert!(
        doc.contains("AZURE") || doc.contains("azure"),
        "Azure secrets doc should mention Azure"
    );
}

#[test]
fn secrets_doc_for_gcp_yaml_mentions_gcp() {
    let yaml = render_full_pipeline(CdPlatform::Gcp, DeployTarget::CloudRun);
    let doc = generate_cd_secrets_doc(&yaml, &CdPlatform::Gcp);
    assert!(
        doc.contains("GCP") || doc.contains("gcp") || doc.contains("Google"),
        "GCP secrets doc should mention GCP/Google"
    );
}

#[test]
fn secrets_doc_for_hetzner_includes_prerequisites() {
    let yaml = render_full_pipeline(CdPlatform::Hetzner, DeployTarget::Vps);
    let doc = generate_cd_secrets_doc(&yaml, &CdPlatform::Hetzner);
    // Hetzner secrets doc always appends prerequisites checklist
    assert!(
        doc.contains("Prerequisite") || doc.contains("prerequisite") || doc.contains("Firewall") || doc.contains("Docker"),
        "Hetzner secrets doc should include prerequisites checklist"
    );
}

#[test]
fn secrets_doc_is_markdown_formatted() {
    let yaml = render_full_pipeline(CdPlatform::Azure, DeployTarget::AppService);
    let doc = generate_cd_secrets_doc(&yaml, &CdPlatform::Azure);
    // Should contain markdown table separators or section headers
    assert!(
        doc.contains("| ") || doc.contains("# ") || doc.contains("## "),
        "Secrets doc should be Markdown formatted"
    );
}

// ── Context collection from language fixtures ─────────────────────────────────

#[test]
fn collect_cd_context_succeeds_for_node_fixture() {
    let ctx =
        collect_cd_context(&fixture("node"), CdPlatform::Azure, None, None, None, None)
            .expect("should collect CD context from Node.js fixture");
    assert_eq!(ctx.platform, CdPlatform::Azure);
    assert!(!ctx.project_name.is_empty(), "project name should be detected");
}

#[test]
fn collect_cd_context_succeeds_for_python_fixture() {
    let ctx =
        collect_cd_context(&fixture("python"), CdPlatform::Gcp, None, None, None, None)
            .expect("should collect CD context from Python fixture");
    assert_eq!(ctx.platform, CdPlatform::Gcp);
}

#[test]
fn collect_cd_context_succeeds_for_rust_fixture() {
    let ctx = collect_cd_context(
        &fixture("rust"),
        CdPlatform::Hetzner,
        None,
        None,
        None,
        None,
    )
    .expect("should collect CD context from Rust fixture");
    assert_eq!(ctx.platform, CdPlatform::Hetzner);
}

#[test]
fn collect_cd_context_succeeds_for_go_fixture() {
    let ctx = collect_cd_context(&fixture("go"), CdPlatform::Azure, None, None, None, None)
        .expect("should collect CD context from Go fixture");
    assert_eq!(ctx.platform, CdPlatform::Azure);
}

#[test]
fn collect_cd_context_succeeds_for_java_fixture() {
    let ctx = collect_cd_context(
        &fixture("java"),
        CdPlatform::Gcp,
        Some(DeployTarget::CloudRun),
        None,
        None,
        None,
    )
    .expect("should collect CD context from Java fixture");
    assert_eq!(ctx.platform, CdPlatform::Gcp);
    assert_eq!(ctx.deploy_target, DeployTarget::CloudRun);
}

// ── Config file loading ───────────────────────────────────────────────────────

#[test]
fn cd_config_loads_from_syncable_cd_toml() {
    use syncable_cli::generator::cd_generation::cd_config::load_cd_config;

    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join(".syncable.cd.toml"),
        r#"
platform = "azure"
target = "app-service"
registry = "acr"
image_name = "my-integration-app"
health_check_path = "/healthz"
default_branch = "develop"
"#,
    )
    .unwrap();

    let config = load_cd_config(tmp.path())
        .expect("should load config")
        .expect("config should exist");
    assert_eq!(config.platform.as_deref(), Some("azure"));
    assert_eq!(config.image_name.as_deref(), Some("my-integration-app"));
    assert_eq!(config.default_branch.as_deref(), Some("develop"));
}

#[test]
fn cd_config_merges_into_context() {
    use syncable_cli::generator::cd_generation::cd_config::{load_cd_config, merge_config_into_cd_context};

    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join(".syncable.cd.toml"),
        r#"
image_name = "merged-app"
health_check_path = "/ready"
"#,
    )
    .unwrap();

    let mut ctx =
        collect_cd_context(tmp.path(), CdPlatform::Azure, None, None, None, None).unwrap();
    let config = load_cd_config(tmp.path()).unwrap().unwrap();
    merge_config_into_cd_context(&config, &mut ctx);

    assert_eq!(ctx.image_name, "merged-app");
    assert_eq!(ctx.health_check_path.as_deref(), Some("/ready"));
}

// ── Cross-platform consistency ────────────────────────────────────────────────

#[test]
fn all_platforms_produce_non_empty_yaml() {
    let combos: Vec<(CdPlatform, DeployTarget)> = vec![
        (CdPlatform::Azure, DeployTarget::AppService),
        (CdPlatform::Azure, DeployTarget::Aks),
        (CdPlatform::Azure, DeployTarget::ContainerApps),
        (CdPlatform::Gcp, DeployTarget::CloudRun),
        (CdPlatform::Gcp, DeployTarget::Gke),
        (CdPlatform::Hetzner, DeployTarget::Vps),
        (CdPlatform::Hetzner, DeployTarget::HetznerK8s),
        (CdPlatform::Hetzner, DeployTarget::Coolify),
    ];

    for (platform, target) in combos {
        let yaml = render_full_pipeline(platform.clone(), target.clone());
        assert!(
            !yaml.is_empty(),
            "YAML should not be empty for {:?}/{:?}",
            platform,
            target
        );
        assert!(
            yaml.len() > 50,
            "YAML is suspiciously short for {:?}/{:?}: {} bytes",
            platform,
            target,
            yaml.len()
        );
    }
}

#[test]
fn all_platform_yamls_use_secrets_expressions() {
    // All rendered YAML should reference secrets via ${{ secrets.* }} — never plain text
    let combos = [
        (CdPlatform::Azure, DeployTarget::AppService),
        (CdPlatform::Gcp, DeployTarget::CloudRun),
        (CdPlatform::Hetzner, DeployTarget::Vps),
    ];

    for (platform, target) in &combos {
        let yaml = render_full_pipeline(platform.clone(), target.clone());
        // Should use GitHub Actions secret expression syntax
        if yaml.contains("secrets.") {
            assert!(
                yaml.contains("${{ secrets."),
                "Secrets in {:?} YAML should use ${{{{ secrets.* }}}} syntax",
                platform
            );
        }
    }
}

#[test]
fn health_check_present_in_all_rendered_pipelines() {
    let combos = [
        (CdPlatform::Azure, DeployTarget::AppService),
        (CdPlatform::Gcp, DeployTarget::CloudRun),
        (CdPlatform::Hetzner, DeployTarget::Vps),
    ];

    for (platform, target) in &combos {
        let yaml = render_full_pipeline(platform.clone(), target.clone());
        assert!(
            yaml.contains("health") || yaml.contains("Health") || yaml.contains("curl") || yaml.contains("/health"),
            "Pipeline for {:?} should reference health check",
            platform
        );
    }
}
