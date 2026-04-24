//! CI-23 — Code Coverage Upload Step
//!
//! Optional step emitted when the detected test runner produces a coverage
//! report (i.e. `TestStep.coverage_report_path` is `Some(_)`).
//!
//! ## Supported services
//!
//! | Service          | YAML emitted                                 | Secret required  |
//! |------------------|----------------------------------------------|------------------|
//! | `Codecov`        | `codecov/codecov-action@v4`                  | `CODECOV_TOKEN`  |
//! | `InlineSummary`  | `github-script` writing to job summary       | none             |
//!
//! `generate_coverage_step` returns `None` when there is no coverage report
//! path, signalling the template builder to omit the step entirely.
//! When `Codecov` is chosen, `CODECOV_TOKEN` is published as an optional
//! entry in `SECRETS_REQUIRED.md`.

use crate::generator::ci_generation::schema::TestStep;

// ── Public types ──────────────────────────────────────────────────────────────

/// Which coverage reporting service to target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverageService {
    /// Upload to Codecov using `codecov/codecov-action@v4`.
    Codecov,
    /// Write coverage numbers inline to the GitHub Actions job summary —
    /// no external service, no extra secret.
    InlineSummary,
}

/// A resolved coverage upload step, ready for YAML rendering.
#[derive(Debug, Clone)]
pub struct CoverageStep {
    pub service: CoverageService,
    /// Path to the coverage report file (relative to workspace root).
    pub report_path: String,
    /// The secret name that must be configured in the repository when
    /// `service == Codecov`.  Empty for `InlineSummary`.
    pub token_secret: String,
}

