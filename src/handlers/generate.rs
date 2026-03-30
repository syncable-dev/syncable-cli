use crate::{analyzer::analyze_monorepo, generator};

pub fn handle_generate(
    path: std::path::PathBuf,
    _output: Option<std::path::PathBuf>,
    dockerfile: bool,
    compose: bool,
    terraform: bool,
    all: bool,
    dry_run: bool,
    _force: bool,
) -> crate::Result<()> {
    println!("🔍 Analyzing project for generation: {}", path.display());

    let monorepo_analysis = analyze_monorepo(&path)?;

    println!("✅ Analysis complete. Generating IaC files...");

    if monorepo_analysis.is_monorepo {
        println!(
            "📦 Detected monorepo with {} projects",
            monorepo_analysis.projects.len()
        );
        println!(
            "🚧 Monorepo IaC generation is coming soon! For now, generating for the overall structure."
        );
        println!(
            "💡 Tip: You can run generate commands on individual project directories for now."
        );
    }

    // For now, use the first/main project for generation
    // TODO: Implement proper monorepo IaC generation
    let main_project = &monorepo_analysis.projects[0];

    let generate_all = all || (!dockerfile && !compose && !terraform);

    if generate_all || dockerfile {
        println!("\n🐳 Generating Dockerfile...");
        let dockerfile_content = generator::generate_dockerfile(&main_project.analysis)?;

        if dry_run {
            println!("--- Dockerfile (dry run) ---");
            println!("{}", dockerfile_content);
        } else {
            std::fs::write("Dockerfile", dockerfile_content)?;
            println!("✅ Dockerfile generated successfully!");
        }
    }

    if generate_all || compose {
        println!("\n🐙 Generating Docker Compose file...");
        let compose_content = generator::generate_compose(&main_project.analysis)?;

        if dry_run {
            println!("--- docker-compose.yml (dry run) ---");
            println!("{}", compose_content);
        } else {
            std::fs::write("docker-compose.yml", compose_content)?;
            println!("✅ Docker Compose file generated successfully!");
        }
    }

    if generate_all || terraform {
        println!("\n🏗️  Generating Terraform configuration...");
        let terraform_content = generator::generate_terraform(&main_project.analysis)?;

        if dry_run {
            println!("--- main.tf (dry run) ---");
            println!("{}", terraform_content);
        } else {
            std::fs::write("main.tf", terraform_content)?;
            println!("✅ Terraform configuration generated successfully!");
        }
    }

    if !dry_run {
        println!("\n🎉 Generation complete! IaC files have been created in the current directory.");

        if monorepo_analysis.is_monorepo {
            println!("🔧 Note: Generated files are based on the main project structure.");
            println!("   Advanced monorepo support with per-project generation is coming soon!");
        }
    }

    Ok(())
}

