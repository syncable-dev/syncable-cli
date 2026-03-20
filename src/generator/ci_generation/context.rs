//! CI-02 — `CiContext` and `collect_ci_context` entry point.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::analyzer::{analyze_monorepo, analyze_project, ProjectAnalysis, TechnologyCategory};
use crate::cli::{CiFormat, CiPlatform};

// ── Domain enums ─────────────────────────────────────────────────────────────

/// Package manager detected for the primary language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
    Pip,
    Poetry,
    Uv,
    Cargo,
    GoMod,
    Maven,
    Gradle,
    Unknown,
}

impl From<&str> for PackageManager {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "npm" => Self::Npm,
            "yarn" => Self::Yarn,
            "pnpm" => Self::Pnpm,
            "bun" => Self::Bun,
            "pip" => Self::Pip,
            "poetry" => Self::Poetry,
            "uv" => Self::Uv,
            "cargo" => Self::Cargo,
            "go mod" | "gomod" | "go" => Self::GoMod,
            "maven" | "mvn" => Self::Maven,
            "gradle" => Self::Gradle,
            _ => Self::Unknown,
        }
    }
}

/// Test framework detected in the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestFramework {
    Jest,
    Vitest,
    Mocha,
    Pytest,
    CargoTest,
    GoTest,
    JunitMaven,
    JunitGradle,
    Unknown,
}

impl From<&str> for TestFramework {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "jest" => Self::Jest,
            "vitest" => Self::Vitest,
            "mocha" => Self::Mocha,
            "pytest" => Self::Pytest,
            "cargo test" | "cargo-test" | "cargotest" => Self::CargoTest,
            "go test" | "gotest" => Self::GoTest,
            "junit" | "junit-maven" | "junit (maven)" => Self::JunitMaven,
            "junit-gradle" | "junit (gradle)" => Self::JunitGradle,
            _ => Self::Unknown,
        }
    }
}

/// Linter or formatter detected in the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Linter {
    Eslint,
    Prettier,
    Pylint,
    Ruff,
    Clippy,
    GolangciLint,
    Checkstyle,
    Ktlint,
    None,
}

impl From<&str> for Linter {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "eslint" => Self::Eslint,
            "prettier" => Self::Prettier,
            "pylint" => Self::Pylint,
            "ruff" => Self::Ruff,
            "clippy" | "cargo clippy" => Self::Clippy,
            "golangci-lint" | "golangci_lint" | "golangci lint" => Self::GolangciLint,
            "checkstyle" => Self::Checkstyle,
            "ktlint" => Self::Ktlint,
            _ => Self::None,
        }
    }
}

// ── Primary struct ────────────────────────────────────────────────────────────

/// Enriched snapshot of a project consumed by all CI generators.
#[derive(Debug, Clone)]
pub struct CiContext {
    /// Raw analyzer output; available to generators that need fields beyond what CiContext promotes.
    pub analysis: ProjectAnalysis,
    pub primary_language: String,
    /// language name → version string
    pub runtime_versions: HashMap<String, String>,
    pub package_manager: PackageManager,
    /// Absolute path to the detected lock file, if present.
    pub lock_file: Option<PathBuf>,
    pub test_framework: Option<TestFramework>,
    pub linter: Option<Linter>,
    /// Command from the default `BuildScript`, if any.
    pub build_command: Option<String>,
    pub has_dockerfile: bool,
    pub monorepo: bool,
    /// Sub-package directory names; empty for single-project repos.
    pub monorepo_packages: Vec<String>,
    pub default_branch: String,
    pub platform: CiPlatform,
    pub format: CiFormat,
    pub project_name: String,
}

// ── Helper functions ──────────────────────────────────────────────────────────

/// Returns the upstream default branch via `git symbolic-ref`; falls back to `"main"`.
fn detect_default_branch(path: &Path) -> String {
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .current_dir(path)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let raw = String::from_utf8_lossy(&out.stdout);
            raw.trim()
                .rsplit('/')
                .next()
                .unwrap_or("main")
                .to_string()
        }
        _ => "main".to_string(),
    }
}

