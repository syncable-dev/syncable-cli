//! GCP Cloud Build CI Template Builder — CI-13
//!
//! Generates `cloudbuild.yaml`. Each CI step maps to a Cloud Build step
//! keyed by a Docker `name:` (container image), an `entrypoint:`, and `args:`.
//!
//! Key design constraints vs. GitHub Actions / Azure Pipelines:
//!   - No "runtime setup" step: the container image IS the runtime.
//!   - No trigger block: GCP triggers are configured in the console/API.
//!   - Artifact upload maps to top-level `artifacts.objects` (GCS path).
//!   - Trivy → `aquasec/trivy` image; Gitleaks → `zricethezav/gitleaks` image.
//!   - Cache: no native dep cache; skipped (GCS volume mounts require bucket info).

use serde::Serialize;

use crate::generator::ci_generation::schema::CiPipeline;

// ── YAML document structs ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct CloudBuildConfig {
    steps: Vec<CloudBuildStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifacts: Option<Artifacts>,
    timeout: String,
}

/// A single Cloud Build step. `name` is always a Docker image URI.
#[derive(Serialize, Default)]
struct CloudBuildStep {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entrypoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Vec<String>>,
    /// Cloud Build env entries are `"KEY=VALUE"` strings.
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<Vec<String>>,
}

#[derive(Serialize)]
struct Artifacts {
    objects: ArtifactObjects,
}

