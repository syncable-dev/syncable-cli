//! Main linting orchestration for kubelint-rs.
//!
//! This module ties together parsing, checks, and pragmas to provide
//! the main linting API.

use crate::analyzer::kubelint::checks::builtin_checks;
use crate::analyzer::kubelint::config::{CheckSpec, KubelintConfig};
use crate::analyzer::kubelint::context::{LintContext, LintContextImpl};
use crate::analyzer::kubelint::parser::{helm, kustomize, yaml};
use crate::analyzer::kubelint::pragma::should_ignore_check;
use crate::analyzer::kubelint::types::{CheckFailure, Severity};

use std::path::Path;

/// Result of linting Kubernetes manifests.
#[derive(Debug, Clone)]
pub struct LintResult {
    /// Check violations found.
    pub failures: Vec<CheckFailure>,
    /// Parse errors (if any).
    pub parse_errors: Vec<String>,
    /// Summary of the lint run.
    pub summary: LintSummary,
}

/// Summary of a lint run.
#[derive(Debug, Clone)]
pub struct LintSummary {
    /// Number of objects analyzed.
    pub objects_analyzed: usize,
    /// Number of checks run.
    pub checks_run: usize,
    /// Whether the lint passed (no failures above threshold).
    pub passed: bool,
}

impl LintResult {
    /// Create a new empty result.
    pub fn new() -> Self {
        Self {
            failures: Vec::new(),
            parse_errors: Vec::new(),
            summary: LintSummary {
                objects_analyzed: 0,
                checks_run: 0,
                passed: true,
            },
        }
    }

    /// Check if there are any failures.
    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    /// Check if there are any errors (failure with Error severity).
    pub fn has_errors(&self) -> bool {
        self.failures.iter().any(|f| f.severity == Severity::Error)
    }

    /// Check if there are any warnings (failure with Warning severity).
    pub fn has_warnings(&self) -> bool {
        self.failures
            .iter()
            .any(|f| f.severity == Severity::Warning)
    }

    /// Get the maximum severity in the results.
    pub fn max_severity(&self) -> Option<Severity> {
        self.failures.iter().map(|f| f.severity).max()
    }

    /// Check if the results should cause a non-zero exit.
    pub fn should_fail(&self, config: &KubelintConfig) -> bool {
        if config.no_fail {
            return false;
        }

        if let Some(max) = self.max_severity() {
            max >= config.failure_threshold
        } else {
            false
        }
    }

    /// Filter failures by severity threshold.
    pub fn filter_by_threshold(&mut self, threshold: Severity) {
        self.failures.retain(|f| f.severity >= threshold);
    }

    /// Sort failures by file path and line number.
    pub fn sort(&mut self) {
        self.failures.sort();
    }
}

impl Default for LintResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Lint Kubernetes manifests from a path.
///
/// The path can be:
/// - A single YAML file
/// - A directory containing YAML files
/// - A Helm chart directory
/// - A Kustomize directory
pub fn lint(path: &Path, config: &KubelintConfig) -> LintResult {
    let mut result = LintResult::new();

    // Check if path should be ignored
    if config.should_ignore_path(path) {
        return result;
    }

    // Load objects from the path
    let (ctx, warning) = match load_context(path, config) {
        Ok((ctx, warning)) => (ctx, warning),
        Err(err) => {
            result.parse_errors.push(err);
            return result;
        }
    };

    // Add warning as parse error if present (for UI to display)
    if let Some(warn) = warning {
        result.parse_errors.push(warn);
    }

    // Run checks
    result = run_checks(&ctx, config);
    result
}

/// Lint a single YAML file.
pub fn lint_file(path: &Path, config: &KubelintConfig) -> LintResult {
    lint(path, config)
}

/// Lint YAML content directly.
pub fn lint_content(content: &str, config: &KubelintConfig) -> LintResult {
    let mut result = LintResult::new();
    let mut ctx = LintContextImpl::new();

    // Parse the YAML content
    match yaml::parse_yaml(content) {
        Ok(objects) => {
            for obj in objects {
                ctx.add_object(obj);
            }
        }
        Err(err) => {
            result.parse_errors.push(err.to_string());
            return result;
        }
    }

    // Run checks
    run_checks(&ctx, config)
}

