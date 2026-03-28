use crate::{
    analyzer::{self, vulnerability::VulnerabilitySeverity},
    cli::{OutputFormat, SeverityThreshold},
};
use std::path::PathBuf;

pub async fn handle_vulnerabilities(
    path: PathBuf,
    severity: Option<SeverityThreshold>,
    format: OutputFormat,
    output: Option<PathBuf>,
    quiet: bool,
) -> crate::Result<()> {
    let project_path = path.canonicalize().unwrap_or_else(|_| path.clone());

    if !quiet {
        println!(
            "🔍 Scanning for vulnerabilities in: {}",
            project_path.display()
        );
    }

    // Discover all project directories and check vulnerabilities per-directory.
    // Audit tools (npm audit, cargo audit, etc.) must run inside the directory
    // that contains the lock file / manifest, not from a parent directory.
    let parser = analyzer::dependency_parser::DependencyParser::new();
    let project_dirs = parser.discover_project_dirs(&project_path);

    // Suppress per-directory tool status banners — we'll print progress ourselves
    // SAFETY: set_var is called on main thread before spawning audit subprocesses
    let was_quiet = std::env::var("SYNCABLE_QUIET").is_ok();
    if !was_quiet {
        unsafe { std::env::set_var("SYNCABLE_QUIET", "1"); }
    }

    // Collect scannable dirs first so we can show progress
    let mut scannable_dirs = Vec::new();
    for dir in &project_dirs {
        let deps = parser.parse_deps_in_dir_standalone(dir)?;
        if !deps.is_empty() {
            let langs: Vec<String> = deps.keys().map(|l| format!("{:?}", l)).collect();
            scannable_dirs.push((dir.clone(), deps, langs));
        }
    }

    if !quiet && scannable_dirs.len() > 1 {
        println!("\n📦 Found {} projects to scan\n", scannable_dirs.len());
    }

    let mut merged_vulnerable_deps = Vec::new();
    let any_deps_found = !scannable_dirs.is_empty();
    let total_dirs = scannable_dirs.len();

    for (i, (dir, deps, langs)) in scannable_dirs.into_iter().enumerate() {
        let dir_name = dir.strip_prefix(&project_path)
            .unwrap_or(&dir)
            .display()
            .to_string();
        let dir_label = if dir_name.is_empty() || dir_name == "." {
            ".".to_string()
        } else {
            dir_name
        };

        if !quiet {
            println!(
                "  [{}/{}] Scanning {} ({})",
                i + 1, total_dirs, dir_label, langs.join(", ")
            );
        }

        let checker = analyzer::vulnerability::VulnerabilityChecker::new();
        match checker.check_all_dependencies(&deps, &dir).await {
            Ok(report) => {
                let count = report.vulnerable_dependencies.iter()
                    .map(|d| d.vulnerabilities.len())
                    .sum::<usize>();
                if !quiet && count > 0 {
                    println!("    ⚠️  {} vulnerabilities found", count);
                }
                // Tag each vulnerable dep with its source directory
                for mut dep in report.vulnerable_dependencies {
                    dep.source_dir = Some(dir_label.clone());
                    merged_vulnerable_deps.push(dep);
                }
            }
            Err(e) => {
                if !quiet {
                    eprintln!("    ⚠️  scan failed: {}", e);
                }
            }
        }
    }

    // Restore env var
    if !was_quiet {
        unsafe { std::env::remove_var("SYNCABLE_QUIET"); }
    }

    if !any_deps_found {
        if !quiet {
            println!("No dependencies found to check.");
        }
        return Ok(());
    }

    // Deduplicate vulnerable deps (same package may appear in multiple dirs)
    merged_vulnerable_deps.sort_by(|a, b| a.name.cmp(&b.name));
    merged_vulnerable_deps.dedup_by(|a, b| a.name == b.name && a.version == b.version);

    // Build merged report
    let mut critical_count = 0;
    let mut high_count = 0;
    let mut medium_count = 0;
    let mut low_count = 0;
    let mut total_vulnerabilities = 0;

    for dep in &merged_vulnerable_deps {
        for vuln in &dep.vulnerabilities {
            total_vulnerabilities += 1;
            match vuln.severity {
                VulnerabilitySeverity::Critical => critical_count += 1,
                VulnerabilitySeverity::High => high_count += 1,
                VulnerabilitySeverity::Medium => medium_count += 1,
                VulnerabilitySeverity::Low => low_count += 1,
                VulnerabilitySeverity::Info => {}
            }
        }
    }

    let report = analyzer::vulnerability::VulnerabilityReport {
        checked_at: chrono::Utc::now(),
        total_vulnerabilities,
        critical_count,
        high_count,
        medium_count,
        low_count,
        vulnerable_dependencies: merged_vulnerable_deps,
    };

    // Filter by severity if requested
    let filtered_report = if let Some(threshold) = severity {
        filter_vulnerabilities_by_severity(report, threshold)
    } else {
        report
    };

    // Format output
    let output_string = match format {
        OutputFormat::Table => {
            format_vulnerabilities_table(&filtered_report, &severity, &project_path)
        }
        OutputFormat::Json => serde_json::to_string_pretty(&filtered_report)?,
    };

    // Output results
    if let Some(output_path) = output {
        std::fs::write(&output_path, output_string)?;
        if !quiet {
            println!("Report saved to: {}", output_path.display());
        }
    } else if !quiet {
        println!("{}", output_string);
    }

    // Exit with non-zero code if critical/high vulnerabilities found (skip in quiet/agent mode)
    if !quiet && (filtered_report.critical_count > 0 || filtered_report.high_count > 0) {
        std::process::exit(1);
    }

    Ok(())
}

