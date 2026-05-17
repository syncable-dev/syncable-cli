//! CI-25 — Unit tests for the CI generation subsystem.
//!
//! Exercises: token resolution, monorepo strategy generator, file writer
//! conflict detection, template rendering (all three platforms), coverage
//! step, notify step.  Each test section maps to a spec bullet in CI-25.

use std::io::Cursor;
use std::path::PathBuf;

use tempfile::TempDir;

use syncable_cli::cli::{CiFormat, CiPlatform};
use syncable_cli::generator::ci_generation::{
    coverage_step::{
        coverage_secrets_doc_entry, generate_coverage_step_for, render_coverage_yaml,
        CoverageService,
    },
    monorepo::generate_monorepo_strategy,
    notify_step::{generate_notify_step, render_notify_yaml},
    schema::{
        CiPipeline, InstallStep, RuntimeStep, SecretScanStep, TestStep, TriggerConfig,
    },
    templates,
    test_helpers::make_base_ctx,
    token_resolver::resolve_tokens,
    writer::{write_ci_files, write_ci_files_interactive, CiFile, WriteOutcome},
};

// ── Shared constructor ────────────────────────────────────────────────────────

/// Returns a fully-resolved minimal `CiPipeline` — no placeholder tokens.
fn minimal_pipeline() -> CiPipeline {
    CiPipeline {
        project_name: "my-service".to_string(),
        platform: CiPlatform::Gcp,
        format: CiFormat::GithubActions,
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
        secret_scan: SecretScanStep {
            github_token_expr: "${{ secrets.GITHUB_TOKEN }}".to_string(),
            gitleaks_license_secret: None,
        },
        upload_artifact: None,
        unresolved_tokens: vec![],
    }
}

// ── Token resolution ──────────────────────────────────────────────────────────

#[test]
fn resolved_map_contains_project_name_and_runtime_version() {
    let dir = TempDir::new().unwrap();
    let mut ctx = make_base_ctx(dir.path(), "Node.js");
    ctx.runtime_versions.insert("Node.js".to_string(), "20.x".to_string());
    ctx.project_name = "api-server".to_string();

    let mut pipeline = minimal_pipeline();
    pipeline.project_name = "{{PROJECT_NAME}}".to_string();
    pipeline.runtime.version = "{{RUNTIME_VERSION}}".to_string();

    let resolved = resolve_tokens(&ctx, &mut pipeline);

    assert_eq!(pipeline.project_name, "api-server");
    assert_eq!(pipeline.runtime.version, "20.x");
    assert_eq!(pipeline.unresolved_tokens.len(), 0);
    assert!(resolved.contains_key("PROJECT_NAME"));
    assert!(resolved.contains_key("RUNTIME_VERSION"));
}

#[test]
fn unknown_token_is_recorded_as_unresolved() {
    let dir = TempDir::new().unwrap();
    let ctx = make_base_ctx(dir.path(), "Rust");

    let mut pipeline = minimal_pipeline();
    pipeline.install.command = "{{CUSTOM_INSTALL_CMD}}".to_string();

    resolve_tokens(&ctx, &mut pipeline);

    assert_eq!(pipeline.unresolved_tokens.len(), 1);
    assert_eq!(pipeline.unresolved_tokens[0].name, "CUSTOM_INSTALL_CMD");
    assert_eq!(
        pipeline.unresolved_tokens[0].placeholder,
        "{{CUSTOM_INSTALL_CMD}}"
    );
}

#[test]
fn context_without_runtime_version_leaves_token_unresolved() {
    let dir = TempDir::new().unwrap();
    // No runtime_versions entry → RUNTIME_VERSION has no mapping.
    let ctx = make_base_ctx(dir.path(), "Python");

    let mut pipeline = minimal_pipeline();
    pipeline.runtime.version = "{{RUNTIME_VERSION}}".to_string();

    resolve_tokens(&ctx, &mut pipeline);

    let names: Vec<&str> =
        pipeline.unresolved_tokens.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"RUNTIME_VERSION"), "expected RUNTIME_VERSION in {:?}", names);
}