/// Load a lint context from a path.
/// Returns (context, optional_warning) - warning is set if fallback was used.
fn load_context(
    path: &Path,
    _config: &KubelintConfig,
) -> Result<(LintContextImpl, Option<String>), String> {
    let mut ctx = LintContextImpl::new();
    let mut warning: Option<String> = None;

    if helm::is_helm_chart(path) {
        // Load as Helm chart - try to render first
        match helm::render_helm_chart(path, None) {
            Ok(objects) => {
                for obj in objects {
                    ctx.add_object(obj);
                }
            }
            Err(err) => {
                // Helm rendering failed - fall back to parsing raw template files
                // This allows linting broken charts that can't be rendered
                let templates_dir = path.join("templates");
                if templates_dir.exists() {
                    warning = Some(format!(
                        "Helm render failed ({}), falling back to raw template parsing",
                        err
                    ));
                    // Parse template files as raw YAML (may contain Go template syntax)
                    match yaml::parse_yaml_dir(&templates_dir) {
                        Ok(objects) => {
                            for obj in objects {
                                ctx.add_object(obj);
                            }
                        }
                        Err(yaml_err) => {
                            // Both Helm render and raw YAML parsing failed
                            return Err(format!(
                                "Failed to render Helm chart: {}. Fallback YAML parsing also failed: {}",
                                err, yaml_err
                            ));
                        }
                    }
                } else {
                    return Err(format!("Failed to render Helm chart: {}", err));
                }
            }
        }
    } else if kustomize::is_kustomize_dir(path) {
        // Load as Kustomize directory
        match kustomize::render_kustomize(path) {
            Ok(objects) => {
                for obj in objects {
                    ctx.add_object(obj);
                }
            }
            Err(err) => return Err(format!("Failed to render Kustomize: {}", err)),
        }
    } else if path.is_dir() {
        // Load directory - but first discover and render any Helm charts or Kustomize dirs
        load_directory_with_rendering(&mut ctx, path)?;
    } else {
        // Load single file
        match yaml::parse_yaml_file(path) {
            Ok(objects) => {
                for obj in objects {
                    ctx.add_object(obj);
                }
            }
            Err(err) => return Err(format!("Failed to parse YAML file: {}", err)),
        }
    }

    Ok((ctx, warning))
}

