//! GitHub Actions CI Template Builder — CI-11
//!
//! Assembles all generated steps into a valid `.github/workflows/ci.yml`
//! by mapping every field of `CiPipeline` onto a typed `GithubWorkflow` struct
//! and serialising it with `serde_yaml`. No string concatenation — the
//! compiler enforces structural validity.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::generator::ci_generation::schema::CiPipeline;

// ── YAML document structs ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct GithubWorkflow {
    name: String,
    /// `on` is a reserved word in Rust; serde renames it in the output.
    #[serde(rename = "on")]
    on: WorkflowOn,
    jobs: Jobs,
}

#[derive(Serialize)]
struct WorkflowOn {
    push: PushTrigger,
    pull_request: PrTrigger,
    #[serde(skip_serializing_if = "Option::is_none")]
    schedule: Option<Vec<CronEntry>>,
}

#[derive(Serialize)]
struct PushTrigger {
    branches: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
}

#[derive(Serialize)]
struct PrTrigger {
    branches: Vec<String>,
}

#[derive(Serialize)]
struct CronEntry {
    cron: String,
}

#[derive(Serialize)]
struct Jobs {
    ci: Job,
}

#[derive(Serialize)]
struct Job {
    #[serde(rename = "runs-on")]
    runs_on: String,
    steps: Vec<Step>,
}

/// A single workflow step. All fields are optional so the same struct covers
/// both `uses:` steps and `run:` steps — absent fields are omitted from the
/// YAML output via `skip_serializing_if`.
#[derive(Serialize, Default)]
struct Step {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uses: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    with: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<BTreeMap<String, String>>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Renders a `CiPipeline` into a GitHub Actions workflow YAML string.
///
/// The returned string is suitable for writing directly to
/// `.github/workflows/ci.yml` or printing for `--dry-run`.
pub fn render(pipeline: &CiPipeline) -> String {
    let workflow = build_workflow(pipeline);
    serde_yaml::to_string(&workflow)
        .expect("GithubWorkflow serialisation is infallible for valid CiPipeline")
}

// ── Builder ───────────────────────────────────────────────────────────────────

fn build_workflow(pipeline: &CiPipeline) -> GithubWorkflow {
    GithubWorkflow {
        name: "CI".to_string(),
        on: build_on(&pipeline.triggers),
        jobs: Jobs {
            ci: Job {
                runs_on: "ubuntu-latest".to_string(),
                steps: build_steps(pipeline),
            },
        },
    }
}

fn build_on(triggers: &crate::generator::ci_generation::schema::TriggerConfig) -> WorkflowOn {
    WorkflowOn {
        push: PushTrigger {
            branches: triggers.push_branches.clone(),
            tags: triggers.tag_pattern.as_ref().map(|p| vec![p.clone()]),
        },
        pull_request: PrTrigger {
            branches: triggers.pr_branches.clone(),
        },
        schedule: triggers.scheduled.as_ref().map(|cron| {
            vec![CronEntry { cron: cron.clone() }]
        }),
    }
}

fn build_steps(pipeline: &CiPipeline) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();

    // 1. Checkout
    steps.push(Step { uses: Some("actions/checkout@v4".to_string()), ..Default::default() });

    // 2. Runtime setup
    let mut runtime_with = BTreeMap::new();
    runtime_with.insert(
        runtime_version_key(&pipeline.runtime.action).to_string(),
        pipeline.runtime.version.clone(),
    );
    steps.push(Step {
        name: Some("Set up runtime".to_string()),
        uses: Some(pipeline.runtime.action.clone()),
        with: Some(runtime_with),
        ..Default::default()
    });

    // 3. Cache (optional)
    if let Some(cache) = &pipeline.cache {
        let mut w = BTreeMap::new();
        w.insert("path".to_string(), cache.paths.join("\n"));
        w.insert("key".to_string(), cache.key.clone());
        if !cache.restore_keys.is_empty() {
            w.insert("restore-keys".to_string(), cache.restore_keys.join("\n"));
        }
        steps.push(Step {
            name: Some("Cache dependencies".to_string()),
            uses: Some("actions/cache@v4".to_string()),
            with: Some(w),
            ..Default::default()
        });
    }

    // 4. Install
    steps.push(Step {
        name: Some("Install dependencies".to_string()),
        run: Some(pipeline.install.command.clone()),
        ..Default::default()
    });

    // 5. Lint (optional)
    if let Some(lint) = &pipeline.lint {
        steps.push(Step {
            name: Some("Lint".to_string()),
            run: Some(lint.command.clone()),
            ..Default::default()
        });
    }

    // 6. Test
    let test_cmd = match &pipeline.test.coverage_flag {
        Some(flag) => format!("{} {}", pipeline.test.command, flag),
        None => pipeline.test.command.clone(),
    };
    steps.push(Step {
        name: Some("Test".to_string()),
        run: Some(test_cmd),
        ..Default::default()
    });

