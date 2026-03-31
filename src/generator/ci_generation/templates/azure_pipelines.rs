//! Azure Pipelines CI Template Builder — CI-12
//!
//! Generates `azure-pipelines.yml` from `CiPipeline` by mapping each step
//! to the Azure Pipelines task vocabulary:
//!
//! - Runtime setup  → `NodeTool@0` / `UsePythonVersion@0` / `GoTool@0` / script
//! - Cache          → `Cache@2`
//! - Shell steps    → `script:` with `displayName:`
//! - Artifact upload→ `PublishBuildArtifacts@1`
//! - Trivy / Gitleaks → inline `script:` (no native Azure task)
//!
//! Azure auto-checks-out the repo before any steps, so no explicit step
//! is emitted for that.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::generator::ci_generation::schema::CiPipeline;

// ── YAML document structs ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct AzurePipeline {
    trigger: AzureTrigger,
    pr: AzurePr,
    #[serde(skip_serializing_if = "Option::is_none")]
    schedules: Option<Vec<AzureSchedule>>,
    pool: Pool,
    steps: Vec<AzureStep>,
}

#[derive(Serialize)]
struct AzureTrigger {
    branches: BranchFilter,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<TagFilter>,
}

#[derive(Serialize)]
struct AzurePr {
    branches: BranchFilter,
}

#[derive(Serialize)]
struct BranchFilter {
    include: Vec<String>,
}

#[derive(Serialize)]
struct TagFilter {
    include: Vec<String>,
}

#[derive(Serialize)]
struct AzureSchedule {
    cron: String,
    #[serde(rename = "displayName")]
    display_name: String,
    branches: BranchFilter,
    always: bool,
}

#[derive(Serialize)]
struct Pool {
    #[serde(rename = "vmImage")]
    vm_image: String,
}

