//! CI Pipeline Schema — CI-14
//!
//! Defines the canonical, platform-agnostic `CiPipeline` intermediate
//! representation. All template builders render from this struct, not
//! directly from `CiContext`. This decouples context collection from
//! output formatting and allows future agent patching of individual steps.

use serde::Serialize;

use crate::cli::{CiFormat, CiPlatform};

// ── Unresolved token ──────────────────────────────────────────────────────────

/// A placeholder that could not be filled deterministically from project files.
///
/// Serialised into `ci-manifest.toml [unresolved]` so the agent fill phase
/// and interactive prompts know exactly what still needs a human decision.
#[derive(Debug, Clone, Serialize)]
pub struct UnresolvedToken {
    /// Token name as it appears in the YAML output, e.g. `"REGISTRY_URL"`.
    pub name: String,
    /// The `{{TOKEN_NAME}}` string injected into the generated YAML.
    pub placeholder: String,
    /// Human-readable hint for what value to supply.
    pub hint: String,
    /// Type annotation used in the manifest file (e.g. `"string"`, `"url"`).
    pub token_type: String,
}

impl UnresolvedToken {
    pub fn new(name: &str, hint: &str, token_type: &str) -> Self {
        Self {
            name: name.to_string(),
            placeholder: format!("{{{{{}}}}}", name),
            hint: hint.to_string(),
            token_type: token_type.to_string(),
        }
    }
}

// ── Step structs ──────────────────────────────────────────────────────────────

/// Trigger events that start the CI workflow.
#[derive(Debug, Clone, Serialize)]
pub struct TriggerConfig {
    /// Branches that trigger the workflow on push.
    pub push_branches: Vec<String>,
    /// Branches that trigger the workflow on pull request.
    pub pr_branches: Vec<String>,
    /// Optional tag pattern (e.g. `"v*"`) for release triggers.
    pub tag_pattern: Option<String>,
    /// Optional cron schedule expression.
    pub scheduled: Option<String>,
}

/// Runtime / toolchain setup step.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeStep {
    /// GitHub Actions action identifier, e.g. `"actions/setup-node@v4"`.
    pub action: String,
    /// Resolved version string or `{{RUNTIME_VERSION}}` placeholder.
    pub version: String,
}

/// Dependency cache step (`actions/cache`).
#[derive(Debug, Clone, Serialize)]
pub struct CacheStep {
    pub paths: Vec<String>,
    pub key: String,
    pub restore_keys: Vec<String>,
}

/// Package install step.
#[derive(Debug, Clone, Serialize)]
pub struct InstallStep {
    /// Shell command to install dependencies, e.g. `"npm ci"`.
    pub command: String,
}

/// Lint step — omitted entirely when no linter is detected.
#[derive(Debug, Clone, Serialize)]
pub struct LintStep {
    pub command: String,
}

/// Test step with optional coverage output.
#[derive(Debug, Clone, Serialize)]
pub struct TestStep {
    /// Primary test invocation command.
    pub command: String,
    /// Optional coverage flag appended to the test command.
    pub coverage_flag: Option<String>,
    /// Relative path to the coverage report file, if produced.
    pub coverage_report_path: Option<String>,
}

/// Build / compile step.
#[derive(Debug, Clone, Serialize)]
pub struct BuildStep {
    pub command: String,
    /// Relative path to the build output used by the artifact upload step.
    pub artifact_path: Option<String>,
}

/// Docker build and optional push step.
#[derive(Debug, Clone, Serialize)]
pub struct DockerBuildStep {
    /// Full image reference including tag, e.g. `"ghcr.io/org/app:${{ github.sha }}"`.
    pub image_tag: String,
    /// Whether to push the image as part of the CI job.
    pub push: bool,
    /// Enable multi-platform QEMU cross-compilation via `docker/setup-qemu-action`.
    pub qemu: bool,
    /// Whether to set up a multi-platform Buildx builder via `docker/setup-buildx-action`.
    pub buildx: bool,
}

/// Container image security scan step (Trivy via `aquasecurity/trivy-action`).
#[derive(Debug, Clone, Serialize)]
pub struct ImageScanStep {
    /// Image reference to scan — typically matches `DockerBuildStep.image_tag`.
    pub image_ref: String,
    /// Comma-separated severity levels that trigger a non-zero exit, e.g. `"CRITICAL,HIGH"`.
    pub fail_on_severity: String,
    /// Output format for the scan report (`"sarif"`, `"table"`, etc.).
    pub format: String,
    /// Output file path for the scan report, e.g. `"trivy-results.sarif"`.
    pub output: String,
    /// Whether to upload the SARIF report to the GitHub Security tab.
    pub upload_sarif: bool,
}

/// Secret / credential leak scan step (Gitleaks via `gitleaks/gitleaks-action@v2`) — always emitted.
#[derive(Debug, Clone, Serialize)]
pub struct SecretScanStep {
    /// `${{ secrets.GITHUB_TOKEN }}` — always available in Actions, never a placeholder.
    pub github_token_expr: String,
    /// Repository secret name for the Gitleaks licence key.
    /// `None` for open-source repos (no licence required).
    /// `Some("GITLEAKS_LICENSE")` when a private-repo licence is detected or requested.
    pub gitleaks_license_secret: Option<String>,
}

/// Artifact upload step.
#[derive(Debug, Clone, Serialize)]
pub struct ArtifactStep {
    /// Display name for the artifact in the GitHub Actions UI.
    pub name: String,
    /// Path glob for files to upload, e.g. `"dist/**"`.
    pub path: String,
}

// ── Top-level pipeline ────────────────────────────────────────────────────────

/// Platform-agnostic intermediate representation of a complete CI pipeline.
///
/// Template builders (CI-11, CI-12, CI-13) render YAML from this struct.
/// The agent fill phase patches individual fields without re-running full
/// context collection.
#[derive(Debug, Clone, Serialize)]
pub struct CiPipeline {
    pub project_name: String,
    pub platform: CiPlatform,
    pub format: CiFormat,
    pub triggers: TriggerConfig,
    pub runtime: RuntimeStep,
    pub cache: Option<CacheStep>,
    pub install: InstallStep,
    pub lint: Option<LintStep>,
    pub test: TestStep,
    pub build: Option<BuildStep>,
    pub docker_build: Option<DockerBuildStep>,
    pub image_scan: Option<ImageScanStep>,
    pub secret_scan: SecretScanStep,
    pub upload_artifact: Option<ArtifactStep>,
    /// Tokens that could not be resolved deterministically.
    pub unresolved_tokens: Vec<UnresolvedToken>,
}