/// Load a directory, discovering and rendering Helm charts and Kustomize dirs within.
fn load_directory_with_rendering(ctx: &mut LintContextImpl, path: &Path) -> Result<(), String> {
    use std::collections::HashSet;

    let mut processed_dirs: HashSet<std::path::PathBuf> = HashSet::new();

    // First pass: discover Helm charts and Kustomize dirs, render them
    for entry in walkdir::WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            // Check for Helm chart
            if helm::is_helm_chart(entry_path) {
                if let Ok(objects) = helm::render_helm_chart(entry_path, None) {
                    for obj in objects {
                        ctx.add_object(obj);
                    }
                }
                // Mark this directory and all subdirs as processed
                processed_dirs.insert(entry_path.to_path_buf());
                continue;
            }

            // Check for Kustomize dir
            if kustomize::is_kustomize_dir(entry_path) {
                if let Ok(objects) = kustomize::render_kustomize(entry_path) {
                    for obj in objects {
                        ctx.add_object(obj);
                    }
                }
                // Mark this directory and all subdirs as processed
                processed_dirs.insert(entry_path.to_path_buf());
                continue;
            }
        }
    }

    // Second pass: parse regular YAML files not inside Helm/Kustomize dirs
    for entry in walkdir::WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let entry_path = entry.path();
        if entry_path.is_file() {
            // Skip files inside already-processed directories
            let should_skip = processed_dirs
                .iter()
                .any(|processed| entry_path.starts_with(processed));
            if should_skip {
                continue;
            }

            // Check for YAML file
            let ext = entry_path.extension().and_then(|e| e.to_str());
            if matches!(ext, Some("yaml") | Some("yml")) {
                if let Ok(objects) = yaml::parse_yaml_file(entry_path) {
                    for obj in objects {
                        ctx.add_object(obj);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Run all enabled checks on a lint context.
fn run_checks(ctx: &LintContextImpl, config: &KubelintConfig) -> LintResult {
    use crate::analyzer::kubelint::templates;
    use crate::analyzer::kubelint::types::CheckFailure;

    let mut result = LintResult::new();

    // Get all available checks
    let all_checks = builtin_checks();

    // Combine with custom checks
    let mut available_checks: Vec<&CheckSpec> = all_checks.iter().collect();
    for custom in &config.custom_checks {
        available_checks.push(custom);
    }

    // Resolve which checks to run
    let checks_to_run = config.resolve_checks(&all_checks);

    result.summary.objects_analyzed = ctx.objects().len();
    result.summary.checks_run = checks_to_run.len();

    // Cache instantiated check functions
    let mut check_funcs: std::collections::HashMap<String, Box<dyn templates::CheckFunc>> =
        std::collections::HashMap::new();

    // Pre-instantiate all check functions
    for check in &checks_to_run {
        if let Some(template) = templates::get_template(&check.template) {
            match template.instantiate(&check.params) {
                Ok(func) => {
                    check_funcs.insert(check.name.clone(), func);
                }
                Err(e) => {
                    // Log template instantiation error but continue
                    eprintln!(
                        "Warning: Failed to instantiate check '{}': {}",
                        check.name, e
                    );
                }
            }
        }
    }

    // Run each check on each object
    for obj in ctx.objects() {
        for check in &checks_to_run {
            // Check if this check applies to this object kind
            if !check.scope.object_kinds.matches(&obj.kind()) {
                continue;
            }

            // Check if this check is ignored via annotation
            if should_ignore_check(obj, &check.name) {
                continue;
            }

            // Run the check function if we have one
            if let Some(func) = check_funcs.get(&check.name) {
                let diagnostics = func.check(obj);

                // Convert diagnostics to CheckFailures
                for diag in diagnostics {
                    let mut failure = CheckFailure::new(
                        check.name.as_str(),
                        Severity::Warning, // Default severity
                        &diag.message,
                        &obj.metadata.file_path,
                        obj.name(),
                        obj.kind().as_str(),
                    );

                    if let Some(ns) = obj.namespace() {
                        failure = failure.with_namespace(ns);
                    }

                    if let Some(line) = obj.metadata.line_number {
                        failure = failure.with_line(line);
                    }

                    if let Some(remediation) = diag.remediation {
                        failure = failure.with_remediation(remediation);
                    }

                    result.failures.push(failure);
                }
            }
        }
    }

    // Filter by threshold
    result.filter_by_threshold(config.failure_threshold);

    // Sort results
    result.sort();

    // Update summary
    result.summary.passed = !result.should_fail(config);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lint_result_new() {
        let result = LintResult::new();
        assert!(result.failures.is_empty());
        assert!(result.parse_errors.is_empty());
        assert!(result.summary.passed);
    }

    #[test]
    fn test_lint_content_empty() {
        let result = lint_content("", &KubelintConfig::default());
        assert!(result.failures.is_empty());
    }

    #[test]
    fn test_should_fail() {
        let mut result = LintResult::new();
        result.failures.push(CheckFailure::new(
            "test-check",
            Severity::Warning,
            "test message",
            "test.yaml",
            "test-obj",
            "Deployment",
        ));

        let config = KubelintConfig::default().with_threshold(Severity::Warning);
        assert!(result.should_fail(&config));

        let config = KubelintConfig::default().with_threshold(Severity::Error);
        assert!(!result.should_fail(&config));

        let mut no_fail_config = KubelintConfig::default();
        no_fail_config.no_fail = true;
        assert!(!result.should_fail(&no_fail_config));
    }

    #[test]
    fn test_lint_real_file() {
        // Test with actual test file if it exists
        let test_file = std::path::Path::new("test-lint/k8s/insecure-deployment.yaml");
        if !test_file.exists() {
            eprintln!("Test file not found, skipping: {:?}", test_file);
            return;
        }

        // Read and print the file content
        let content = std::fs::read_to_string(test_file).unwrap();
        println!("=== File Content ===\n{}\n", content);

        // Create config with all builtin checks
        let config = KubelintConfig::default().with_all_builtin();
        println!("=== Config ===");
        println!("add_all_builtin: {}", config.add_all_builtin);

        // First test: lint from content
        let result_content = lint_content(&content, &config);
        println!("\n=== Lint Content Result ===");
        println!(
            "Objects analyzed: {}",
            result_content.summary.objects_analyzed
        );
        println!("Checks run: {}", result_content.summary.checks_run);
        println!("Failures: {}", result_content.failures.len());
        for f in &result_content.failures {
            println!("  - {} [{:?}]: {}", f.code, f.severity, f.message);
        }
        for e in &result_content.parse_errors {
            println!("  Parse error: {}", e);
        }

        // Second test: lint from file
        let result_file = lint_file(test_file, &config);
        println!("\n=== Lint File Result ===");
        println!("Objects analyzed: {}", result_file.summary.objects_analyzed);
        println!("Checks run: {}", result_file.summary.checks_run);
        println!("Failures: {}", result_file.failures.len());
        for f in &result_file.failures {
            println!("  - {} [{:?}]: {}", f.code, f.severity, f.message);
        }
        for e in &result_file.parse_errors {
            println!("  Parse error: {}", e);
        }

        // Assert we found issues
        assert!(
            result_content.has_failures() || result_file.has_failures(),
            "Expected to find security issues in the test file!"
        );
    }

    #[test]
    fn test_lint_content_finds_issues() {
        // Test a deployment with multiple security issues
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: insecure-deploy
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:latest
        securityContext:
          privileged: true
"#;
        // Use a config with all built-in checks enabled
        let config = KubelintConfig::default().with_all_builtin();
        let result = lint_content(yaml, &config);

        // Should find issues: privileged container, latest tag, no probes, no resources, etc.
        assert!(
            result.has_failures(),
            "Expected linting failures for insecure deployment"
        );

        // Verify we found the privileged container issue
        let privileged_failures: Vec<_> = result
            .failures
            .iter()
            .filter(|f| f.code.as_str() == "privileged-container")
            .collect();
        assert!(
            !privileged_failures.is_empty(),
            "Should detect privileged container"
        );

        // Verify we found the latest tag issue
        let latest_tag_failures: Vec<_> = result
            .failures
            .iter()
            .filter(|f| f.code.as_str() == "latest-tag")
            .collect();
        assert!(!latest_tag_failures.is_empty(), "Should detect latest tag");
    }

    #[test]
    fn test_lint_content_secure_deployment() {
        // Test a secure deployment
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: secure-deploy
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    spec:
      serviceAccountName: my-service-account
      securityContext:
        runAsNonRoot: true
      containers:
      - name: nginx
        image: nginx:1.21.0
        securityContext:
          privileged: false
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop:
            - ALL
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 200m
            memory: 256Mi
        livenessProbe:
          httpGet:
            path: /healthz
            port: 8080
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
"#;
        // Only include a subset of checks that this deployment should pass
        let config = KubelintConfig::default()
            .include("privileged-container")
            .include("latest-tag");

        let result = lint_content(yaml, &config);

        // Should not find privileged or latest-tag issues
        let critical_failures: Vec<_> = result
            .failures
            .iter()
            .filter(|f| {
                f.code.as_str() == "privileged-container" || f.code.as_str() == "latest-tag"
            })
            .collect();
        assert!(
            critical_failures.is_empty(),
            "Secure deployment should not have privileged/latest-tag failures: {:?}",
            critical_failures
        );
    }

    #[test]
    fn test_lint_content_with_ignore_annotation() {
        // Test that ignore annotations work
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ignored-deploy
  annotations:
    ignore-check.kube-linter.io/privileged-container: "intentionally privileged"
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
        securityContext:
          privileged: true
"#;
        let config = KubelintConfig::default().include("privileged-container");
        let result = lint_content(yaml, &config);

        // Should NOT find privileged container issue due to ignore annotation
        let privileged_failures: Vec<_> = result
            .failures
            .iter()
            .filter(|f| f.code.as_str() == "privileged-container")
            .collect();
        assert!(
            privileged_failures.is_empty(),
            "Ignored check should not produce failures"
        );
    }
}
