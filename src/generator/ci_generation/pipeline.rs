//! CI Pipeline Orchestrator — CI-01 (wiring)
//!
//! `build_ci_pipeline` is the single entry point that assembles a complete
//! `CiPipeline` from a `CiContext`.  It calls every step-generator module in
//! canonical order and collects unresolved tokens from each.

use crate::generator::ci_generation::{
    build_step::generate_build_step,
    cache::resolve_cache,
    context::{CiContext, PackageManager},
    docker_step::generate_docker_step,
    image_scan_step::generate_image_scan_step,
    lint_step::generate_lint_step,
    runtime_resolver::resolve_runtime,
    schema::{
        ArtifactStep, BuildStep, CacheStep, CiPipeline, InstallStep, LintStep, RuntimeStep,
        TestStep, UnresolvedToken,
    },
    secret_scan_step::generate_secret_scan_step,
    test_step::generate_test_step,
    triggers::resolve_triggers,
};

/// Assembles a complete `CiPipeline` from a collected `CiContext`.
///
/// When `skip_docker` is `true` the Docker build, image scan, and artifact
/// upload steps are omitted even if a Dockerfile is present.
pub fn build_ci_pipeline(ctx: &CiContext, skip_docker: bool) -> CiPipeline {
    let mut unresolved: Vec<UnresolvedToken> = Vec::new();

    // ── Triggers ──────────────────────────────────────────────────────────
    let triggers = resolve_triggers(ctx);

    // ── Runtime / toolchain ───────────────────────────────────────────────
    let runtime_setup = resolve_runtime(ctx);
    for token_name in &runtime_setup.unresolved_tokens {
        unresolved.push(UnresolvedToken::new(
            token_name,
            "Runtime version — check your version file or CI requirements",
            "string",
        ));
    }
    let runtime = RuntimeStep {
        action: runtime_setup.action.to_string(),
        version: runtime_setup.version,
    };

    // ── Cache ─────────────────────────────────────────────────────────────
    let cache = resolve_cache(ctx).map(|c| CacheStep {
        paths: c.paths,
        key: c.key,
        restore_keys: c.restore_keys,
    });

    // ── Install ───────────────────────────────────────────────────────────
    let install = InstallStep { command: install_command(&ctx.package_manager) };

    // ── Lint ──────────────────────────────────────────────────────────────
    let lint = generate_lint_step(ctx).map(|l| LintStep { command: l.command });

    // ── Test ──────────────────────────────────────────────────────────────
    let test_step_raw = generate_test_step(ctx);
    if test_step_raw.command.contains("{{TEST_COMMAND}}") {
        unresolved.push(UnresolvedToken::new(
            "TEST_COMMAND",
            "Command to run your test suite",
            "string",
        ));
    }
    let test = TestStep {
        command: test_step_raw.command,
        coverage_flag: test_step_raw.coverage_flag,
        coverage_report_path: test_step_raw.coverage_report_path,
    };

    // ── Build ─────────────────────────────────────────────────────────────
    let build = generate_build_step(ctx).map(|b| {
        if b.command.contains("{{BUILD_COMMAND}}") {
            unresolved.push(UnresolvedToken::new(
                "BUILD_COMMAND",
                "Command to compile or bundle your project",
                "string",
            ));
        }
        BuildStep { command: b.command, artifact_path: b.artifact_path }
    });

    // ── Docker & image scan ───────────────────────────────────────────────
    let (docker_build, image_scan) = if skip_docker {
        (None, None)
    } else {
        let d = generate_docker_step(ctx);
        if let Some(ref ds) = d {
            if ds.image_tag.contains("{{REGISTRY_URL}}") || ds.image_tag.contains("{{IMAGE_NAME}}") {
                unresolved.push(UnresolvedToken::new(
                    "REGISTRY_URL",
                    "Container registry URL e.g. ghcr.io/org/repo",
                    "url",
                ));
                unresolved.push(UnresolvedToken::new(
                    "IMAGE_NAME",
                    "Image name e.g. my-app",
                    "string",
                ));
            }
        }
        let scan = generate_image_scan_step(&d);
        (d, scan)
    };

    // ── Secret scan ───────────────────────────────────────────────────────
    let secret_scan = generate_secret_scan_step();

    // ── Artifact upload ───────────────────────────────────────────────────
    let upload_artifact = build.as_ref().and_then(|b| {
        b.artifact_path.as_ref().map(|path| ArtifactStep {
            name: ctx.project_name.clone(),
            path: path.clone(),
        })
    });

    CiPipeline {
        project_name: ctx.project_name.clone(),
        platform: ctx.platform.clone(),
        format: ctx.format.clone(),
        triggers,
        runtime,
        cache,
        install,
        lint,
        test,
        build,
        docker_build,
        image_scan,
        secret_scan,
        upload_artifact,
        unresolved_tokens: unresolved,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Maps a `PackageManager` to its standard install command.
fn install_command(pm: &PackageManager) -> String {
    match pm {
        PackageManager::Npm => "npm ci".to_string(),
        PackageManager::Yarn => "yarn install --frozen-lockfile".to_string(),
        PackageManager::Pnpm => "pnpm install --frozen-lockfile".to_string(),
        PackageManager::Bun => "bun install".to_string(),
        PackageManager::Pip => "pip install -r requirements.txt".to_string(),
        PackageManager::Poetry => "poetry install --no-interaction".to_string(),
        PackageManager::Uv => "uv sync".to_string(),
        PackageManager::Cargo => "cargo fetch".to_string(),
        PackageManager::GoMod => "go mod download".to_string(),
        PackageManager::Maven => "mvn dependency:resolve -q".to_string(),
        PackageManager::Gradle => "./gradlew dependencies --quiet".to_string(),
        PackageManager::Unknown => "{{INSTALL_COMMAND}}".to_string(),
    }
}
