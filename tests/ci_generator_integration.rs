//! CI-26 — End-to-end integration tests for the CI generation subsystem.
//!
//! Tests template rendering at the full-pipeline level — valid YAML output,
//! required structural fields, and absence of hardcoded secrets.
//!
//! Also exercises `collect_ci_context` against the language fixture projects
//! in `tests/fixtures/ci/` to verify that context collection succeeds and
//! produces the expected primary language for each ecosystem.
//!
//! # Note on CI-01 wiring
//!
//! The CLI handler `handle_generate_ci` currently returns a static skeleton
//! rather than invoking the full pipeline.  These tests exercise the template
//! layer directly.  A companion test asserting the full CLI binary output will
//! be added once CI-01 (final wiring) replaces the stub.

use std::path::PathBuf;

use syncable_cli::cli::{CiFormat, CiPlatform};
use syncable_cli::generator::ci_generation::{
    context::collect_ci_context,
    schema::{
        CiPipeline, InstallStep, RuntimeStep, SecretScanStep, TestStep, TriggerConfig,
    },
    templates,
};

// ── Shared helpers ────────────────────────────────────────────────────────────

fn minimal_pipeline(platform: CiPlatform, format: CiFormat) -> CiPipeline {
    CiPipeline {
        project_name: "integration-test-app".to_string(),
        platform,
        format,
        triggers: TriggerConfig {
            push_branches: vec!["main".to_string()],
            pr_branches: vec!["main".to_string()],
            tag_pattern: None,
            scheduled: None,
        },
        runtime: RuntimeStep {
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
        secret_scan: syncable_cli::generator::ci_generation::schema::SecretScanStep {
            github_token_expr: "${{ secrets.GITHUB_TOKEN }}".to_string(),
            gitleaks_license_secret: None,
        },
        upload_artifact: None,
        unresolved_tokens: vec![],
    }
}

/// Returns the absolute path to a CI language fixture directory.
fn fixture(lang: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ci")
        .join(lang)
}

/// Asserts that `yaml` contains no string patterns that look like real
/// credential values (GitHub tokens, AWS keys, etc.).
fn assert_no_hardcoded_secrets(yaml: &str) {
    // Real GitHub personal-access tokens start with "ghp_" followed by 36+ alphanum chars.
    assert!(
        !yaml.split_whitespace().any(|w| w.starts_with("ghp_") && w.len() > 10),
        "output contains a GitHub token pattern: {yaml}"
    );
    // Real AWS access key IDs start with "AKIA" followed by exactly 16 uppercase chars.
    assert!(
        !yaml.split_whitespace().any(|w| {
            w.starts_with("AKIA")
                && w.len() == 20
                && w[4..].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        }),
        "output contains an AWS access key pattern: {yaml}"
    );
}

// ── GitHub Actions end-to-end ─────────────────────────────────────────────────

#[test]
fn github_actions_output_is_valid_yaml() {
    let yaml = templates::github_actions::render(&minimal_pipeline(
        CiPlatform::Hetzner,
        CiFormat::GithubActions,
    ));
    serde_yaml::from_str::<serde_yaml::Value>(&yaml)
        .expect("GitHub Actions output must be valid YAML");
}

#[test]
fn github_actions_output_contains_checkout_step() {
    let yaml = templates::github_actions::render(&minimal_pipeline(
        CiPlatform::Hetzner,
        CiFormat::GithubActions,
    ));
    assert!(
        yaml.contains("actions/checkout"),
        "GitHub Actions pipeline must include a checkout step"
    );
}

#[test]
fn github_actions_output_contains_runtime_setup_step() {
    let yaml = templates::github_actions::render(&minimal_pipeline(
        CiPlatform::Hetzner,
        CiFormat::GithubActions,
    ));
    // Runtime setup action was injected into the pipeline.
    assert!(
        yaml.contains("setup-node"),
        "pipeline must contain a runtime setup step"
    );
}

#[test]
fn github_actions_output_contains_test_step() {
    let yaml = templates::github_actions::render(&minimal_pipeline(
        CiPlatform::Hetzner,
        CiFormat::GithubActions,
    ));
    assert!(yaml.contains("npm test"), "pipeline must contain the test command");
}

#[test]
fn github_actions_output_has_no_hardcoded_secrets() {
    let yaml = templates::github_actions::render(&minimal_pipeline(
        CiPlatform::Hetzner,
        CiFormat::GithubActions,
    ));
    assert_no_hardcoded_secrets(&yaml);
}

// ── Azure Pipelines end-to-end ────────────────────────────────────────────────

#[test]
fn azure_pipelines_output_is_valid_yaml() {
    let yaml = templates::azure_pipelines::render(&minimal_pipeline(
        CiPlatform::Azure,
        CiFormat::AzurePipelines,
    ));
    serde_yaml::from_str::<serde_yaml::Value>(&yaml)
        .expect("Azure Pipelines output must be valid YAML");
}

#[test]
fn azure_pipelines_output_contains_required_fields() {
    let yaml = templates::azure_pipelines::render(&minimal_pipeline(
        CiPlatform::Azure,
        CiFormat::AzurePipelines,
    ));
    // Azure auto-checkouts; runtime setup and test step are required.
    assert!(yaml.contains("npm test"), "Azure pipeline must contain the test command");
    assert!(
        yaml.contains("ubuntu") || yaml.contains("ubuntu-latest"),
        "Azure pipeline must specify an agent VM image"
    );
}

#[test]
fn azure_pipelines_output_has_no_hardcoded_secrets() {
    let yaml = templates::azure_pipelines::render(&minimal_pipeline(
        CiPlatform::Azure,
        CiFormat::AzurePipelines,
    ));
    assert_no_hardcoded_secrets(&yaml);
}

// ── Cloud Build end-to-end ────────────────────────────────────────────────────

#[test]
fn cloud_build_output_is_valid_yaml() {
    let yaml = templates::cloud_build::render(&minimal_pipeline(
        CiPlatform::Gcp,
        CiFormat::CloudBuild,
    ));
    serde_yaml::from_str::<serde_yaml::Value>(&yaml)
        .expect("Cloud Build output must be valid YAML");
}

#[test]
fn cloud_build_output_contains_test_step() {
    let yaml = templates::cloud_build::render(&minimal_pipeline(
        CiPlatform::Gcp,
        CiFormat::CloudBuild,
    ));
    assert!(yaml.contains("npm test"), "Cloud Build pipeline must contain the test command");
}

#[test]
fn cloud_build_output_has_no_hardcoded_secrets() {
    let yaml = templates::cloud_build::render(&minimal_pipeline(
        CiPlatform::Gcp,
        CiFormat::CloudBuild,
    ));
    assert_no_hardcoded_secrets(&yaml);
}

// ── CiContext collection from language fixtures ───────────────────────────────

#[test]
fn collect_ci_context_succeeds_for_node_fixture() {
    let ctx = collect_ci_context(&fixture("node"), CiPlatform::Hetzner, CiFormat::GithubActions)
        .expect("should collect context from Node.js fixture");
    assert_ne!(
        ctx.primary_language.to_lowercase(),
        "unknown",
        "should detect a real language for Node.js fixture"
    );
}

#[test]
fn collect_ci_context_succeeds_for_python_fixture() {
    let ctx =
        collect_ci_context(&fixture("python"), CiPlatform::Gcp, CiFormat::GithubActions)
            .expect("should collect context from Python fixture");
    assert_ne!(ctx.primary_language.to_lowercase(), "unknown");
}

#[test]
fn collect_ci_context_succeeds_for_rust_fixture() {
    let ctx =
        collect_ci_context(&fixture("rust"), CiPlatform::Hetzner, CiFormat::GithubActions)
            .expect("should collect context from Rust fixture");
    assert!(
        ctx.primary_language.to_lowercase().contains("rust"),
        "expected Rust primary language, got: {}",
        ctx.primary_language
    );
}

#[test]
fn collect_ci_context_succeeds_for_go_fixture() {
    let ctx = collect_ci_context(&fixture("go"), CiPlatform::Gcp, CiFormat::GithubActions)
        .expect("should collect context from Go fixture");
    assert_ne!(ctx.primary_language.to_lowercase(), "unknown");
}

#[test]
fn collect_ci_context_succeeds_for_java_fixture() {
    let ctx = collect_ci_context(&fixture("java"), CiPlatform::Azure, CiFormat::AzurePipelines)
        .expect("should collect context from Java fixture");
    assert_ne!(ctx.primary_language.to_lowercase(), "unknown");
}