pub fn handle_validate(
    path: std::path::PathBuf,
    types: Option<Vec<String>>,
    fix: bool,
    quiet: bool,
) -> crate::Result<String> {
    use crate::analyzer::{dclint, hadolint, helmlint, kubelint};
    use std::path::Path;

    let project_path = path.canonicalize().unwrap_or_else(|_| path.clone());

    if !quiet {
        println!("🔍 Validating IaC files in: {}", project_path.display());
    }

    let type_filter: Option<Vec<String>> = types.map(|t| {
        t.iter()
            .flat_map(|s| s.split(','))
            .map(|s| s.trim().to_lowercase())
            .collect()
    });
    let check_all = type_filter.is_none();
    let should_check = |name: &str| {
        check_all
            || type_filter
                .as_ref()
                .map_or(false, |f| f.iter().any(|t| t == name))
    };

    let mut all_results: Vec<serde_json::Value> = Vec::new();
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;
    let mut total_info = 0usize;
    let mut files_checked = 0usize;

    // --- Dockerfiles (hadolint) ---
    if should_check("dockerfile") {
        let dockerfiles = find_dockerfiles(&project_path);
        if !dockerfiles.is_empty() {
            if !quiet {
                println!("\n🐳 Checking {} Dockerfile(s)...", dockerfiles.len());
            }
            let config = hadolint::HadolintConfig::default();
            for df in &dockerfiles {
                let result = hadolint::lint_file(df, &config);
                files_checked += 1;
                let rel = df.strip_prefix(&project_path).unwrap_or(df);
                let (e, w, i) = count_severities_hadolint(&result);
                total_errors += e;
                total_warnings += w;
                total_info += i;
                if !quiet && result.has_failures() {
                    println!("  {} — {} error(s), {} warning(s)", rel.display(), e, w);
                }
                for f in &result.failures {
                    all_results.push(serde_json::json!({
                        "type": "dockerfile",
                        "file": rel.display().to_string(),
                        "line": f.line,
                        "code": f.code.to_string(),
                        "severity": format!("{:?}", f.severity),
                        "message": f.message,
                    }));
                }
            }
        }
    }

    // --- Docker Compose (dclint) ---
    if should_check("compose") {
        let compose_files = find_compose_files(&project_path);
        if !compose_files.is_empty() {
            if !quiet {
                println!("\n🐙 Checking {} Compose file(s)...", compose_files.len());
            }
            let config = dclint::DclintConfig::default();
            for cf in &compose_files {
                let result = dclint::lint_file(cf, &config);
                files_checked += 1;
                let rel = cf.strip_prefix(&project_path).unwrap_or(cf);
                let (e, w, i) = count_severities_dclint(&result);
                total_errors += e;
                total_warnings += w;
                total_info += i;
                if !quiet && result.has_failures() {
                    println!("  {} — {} error(s), {} warning(s)", rel.display(), e, w);
                }
                for f in &result.failures {
                    all_results.push(serde_json::json!({
                        "type": "compose",
                        "file": rel.display().to_string(),
                        "line": f.line,
                        "code": f.code.to_string(),
                        "severity": format!("{:?}", f.severity),
                        "message": f.message,
                    }));
                }

                // Auto-fix if requested
                if fix {
                    if let Ok(Some(fixed)) = dclint::fix_file(cf, &config, false) {
                        if !quiet {
                            println!("    ✅ Auto-fixed {}", rel.display());
                        }
                        let _ = fixed; // fix_file already writes when dry_run=false
                    }
                }
            }
        }
    }

    // --- Kubernetes manifests (kubelint) ---
    if should_check("kubernetes") || should_check("k8s") {
        let k8s_dirs = find_k8s_dirs(&project_path);
        if !k8s_dirs.is_empty() {
            if !quiet {
                println!(
                    "\n☸️  Checking {} K8s manifest location(s)...",
                    k8s_dirs.len()
                );
            }
            let config = kubelint::KubelintConfig::default();
            for dir in &k8s_dirs {
                let result = kubelint::lint(dir, &config);
                let rel = dir.strip_prefix(&project_path).unwrap_or(dir);
                files_checked += result.summary.objects_analyzed;
                let (e, w, i) = count_severities_kubelint(&result);
                total_errors += e;
                total_warnings += w;
                total_info += i;
                if !quiet && result.has_failures() {
                    println!("  {} — {} error(s), {} warning(s)", rel.display(), e, w);
                }
                for f in &result.failures {
                    all_results.push(serde_json::json!({
                        "type": "kubernetes",
                        "object": format!("{}/{}", f.object_kind, f.object_name),
                        "file": f.file_path.display().to_string(),
                        "code": f.code.to_string(),
                        "severity": format!("{:?}", f.severity),
                        "message": f.message,
                        "remediation": f.remediation,
                    }));
                }
            }
        }
    }

    // --- Helm charts (helmlint) ---
    if should_check("helm") {
        let helm_charts = find_helm_charts_validate(&project_path);
        if !helm_charts.is_empty() {
            if !quiet {
                println!("\n⎈ Checking {} Helm chart(s)...", helm_charts.len());
            }
            let config = helmlint::HelmlintConfig::default();
            for chart in &helm_charts {
                let result = helmlint::lint_chart(chart, &config);
                files_checked += result.files_checked;
                let rel = chart.strip_prefix(&project_path).unwrap_or(chart);
                let (e, w, i) = count_severities_helmlint(&result);
                total_errors += e;
                total_warnings += w;
                total_info += i;
                if !quiet && result.has_failures() {
                    println!("  {} — {} error(s), {} warning(s)", rel.display(), e, w);
                }
                for f in &result.failures {
                    all_results.push(serde_json::json!({
                        "type": "helm",
                        "file": f.file.display().to_string(),
                        "line": f.line,
                        "code": f.code.to_string(),
                        "severity": format!("{:?}", f.severity),
                        "message": f.message,
                    }));
                }
            }
        }
    }

    if files_checked == 0 {
        if !quiet {
            println!("\n⚠️  No IaC files found to validate.");
        }
        let output = serde_json::json!({
            "status": "NO_FILES",
            "message": "No IaC files found. Use sync-ctl analyze to check what IaC exists.",
            "files_checked": 0,
            "violations": []
        });
        return Ok(serde_json::to_string_pretty(&output)?);
    }

    // Summary
    if !quiet {
        println!("\n{}", "─".repeat(60));
        println!(
            "📊 {} file(s) checked — {} error(s), {} warning(s), {} info",
            files_checked, total_errors, total_warnings, total_info
        );
        if total_errors == 0 && total_warnings == 0 {
            println!("✅ All checks passed!");
        }
    }

    let output = serde_json::json!({
        "files_checked": files_checked,
        "total_errors": total_errors,
        "total_warnings": total_warnings,
        "total_info": total_info,
        "violations": all_results,
    });

    Ok(serde_json::to_string_pretty(&output)?)
}

