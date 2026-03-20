//! CI-03 — Runtime version resolver.
//!
//! Maps `CiContext.primary_language` to the correct GitHub Actions setup
//! action and version string, reading version files from disk when needed.

use std::path::Path;

use crate::generator::ci_generation::context::CiContext;

// ── Public types ──────────────────────────────────────────────────────────────

/// Resolved setup step for the project's primary runtime.
#[derive(Debug, Clone)]
pub struct RuntimeSetup {
    /// GitHub Actions action identifier, e.g. `"actions/setup-node@v4"`.
    pub action: &'static str,
    /// Resolved version string, or `"{{RUNTIME_VERSION}}"` when unknown.
    pub version: String,
    /// Token names that could not be resolved and require manual substitution.
    pub unresolved_tokens: Vec<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Resolves the runtime setup step from a `CiContext`.
///
/// Falls back to `{{RUNTIME_VERSION}}` when no version file is found and
/// records the token name in `unresolved_tokens` for downstream warning.
pub fn resolve_runtime(ctx: &CiContext) -> RuntimeSetup {
    let root = &ctx.analysis.project_root;
    let lang = ctx.primary_language.to_lowercase();

    match lang.as_str() {
        "typescript" | "javascript" => resolve_node(root, ctx),
        "python" => resolve_python(root, ctx),
        "go" => resolve_go(root, ctx),
        "rust" => resolve_rust(root),
        "java" | "kotlin" => resolve_java(root, ctx),
        _ => unresolved("RUNTIME_VERSION"),
    }
}

// ── Language resolvers ────────────────────────────────────────────────────────

fn resolve_node(root: &Path, ctx: &CiContext) -> RuntimeSetup {
    // Priority: .nvmrc → .node-version → engines.node in package.json → CiContext
    let version = read_first_line(root, ".nvmrc")
        .or_else(|| read_first_line(root, ".node-version"))
        .or_else(|| extract_engines_node(root))
        .or_else(|| ctx.runtime_versions.get("TypeScript").or_else(|| ctx.runtime_versions.get("JavaScript")).cloned())
        .unwrap_or_else(|| "{{RUNTIME_VERSION}}".to_string());

    make_setup("actions/setup-node@v4", version, "RUNTIME_VERSION")
}

fn resolve_python(root: &Path, ctx: &CiContext) -> RuntimeSetup {
    // Priority: .python-version → pyproject.toml requires-python → Pipfile → CiContext
    let version = read_first_line(root, ".python-version")
        .or_else(|| extract_pyproject_python(root))
        .or_else(|| extract_pipfile_python(root))
        .or_else(|| ctx.runtime_versions.get("Python").cloned())
        .unwrap_or_else(|| "{{RUNTIME_VERSION}}".to_string());

    make_setup("actions/setup-python@v5", version, "RUNTIME_VERSION")
}

fn resolve_go(root: &Path, ctx: &CiContext) -> RuntimeSetup {
    // go.mod `go X.YY` directive → CiContext
    let version = extract_go_mod(root)
        .or_else(|| ctx.runtime_versions.get("Go").cloned())
        .unwrap_or_else(|| "{{RUNTIME_VERSION}}".to_string());

    make_setup("actions/setup-go@v5", version, "RUNTIME_VERSION")
}

fn resolve_rust(root: &Path) -> RuntimeSetup {
    // rust-toolchain.toml `channel` field → rust-toolchain file → "stable"
    let version = extract_rust_toolchain(root).unwrap_or_else(|| "stable".to_string());
    RuntimeSetup {
        action: "dtolnay/rust-toolchain@master",
        version,
        unresolved_tokens: Vec::new(),
    }
}

fn resolve_java(root: &Path, ctx: &CiContext) -> RuntimeSetup {
    // pom.xml <java.version> → build.gradle targetCompatibility → CiContext
    let version = extract_pom_java_version(root)
        .or_else(|| extract_gradle_java_version(root))
        .or_else(|| ctx.runtime_versions.get("Java").or_else(|| ctx.runtime_versions.get("Kotlin")).cloned())
        .unwrap_or_else(|| "{{RUNTIME_VERSION}}".to_string());

    make_setup("actions/setup-java@v4", version, "RUNTIME_VERSION")
}

// ── File extraction helpers ───────────────────────────────────────────────────

/// Reads the first non-empty, non-comment line from a file.
fn read_first_line(root: &Path, file: &str) -> Option<String> {
    let content = std::fs::read_to_string(root.join(file)).ok()?;
    content
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.trim_start_matches('v').to_string())
}

