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
/// Collects project context, assembles a `CiPipeline`, renders it to YAML,
/// and either prints it (dry-run) or writes it to disk.
pub fn handle_generate_ci(
    path: std::path::PathBuf,
    platform: crate::cli::CiPlatform,
    format: Option<crate::cli::CiFormat>,
    dry_run: bool,
    output: Option<std::path::PathBuf>,
    env_prefix: Option<String>,
    skip_docker: bool,
    notify: bool,
) -> crate::Result<()> {
    use crate::cli::{CiFormat, CiPlatform};
    use crate::generator::ci_generation::{
        context::collect_ci_context,
        dry_run::print_dry_run,
        notify_step::{render_notify_yaml, NotifyStep},
        pipeline::build_ci_pipeline,
        secrets_doc::generate_secrets_doc,
        templates,
        token_resolver::resolve_tokens,
        writer::{write_ci_files, CiFile},
    };

    // Resolve effective format from CLI choice or platform default.
    let effective_format = format.unwrap_or(match platform {
        CiPlatform::Azure => CiFormat::AzurePipelines,
        CiPlatform::Gcp => CiFormat::CloudBuild,
        CiPlatform::Hetzner => CiFormat::GithubActions,
    });

    // ── Context collection ────────────────────────────────────────────────
    let mut ctx = collect_ci_context(&path, platform, effective_format.clone())?;
    if let Some(prefix) = env_prefix {
        ctx.env_prefix = Some(prefix);
    }

    // ── Pipeline assembly ─────────────────────────────────────────────────
    let mut pipeline = build_ci_pipeline(&ctx, skip_docker);

    // ── Token resolution (two-pass) ───────────────────────────────────────
    resolve_tokens(&ctx, &mut pipeline);

    // ── YAML rendering ────────────────────────────────────────────────────
    let pipeline_yaml = match effective_format {
        CiFormat::GithubActions => templates::github_actions::render(&pipeline),
        CiFormat::AzurePipelines => templates::azure_pipelines::render(&pipeline),
        CiFormat::CloudBuild => templates::cloud_build::render(&pipeline),
    };

    // Append notify step snippet when requested (CI-24).
    let notify_snippet = if notify {
        render_notify_yaml(&NotifyStep::default())
    } else {
        String::new()
    };
    let full_pipeline_yaml = format!("{}{}", pipeline_yaml, notify_snippet);

    // ── Secrets documentation ─────────────────────────────────────────────
    let secrets_content =
        generate_secrets_doc(&full_pipeline_yaml, ctx.platform.clone(), effective_format.clone());

    // ── Dry-run or write ──────────────────────────────────────────────────
    let output_dir = output.unwrap_or_else(|| path.clone());

    let files = vec![
        CiFile::pipeline(full_pipeline_yaml, effective_format.clone()),
        CiFile::secrets_doc(secrets_content),
    ];

    if dry_run {
        print_dry_run(&files, &pipeline, &output_dir);
    } else {
        let summary = write_ci_files(&files, &output_dir, false)?;
        println!(
            "✅ CI pipeline generated — {} created, {} skipped",
            summary.created() + summary.overwritten(),
            summary.skipped(),
        );
        if summary.invalid() > 0 {
            eprintln!("⚠️  {} file(s) had invalid YAML and were not written.", summary.invalid());
        }
    }

    // ── Telemetry (CI-27) ─────────────────────────────────────────────────
    if let Some(client) = crate::telemetry::get_telemetry_client() {
        use serde_json::json;
        let total = pipeline.unresolved_tokens.len()
            + pipeline
                .triggers
                .push_branches
                .len(); // non-zero field just to avoid div-by-zero
        let resolved_count = {
            // Estimate: each resolved token reduces the placeholder count.
            // unresolved_tokens holds only those that remain after resolution.
            let placeholder_count = pipeline.unresolved_tokens.len();
            // A rough 5-token baseline (RUNTIME_VERSION, TEST_COMMAND, BUILD_COMMAND,
            // REGISTRY_URL, IMAGE_NAME) for the resolution rate denominator.
            let baseline = 5usize;
            let rate = if baseline > 0 {
                let resolved = baseline.saturating_sub(placeholder_count);
                (resolved as f64 / baseline as f64 * 100.0).round() as u64
            } else {
                100
            };
            rate
        };
        let _ = total; // suppress unused warning

        let mut props = std::collections::HashMap::new();
        props.insert("platform".to_string(), json!(format!("{:?}", ctx.platform)));
        props.insert("format".to_string(), json!(format!("{:?}", effective_format)));
        props.insert("language".to_string(), json!(ctx.primary_language));
        props.insert("has_docker".to_string(), json!(ctx.has_dockerfile));
        props.insert("monorepo".to_string(), json!(ctx.monorepo));
        props.insert("token_resolution_rate".to_string(), json!(resolved_count));
        client.track_event("generate_ci", props);
    }

    Ok(())
}