// --- File discovery helpers ---

fn find_dockerfiles(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let names = ["Dockerfile", "dockerfile", "Containerfile"];
    walk_for_files(root, 0, 4, &mut files, &|name| {
        names
            .iter()
            .any(|n| name == *n || name.starts_with(&format!("{}.", n)))
    });
    files
}

fn find_compose_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    walk_for_files(root, 0, 4, &mut files, &|name| {
        let n = name.to_lowercase();
        n == "docker-compose.yml"
            || n == "docker-compose.yaml"
            || n == "compose.yml"
            || n == "compose.yaml"
    });
    files
}

fn find_k8s_dirs(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    // Look for directories containing K8s YAML files (with kind: field)
    let k8s_dir_names = [
        "k8s",
        "kubernetes",
        "manifests",
        "deploy",
        "deployments",
        "kube",
    ];
    let mut dirs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if k8s_dir_names.contains(&name.to_lowercase().as_str()) {
                    dirs.push(p);
                }
            }
        }
    }
    // Also check root for K8s files
    if has_k8s_files(root) && dirs.is_empty() {
        dirs.push(root.to_path_buf());
    }
    dirs
}

fn has_k8s_files(dir: &std::path::Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                if (ext == "yml" || ext == "yaml") && !is_compose_file(&p) {
                    if let Ok(content) = std::fs::read_to_string(&p) {
                        if content.contains("apiVersion:") && content.contains("kind:") {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

fn is_compose_file(p: &std::path::Path) -> bool {
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    name.contains("compose") || name.contains("docker-compose")
}

fn find_helm_charts_validate(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut charts = Vec::new();
    if root.join("Chart.yaml").exists() {
        charts.push(root.to_path_buf());
        return charts;
    }
    walk_for_dirs(root, 0, 3, &mut charts, &|dir| {
        dir.join("Chart.yaml").exists()
    });
    charts
}

fn walk_for_files(
    dir: &std::path::Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<std::path::PathBuf>,
    matcher: &dyn Fn(&str) -> bool,
) {
    if depth >= max_depth {
        return;
    }
    let skip = [
        "node_modules",
        "target",
        ".git",
        "vendor",
        "dist",
        "build",
        "__pycache__",
    ];
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() {
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if matcher(name) {
                    out.push(p);
                }
            }
        } else if p.is_dir() {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.') && !skip.contains(&name) {
                walk_for_files(&p, depth + 1, max_depth, out, matcher);
            }
        }
    }
}

fn walk_for_dirs(
    dir: &std::path::Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<std::path::PathBuf>,
    matcher: &dyn Fn(&std::path::Path) -> bool,
) {
    if depth >= max_depth {
        return;
    }
    let skip = ["node_modules", "target", ".git", "vendor"];
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.') && !skip.contains(&name) {
                if matcher(&p) {
                    out.push(p.clone());
                }
                walk_for_dirs(&p, depth + 1, max_depth, out, matcher);
            }
        }
    }
}

// --- Severity counting helpers (each linter has its own types) ---

fn count_severities_hadolint(
    result: &crate::analyzer::hadolint::LintResult,
) -> (usize, usize, usize) {
    use crate::analyzer::hadolint::Severity;
    let (mut e, mut w, mut i) = (0, 0, 0);
    for f in &result.failures {
        match f.severity {
            Severity::Error => e += 1,
            Severity::Warning => w += 1,
            Severity::Info | Severity::Style | Severity::Ignore => i += 1,
        }
    }
    (e, w, i)
}