/// Extracts `engines.node` from `package.json` (e.g. `">=18.0.0"` → `"18"`).
fn extract_engines_node(root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(root.join("package.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let raw = json["engines"]["node"].as_str()?.to_string();
    // Strip leading range operators: >=18.0.0 → 18
    let stripped = raw.trim_start_matches(|c: char| !c.is_ascii_digit());
    let major = stripped.split('.').next()?;
    Some(major.to_string())
}

/// Extracts `requires-python` from `pyproject.toml` (e.g. `">=3.11"` → `"3.11"`).
fn extract_pyproject_python(root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(root.join("pyproject.toml")).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("requires-python") {
            let value = line.split('=').nth(1)?.trim().trim_matches('"').trim_matches('\'');
            let stripped = value.trim_start_matches(|c: char| !c.is_ascii_digit());
            return Some(stripped.to_string());
        }
    }
    None
}

/// Extracts `python_requires` from `Pipfile`.
fn extract_pipfile_python(root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(root.join("Pipfile")).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("python_version") || line.starts_with("python_full_version") {
            let value = line.split('=').nth(1)?.trim().trim_matches('"').trim_matches('\'');
            return Some(value.to_string());
        }
    }
    None
}

/// Extracts the `go X.YY` directive from `go.mod`.
fn extract_go_mod(root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(root.join("go.mod")).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("go ") {
            return Some(line[3..].trim().to_string());
        }
    }
    None
}

/// Extracts `channel` from `rust-toolchain.toml`, or reads a bare `rust-toolchain` file.
fn extract_rust_toolchain(root: &Path) -> Option<String> {
    // TOML form
    if let Ok(content) = std::fs::read_to_string(root.join("rust-toolchain.toml")) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("channel") {
                let value = line.split('=').nth(1)?.trim().trim_matches('"').trim_matches('\'');
                return Some(value.to_string());
            }
        }
    }
    // Legacy single-line form
    read_first_line(root, "rust-toolchain")
}

/// Extracts `<java.version>` from `pom.xml`.
fn extract_pom_java_version(root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(root.join("pom.xml")).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("<java.version>") {
            let inner = line
                .trim_start_matches("<java.version>")
                .trim_end_matches("</java.version>");
            return Some(inner.to_string());
        }
    }
    None
}

/// Extracts `targetCompatibility` or `sourceCompatibility` from `build.gradle`.
fn extract_gradle_java_version(root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(root.join("build.gradle"))
        .or_else(|_| std::fs::read_to_string(root.join("build.gradle.kts")))
        .ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("targetCompatibility") || line.starts_with("sourceCompatibility") {
            let value = line
                .split(['=', ' '])
                .last()?
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            return Some(value.to_string());
        }
    }
    None
}

// ── Internal utilities ────────────────────────────────────────────────────────

/// Builds a `RuntimeSetup`, recording a token if the version was not resolved.
fn make_setup(action: &'static str, version: String, token: &str) -> RuntimeSetup {
    let unresolved_tokens = if version.contains("{{") {
        vec![token.to_string()]
    } else {
        Vec::new()
    };
    RuntimeSetup { action, version, unresolved_tokens }
}