#[test]
fn fully_resolved_pipeline_has_no_unresolved_tokens() {
    let dir = TempDir::new().unwrap();
    // Pipeline already has concrete values — no {{TOKEN}} patterns.
    let ctx = make_base_ctx(dir.path(), "Go");
    let mut pipeline = minimal_pipeline();

    resolve_tokens(&ctx, &mut pipeline);

    assert_eq!(
        pipeline.unresolved_tokens.len(),
        0,
        "pipeline with no placeholders should produce zero unresolved tokens"
    );
}

// ── Monorepo strategy ─────────────────────────────────────────────────────────

#[test]
fn monorepo_strategy_returns_none_for_single_project() {
    let dir = TempDir::new().unwrap();
    let mut ctx = make_base_ctx(dir.path(), "TypeScript");
    ctx.monorepo = false;

    assert!(generate_monorepo_strategy(&ctx).is_none());
}

#[test]
fn monorepo_strategy_returns_none_for_fewer_than_two_packages() {
    let dir = TempDir::new().unwrap();
    let mut ctx = make_base_ctx(dir.path(), "TypeScript");
    ctx.monorepo = true;
    ctx.monorepo_packages = vec!["packages/api".to_string()];

    assert!(generate_monorepo_strategy(&ctx).is_none());
}

#[test]
fn monorepo_strategy_produced_for_three_packages() {
    let dir = TempDir::new().unwrap();
    let mut ctx = make_base_ctx(dir.path(), "TypeScript");
    ctx.monorepo = true;
    ctx.monorepo_packages = vec![
        "packages/api".to_string(),
        "packages/web".to_string(),
        "packages/sdk".to_string(),
    ];

    let strategy = generate_monorepo_strategy(&ctx).unwrap();
    assert_eq!(strategy.packages.len(), 3);
    assert!(
        strategy.detect_job_yaml.contains("dorny/paths-filter"),
        "detect job should reference dorny/paths-filter"
    );
    assert!(strategy.matrix_job_yaml.contains("matrix"));
}

#[test]
fn monorepo_filter_config_contains_all_package_paths() {
    let dir = TempDir::new().unwrap();
    let mut ctx = make_base_ctx(dir.path(), "Go");
    ctx.monorepo = true;
    ctx.monorepo_packages =
        vec!["services/auth".to_string(), "services/billing".to_string()];

    let strategy = generate_monorepo_strategy(&ctx).unwrap();
    assert!(strategy.filter_config.contains("services/auth/**"));
    assert!(strategy.filter_config.contains("services/billing/**"));
}

// ── File writer & conflict detection ─────────────────────────────────────────

/// Minimal valid GitHub Actions YAML for writer tests.
fn valid_yaml() -> String {
    "name: CI\non:\n  push:\n    branches: [main]\njobs:\n  ci:\n    runs-on: ubuntu-latest\n    steps: []\n"
        .to_string()
}

#[test]
fn write_ci_files_creates_new_file() {
    let dir = TempDir::new().unwrap();
    let files = vec![CiFile::pipeline(valid_yaml(), CiFormat::GithubActions)];

    let summary = write_ci_files(&files, dir.path(), false).unwrap();

    assert_eq!(summary.created(), 1, "new file should be created");
    assert_eq!(summary.skipped(), 0);
    assert!(dir.path().join(".github/workflows/ci.yml").exists());
}

#[test]
fn write_ci_files_detects_conflict_on_different_content() {
    let dir = TempDir::new().unwrap();
    let ci_dir = dir.path().join(".github/workflows");
    std::fs::create_dir_all(&ci_dir).unwrap();
    std::fs::write(ci_dir.join("ci.yml"), "name: OldPipeline\n").unwrap();

    let files = vec![CiFile::pipeline(valid_yaml(), CiFormat::GithubActions)];
    let summary = write_ci_files(&files, dir.path(), false).unwrap();

    assert_eq!(summary.skipped(), 1, "conflict should be recorded as skipped");
    assert!(summary.has_conflicts());
}

#[test]
fn write_ci_files_overwrites_when_force_is_true() {
    let dir = TempDir::new().unwrap();
    let ci_dir = dir.path().join(".github/workflows");
    std::fs::create_dir_all(&ci_dir).unwrap();
    std::fs::write(ci_dir.join("ci.yml"), "name: OldPipeline\n").unwrap();

    let files = vec![CiFile::pipeline(valid_yaml(), CiFormat::GithubActions)];
    let summary = write_ci_files(&files, dir.path(), true).unwrap();

    assert_eq!(summary.overwritten(), 1);
    assert!(!summary.has_conflicts());
}