/// A single pipeline step. Either `task:` or `script:` will be set, never both.
/// All fields default to `None` so optional keys are omitted from the YAML output.
#[derive(Serialize, Default)]
struct AzureStep {
    #[serde(skip_serializing_if = "Option::is_none")]
    task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    script: Option<String>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inputs: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<BTreeMap<String, String>>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Renders a `CiPipeline` into an Azure Pipelines YAML string.
///
/// The returned string is suitable for writing to `azure-pipelines.yml`
/// at the repository root.
pub fn render(pipeline: &CiPipeline) -> String {
    let doc = build_pipeline(pipeline);
    serde_yaml::to_string(&doc)
        .expect("AzurePipeline serialisation is infallible for valid CiPipeline")
}

// ── Builder ───────────────────────────────────────────────────────────────────

fn build_pipeline(pipeline: &CiPipeline) -> AzurePipeline {
    let triggers = &pipeline.triggers;
    AzurePipeline {
        trigger: AzureTrigger {
            branches: BranchFilter { include: triggers.push_branches.clone() },
            tags: triggers.tag_pattern.as_ref().map(|p| TagFilter { include: vec![p.clone()] }),
        },
        pr: AzurePr {
            branches: BranchFilter { include: triggers.pr_branches.clone() },
        },
        schedules: triggers.scheduled.as_ref().map(|cron| {
            vec![AzureSchedule {
                cron: cron.clone(),
                display_name: "Scheduled build".to_string(),
                branches: BranchFilter { include: triggers.push_branches.clone() },
                always: true,
            }]
        }),
        pool: Pool { vm_image: "ubuntu-latest".to_string() },
        steps: build_steps(pipeline),
    }
}

fn build_steps(pipeline: &CiPipeline) -> Vec<AzureStep> {
    let mut steps: Vec<AzureStep> = Vec::new();

    // 1. Runtime setup
    match azure_runtime_task(&pipeline.runtime.action) {
        Some((task_name, input_key)) => {
            let mut inputs = BTreeMap::new();
            inputs.insert(input_key.to_string(), pipeline.runtime.version.clone());
            steps.push(AzureStep {
                task: Some(task_name.to_string()),
                display_name: Some("Set up runtime".to_string()),
                inputs: Some(inputs),
                ..Default::default()
            });
        }
        None => {
            // Rust and unknown runtimes — rustup handles toolchain install
            steps.push(AzureStep {
                script: Some(format!("rustup default {}", pipeline.runtime.version)),
                display_name: Some("Set up runtime".to_string()),
                ..Default::default()
            });
        }
    }

    // 2. Cache (optional)
    if let Some(cache) = &pipeline.cache {
        let mut inputs = BTreeMap::new();
        inputs.insert("key".to_string(), gh_cache_key_to_azure(&cache.key));
        inputs.insert("path".to_string(), cache.paths.join("\n"));
        if !cache.restore_keys.is_empty() {
            let azure_restore_keys: Vec<String> =
                cache.restore_keys.iter().map(|k| gh_cache_key_to_azure(k)).collect();
            inputs.insert("restoreKeys".to_string(), azure_restore_keys.join("\n"));
        }
        steps.push(AzureStep {
            task: Some("Cache@2".to_string()),
            display_name: Some("Cache dependencies".to_string()),
            inputs: Some(inputs),
            ..Default::default()
        });
    }

    // 3. Install
    steps.push(AzureStep {
        script: Some(pipeline.install.command.clone()),
        display_name: Some("Install dependencies".to_string()),
        ..Default::default()
    });

    // 4. Lint (optional)
    if let Some(lint) = &pipeline.lint {
        steps.push(AzureStep {
            script: Some(lint.command.clone()),
            display_name: Some("Lint".to_string()),
            ..Default::default()
        });
    }

    // 5. Test
    let test_cmd = match &pipeline.test.coverage_flag {
        Some(flag) => format!("{} {}", pipeline.test.command, flag),
        None => pipeline.test.command.clone(),
    };
    steps.push(AzureStep {
        script: Some(test_cmd),
        display_name: Some("Test".to_string()),
        ..Default::default()
    });

    // 6. Build (optional)
    if let Some(build) = &pipeline.build {
        steps.push(AzureStep {
            script: Some(build.command.clone()),
            display_name: Some("Build".to_string()),
            ..Default::default()
        });
    }

    // 7. Docker (optional) — no QEMU/Buildx tasks in Azure; plain script steps
    if let Some(docker) = &pipeline.docker_build {
        steps.push(AzureStep {
            script: Some(format!("docker build -t {} .", docker.image_tag)),
            display_name: Some("Build Docker image".to_string()),
            ..Default::default()
        });
        if docker.push {
            steps.push(AzureStep {
                script: Some(format!("docker push {}", docker.image_tag)),
                display_name: Some("Push Docker image".to_string()),
                ..Default::default()
            });
        }
    }

    // 8. Image scan (optional) — Trivy installed inline
    if let Some(scan) = &pipeline.image_scan {
        let trivy_script = format!(
            "curl -sfL https://raw.githubusercontent.com/aquasecurity/trivy/main/contrib/install.sh | sh -s -- -b /usr/local/bin\n\
             trivy image --exit-code 1 --severity {} --format {} --output {} {}",
            scan.fail_on_severity, scan.format, scan.output, scan.image_ref,
        );
        steps.push(AzureStep {
            script: Some(trivy_script),
            display_name: Some("Scan image (Trivy)".to_string()),
            ..Default::default()
        });
    }

    // 9. Secret scan (always) — Gitleaks installed inline
    let gitleaks_script =
        "curl -sSfL https://github.com/gitleaks/gitleaks/releases/latest/download/\
         gitleaks_linux_x64.tar.gz | tar xz -C /usr/local/bin\n\
         gitleaks detect --source . --exit-code 1"
            .to_string();
    let mut sec_env = BTreeMap::new();
    // Azure Pipelines variables are accessed via $(VAR_NAME), not ${{ secrets.VAR }}
    sec_env.insert(
        "GITHUB_TOKEN".to_string(),
        "$(GITHUB_TOKEN)".to_string(),
    );
    if let Some(license) = &pipeline.secret_scan.gitleaks_license_secret {
        sec_env.insert(
            "GITLEAKS_LICENSE".to_string(),
            format!("$({})", license),
        );
    }
    steps.push(AzureStep {
        script: Some(gitleaks_script),
        display_name: Some("Secret scan (Gitleaks)".to_string()),
        env: Some(sec_env),
        ..Default::default()
    });

    // 10. Artifact upload (optional)
    if let Some(artifact) = &pipeline.upload_artifact {
        let mut inputs = BTreeMap::new();
        inputs.insert("pathToPublish".to_string(), artifact.path.clone());
        inputs.insert("artifactName".to_string(), artifact.name.clone());
        steps.push(AzureStep {
            task: Some("PublishBuildArtifacts@1".to_string()),
            display_name: Some("Upload artifact".to_string()),
            inputs: Some(inputs),
            ..Default::default()
        });
    }

    steps
}

/// Translates a GitHub Actions cache key expression to the Azure Pipelines
/// `Cache@2` key format.
///
/// Conversions applied:
///   `${{ runner.os }}`          → `$(Agent.OS)`
///   `${{ hashFiles('GLOB') }}`  → `GLOB`   (Azure hashes file content natively)
///   `pm-$(Agent.OS)-glob`       → `pm | $(Agent.OS) | glob`
///
/// `split_once` is used for the separator conversion so that hyphens **inside**
/// file names (e.g. `package-lock.json`) are never corrupted.
fn gh_cache_key_to_azure(key: &str) -> String {
    let key = key.replace("${{ runner.os }}", "$(Agent.OS)");
    let key = strip_hash_files_wrapper(&key);
    // Rebuild as pipe-separated Azure key.  The OS variable is the fixed
    // boundary; everything before it is the PM prefix, everything after is
    // the lock-file glob.
    if let Some((prefix, rest)) = key.split_once("-$(Agent.OS)-") {
        let trimmed = rest.trim_end_matches('-');
        let combined = format!("{prefix} | $(Agent.OS) | {trimmed}");
        return combined.trim_end_matches(|c: char| c == ' ' || c == '|').to_string();
    }
    // Restore key: `pm-$(Agent.OS)` with no trailing glob.
    if let Some((prefix, _)) = key.split_once("-$(Agent.OS)") {
        return format!("{prefix} | $(Agent.OS)");
    }
    key
}

/// Removes `${{ hashFiles('GLOB') }}` wrappers, leaving only the glob(s).
/// Inner single-quotes from multi-argument calls are stripped so the result
/// is a clean comma-separated list compatible with Azure `Cache@2`.
fn strip_hash_files_wrapper(s: &str) -> String {
    let mut result = s.to_string();
    let prefix = "${{ hashFiles('";
    let suffix = "') }}";
    loop {
        match result.find(prefix) {
            None => break,
            Some(start) => {
                let content_start = start + prefix.len();
                match result[content_start..].find(suffix) {
                    None => break,
                    Some(rel_end) => {
                        let content_end = content_start + rel_end;
                        let full_end = content_end + suffix.len();
                        // Strip inner quotes produced by multi-arg hashFiles calls.
                        let glob = result[content_start..content_end].replace('\'', "");
                        result.replace_range(start..full_end, &glob);
                    }
                }
            }
        }
    }
    result
}

/// Maps a GitHub Actions runtime action identifier to the equivalent Azure
/// Pipelines task name and its version-input key. Returns `None` for runtimes
/// that have no native Azure task (e.g. Rust / rust-toolchain).
fn azure_runtime_task(action: &str) -> Option<(&'static str, &'static str)> {
    if action.contains("setup-node") { Some(("NodeTool@0", "versionSpec")) }
    else if action.contains("setup-python") { Some(("UsePythonVersion@0", "versionSpec")) }
    else if action.contains("setup-go") { Some(("GoTool@0", "version")) }
    else if action.contains("setup-java") { Some(("JavaToolInstaller@0", "versionSpec")) }
    else { None }
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
            platform: CiPlatform::Azure,
            format: CiFormat::AzurePipelines,
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
                github_token_expr: "$(System.AccessToken)".to_string(),
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
    fn test_trigger_branches_emitted() {
        let output = render(&make_pipeline());
        assert!(output.contains("trigger"));
        assert!(output.contains("main"));
    }

    #[test]
    fn test_pool_vm_image_ubuntu() {
        let output = render(&make_pipeline());
        assert!(output.contains("ubuntu-latest"));
    }

    #[test]
    fn test_node_tool_task_emitted() {
        let output = render(&make_pipeline());
        assert!(output.contains("NodeTool@0"));
        assert!(output.contains("versionSpec"));
    }

    #[test]
    fn test_rust_toolchain_uses_script_step() {
        let mut p = make_pipeline();
        p.runtime = RuntimeStep {
            action: "dtolnay/rust-toolchain@stable".to_string(),
            version: "stable".to_string(),
        };
        let output = render(&p);
        assert!(!output.contains("NodeTool"));
        assert!(output.contains("rustup default stable"));
    }

    #[test]
    fn test_install_command_emitted() {
        let output = render(&make_pipeline());
        assert!(output.contains("npm ci"));
    }

    #[test]
    fn test_lint_omitted_when_none() {
        let output = render(&make_pipeline());
        // no displayName: Lint entry
        assert!(!output.contains("displayName: Lint"));
    }

    #[test]
    fn test_lint_present_when_some() {
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
    fn test_coverage_flag_appended() {
        let mut p = make_pipeline();
        p.test.coverage_flag = Some("--coverage".to_string());
        let output = render(&p);
        assert!(output.contains("npx jest --coverage"));
    }

    #[test]
    fn test_build_omitted_when_none() {
        let output = render(&make_pipeline());
        assert!(!output.contains("displayName: Build"));
    }

    #[test]
    fn test_build_emitted_when_some() {
        let mut p = make_pipeline();
        p.build = Some(BuildStep { command: "cargo build --release".to_string(), artifact_path: None });
        let output = render(&p);
        assert!(output.contains("cargo build --release"));
    }

    #[test]
    fn test_docker_omitted_when_none() {
        let output = render(&make_pipeline());
        assert!(!output.contains("docker build"));
    }

    #[test]
    fn test_docker_script_emitted() {
        let mut p = make_pipeline();
        p.docker_build = Some(DockerBuildStep {
            image_tag: "myrepo/app:latest".to_string(),
            push: true,
            qemu: false,
            buildx: false,
        });
        let output = render(&p);
        assert!(output.contains("docker build -t myrepo/app:latest ."));
        assert!(output.contains("docker push myrepo/app:latest"));
    }

    #[test]
    fn test_image_scan_omitted_when_none() {
        let output = render(&make_pipeline());
        assert!(!output.contains("trivy"));
    }

    #[test]
    fn test_image_scan_script_emitted() {
        let mut p = make_pipeline();
        p.image_scan = Some(ImageScanStep {
            image_ref: "myrepo/app:latest".to_string(),
            fail_on_severity: "CRITICAL,HIGH".to_string(),
            format: "table".to_string(),
            output: "trivy.txt".to_string(),
            upload_sarif: false,
        });
        let output = render(&p);
        assert!(output.contains("trivy image"));
        assert!(output.contains("myrepo/app:latest"));
    }

    #[test]
    fn test_secret_scan_always_present() {
        let output = render(&make_pipeline());
        assert!(output.contains("gitleaks detect"));
        assert!(output.contains("GITHUB_TOKEN"));
        // Must use Azure variable syntax, not GitHub Actions expression
        assert!(output.contains("$(GITHUB_TOKEN)"));
        assert!(!output.contains("secrets.GITHUB_TOKEN"));
    }

    #[test]
    fn test_artifact_task_emitted() {
        let mut p = make_pipeline();
        p.upload_artifact = Some(ArtifactStep {
            name: "dist".to_string(),
            path: "dist/**".to_string(),
        });
        let output = render(&p);
        assert!(output.contains("PublishBuildArtifacts@1"));
        assert!(output.contains("dist/**"));
    }

    #[test]
    fn test_cache_task_emitted() {
        let mut p = make_pipeline();
        p.cache = Some(CacheStep {
            paths: vec!["~/.npm".to_string()],
            key: "npm-$(Agent.OS)-$(Build.SourceVersion)".to_string(),
            restore_keys: vec!["npm-$(Agent.OS)-".to_string()],
        });
        let output = render(&p);
        assert!(output.contains("Cache@2"));
        assert!(output.contains("~/.npm"));
    }

    #[test]
    fn test_scheduled_trigger_emitted() {
        let mut p = make_pipeline();
        p.triggers.scheduled = Some("0 3 * * 1".to_string());
        let output = render(&p);
        assert!(output.contains("schedules"));
        assert!(output.contains("0 3 * * 1"));
    }

    #[test]
    fn test_tag_pattern_in_trigger() {
        let mut p = make_pipeline();
        p.triggers.tag_pattern = Some("v*".to_string());
        let output = render(&p);
        assert!(output.contains("tags"));
        assert!(output.contains("v*"));
    }

    #[test]
    fn test_gitleaks_license_env_when_some() {
        let mut p = make_pipeline();
        p.secret_scan.gitleaks_license_secret = Some("GITLEAKS_LICENSE".to_string());
        let output = render(&p);
        assert!(output.contains("GITLEAKS_LICENSE"));
    }

    #[test]
    fn test_cache_key_translated_to_azure_syntax() {
        let mut p = make_pipeline();
        p.cache = Some(CacheStep {
            paths: vec!["~/.cargo/registry".to_string()],
            key: "cargo-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}".to_string(),
            restore_keys: vec!["cargo-${{ runner.os }}-".to_string()],
        });
        let output = render(&p);
        // GitHub Actions expressions must be absent
        assert!(!output.contains("runner.os"), "runner.os should be translated");
        assert!(!output.contains("hashFiles"), "hashFiles() should be stripped");
        // Azure syntax must be present
        assert!(output.contains("Agent.OS"));
        assert!(output.contains("Cargo.lock"));
    }

    #[test]
    fn test_gh_cache_key_to_azure_cargo() {
        let input = "cargo-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}";
        let result = gh_cache_key_to_azure(input);
        assert_eq!(result, "cargo | $(Agent.OS) | **/Cargo.lock");
    }

    #[test]
    fn test_gh_cache_key_to_azure_restore_key() {
        let input = "cargo-${{ runner.os }}-";
        let result = gh_cache_key_to_azure(input);
        assert_eq!(result, "cargo | $(Agent.OS)");
    }

    #[test]
    fn test_gh_cache_key_to_azure_npm() {
        let input = "npm-${{ runner.os }}-${{ hashFiles('**/package-lock.json') }}";
        let result = gh_cache_key_to_azure(input);
        assert_eq!(result, "npm | $(Agent.OS) | **/package-lock.json");
    }
}