/// Returns an unresolved `RuntimeSetup` for unknown languages.
fn unresolved(token: &str) -> RuntimeSetup {
    RuntimeSetup {
        action: "{{SETUP_ACTION}}",
        version: format!("{{{{{token}}}}}"),
        unresolved_tokens: vec![token.to_string(), "SETUP_ACTION".to_string()],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_ctx(lang: &str, root: &Path) -> CiContext {
        use crate::analyzer::{ProjectAnalysis, AnalysisMetadata};
        use crate::generator::ci_generation::context::{PackageManager, CiContext};
        use crate::cli::{CiPlatform, CiFormat};
        use std::collections::HashMap;

        #[allow(deprecated)]
        CiContext {
            analysis: ProjectAnalysis {
                project_root: root.to_path_buf(),
                languages: vec![],
                technologies: vec![],
                frameworks: vec![],
                dependencies: Default::default(),
                entry_points: vec![],
                ports: vec![],
                health_endpoints: vec![],
                environment_variables: vec![],
                project_type: crate::analyzer::ProjectType::Unknown,
                build_scripts: vec![],
                services: vec![],
                architecture_type: crate::analyzer::ArchitectureType::Monolithic,
                docker_analysis: None,
                infrastructure: None,
                analysis_metadata: AnalysisMetadata {
                    timestamp: String::new(),
                    analyzer_version: String::new(),
                    analysis_duration_ms: 0,
                    files_analyzed: 0,
                    confidence_score: 0.0,
                },
            },
            primary_language: lang.to_string(),
            runtime_versions: HashMap::new(),
            package_manager: PackageManager::Unknown,
            lock_file: None,
            test_framework: None,
            linter: None,
            build_command: None,
            has_dockerfile: false,
            monorepo: false,
            monorepo_packages: vec![],
            default_branch: "main".to_string(),
            platform: CiPlatform::Gcp,
            format: CiFormat::GithubActions,
            project_name: "test-project".to_string(),
        }
    }

    #[test]
    fn node_nvmrc() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".nvmrc"), "20.11.0\n").unwrap();
        let ctx = make_ctx("TypeScript", dir.path());
        let setup = resolve_runtime(&ctx);
        assert_eq!(setup.action, "actions/setup-node@v4");
        assert_eq!(setup.version, "20.11.0");
        assert!(setup.unresolved_tokens.is_empty());
    }

    #[test]
    fn node_no_version_file_emits_placeholder() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx("JavaScript", dir.path());
        let setup = resolve_runtime(&ctx);
        assert_eq!(setup.version, "{{RUNTIME_VERSION}}");
        assert!(setup.unresolved_tokens.contains(&"RUNTIME_VERSION".to_string()));
    }

    #[test]
    fn python_python_version_file() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".python-version"), "3.12\n").unwrap();
        let ctx = make_ctx("Python", dir.path());
        let setup = resolve_runtime(&ctx);
        assert_eq!(setup.action, "actions/setup-python@v5");
        assert_eq!(setup.version, "3.12");
        assert!(setup.unresolved_tokens.is_empty());
    }

    #[test]
    fn go_mod_version() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("go.mod"), "module example.com/app\n\ngo 1.22\n").unwrap();
        let ctx = make_ctx("Go", dir.path());
        let setup = resolve_runtime(&ctx);
        assert_eq!(setup.action, "actions/setup-go@v5");
        assert_eq!(setup.version, "1.22");
        assert!(setup.unresolved_tokens.is_empty());
    }

    #[test]
    fn rust_toolchain_toml() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("rust-toolchain.toml"), "[toolchain]\nchannel = \"1.77\"\n").unwrap();
        let ctx = make_ctx("Rust", dir.path());
        let setup = resolve_runtime(&ctx);
        assert_eq!(setup.action, "dtolnay/rust-toolchain@master");
        assert_eq!(setup.version, "1.77");
        assert!(setup.unresolved_tokens.is_empty());
    }

    #[test]
    fn rust_no_toolchain_file_defaults_stable() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx("Rust", dir.path());
        let setup = resolve_runtime(&ctx);
        assert_eq!(setup.version, "stable");
        assert!(setup.unresolved_tokens.is_empty());
    }

    #[test]
    fn java_pom_xml() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("pom.xml"), "<project>\n<properties>\n<java.version>17</java.version>\n</properties>\n</project>").unwrap();
        let ctx = make_ctx("Java", dir.path());
        let setup = resolve_runtime(&ctx);
        assert_eq!(setup.action, "actions/setup-java@v4");
        assert_eq!(setup.version, "17");
        assert!(setup.unresolved_tokens.is_empty());
    }

    #[test]
    fn unknown_language_emits_both_placeholders() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx("Elixir", dir.path());
        let setup = resolve_runtime(&ctx);
        assert!(setup.version.contains("{{"));
        assert!(setup.unresolved_tokens.contains(&"SETUP_ACTION".to_string()));
    }
}