#[derive(Serialize)]
struct ArtifactObjects {
    location: String,
    paths: Vec<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Renders a `CiPipeline` into a GCP Cloud Build YAML string.
///
/// The returned string is ready to write as `cloudbuild.yaml` at the
/// repository root. Triggers must be configured separately in the GCP console.
pub fn render(pipeline: &CiPipeline) -> String {
    let doc = build_config(pipeline);
    serde_yaml::to_string(&doc)
        .expect("CloudBuildConfig serialisation is infallible for valid CiPipeline")
}

// ── Builder ───────────────────────────────────────────────────────────────────

fn build_config(pipeline: &CiPipeline) -> CloudBuildConfig {
    CloudBuildConfig {
        steps: build_steps(pipeline),
        artifacts: pipeline.upload_artifact.as_ref().map(|art| Artifacts {
            objects: ArtifactObjects {
                location: format!("gs://{{{{GCS_ARTIFACTS_BUCKET}}}}/{}", art.name),
                paths: vec![art.path.clone()],
            },
        }),
        timeout: "3600s".to_string(),
    }
}

fn build_steps(pipeline: &CiPipeline) -> Vec<CloudBuildStep> {
    let runtime_image = runtime_docker_image(&pipeline.runtime.action, &pipeline.runtime.version);
    let mut steps: Vec<CloudBuildStep> = Vec::new();

    // NOTE: Cloud Build auto-clones the source repo — no checkout step needed.

    // 1. Install
    steps.push(shell_step(
        &runtime_image,
        Some("Install dependencies"),
        &pipeline.install.command,
        None,
    ));

    // 2. Lint (optional)
    if let Some(lint) = &pipeline.lint {
        steps.push(shell_step(&runtime_image, Some("Lint"), &lint.command, None));
    }

    // 3. Test
    let test_cmd = match &pipeline.test.coverage_flag {
        Some(flag) => format!("{} {}", pipeline.test.command, flag),
        None => pipeline.test.command.clone(),
    };
    steps.push(shell_step(&runtime_image, Some("Test"), &test_cmd, None));

    // 4. Build (optional)
    if let Some(build) = &pipeline.build {
        steps.push(shell_step(&runtime_image, Some("Build"), &build.command, None));
    }

    // 5. Docker (optional) — gcr.io/cloud-builders/docker is the canonical builder image
    if let Some(docker) = &pipeline.docker_build {
        steps.push(CloudBuildStep {
            name: "gcr.io/cloud-builders/docker".to_string(),
            id: Some("Build Docker image".to_string()),
            args: Some(vec![
                "build".to_string(),
                "-t".to_string(),
                docker.image_tag.clone(),
                ".".to_string(),
            ]),
            ..Default::default()
        });
        if docker.push {
            steps.push(CloudBuildStep {
                name: "gcr.io/cloud-builders/docker".to_string(),
                id: Some("Push Docker image".to_string()),
                args: Some(vec!["push".to_string(), docker.image_tag.clone()]),
                ..Default::default()
            });
        }
    }

    // 6. Image scan (optional) — aquasec/trivy image
    if let Some(scan) = &pipeline.image_scan {
        steps.push(CloudBuildStep {
            name: "aquasec/trivy".to_string(),
            id: Some("Scan image (Trivy)".to_string()),
            args: Some(vec![
                "image".to_string(),
                "--exit-code".to_string(),
                "1".to_string(),
                "--severity".to_string(),
                scan.fail_on_severity.clone(),
                "--format".to_string(),
                scan.format.clone(),
                "--output".to_string(),
                scan.output.clone(),
                scan.image_ref.clone(),
            ]),
            ..Default::default()
        });
    }

    // 7. Secret scan (always) — zricethezav/gitleaks image
    let mut sec_env = vec![format!(
        "GITHUB_TOKEN={}",
        pipeline.secret_scan.github_token_expr
    )];
    if let Some(license) = &pipeline.secret_scan.gitleaks_license_secret {
        sec_env.push(format!("GITLEAKS_LICENSE=${{{}}}", license));
    }
    steps.push(CloudBuildStep {
        name: "zricethezav/gitleaks".to_string(),
        id: Some("Secret scan (Gitleaks)".to_string()),
        args: Some(vec![
            "detect".to_string(),
            "--source".to_string(),
            "/workspace".to_string(),
            "--exit-code".to_string(),
            "1".to_string(),
        ]),
        env: Some(sec_env),
        ..Default::default()
    });

    steps
}

/// Constructs a step that runs a shell command via `bash -c` inside the
/// given container image. Suitable for any arbitrary `run:` equivalent.
fn shell_step(
    image: &str,
    id: Option<&str>,
    command: &str,
    env: Option<Vec<String>>,
) -> CloudBuildStep {
    CloudBuildStep {
        name: image.to_string(),
        id: id.map(|s| s.to_string()),
        entrypoint: Some("bash".to_string()),
        args: Some(vec!["-c".to_string(), command.to_string()]),
        env,
        ..Default::default()
    }
}

/// Maps a GitHub Actions runtime action to the equivalent Docker Hub image URI
/// used as the Cloud Build step `name:`.
fn runtime_docker_image(action: &str, version: &str) -> String {
    if action.contains("setup-node") {
        format!("node:{version}")
    } else if action.contains("setup-python") {
        format!("python:{version}")
    } else if action.contains("setup-go") {
        format!("golang:{version}")
    } else if action.contains("setup-java") {
        format!("eclipse-temurin:{version}")
    } else if action.contains("rust-toolchain") {
        format!("rust:{version}")
    } else {
        // Unknown runtime: fall back to a generic Debian image with bash
        "debian:bookworm-slim".to_string()
    }
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
            platform: CiPlatform::Gcp,
            format: CiFormat::CloudBuild,
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
                github_token_expr: "$_GITHUB_TOKEN".to_string(),
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
    fn test_no_trigger_block_emitted() {
        // Cloud Build triggers live in the GCP console, not in the YAML.
        let output = render(&make_pipeline());
        assert!(!output.contains("trigger:"));
        assert!(!output.contains("on:"));
    }

    #[test]
    fn test_timeout_emitted() {
        let output = render(&make_pipeline());
        assert!(output.contains("3600s"));
    }

    #[test]
    fn test_node_runtime_image_used() {
        let output = render(&make_pipeline());
        assert!(output.contains("node:20"));
    }

    #[test]
    fn test_python_runtime_image() {
        let mut p = make_pipeline();
        p.runtime = RuntimeStep {
            action: "actions/setup-python@v4".to_string(),
            version: "3.11".to_string(),
        };
        let output = render(&p);
        assert!(output.contains("python:3.11"));
    }

    #[test]
    fn test_rust_runtime_image() {
        let mut p = make_pipeline();
        p.runtime = RuntimeStep {
            action: "dtolnay/rust-toolchain@stable".to_string(),
            version: "stable".to_string(),
        };
        let output = render(&p);
        assert!(output.contains("rust:stable"));
    }

    #[test]
    fn test_install_step_uses_bash_entrypoint() {
        let output = render(&make_pipeline());
        assert!(output.contains("bash"));
        assert!(output.contains("npm ci"));
    }

    #[test]
    fn test_lint_omitted_when_none() {
        let output = render(&make_pipeline());
        assert!(!output.contains("Lint"));
    }

    #[test]
    fn test_lint_step_emitted() {
        let mut p = make_pipeline();
        p.lint = Some(LintStep { command: "cargo clippy -- -D warnings".to_string() });
        let output = render(&p);
        assert!(output.contains("cargo clippy -- -D warnings"));
        assert!(output.contains("Lint"));
    }

    #[test]
    fn test_test_command_emitted() {
        let output = render(&make_pipeline());
        assert!(output.contains("npx jest"));
    }

    #[test]
    fn test_coverage_flag_appended() {
        let mut p = make_pipeline();
        p.test.coverage_flag = Some("--coverage".to_string());
        let output = render(&p);
        assert!(output.contains("npx jest --coverage"));
    }

    #[test]
    fn test_build_omitted_when_none() {
        let output = render(&make_pipeline());
        assert!(!output.contains("id: Build"));
    }

    #[test]
    fn test_build_step_emitted() {
        let mut p = make_pipeline();
        p.build = Some(BuildStep {
            command: "cargo build --release".to_string(),
            artifact_path: None,
        });
        let output = render(&p);
        assert!(output.contains("cargo build --release"));
    }

    #[test]
    fn test_docker_omitted_when_none() {
        let output = render(&make_pipeline());
        assert!(!output.contains("gcr.io/cloud-builders/docker"));
    }

    #[test]
    fn test_docker_build_step_emitted() {
        let mut p = make_pipeline();
        p.docker_build = Some(DockerBuildStep {
            image_tag: "gcr.io/my-project/app:latest".to_string(),
            push: false,
            qemu: false,
            buildx: false,
        });
        let output = render(&p);
        assert!(output.contains("gcr.io/cloud-builders/docker"));
        assert!(output.contains("gcr.io/my-project/app:latest"));
    }

    #[test]
    fn test_docker_push_step_emitted() {
        let mut p = make_pipeline();
        p.docker_build = Some(DockerBuildStep {
            image_tag: "gcr.io/my-project/app:latest".to_string(),
            push: true,
            qemu: false,
            buildx: false,
        });
        let output = render(&p);
        assert!(output.contains("Push Docker image"));
        assert!(output.contains("push"));
    }

    #[test]
    fn test_image_scan_omitted_when_none() {
        let output = render(&make_pipeline());
        assert!(!output.contains("aquasec/trivy"));
    }

    #[test]
    fn test_trivy_step_emitted() {
        let mut p = make_pipeline();
        p.image_scan = Some(ImageScanStep {
            image_ref: "gcr.io/my-project/app:latest".to_string(),
            fail_on_severity: "CRITICAL,HIGH".to_string(),
            format: "table".to_string(),
            output: "trivy.txt".to_string(),
            upload_sarif: false,
        });
        let output = render(&p);
        assert!(output.contains("aquasec/trivy"));
        assert!(output.contains("CRITICAL,HIGH"));
    }

    #[test]
    fn test_secret_scan_always_present() {
        let output = render(&make_pipeline());
        assert!(output.contains("zricethezav/gitleaks"));
        assert!(output.contains("GITHUB_TOKEN"));
    }

    #[test]
    fn test_gitleaks_license_env_when_some() {
        let mut p = make_pipeline();
        p.secret_scan.gitleaks_license_secret = Some("GITLEAKS_LICENSE".to_string());
        let output = render(&p);
        assert!(output.contains("GITLEAKS_LICENSE"));
    }

    #[test]
    fn test_artifact_objects_emitted() {
        let mut p = make_pipeline();
        p.upload_artifact = Some(ArtifactStep {
            name: "build-output".to_string(),
            path: "dist/**".to_string(),
        });
        let output = render(&p);
        assert!(output.contains("artifacts"));
        assert!(output.contains("GCS_ARTIFACTS_BUCKET"));
        assert!(output.contains("dist/**"));
    }

    #[test]
    fn test_no_artifacts_section_when_none() {
        let output = render(&make_pipeline());
        assert!(!output.contains("artifacts:"));
    }

    #[test]
    fn test_cache_step_not_emitted() {
        // Cloud Build has no native dep cache — CacheStep is deliberately skipped.
        let mut p = make_pipeline();
        p.cache = Some(CacheStep {
            paths: vec!["~/.npm".to_string()],
            key: "npm-key".to_string(),
            restore_keys: vec![],
        });
        let output = render(&p);
        assert!(!output.contains("Cache@2"));
        assert!(!output.contains("actions/cache"));
    }
}