    // 7. Build (optional)
    if let Some(build) = &pipeline.build {
        steps.push(Step {
            name: Some("Build".to_string()),
            run: Some(build.command.clone()),
            ..Default::default()
        });
    }

    // 8. Docker steps (optional)
    if let Some(docker) = &pipeline.docker_build {
        if docker.qemu {
            steps.push(Step {
                uses: Some("docker/setup-qemu-action@v3".to_string()),
                ..Default::default()
            });
        }
        if docker.buildx {
            steps.push(Step {
                uses: Some("docker/setup-buildx-action@v3".to_string()),
                ..Default::default()
            });
        }
        steps.push(Step {
            name: Some("Build Docker image".to_string()),
            run: Some(format!("docker build -t {} .", docker.image_tag)),
            ..Default::default()
        });
        if docker.push {
            steps.push(Step {
                name: Some("Push Docker image".to_string()),
                run: Some(format!("docker push {}", docker.image_tag)),
                ..Default::default()
            });
        }
    }

    // 9. Image scan (optional)
    if let Some(scan) = &pipeline.image_scan {
        let mut w = BTreeMap::new();
        w.insert("exit-code".to_string(), "1".to_string());
        w.insert("format".to_string(), scan.format.clone());
        w.insert("image-ref".to_string(), scan.image_ref.clone());
        w.insert("output".to_string(), scan.output.clone());
        w.insert("severity".to_string(), scan.fail_on_severity.clone());
        steps.push(Step {
            uses: Some("aquasecurity/trivy-action@master".to_string()),
            with: Some(w),
            ..Default::default()
        });

        if scan.upload_sarif {
            let mut w = BTreeMap::new();
            w.insert("sarif_file".to_string(), scan.output.clone());
            steps.push(Step {
                uses: Some("github/codeql-action/upload-sarif@v3".to_string()),
                with: Some(w),
                ..Default::default()
            });
        }
    }

    // 10. Secret scan (always)
    let mut sec_env = BTreeMap::new();
    sec_env.insert("GITHUB_TOKEN".to_string(), pipeline.secret_scan.github_token_expr.clone());
    if let Some(license) = &pipeline.secret_scan.gitleaks_license_secret {
        sec_env.insert(
            "GITLEAKS_LICENSE".to_string(),
            format!("${{{{ secrets.{} }}}}", license),
        );
    }
    steps.push(Step {
        uses: Some("gitleaks/gitleaks-action@v2".to_string()),
        env: Some(sec_env),
        ..Default::default()
    });

    // 11. Artifact upload (optional)
    if let Some(artifact) = &pipeline.upload_artifact {
        let mut w = BTreeMap::new();
        w.insert("name".to_string(), artifact.name.clone());
        w.insert("path".to_string(), artifact.path.clone());
        steps.push(Step {
            name: Some("Upload artifact".to_string()),
            uses: Some("actions/upload-artifact@v4".to_string()),
            with: Some(w),
            ..Default::default()
        });
    }

    steps
}