fn filter_vulnerabilities_by_severity(
    report: analyzer::vulnerability::VulnerabilityReport,
    threshold: SeverityThreshold,
) -> analyzer::vulnerability::VulnerabilityReport {
    let min_severity = match threshold {
        SeverityThreshold::Low => VulnerabilitySeverity::Low,
        SeverityThreshold::Medium => VulnerabilitySeverity::Medium,
        SeverityThreshold::High => VulnerabilitySeverity::High,
        SeverityThreshold::Critical => VulnerabilitySeverity::Critical,
    };

    let filtered_deps: Vec<_> = report
        .vulnerable_dependencies
        .into_iter()
        .filter_map(|mut dep| {
            dep.vulnerabilities.retain(|v| v.severity >= min_severity);
            if dep.vulnerabilities.is_empty() {
                None
            } else {
                Some(dep)
            }
        })
        .collect();

    use analyzer::vulnerability::VulnerabilityReport;
    let mut filtered = VulnerabilityReport {
        checked_at: report.checked_at,
        total_vulnerabilities: 0,
        critical_count: 0,
        high_count: 0,
        medium_count: 0,
        low_count: 0,
        vulnerable_dependencies: filtered_deps,
    };

    // Recalculate counts
    for dep in &filtered.vulnerable_dependencies {
        for vuln in &dep.vulnerabilities {
            filtered.total_vulnerabilities += 1;
            match vuln.severity {
                VulnerabilitySeverity::Critical => filtered.critical_count += 1,
                VulnerabilitySeverity::High => filtered.high_count += 1,
                VulnerabilitySeverity::Medium => filtered.medium_count += 1,
                VulnerabilitySeverity::Low => filtered.low_count += 1,
                VulnerabilitySeverity::Info => {}
            }
        }
    }

    filtered
}

fn format_vulnerabilities_table(
    report: &analyzer::vulnerability::VulnerabilityReport,
    severity: &Option<SeverityThreshold>,
    project_path: &std::path::Path,
) -> String {
    let mut output = String::new();

    output.push_str("\n🛡️  Vulnerability Scan Report\n");
    output.push_str(&format!("{}\n", "=".repeat(80)));
    output.push_str(&format!(
        "Scanned at: {}\n",
        report.checked_at.format("%Y-%m-%d %H:%M:%S UTC")
    ));
    output.push_str(&format!("Path: {}\n", project_path.display()));

    if let Some(threshold) = severity {
        output.push_str(&format!("Severity filter: >= {:?}\n", threshold));
    }

    output.push_str("\nSummary:\n");
    output.push_str(&format!(
        "Total vulnerabilities: {}\n",
        report.total_vulnerabilities
    ));

    if report.total_vulnerabilities > 0 {
        output.push_str("\nBy Severity:\n");
        if report.critical_count > 0 {
            output.push_str(&format!("  🔴 CRITICAL: {}\n", report.critical_count));
        }
        if report.high_count > 0 {
            output.push_str(&format!("  🔴 HIGH: {}\n", report.high_count));
        }
        if report.medium_count > 0 {
            output.push_str(&format!("  🟡 MEDIUM: {}\n", report.medium_count));
        }
        if report.low_count > 0 {
            output.push_str(&format!("  🔵 LOW: {}\n", report.low_count));
        }

        output.push_str(&format!("\n{}\n", "-".repeat(80)));
        output.push_str("Vulnerable Dependencies:\n\n");

        for vuln_dep in &report.vulnerable_dependencies {
            let source = vuln_dep.source_dir.as_deref().unwrap_or(".");
            output.push_str(&format!(
                "📦 {} v{} ({}) [{}]\n",
                vuln_dep.name,
                vuln_dep.version,
                vuln_dep.language.as_str(),
                source,
            ));

            for vuln in &vuln_dep.vulnerabilities {
                let severity_str = match vuln.severity {
                    VulnerabilitySeverity::Critical => "CRITICAL",
                    VulnerabilitySeverity::High => "HIGH",
                    VulnerabilitySeverity::Medium => "MEDIUM",
                    VulnerabilitySeverity::Low => "LOW",
                    VulnerabilitySeverity::Info => "INFO",
                };

                output.push_str(&format!("\n  ⚠️  {} [{}]\n", vuln.id, severity_str));
                output.push_str(&format!("     {}\n", vuln.title));

                if !vuln.description.is_empty() && vuln.description != vuln.title {
                    // Wrap description
                    let wrapped = textwrap::fill(&vuln.description, 70);
                    for line in wrapped.lines() {
                        output.push_str(&format!("     {}\n", line));
                    }
                }

                if let Some(ref cve) = vuln.cve {
                    output.push_str(&format!("     CVE: {}\n", cve));
                }

                if let Some(ref ghsa) = vuln.ghsa {
                    output.push_str(&format!("     GHSA: {}\n", ghsa));
                }

                output.push_str(&format!("     Affected: {}\n", vuln.affected_versions));

                if let Some(ref patched) = vuln.patched_versions {
                    output.push_str(&format!("     ✅ Fix: Upgrade to {}\n", patched));
                }
            }
            output.push('\n');
        }
    } else {
        output.push_str("\n✅ No vulnerabilities found!\n");
    }

    output
}
