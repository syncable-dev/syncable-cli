//! Build Step Generator — CI-07
//!
//! Determines the build command and artifact output path for the project.
//! Returns `None` when no build step can be inferred (e.g. a library-only
//! project with no binary output).
//!
//! Resolution order:
//!   1. Explicitly detected `build_command` from project scripts (e.g. package.json `build`)
//!   2. Deterministic command inferred from `package_manager` / `primary_language`
//!   3. `{{BUILD_COMMAND}}` placeholder when nothing can be inferred

use crate::generator::ci_generation::{
    context::{CiContext, PackageManager},
    schema::BuildStep,
};

/// Generates the build step, or `None` if the project produces no build artifact.
pub fn generate_build_step(ctx: &CiContext) -> Option<BuildStep> {
    // JS/TS projects: use the detected build script if present; fallback per package manager.
    if matches!(
        ctx.primary_language.to_lowercase().as_str(),
        "javascript" | "typescript" | "js" | "ts"
    ) {
        return Some(js_build_step(ctx));
    }

    let (command, artifact_path) = match ctx.primary_language.to_lowercase().as_str() {
        "rust" => (
            "cargo build --release".to_string(),
            Some("target/release/".to_string()),
        ),
        "go" | "golang" => (
            "go build -o ./bin/app ./...".to_string(),
            Some("bin/".to_string()),
        ),
        "python" => (
            "python -m build".to_string(),
            Some("dist/".to_string()),
        ),
        "java" | "kotlin" => match &ctx.package_manager {
            PackageManager::Gradle => (
                "./gradlew assemble".to_string(),
                Some("build/libs/".to_string()),
            ),
            _ => (
                "mvn package -DskipTests".to_string(),
                Some("target/".to_string()),
            ),
        },
        _ => {
            // Fall back to an explicitly detected build command if we have one.
            let cmd = ctx.build_command.clone().unwrap_or_else(|| "{{BUILD_COMMAND}}".to_string());
            return Some(BuildStep { command: cmd, artifact_path: None });
        }
    };

    Some(BuildStep { command, artifact_path })
}

/// Builds the step for JavaScript/TypeScript projects.
fn js_build_step(ctx: &CiContext) -> BuildStep {
    // Prefer the build script surfaced from package.json scripts.
    if let Some(cmd) = &ctx.build_command {
        return BuildStep {
            command: cmd.clone(),
            artifact_path: Some("dist/".to_string()),
        };
    }

    // Derive `<pm> run build` from the detected package manager.
    let command = match &ctx.package_manager {
        PackageManager::Yarn => "yarn build",
        PackageManager::Pnpm => "pnpm run build",
        PackageManager::Bun => "bun run build",
        _ => "npm run build",
    };

    BuildStep {
        command: command.to_string(),
        artifact_path: Some("dist/".to_string()),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::ci_generation::{context::CiContext, test_helpers::make_base_ctx};
    use tempfile::TempDir;

    fn ctx(language: &str, pm: PackageManager, build_cmd: Option<&str>) -> (CiContext, TempDir) {
        let dir = TempDir::new().unwrap();
        let ctx = CiContext {
            primary_language: language.to_string(),
            package_manager: pm,
            build_command: build_cmd.map(|s| s.to_string()),
            ..make_base_ctx(dir.path(), language)
        };
        (ctx, dir)
    }

    // ── Rust ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_rust_release_build() {
        let (c, _d) = ctx("rust", PackageManager::Cargo, None);
        let step = generate_build_step(&c).expect("should produce step");
        assert_eq!(step.command, "cargo build --release");
        assert_eq!(step.artifact_path.as_deref(), Some("target/release/"));
    }

    // ── Go ────────────────────────────────────────────────────────────────────

    #[test]
    fn test_go_build() {
        let (c, _d) = ctx("go", PackageManager::GoMod, None);
        let step = generate_build_step(&c).expect("should produce step");
        assert_eq!(step.command, "go build -o ./bin/app ./...");
        assert_eq!(step.artifact_path.as_deref(), Some("bin/"));
    }

    // ── Python ────────────────────────────────────────────────────────────────

    #[test]
    fn test_python_wheel_build() {
        let (c, _d) = ctx("python", PackageManager::Poetry, None);
        let step = generate_build_step(&c).expect("should produce step");
        assert_eq!(step.command, "python -m build");
        assert_eq!(step.artifact_path.as_deref(), Some("dist/"));
    }

    // ── Java ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_java_maven_package() {
        let (c, _d) = ctx("java", PackageManager::Maven, None);
        let step = generate_build_step(&c).expect("should produce step");
        assert_eq!(step.command, "mvn package -DskipTests");
        assert_eq!(step.artifact_path.as_deref(), Some("target/"));
    }

    #[test]
    fn test_java_gradle_assemble() {
        let (c, _d) = ctx("java", PackageManager::Gradle, None);
        let step = generate_build_step(&c).expect("should produce step");
        assert_eq!(step.command, "./gradlew assemble");
        assert_eq!(step.artifact_path.as_deref(), Some("build/libs/"));
    }

    // ── JavaScript / TypeScript ───────────────────────────────────────────────

    #[test]
    fn test_js_uses_detected_build_script() {
        let (c, _d) = ctx("javascript", PackageManager::Npm, Some("vite build"));
        let step = generate_build_step(&c).expect("should produce step");
        assert_eq!(step.command, "vite build");
    }

    #[test]
    fn test_js_npm_fallback() {
        let (c, _d) = ctx("javascript", PackageManager::Npm, None);
        let step = generate_build_step(&c).expect("should produce step");
        assert_eq!(step.command, "npm run build");
        assert_eq!(step.artifact_path.as_deref(), Some("dist/"));
    }

    #[test]
    fn test_js_yarn_fallback() {
        let (c, _d) = ctx("javascript", PackageManager::Yarn, None);
        let step = generate_build_step(&c).expect("should produce step");
        assert_eq!(step.command, "yarn build");
    }

    #[test]
    fn test_ts_pnpm_fallback() {
        let (c, _d) = ctx("typescript", PackageManager::Pnpm, None);
        let step = generate_build_step(&c).expect("should produce step");
        assert_eq!(step.command, "pnpm run build");
    }

    // ── Unknown language fallback ─────────────────────────────────────────────

    #[test]
    fn test_unknown_language_with_build_command() {
        let (c, _d) = ctx("elixir", PackageManager::Unknown, Some("mix compile"));
        let step = generate_build_step(&c).expect("should produce step");
        assert_eq!(step.command, "mix compile");
    }

    #[test]
    fn test_unknown_language_no_build_command_yields_placeholder() {
        let (c, _d) = ctx("elixir", PackageManager::Unknown, None);
        let step = generate_build_step(&c).expect("should produce step");
        assert!(step.command.contains("{{BUILD_COMMAND}}"));
    }
}