/// Derives the `with:` key name for the runtime version from the action string.
fn runtime_version_key(action: &str) -> &'static str {
    if action.contains("setup-node") { "node-version" }
    else if action.contains("setup-python") { "python-version" }
    else if action.contains("setup-go") { "go-version" }
    else if action.contains("setup-java") { "java-version" }
    else if action.contains("rust-toolchain") { "toolchain" }
    else { "version" }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{CiFormat, CiPlatform};
    use crate::generator::ci_generation::schema::{
        ArtifactStep, BuildStep, CacheStep, CiPipeline, DockerBuildStep, ImageScanStep,
        InstallStep, LintStep, RuntimeStep, SecretScanStep, TestStep, TriggerConfig,
    };

    fn make_pipeline() -> CiPipeline {
        CiPipeline {
            project_name: "my-app".to_string(),
            platform: CiPlatform::Hetzner,
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
                command: "npx jest".to_string(),
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

    #[test]
    fn test_render_produces_valid_yaml() {
        let output = render(&make_pipeline());
        let parsed: Result<serde_yaml::Value, _> = serde_yaml::from_str(&output);
        assert!(parsed.is_ok(), "render output must be valid YAML:\n{output}");
    }

    #[test]
    fn test_render_contains_checkout_step() {
        let output = render(&make_pipeline());
        assert!(output.contains("actions/checkout@v4"));
    }

    #[test]
    fn test_render_job_runs_on_ubuntu() {
        let output = render(&make_pipeline());
        assert!(output.contains("ubuntu-latest"));
    }

    #[test]
    fn test_render_workflow_name_is_ci() {
        let output = render(&make_pipeline());
        assert!(output.contains("name: CI"));
    }

    #[test]
    fn test_push_branches_emitted() {
        let output = render(&make_pipeline());
        assert!(output.contains("main"));
    }

    #[test]
    fn test_runtime_action_and_version_emitted() {
        let output = render(&make_pipeline());
        assert!(output.contains("actions/setup-node@v4"));
        assert!(output.contains("node-version"));
        assert!(output.contains("'20'") || output.contains("\"20\"") || output.contains("20"));
    }

    #[test]
    fn test_lint_step_omitted_when_none() {
        let output = render(&make_pipeline());
        assert!(!output.contains("Lint"));
    }

    #[test]
    fn test_lint_step_present_when_some() {
        let mut p = make_pipeline();
        p.lint = Some(LintStep { command: "cargo clippy -- -D warnings".to_string() });
        let output = render(&p);
        assert!(output.contains("cargo clippy -- -D warnings"));
    }

    #[test]
    fn test_test_command_emitted() {
        let output = render(&make_pipeline());
        assert!(output.contains("npx jest"));
    }

    #[test]
    fn test_coverage_flag_appended_to_test_command() {
        let mut p = make_pipeline();
        p.test.coverage_flag = Some("--coverage".to_string());
        let output = render(&p);
        assert!(output.contains("npx jest --coverage"));
    }

    #[test]
    fn test_build_step_omitted_when_none() {
        let output = render(&make_pipeline());
        assert!(!output.contains("Build\n") && !output.contains("name: Build"));
    }

    #[test]
    fn test_build_step_present_when_some() {
        let mut p = make_pipeline();
        p.build = Some(BuildStep { command: "cargo build --release".to_string(), artifact_path: None });
        let output = render(&p);
        assert!(output.contains("cargo build --release"));
    }

    #[test]
    fn test_docker_steps_omitted_when_none() {
        let output = render(&make_pipeline());
        assert!(!output.contains("docker"));
    }

    #[test]
    fn test_docker_buildx_step_emitted() {
        let mut p = make_pipeline();
        p.docker_build = Some(DockerBuildStep {
            image_tag: "ghcr.io/org/app:sha".to_string(),
            push: false,
            qemu: false,
            buildx: true,
        });
        let output = render(&p);
        assert!(output.contains("docker/setup-buildx-action@v3"));
        assert!(output.contains("docker build"));
    }

    #[test]
    fn test_image_scan_omitted_when_none() {
        let output = render(&make_pipeline());
        assert!(!output.contains("trivy-action"));
    }

    #[test]
    fn test_image_scan_step_emitted() {
        let mut p = make_pipeline();
        p.docker_build = Some(DockerBuildStep {
            image_tag: "ghcr.io/org/app:sha".to_string(),
            push: false, qemu: false, buildx: true,
        });
        p.image_scan = Some(ImageScanStep {
            image_ref: "ghcr.io/org/app:sha".to_string(),
            fail_on_severity: "CRITICAL,HIGH".to_string(),
            format: "sarif".to_string(),
            output: "trivy-results.sarif".to_string(),
            upload_sarif: true,
        });
        let output = render(&p);
        assert!(output.contains("aquasecurity/trivy-action@master"));
        assert!(output.contains("github/codeql-action/upload-sarif@v3"));
    }

    #[test]
    fn test_secret_scan_always_present() {
        let output = render(&make_pipeline());
        assert!(output.contains("gitleaks/gitleaks-action@v2"));
        assert!(output.contains("GITHUB_TOKEN"));
    }

    #[test]
    fn test_artifact_upload_emitted_when_some() {
        let mut p = make_pipeline();
        p.upload_artifact = Some(ArtifactStep {
            name: "build-output".to_string(),
            path: "dist/**".to_string(),
        });
        let output = render(&p);
        assert!(output.contains("actions/upload-artifact@v4"));
        assert!(output.contains("dist/**"));
    }

    #[test]
    fn test_scheduled_trigger_emitted() {
        let mut p = make_pipeline();
        p.triggers.scheduled = Some("0 3 * * 1".to_string());
        let output = render(&p);
        assert!(output.contains("schedule"));
        assert!(output.contains("0 3 * * 1"));
    }

    #[test]
    fn test_tag_pattern_emitted_in_push_trigger() {
        let mut p = make_pipeline();
        p.triggers.tag_pattern = Some("v*".to_string());
        let output = render(&p);
        assert!(output.contains("tags"));
        assert!(output.contains("v*"));
    }

    #[test]
    fn test_cache_step_emitted_when_some() {
        let mut p = make_pipeline();
        p.cache = Some(CacheStep {
            paths: vec!["~/.npm".to_string()],
            key: "${{ runner.os }}-npm-${{ hashFiles('**/package-lock.json') }}".to_string(),
            restore_keys: vec!["${{ runner.os }}-npm-".to_string()],
        });
        let output = render(&p);
        assert!(output.contains("actions/cache@v4"));
        assert!(output.contains("~/.npm"));
    }
}