fn count_severities_dclint(result: &crate::analyzer::dclint::LintResult) -> (usize, usize, usize) {
    use crate::analyzer::dclint::Severity;
    let (mut e, mut w, mut i) = (0, 0, 0);
    for f in &result.failures {
        match f.severity {
            Severity::Error => e += 1,
            Severity::Warning => w += 1,
            Severity::Info | Severity::Style => i += 1,
        }
    }
    (e, w, i)
}

fn count_severities_kubelint(
    result: &crate::analyzer::kubelint::LintResult,
) -> (usize, usize, usize) {
    use crate::analyzer::kubelint::Severity;
    let (mut e, mut w, mut i) = (0, 0, 0);
    for f in &result.failures {
        match f.severity {
            Severity::Error => e += 1,
            Severity::Warning => w += 1,
            Severity::Info => i += 1,
        }
    }
    (e, w, i)
}

fn count_severities_helmlint(
    result: &crate::analyzer::helmlint::LintResult,
) -> (usize, usize, usize) {
    use crate::analyzer::helmlint::Severity;
    let (mut e, mut w, mut i) = (0, 0, 0);
    for f in &result.failures {
        match f.severity {
            Severity::Error => e += 1,
            Severity::Warning => w += 1,
            Severity::Info | Severity::Style | Severity::Ignore => i += 1,
        }
    }
    (e, w, i)
}