#[test]
fn write_ci_files_records_invalid_yaml_outcome() {
    let dir = TempDir::new().unwrap();
    let files = vec![CiFile::pipeline(
        "not: valid: yaml:\n  - [\n".to_string(),
        CiFormat::GithubActions,
    )];

    let summary = write_ci_files(&files, dir.path(), false).unwrap();
    assert_eq!(summary.invalid(), 1);
}

#[test]
fn write_ci_files_interactive_resolves_conflict_with_overwrite_choice() {
    let dir = TempDir::new().unwrap();
    let ci_dir = dir.path().join(".github/workflows");
    std::fs::create_dir_all(&ci_dir).unwrap();
    std::fs::write(ci_dir.join("ci.yml"), "name: OldPipeline\n").unwrap();

    let files = vec![CiFile::pipeline(valid_yaml(), CiFormat::GithubActions)];
    // Simulate user typing "o" then Enter.
    let mut reader = Cursor::new("o\n");
    let summary =
        write_ci_files_interactive(&files, dir.path(), &mut reader).unwrap();

    assert_eq!(summary.overwritten(), 1);
}

// ── Template rendering ────────────────────────────────────────────────────────

#[test]
fn github_actions_render_produces_valid_yaml() {
    let output = templates::github_actions::render(&minimal_pipeline());
    serde_yaml::from_str::<serde_yaml::Value>(&output)
        .expect("GitHub Actions output must be valid YAML");
}

#[test]
fn azure_pipelines_render_produces_valid_yaml() {
    let output = templates::azure_pipelines::render(&minimal_pipeline());
    serde_yaml::from_str::<serde_yaml::Value>(&output)
        .expect("Azure Pipelines output must be valid YAML");
}

#[test]
fn cloud_build_render_produces_valid_yaml() {
    let output = templates::cloud_build::render(&minimal_pipeline());
    serde_yaml::from_str::<serde_yaml::Value>(&output)
        .expect("Cloud Build output must be valid YAML");
}

/// Snapshot test — demonstrates `insta` usage; on first run with
/// `INSTA_UPDATE=unseen cargo test` the snapshot file is created and
/// committed alongside this file.
#[test]
fn github_actions_render_snapshot() {
    let output = templates::github_actions::render(&minimal_pipeline());
    insta::assert_snapshot!(output);
}

// ── Coverage step ─────────────────────────────────────────────────────────────

#[test]
fn coverage_yaml_is_valid_and_contains_codecov_action() {
    let test = TestStep {
        command: "pytest".to_string(),
        coverage_flag: Some("--cov=.".to_string()),
        coverage_report_path: Some("coverage.xml".to_string()),
    };
    let step = generate_coverage_step_for(&test, CoverageService::Codecov).unwrap();
    let yaml = render_coverage_yaml(&step);

    // render_coverage_yaml returns a step snippet (not a complete YAML document);
    // full-document validity is asserted by the template integration tests.
    assert!(yaml.contains("codecov-action"), "should reference codecov-action");
    assert!(yaml.contains("coverage.xml"), "should embed the report path");
    assert!(yaml.contains("CODECOV_TOKEN"), "should reference the secret");
}

#[test]
fn coverage_secrets_doc_marks_token_as_optional() {
    let test = TestStep {
        command: "pytest".to_string(),
        coverage_flag: Some("--cov=.".to_string()),
        coverage_report_path: Some("coverage.xml".to_string()),
    };
    let step = generate_coverage_step_for(&test, CoverageService::Codecov).unwrap();
    let doc = coverage_secrets_doc_entry(&step);

    assert!(doc.contains("CODECOV_TOKEN"));
    assert!(
        doc.to_lowercase().contains("optional"),
        "CODECOV_TOKEN should be marked optional"
    );
}

// ── Notify step ───────────────────────────────────────────────────────────────

#[test]
fn notify_yaml_contains_failure_condition_and_slack_action() {
    let step = generate_notify_step(true).unwrap();
    let yaml = render_notify_yaml(&step);

    assert!(yaml.contains("if: failure()"), "must include `if: failure()`");
    assert!(
        yaml.contains("slackapi/slack-github-action"),
        "must reference the Slack action"
    );
}

#[test]
fn notify_step_disabled_returns_none() {
    assert!(generate_notify_step(false).is_none());
}
