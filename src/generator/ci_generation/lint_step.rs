//! Lint Step Generator — CI-06
//!
//! Maps the detected `Linter` to the correct `LintStep` command.
//! Returns `None` when no linter is detected — the lint step is entirely
//! optional in the CI pipeline model.

use crate::generator::ci_generation::{
    context::{CiContext, Linter, PackageManager},
    schema::LintStep,
};

/// Generates the lint invocation step, or `None` if no linter is detected.
pub fn generate_lint_step(ctx: &CiContext) -> Option<LintStep> {
    let command = match &ctx.linter {
        Some(Linter::Eslint) => "npx eslint .",
        Some(Linter::Prettier) => "npx prettier --check .",
        Some(Linter::Pylint) => "pylint src/",
        Some(Linter::Ruff) => "ruff check .",
        Some(Linter::Clippy) => "cargo clippy -- -D warnings",
        Some(Linter::GolangciLint) => "golangci-lint run",
        Some(Linter::Checkstyle) => {
            if matches!(ctx.package_manager, PackageManager::Gradle) {
                "./gradlew checkstyleMain"
            } else {
                "mvn checkstyle:check"
            }
        }
        Some(Linter::Ktlint) => "ktlint",
        Some(Linter::None) | None => return None,
    };

    Some(LintStep { command: command.to_string() })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::ci_generation::{
        context::{CiContext, PackageManager},
        test_helpers::make_base_ctx,
    };
    use tempfile::TempDir;

    fn ctx_with_linter(linter: Option<Linter>) -> (CiContext, TempDir) {
        let dir = TempDir::new().unwrap();
        let ctx = CiContext { linter, ..make_base_ctx(dir.path(), "") };
        (ctx, dir)
    }

    #[test]
    fn test_eslint_command() {
        let (ctx, _dir) = ctx_with_linter(Some(Linter::Eslint));
        let step = generate_lint_step(&ctx).expect("should produce step");
        assert_eq!(step.command, "npx eslint .");
    }

    #[test]
    fn test_prettier_command() {
        let (ctx, _dir) = ctx_with_linter(Some(Linter::Prettier));
        let step = generate_lint_step(&ctx).expect("should produce step");
        assert_eq!(step.command, "npx prettier --check .");
    }

    #[test]
    fn test_pylint_command() {
        let (ctx, _dir) = ctx_with_linter(Some(Linter::Pylint));
        let step = generate_lint_step(&ctx).expect("should produce step");
        assert_eq!(step.command, "pylint src/");
    }

    #[test]
    fn test_ruff_command() {
        let (ctx, _dir) = ctx_with_linter(Some(Linter::Ruff));
        let step = generate_lint_step(&ctx).expect("should produce step");
        assert_eq!(step.command, "ruff check .");
    }

    #[test]
    fn test_clippy_command() {
        let (ctx, _dir) = ctx_with_linter(Some(Linter::Clippy));
        let step = generate_lint_step(&ctx).expect("should produce step");
        assert_eq!(step.command, "cargo clippy -- -D warnings");
    }

    #[test]
    fn test_golangci_lint_command() {
        let (ctx, _dir) = ctx_with_linter(Some(Linter::GolangciLint));
        let step = generate_lint_step(&ctx).expect("should produce step");
        assert_eq!(step.command, "golangci-lint run");
    }

    #[test]
    fn test_checkstyle_maven_command() {
        // make_base_ctx defaults to a non-Gradle package manager
        let (ctx, _dir) = ctx_with_linter(Some(Linter::Checkstyle));
        let step = generate_lint_step(&ctx).expect("should produce step");
        assert_eq!(step.command, "mvn checkstyle:check");
    }

    #[test]
    fn test_checkstyle_gradle_command() {
        let dir = TempDir::new().unwrap();
        let ctx = CiContext {
            linter: Some(Linter::Checkstyle),
            package_manager: PackageManager::Gradle,
            ..make_base_ctx(dir.path(), "")
        };
        let step = generate_lint_step(&ctx).expect("should produce step");
        assert_eq!(step.command, "./gradlew checkstyleMain");
    }

    #[test]
    fn test_ktlint_command() {
        let (ctx, _dir) = ctx_with_linter(Some(Linter::Ktlint));
        let step = generate_lint_step(&ctx).expect("should produce step");
        assert_eq!(step.command, "ktlint");
    }

    #[test]
    fn test_no_linter_returns_none() {
        let (ctx, _dir) = ctx_with_linter(None);
        assert!(generate_lint_step(&ctx).is_none());
    }

    #[test]
    fn test_linter_none_variant_returns_none() {
        let (ctx, _dir) = ctx_with_linter(Some(Linter::None));
        assert!(generate_lint_step(&ctx).is_none());
    }
}
