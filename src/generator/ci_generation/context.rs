//! CI-02 — `CiContext` and `collect_ci_context` entry point.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use std::fmt;

use serde::Serialize;

use crate::analyzer::{analyze_monorepo, analyze_project, ProjectAnalysis, TechnologyCategory};
use crate::cli::{CiFormat, CiPlatform};

// ── Domain enums ─────────────────────────────────────────────────────────────

/// Package manager detected for the primary language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

impl fmt::Display for PackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Npm => "npm",
            Self::Yarn => "yarn",
            Self::Pnpm => "pnpm",
            Self::Bun => "bun",
            Self::Pip => "pip",
            Self::Poetry => "poetry",
            Self::Uv => "uv",
            Self::Cargo => "cargo",
            Self::GoMod => "go mod",
            Self::Maven => "maven",
            Self::Gradle => "gradle",
            Self::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

/// Test framework detected in the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

impl fmt::Display for TestFramework {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Jest => "jest",
            Self::Vitest => "vitest",
            Self::Mocha => "mocha",
            Self::Pytest => "pytest",
            Self::CargoTest => "cargo test",
            Self::GoTest => "go test",
            Self::JunitMaven => "junit (maven)",
            Self::JunitGradle => "junit (gradle)",
            Self::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

/// Linter or formatter detected in the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

impl fmt::Display for Linter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Eslint => "eslint",
            Self::Prettier => "prettier",
            Self::Pylint => "pylint",
            Self::Ruff => "ruff",
            Self::Clippy => "clippy",
            Self::GolangciLint => "golangci-lint",
            Self::Checkstyle => "checkstyle",
            Self::Ktlint => "ktlint",
            Self::None => "",
        };
        write!(f, "{}", s)
    }
}

// ── Primary struct ────────────────────────────────────────────────────────────

/// Enriched snapshot of a project consumed by all CI generators.
#[derive(Debug, Clone, Serialize)]
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
    /// Test command override from `.syncable.ci.toml` (CI-22).
    pub config_test_command: Option<String>,
    /// Env/secret variable name prefix override from config or CLI.
    pub env_prefix: Option<String>,
    /// Step names to skip (from config file).
    pub skip_steps: Vec<String>,
    /// Extra push/PR branches from config file.
    pub extra_branches: Vec<String>,
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

/// Checks for a canonical manifest file directly at `project_root` and returns
/// the language name that should take priority over confidence-score ranking.
///
/// Manifests are tested in priority order so compiled/backend languages always
/// win over a companion `package.json` that lives in a sub-directory but gets
/// scanned by the project analyzer.
fn detect_root_manifest_language(project_root: &Path) -> Option<&'static str> {
    const MANIFESTS: &[(&str, &str)] = &[
        ("Cargo.toml",       "Rust"),
        ("go.mod",           "Go"),
        ("pyproject.toml",   "Python"),
        ("setup.py",         "Python"),
        ("requirements.txt", "Python"),
        ("pom.xml",          "Java"),
        ("build.gradle",     "Java"),
        ("build.gradle.kts", "Kotlin"),
        ("package.json",     "TypeScript"),
    ];
    MANIFESTS.iter().find_map(|(file, lang)| project_root.join(file).exists().then_some(*lang))
}

/// Returns `true` when `tf` is a reasonable test framework for `language`.
/// Used to discard cross-language detections (e.g. Vitest when primary is Rust).
fn test_framework_matches_language(language: &str, tf: &TestFramework) -> bool {
    match language.to_lowercase().as_str() {
        "typescript" | "javascript" => {
            matches!(tf, TestFramework::Jest | TestFramework::Vitest | TestFramework::Mocha)
        }
        "python" => matches!(tf, TestFramework::Pytest),
        "rust" => matches!(tf, TestFramework::CargoTest),
        "go" => matches!(tf, TestFramework::GoTest),
        "java" | "kotlin" => {
            matches!(tf, TestFramework::JunitMaven | TestFramework::JunitGradle)
        }
        _ => true,
    }
}

/// Returns `true` when `linter` is appropriate for `language`.
fn linter_matches_language(language: &str, linter: &Linter) -> bool {
    match language.to_lowercase().as_str() {
        "typescript" | "javascript" => matches!(linter, Linter::Eslint | Linter::Prettier),
        "python" => matches!(linter, Linter::Pylint | Linter::Ruff),
        "rust" => matches!(linter, Linter::Clippy),
        "go" => matches!(linter, Linter::GolangciLint),
        "java" => matches!(linter, Linter::Checkstyle),
        "kotlin" => matches!(linter, Linter::Ktlint),
        _ => true,
    }
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
    // Prefer the language whose manifest lives directly at the project root so
    // a companion sub-project (e.g. a TypeScript IDE extension in a sub-dir)
    // cannot outrank the primary manifest by raw file-count confidence alone.
    let primary_language = detect_root_manifest_language(&analysis.project_root)
        .map(|s| s.to_string())
        .or_else(|| {
            analysis
                .languages
                .iter()
                .max_by(|a, b| {
                    a.confidence
                        .partial_cmp(&b.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|l| l.name.clone())
        })
        .unwrap_or_else(|| "unknown".to_string());

    // ── Runtime versions ──────────────────────────────────────────────────
    let runtime_versions: HashMap<String, String> = analysis
        .languages
        .iter()
        .filter_map(|l| l.version.as_ref().map(|v| (l.name.clone(), v.clone())))
        .collect();

    // ── Package manager ───────────────────────────────────────────────────
    // Look up the package manager from the root language's DetectedLanguage
    // entry so the sub-project's manager does not override the primary one.
    let package_manager = analysis
        .languages
        .iter()
        .find(|l| l.name.to_lowercase() == primary_language.to_lowercase())
        .and_then(|l| l.package_manager.as_deref())
        .map(PackageManager::from)
        .or_else(|| {
            analysis
                .languages
                .iter()
                .max_by(|a, b| {
                    a.confidence
                        .partial_cmp(&b.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .and_then(|l| l.package_manager.as_deref())
                .map(PackageManager::from)
        })
        .unwrap_or(PackageManager::Unknown);

    let lock_file = detect_lock_file(&analysis.project_root, &package_manager);

    // ── Test framework ────────────────────────────────────────────────────
    // Filter to frameworks belonging to the primary language so a Vitest
    // detection in a companion sub-project does not shadow `cargo test`.
    let test_framework = analysis
        .technologies
        .iter()
        .filter(|t| t.category == TechnologyCategory::Testing)
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
        .map(|t| TestFramework::from(t.name.as_str()))
        .filter(|tf| *tf != TestFramework::Unknown)
        .filter(|tf| test_framework_matches_language(&primary_language, tf))
        // cargo test is always available even without an explicit tech entry.
        .or_else(|| {
            if primary_language.to_lowercase() == "rust" {
                Some(TestFramework::CargoTest)
            } else {
                None
            }
        });

    // ── Linter ────────────────────────────────────────────────────────────
    // Apply the same root-language filter so a detected eslint from a companion
    // project does not suppress clippy for a Rust workspace.
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
        .filter(|l| *l != Linter::None)
        .filter(|l| linter_matches_language(&primary_language, l))
        // Clippy is always available for Rust projects.
        .or_else(|| {
            if primary_language.to_lowercase() == "rust" {
                Some(Linter::Clippy)
            } else {
                None
            }
        });

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
        config_test_command: None,
        env_prefix: None,
        skip_steps: vec![],
        extra_branches: vec![],
    })
}