impl CoverageStep {
    /// Returns `true` when a repository secret must be configured before the
    /// workflow can succeed.
    pub fn requires_secret(&self) -> bool {
        !self.token_secret.is_empty()
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Generates a coverage step from the detected `TestStep`, choosing the
/// default service (`Codecov`) when a coverage report path is present.
///
/// Returns `None` when `test.coverage_report_path` is `None`, which instructs
/// the template builder to omit the step.
pub fn generate_coverage_step(test: &TestStep) -> Option<CoverageStep> {
    generate_coverage_step_for(test, CoverageService::Codecov)
}

/// Same as `generate_coverage_step` but lets the caller choose the service.
/// Primarily used for testing the `InlineSummary` path.
pub fn generate_coverage_step_for(
    test: &TestStep,
    service: CoverageService,
) -> Option<CoverageStep> {
    let report_path = test.coverage_report_path.as_ref()?.clone();
    let token_secret = match service {
        CoverageService::Codecov => "CODECOV_TOKEN".to_string(),
        CoverageService::InlineSummary => String::new(),
    };
    Some(CoverageStep { service, report_path, token_secret })
}

/// Renders the coverage step as a GitHub Actions YAML step snippet.
pub fn render_coverage_yaml(step: &CoverageStep) -> String {
    match step.service {
        CoverageService::Codecov => format!(
            "\
      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v4
        with:
          files: {}
          fail_ci_if_error: false
        env:
          CODECOV_TOKEN: ${{{{ secrets.CODECOV_TOKEN }}}}\n",
            step.report_path
        ),

        CoverageService::InlineSummary => format!(
            "\
      - name: Coverage summary
        if: always()
        uses: actions/github-script@v7
        with:
          script: |
            const fs = require('fs');
            const report = fs.existsSync('{}')
              ? fs.readFileSync('{}', 'utf8').slice(0, 2000)
              : 'Coverage report not found.';
            await core.summary.addRaw('## Coverage\\n```\\n' + report + '\\n```').write();\n",
            step.report_path, step.report_path
        ),
    }
}

/// Renders the `CODECOV_TOKEN` entry for `SECRETS_REQUIRED.md`.
/// Returns an empty string for `InlineSummary` (no secret needed).
pub fn coverage_secrets_doc_entry(step: &CoverageStep) -> String {
    if step.service != CoverageService::Codecov {
        return String::new();
    }
    "\
### `CODECOV_TOKEN` *(optional)*

Upload coverage reports to Codecov.

**Where to set:** Repository → Settings → Secrets and variables → Actions

**How to obtain:** <https://app.codecov.io> → your repo → Settings → Repository Token\n"
        .to_string()
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::ci_generation::schema::TestStep;

    fn test_step_with_coverage(path: &str) -> TestStep {
        TestStep {
            command: "cargo test".into(),
            coverage_flag: Some("--coverage".into()),
            coverage_report_path: Some(path.to_string()),
        }
    }

    fn test_step_no_coverage() -> TestStep {
        TestStep {
            command: "cargo test".into(),
            coverage_flag: None,
            coverage_report_path: None,
        }
    }

    // ── generate_coverage_step ─────────────────────────────────────────

    #[test]
    fn test_returns_none_without_coverage_path() {
        assert!(generate_coverage_step(&test_step_no_coverage()).is_none());
    }

    #[test]
    fn test_returns_some_with_coverage_path() {
        let step = generate_coverage_step(&test_step_with_coverage("coverage.xml"));
        assert!(step.is_some());
    }

    #[test]
    fn test_defaults_to_codecov_service() {
        let step = generate_coverage_step(&test_step_with_coverage("coverage.xml")).unwrap();
        assert_eq!(step.service, CoverageService::Codecov);
    }

    #[test]
    fn test_codecov_requires_secret() {
        let step = generate_coverage_step(&test_step_with_coverage("lcov.info")).unwrap();
        assert!(step.requires_secret());
        assert_eq!(step.token_secret, "CODECOV_TOKEN");
    }

    #[test]
    fn test_inline_summary_does_not_require_secret() {
        let step = generate_coverage_step_for(
            &test_step_with_coverage("lcov.info"),
            CoverageService::InlineSummary,
        )
        .unwrap();
        assert!(!step.requires_secret());
        assert!(step.token_secret.is_empty());
    }

    #[test]
    fn test_report_path_preserved() {
        let step =
            generate_coverage_step(&test_step_with_coverage("target/coverage/lcov.info")).unwrap();
        assert_eq!(step.report_path, "target/coverage/lcov.info");
    }

    // ── render_coverage_yaml ───────────────────────────────────────────

    #[test]
    fn test_codecov_yaml_contains_action() {
        let step = generate_coverage_step(&test_step_with_coverage("coverage.xml")).unwrap();
        let yaml = render_coverage_yaml(&step);
        assert!(yaml.contains("codecov/codecov-action@v4"));
    }

    #[test]
    fn test_codecov_yaml_contains_report_path() {
        let step = generate_coverage_step(&test_step_with_coverage("coverage.xml")).unwrap();
        let yaml = render_coverage_yaml(&step);
        assert!(yaml.contains("coverage.xml"));
    }

    #[test]
    fn test_codecov_yaml_contains_secret_ref() {
        let step = generate_coverage_step(&test_step_with_coverage("coverage.xml")).unwrap();
        let yaml = render_coverage_yaml(&step);
        assert!(yaml.contains("CODECOV_TOKEN"));
    }

    #[test]
    fn test_inline_summary_yaml_uses_github_script() {
        let step = generate_coverage_step_for(
            &test_step_with_coverage("lcov.info"),
            CoverageService::InlineSummary,
        )
        .unwrap();
        let yaml = render_coverage_yaml(&step);
        assert!(yaml.contains("github-script"));
        assert!(yaml.contains("lcov.info"));
        assert!(!yaml.contains("CODECOV_TOKEN"));
    }

    // ── coverage_secrets_doc_entry ─────────────────────────────────────

    #[test]
    fn test_secrets_doc_entry_for_codecov() {
        let step = generate_coverage_step(&test_step_with_coverage("coverage.xml")).unwrap();
        let entry = coverage_secrets_doc_entry(&step);
        assert!(entry.contains("CODECOV_TOKEN"));
        assert!(entry.contains("optional"));
    }

    #[test]
    fn test_secrets_doc_entry_empty_for_inline() {
        let step = generate_coverage_step_for(
            &test_step_with_coverage("lcov.info"),
            CoverageService::InlineSummary,
        )
        .unwrap();
        let entry = coverage_secrets_doc_entry(&step);
        assert!(entry.is_empty());
    }
}