/// Returns the first matching lock file path for the given package manager.
fn detect_lock_file(project_root: &Path, pm: &PackageManager) -> Option<PathBuf> {
    let candidates: &[&str] = match pm {
        PackageManager::Npm => &["package-lock.json"],
        PackageManager::Yarn => &["yarn.lock"],
        PackageManager::Pnpm => &["pnpm-lock.yaml"],
        PackageManager::Bun => &["bun.lockb", "bun.lock"],
        PackageManager::Pip => &["requirements.txt", "requirements-lock.txt"],
        PackageManager::Poetry => &["poetry.lock"],
        PackageManager::Uv => &["uv.lock"],
        PackageManager::Cargo => &["Cargo.lock"],
        PackageManager::GoMod => &["go.sum"],
        PackageManager::Maven => &[],
        PackageManager::Gradle => &[],
        PackageManager::Unknown => &[],
    };

    candidates.iter().find_map(|name| {
        let p = project_root.join(name);
        p.exists().then_some(p)
    })
}

/// Returns the project root's directory name as the project identifier.
fn detect_project_name(analysis: &ProjectAnalysis) -> String {
    analysis
        .project_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".to_string())
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Runs the project analyzer and assembles a `CiContext` for the given path.
pub fn collect_ci_context(
    path: &Path,
    platform: CiPlatform,
    format: CiFormat,
) -> crate::Result<CiContext> {
    let analysis = analyze_project(path)?;

    // ── Primary language ──────────────────────────────────────────────────
    let primary_language = analysis
        .languages
        .iter()
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
        .map(|l| l.name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // ── Runtime versions ──────────────────────────────────────────────────
    let runtime_versions: HashMap<String, String> = analysis
        .languages
        .iter()
        .filter_map(|l| l.version.as_ref().map(|v| (l.name.clone(), v.clone())))
        .collect();

    // ── Package manager ───────────────────────────────────────────────────
    let package_manager = analysis
        .languages
        .iter()
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
        .and_then(|l| l.package_manager.as_deref())
        .map(PackageManager::from)
        .unwrap_or(PackageManager::Unknown);

    let lock_file = detect_lock_file(&analysis.project_root, &package_manager);

    // ── Test framework ────────────────────────────────────────────────────
    let test_framework = analysis
        .technologies
        .iter()
        .filter(|t| t.category == TechnologyCategory::Testing)
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
        .map(|t| TestFramework::from(t.name.as_str()))
        .filter(|tf| *tf != TestFramework::Unknown);

    // ── Linter ────────────────────────────────────────────────────────────
    let linter_tech = analysis.technologies.iter().find(|t| {
        matches!(
            t.name.to_lowercase().as_str(),
            "eslint"
                | "prettier"
                | "pylint"
                | "ruff"
                | "clippy"
                | "golangci-lint"
                | "checkstyle"
                | "ktlint"
        )
    });
    let linter = linter_tech
        .map(|t| Linter::from(t.name.as_str()))
        .filter(|l| *l != Linter::None);

    // ── Build command ─────────────────────────────────────────────────────
    let build_command = analysis
        .build_scripts
        .iter()
        .find(|s| s.is_default)
        .map(|s| s.command.clone());

    // ── Dockerfile ────────────────────────────────────────────────────────
    let has_dockerfile = analysis.docker_analysis.is_some();

    // ── Monorepo ──────────────────────────────────────────────────────────
    let mono = analyze_monorepo(path)?;
    let monorepo = mono.is_monorepo;
    let monorepo_packages = if monorepo {
        mono.projects
            .iter()
            .filter_map(|p| {
                p.analysis
                    .project_root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .collect()
    } else {
        Vec::new()
    };

    // ── Git default branch ────────────────────────────────────────────────
    let default_branch = detect_default_branch(path);

    // ── Project name ──────────────────────────────────────────────────────
    let project_name = detect_project_name(&analysis);

    Ok(CiContext {
        analysis,
        primary_language,
        runtime_versions,
        package_manager,
        lock_file,
        test_framework,
        linter,
        build_command,
        has_dockerfile,
        monorepo,
        monorepo_packages,
        default_branch,
        platform,
        format,
        project_name,
    })
}
