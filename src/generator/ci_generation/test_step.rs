//! Test Step Generator — CI-05
//!
//! Maps the detected `TestFramework` to the correct `TestStep` command
//! and optional coverage flags. Unknown or absent framework → placeholder token.

use crate::generator::ci_generation::{context::{CiContext, TestFramework}, schema::TestStep};

/// Generates the test invocation step from the project's detected test framework.
///
/// Every `TestFramework` variant maps to a specific command, optional coverage
/// flag, and optional coverage report path. `None` or `Unknown` → placeholder
/// so the pipeline is still valid YAML that the user can fill in.
pub fn generate_test_step(ctx: &CiContext) -> TestStep {
    match &ctx.test_framework {
        Some(TestFramework::Jest) => TestStep {
            command: "npx jest".to_string(),
            coverage_flag: Some("--coverage".to_string()),
            coverage_report_path: Some("coverage/lcov.info".to_string()),
        },
        Some(TestFramework::Vitest) => TestStep {
            command: "npx vitest run".to_string(),
            coverage_flag: Some("--coverage".to_string()),
            coverage_report_path: Some("coverage/lcov.info".to_string()),
        },
        Some(TestFramework::Mocha) => TestStep {
            command: "npx mocha".to_string(),
            coverage_flag: None,
            coverage_report_path: None,
        },
        Some(TestFramework::Pytest) => TestStep {
            command: "pytest".to_string(),
            coverage_flag: Some("--cov=. --cov-report=xml".to_string()),
            coverage_report_path: Some("coverage.xml".to_string()),
        },
        Some(TestFramework::CargoTest) => TestStep {
            command: "cargo test".to_string(),
            coverage_flag: None,
            coverage_report_path: None,
        },
        Some(TestFramework::GoTest) => TestStep {
            command: "go test ./...".to_string(),
            coverage_flag: Some("-coverprofile=coverage.out".to_string()),
            coverage_report_path: Some("coverage.out".to_string()),
        },
        Some(TestFramework::JunitMaven) => TestStep {
            command: "mvn test".to_string(),
            coverage_flag: None,
            coverage_report_path: Some("target/surefire-reports".to_string()),
        },
        Some(TestFramework::JunitGradle) => TestStep {
            command: "./gradlew test".to_string(),
            coverage_flag: None,
            coverage_report_path: Some("build/reports/tests".to_string()),
        },
        Some(TestFramework::Unknown) | None => TestStep {
            command: "{{TEST_COMMAND}}".to_string(),
            coverage_flag: None,
            coverage_report_path: None,
        },
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::ci_generation::{context::CiContext, test_helpers::make_base_ctx};
    use tempfile::TempDir;

    fn ctx_with_framework(tf: Option<TestFramework>) -> (CiContext, TempDir) {
        let dir = TempDir::new().unwrap();
        let ctx = CiContext { test_framework: tf, ..make_base_ctx(dir.path(), "") };
        (ctx, dir)
    }

    #[test]
    fn test_jest_command_and_coverage() {
        let (ctx, _dir) = ctx_with_framework(Some(TestFramework::Jest));
        let step = generate_test_step(&ctx);
        assert_eq!(step.command, "npx jest");
        assert_eq!(step.coverage_flag.as_deref(), Some("--coverage"));
        assert_eq!(step.coverage_report_path.as_deref(), Some("coverage/lcov.info"));
    }

    #[test]
    fn test_vitest_command_and_coverage() {
        let (ctx, _dir) = ctx_with_framework(Some(TestFramework::Vitest));
        let step = generate_test_step(&ctx);
        assert_eq!(step.command, "npx vitest run");
        assert_eq!(step.coverage_flag.as_deref(), Some("--coverage"));
    }

    #[test]
    fn test_mocha_no_coverage() {
        let (ctx, _dir) = ctx_with_framework(Some(TestFramework::Mocha));
        let step = generate_test_step(&ctx);
        assert_eq!(step.command, "npx mocha");
        assert!(step.coverage_flag.is_none());
    }

    #[test]
    fn test_pytest_coverage_xml() {
        let (ctx, _dir) = ctx_with_framework(Some(TestFramework::Pytest));
        let step = generate_test_step(&ctx);
        assert_eq!(step.command, "pytest");
        assert!(step.coverage_flag.unwrap().contains("--cov"));
        assert_eq!(step.coverage_report_path.as_deref(), Some("coverage.xml"));
    }

    #[test]
    fn test_cargo_test_no_coverage_flag() {
        let (ctx, _dir) = ctx_with_framework(Some(TestFramework::CargoTest));
        let step = generate_test_step(&ctx);
        assert_eq!(step.command, "cargo test");
        assert!(step.coverage_flag.is_none());
    }

    #[test]
    fn test_go_test_coverage_profile() {
        let (ctx, _dir) = ctx_with_framework(Some(TestFramework::GoTest));
        let step = generate_test_step(&ctx);
        assert_eq!(step.command, "go test ./...");
        assert_eq!(step.coverage_flag.as_deref(), Some("-coverprofile=coverage.out"));
    }

    #[test]
    fn test_junit_maven_surefire_report() {
        let (ctx, _dir) = ctx_with_framework(Some(TestFramework::JunitMaven));
        let step = generate_test_step(&ctx);
        assert_eq!(step.command, "mvn test");
        assert!(step.coverage_report_path.as_deref().unwrap().contains("surefire"));
    }

    #[test]
    fn test_junit_gradle_jacoco_report() {
        let (ctx, _dir) = ctx_with_framework(Some(TestFramework::JunitGradle));
        let step = generate_test_step(&ctx);
        assert_eq!(step.command, "./gradlew test");
        assert!(step.coverage_report_path.as_deref().unwrap().contains("build/reports"));
    }

    #[test]
    fn test_unknown_framework_yields_placeholder() {
        let (ctx, _dir) = ctx_with_framework(Some(TestFramework::Unknown));
        let step = generate_test_step(&ctx);
        assert!(step.command.contains("{{TEST_COMMAND}}"));
    }

    #[test]
    fn test_no_framework_yields_placeholder() {
        let (ctx, _dir) = ctx_with_framework(None);
        let step = generate_test_step(&ctx);
        assert!(step.command.contains("{{TEST_COMMAND}}"));
    }
}