/// CI-01: entry-point stub for `sync-ctl generate ci`.
///
/// Produces a minimal but syntactically valid pipeline skeleton so that the
/// acceptance criterion (`--dry-run` prints valid YAML) is satisfied at the
/// CLI layer. Full template rendering (CI-11/12/13) replaces this output once
/// the context and schema layers (CI-02, CI-14) are implemented.
///
/// TODO(CI-WIRE): replace stub body with:
///   collect_ci_context(path) → build_ci_pipeline(ctx) → CiFileWriter or dry-run print.
///   See Session 10 in the implementation plan.
pub fn handle_generate_ci(
    path: std::path::PathBuf,
    platform: crate::cli::CiPlatform,
    format: Option<crate::cli::CiFormat>,
    dry_run: bool,
    output: Option<std::path::PathBuf>,
    env_prefix: Option<String>,
    skip_docker: bool,
) -> crate::Result<()> {
    use crate::cli::{CiFormat, CiPlatform};

    // Resolve the effective format: use the caller's choice when given, otherwise
    // pick the canonical default for the chosen platform.
    let effective_format = format.unwrap_or(match platform {
        CiPlatform::Azure => CiFormat::AzurePipelines,
        CiPlatform::Gcp => CiFormat::CloudBuild,
        CiPlatform::Hetzner => CiFormat::GithubActions,
    });

    let prefix = env_prefix.as_deref().unwrap_or("APP");

    // Build a minimal valid YAML skeleton per format.  All values that cannot
    // be resolved without project analysis become {{PLACEHOLDER}} tokens.
    // CI-02 through CI-14 will replace this with a fully rendered CiPipeline.
    let skeleton = match effective_format {
        CiFormat::GithubActions => format!(
            r#"# Generated by sync-ctl generate ci (skeleton — CI-02+ fills placeholders)
# Project path : {path}
# Platform     : {platform_label}
# Env prefix   : {prefix}
# Skip docker  : {skip_docker}
name: CI
on:
  push:
    branches: ["{{{{DEFAULT_BRANCH}}}}", develop]
  pull_request:
    branches: ["{{{{DEFAULT_BRANCH}}}}"]
jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # CI-03: setup-runtime
      - uses: "{{{{SETUP_ACTION}}}}"
        with:
          "{{{{RUNTIME_KEY}}}}": "{{{{RUNTIME_VERSION}}}}"

      # CI-04: cache-deps
      - uses: actions/cache@v4
        with:
          path: "{{{{CACHE_PATH}}}}"
          key: "${{{{ runner.os }}}}-deps-${{{{ hashFiles('{{{{LOCK_FILE}}}}') }}}}"

      # CI-04: install
      - name: Install dependencies
        run: "{{{{INSTALL_COMMAND}}}}"

      # CI-06: lint (omitted if no linter detected)
      # - name: Lint
      #   run: {{{{LINT_COMMAND}}}}

      # CI-05: test
      - name: Test
        run: "{{{{TEST_COMMAND}}}}"

      # CI-07: build
      - name: Build
        run: "{{{{BUILD_COMMAND}}}}"
{docker_block}
      # CI-10: secret scan
      - uses: gitleaks/gitleaks-action@v2
        env:
          GITHUB_TOKEN: "${{{{ secrets.GITHUB_TOKEN }}}}"
"#,
            path = path.display(),
            platform_label = "GitHub Actions",
            prefix = prefix,
            skip_docker = skip_docker,
            docker_block = if skip_docker {
                String::new()
            } else {
                format!(
                    r#"
      # CI-08: docker build (omitted if --skip-docker or no Dockerfile detected)
      - name: Build Docker image
        run: docker build -t "${{{{ secrets.{prefix}_REGISTRY_URL }}}}/{{{{IMAGE_NAME}}}}:${{{{ github.sha }}}}" .
"#
                )
            },
        ),

        CiFormat::AzurePipelines => format!(
            r#"# Generated by sync-ctl generate ci (skeleton — CI-02+ fills placeholders)
# Project path : {path}
# Platform     : Azure Pipelines
# Env prefix   : {prefix}
trigger:
  branches:
    include:
      - "{{{{DEFAULT_BRANCH}}}}"
      - develop
pool:
  vmImage: ubuntu-latest
steps:
  - checkout: self

  # CI-03: setup-runtime
  - task: "{{{{AZURE_SETUP_TASK}}}}"
    inputs:
      versionSpec: "{{{{RUNTIME_VERSION}}}}"

  # CI-04: cache-deps
  - task: Cache@2
    inputs:
      key: '"deps" | "$(Agent.OS)" | "{{{{LOCK_FILE}}}}"'
      path: "{{{{CACHE_PATH}}}}"

  # CI-04: install
  - script: "{{{{INSTALL_COMMAND}}}}"
    displayName: Install dependencies

  # CI-05: test
  - script: "{{{{TEST_COMMAND}}}}"
    displayName: Run tests

  # CI-07: build
  - script: "{{{{BUILD_COMMAND}}}}"
    displayName: Build

  # CI-09/10: scanning steps added by CI-09/CI-10
"#,
            path = path.display(),
            prefix = prefix,
        ),

        CiFormat::CloudBuild => format!(
            r#"# Generated by sync-ctl generate ci (skeleton — CI-02+ fills placeholders)
# Project path : {path}
# Platform     : Google Cloud Build
# Env prefix   : {prefix}
steps:
  # CI-04: install
  - name: "{{{{BUILDER_IMAGE}}}}"
    entrypoint: "{{{{PACKAGE_MANAGER}}}}"
    args: ["{{{{INSTALL_ARGS}}}}"]

  # CI-05: test
  - name: "{{{{BUILDER_IMAGE}}}}"
    entrypoint: sh
    args:
      - "-c"
      - "{{{{TEST_COMMAND}}}}"

  # CI-07: build
  - name: "{{{{BUILDER_IMAGE}}}}"
    entrypoint: sh
    args:
      - "-c"
      - "{{{{BUILD_COMMAND}}}}"
{gcp_docker_block}
options:
  logging: CLOUD_LOGGING_ONLY
"#,
            path = path.display(),
            prefix = prefix,
            gcp_docker_block = if skip_docker {
                String::new()
            } else {
                r#"
  # CI-08: docker build
  - name: "gcr.io/cloud-builders/docker"
    args:
      - build
      - "-t"
      - "{{REGISTRY_URL}}/{{IMAGE_NAME}}:$SHORT_SHA"
      - "."
"#
                .to_string()
            },
        ),
    };

    if dry_run {
        println!("{}", skeleton);
    } else {
        // Full file writing arrives in CI-20 (writer.rs).  Until then, inform
        // the user that non-dry-run mode requires CI-20 to be implemented.
        let out_dir = output
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| ".".to_string());
        println!("🔧 CI pipeline skeleton ready (platform: {:?})", platform);
        println!("   Would write to: {}", out_dir);
        println!(
            "⚠️  File writing (CI-20) not yet implemented — use --dry-run to preview the skeleton."
        );
    }

    Ok(())
}
