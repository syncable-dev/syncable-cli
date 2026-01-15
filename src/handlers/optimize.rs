//! Handler for the `optimize` command.
//!
//! Analyzes Kubernetes manifests for resource optimization opportunities.
//! Supports both static analysis (Phase 1) and live cluster analysis (Phase 2).
//!
//! With `--full` flag, also runs:
//! - kubelint: Security and best practice checks
//! - helmlint: Helm chart structure validation

use crate::analyzer::helmlint::{HelmlintConfig, lint_chart as helmlint};
use crate::analyzer::k8s_optimize::{
    DataSource, K8sOptimizeConfig, LiveAnalyzer, LiveAnalyzerConfig, OutputFormat, Severity,
    analyze, format_result,
};
use crate::analyzer::kubelint::{KubelintConfig, lint as kubelint};
use crate::error::Result;
use std::path::Path;

/// Configuration for the optimize command
pub struct OptimizeOptions {
    /// Connect to a live cluster (context name or empty for current)
    pub cluster: Option<String>,
    /// Prometheus URL for historical metrics
    pub prometheus: Option<String>,
    /// Target namespace
    pub namespace: Option<String>,
    /// Analysis period for historical data
    pub period: String,
    /// Minimum severity to report
    pub severity: Option<String>,
    /// Minimum waste percentage to report
    pub threshold: Option<u8>,
    /// Safety margin percentage
    pub safety_margin: Option<u8>,
    /// Include info-level suggestions
    pub include_info: bool,
    /// Include system namespaces
    pub include_system: bool,
    /// Output format
    pub format: String,
    /// Output file
    pub output: Option<String>,
    /// Generate fixes
    pub fix: bool,
    /// Run comprehensive analysis (kubelint + helmlint + optimize)
    pub full: bool,
    /// Apply fixes to manifest files
    pub apply: bool,
    /// Dry-run mode (preview without applying)
    pub dry_run: bool,
    /// Backup directory for original files
    pub backup_dir: Option<String>,
    /// Minimum confidence threshold for auto-apply
    pub min_confidence: u8,
    /// Cloud provider for cost estimation
    pub cloud_provider: Option<String>,
    /// Region for cloud pricing
    pub region: String,
}

impl Default for OptimizeOptions {
    fn default() -> Self {
        Self {
            cluster: None,
            prometheus: None,
            namespace: None,
            period: "7d".to_string(),
            severity: None,
            threshold: None,
            safety_margin: None,
            include_info: false,
            include_system: false,
            format: "table".to_string(),
            output: None,
            fix: false,
            full: false,
            apply: false,
            dry_run: false,
            backup_dir: None,
            min_confidence: 70,
            cloud_provider: None,
            region: "us-east-1".to_string(),
        }
    }
}

/// Handle the `optimize` command.
pub async fn handle_optimize(path: &Path, options: OptimizeOptions) -> Result<()> {
    // Check if we should use live cluster analysis
    if options.cluster.is_some() {
        return handle_live_optimize(path, options).await;
    }

    // Static analysis mode (Phase 1)
    handle_static_optimize(path, options)
}

/// Handle static analysis (Phase 1) - analyzes manifests without cluster connection.
fn handle_static_optimize(path: &Path, options: OptimizeOptions) -> Result<()> {
    // Build config
    let mut config = K8sOptimizeConfig::default();

    if let Some(severity_str) = &options.severity
        && let Some(severity) = Severity::parse(severity_str)
    {
        config = config.with_severity(severity);
    }

    if let Some(threshold) = options.threshold {
        config = config.with_threshold(threshold);
    }

    if let Some(margin) = options.safety_margin {
        config = config.with_safety_margin(margin);
    }

    if options.include_info {
        config = config.with_info();
    }

    if options.include_system {
        config = config.with_system();
    }

    // Run resource optimization analysis
    let result = analyze(path, &config);

    // Determine output format
    let format = OutputFormat::parse(&options.format).unwrap_or(OutputFormat::Table);
    let is_json = options.format == "json";

    // If using --full with JSON, skip individual output and only show unified report
    let skip_individual_output = options.full && is_json;

    // Output resource optimization result (unless skipping for unified JSON)
    if !skip_individual_output {
        if let Some(output_path) = &options.output {
            // Write to file
            use crate::analyzer::k8s_optimize::format_result_to_string;
            let output = format_result_to_string(&result, format);
            std::fs::write(output_path, output)?;
            println!("Report written to: {}", output_path);
        } else {
            // Print to stdout
            format_result(&result, format);
        }
    }

    // Run comprehensive analysis if --full flag is set
    if options.full {
        run_comprehensive_analysis(path, &result, is_json)?;
    }

    // Generate fixes if requested
    if options.fix {
        generate_fixes(&result, path)?;
    }

    // Exit with non-zero if critical issues found
    if result.summary.missing_requests > 0 || result.summary.over_provisioned > 0 {
        // We could exit with error here for CI/CD
        // std::process::exit(1);
    }

    Ok(())
}

/// Run comprehensive analysis with kubelint and helmlint.
fn run_comprehensive_analysis(
    path: &Path,
    resource_result: &crate::analyzer::k8s_optimize::OptimizationResult,
    json_output: bool,
) -> Result<()> {
    use crate::analyzer::k8s_optimize::{
        ChartValidation, HelmIssue, HelmValidationReport, HelmValidationSummary,
        ResourceOptimizationReport, ResourceOptimizationSummary, SecurityFinding, SecurityReport,
        SecuritySummary, UnifiedMetadata, UnifiedReport, UnifiedSummary,
    };
    use colored::Colorize;

    // Run kubelint
    let kubelint_config = KubelintConfig::default().with_all_builtin();
    let kubelint_result = kubelint(path, &kubelint_config);

    // Run helmlint on all charts
    let helm_charts = find_helm_charts(path);
    let helmlint_config = HelmlintConfig::default();
    let mut chart_validations: Vec<ChartValidation> = Vec::new();

    for chart_path in &helm_charts {
        let chart_name = chart_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let helmlint_result = helmlint(chart_path, &helmlint_config);
        chart_validations.push(ChartValidation {
            chart_name,
            issues: helmlint_result
                .failures
                .iter()
                .map(|f| HelmIssue {
                    code: f.code.to_string(),
                    severity: format!("{:?}", f.severity).to_lowercase(),
                    message: f.message.clone(),
                })
                .collect(),
        });
    }

    // If JSON output, build unified report and print
    if json_output {
        let critical_count = kubelint_result
            .failures
            .iter()
            .filter(|f| f.severity == crate::analyzer::kubelint::Severity::Error)
            .count();
        let warning_count = kubelint_result.failures.len() - critical_count;
        let helm_issues: usize = chart_validations.iter().map(|c| c.issues.len()).sum();

        let report = UnifiedReport {
            summary: UnifiedSummary {
                total_resources: resource_result.summary.resources_analyzed as usize
                    + kubelint_result.summary.objects_analyzed,
                total_issues: resource_result.recommendations.len()
                    + kubelint_result.failures.len()
                    + helm_issues,
                critical_issues: resource_result
                    .recommendations
                    .iter()
                    .filter(|r| r.severity == crate::analyzer::k8s_optimize::Severity::Critical)
                    .count()
                    + critical_count,
                high_issues: resource_result
                    .recommendations
                    .iter()
                    .filter(|r| r.severity == crate::analyzer::k8s_optimize::Severity::High)
                    .count(),
                medium_issues: resource_result
                    .recommendations
                    .iter()
                    .filter(|r| r.severity == crate::analyzer::k8s_optimize::Severity::Medium)
                    .count()
                    + warning_count,
                confidence: 60, // Static analysis confidence
                health_score: calculate_health_score(
                    resource_result,
                    &kubelint_result,
                    &chart_validations,
                ),
            },
            live_analysis: None,
            resource_optimization: ResourceOptimizationReport {
                summary: ResourceOptimizationSummary {
                    resources: resource_result.summary.resources_analyzed as usize,
                    containers: resource_result.summary.containers_analyzed as usize,
                    over_provisioned: resource_result.summary.over_provisioned as usize,
                    missing_requests: resource_result.summary.missing_requests as usize,
                    optimal: resource_result.summary.optimal as usize,
                    estimated_waste_percent: resource_result.summary.total_waste_percentage,
                },
                recommendations: resource_result.recommendations.clone(),
            },
            security: SecurityReport {
                summary: SecuritySummary {
                    objects_analyzed: kubelint_result.summary.objects_analyzed,
                    checks_run: kubelint_result.summary.checks_run,
                    critical: critical_count,
                    warnings: warning_count,
                },
                findings: kubelint_result
                    .failures
                    .iter()
                    .map(|f| SecurityFinding {
                        code: f.code.to_string(),
                        severity: format!("{:?}", f.severity).to_lowercase(),
                        object_kind: f.object_kind.clone(),
                        object_name: f.object_name.clone(),
                        message: f.message.clone(),
                        remediation: f.remediation.clone(),
                    })
                    .collect(),
            },
            helm_validation: HelmValidationReport {
                summary: HelmValidationSummary {
                    charts_analyzed: chart_validations.len(),
                    charts_with_issues: chart_validations
                        .iter()
                        .filter(|c| !c.issues.is_empty())
                        .count(),
                    total_issues: helm_issues,
                },
                charts: chart_validations,
            },
            live_fixes: None, // No live data in static-only analysis
            trend_analysis: None,
            cost_estimation: None,
            precise_fixes: None,
            metadata: UnifiedMetadata {
                path: path.display().to_string(),
                analysis_time_ms: resource_result.metadata.duration_ms,
                timestamp: chrono::Utc::now().to_rfc3339(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        return Ok(());
    }

    // Table output (existing code)
    println!("\n{}", "═".repeat(91).bright_blue());
    println!(
        "{}",
        "🔒 SECURITY & BEST PRACTICES ANALYSIS (kubelint)"
            .bright_blue()
            .bold()
    );
    println!("{}\n", "═".repeat(91).bright_blue());

    if kubelint_result.failures.is_empty() {
        println!(
            "{}  No security or best practice issues found!\n",
            "✅".green()
        );
    } else {
        // Group by priority
        let critical: Vec<_> = kubelint_result
            .failures
            .iter()
            .filter(|f| f.severity == crate::analyzer::kubelint::Severity::Error)
            .collect();
        let warnings: Vec<_> = kubelint_result
            .failures
            .iter()
            .filter(|f| f.severity == crate::analyzer::kubelint::Severity::Warning)
            .collect();

        println!(
            "┌─ Summary ─────────────────────────────────────────────────────────────────────────────────┐"
        );
        println!(
            "│ Objects analyzed: {:>3}     Checks run: {:>3}     Issues: {:>3}",
            kubelint_result.summary.objects_analyzed,
            kubelint_result.summary.checks_run,
            kubelint_result.failures.len()
        );
        println!(
            "│ Critical: {:>3}     Warnings: {:>3}",
            critical.len(),
            warnings.len()
        );
        println!(
            "└───────────────────────────────────────────────────────────────────────────────────────────┘\n"
        );

        // Show critical issues
        for failure in critical.iter().take(10) {
            println!(
                "🔴 {} {}/{}",
                format!("[{}]", failure.code).red().bold(),
                failure.object_kind,
                failure.object_name
            );
            println!("   {}", failure.message);
            if let Some(remediation) = &failure.remediation {
                println!("   {} {}", "Fix:".yellow(), remediation);
            }
            println!();
        }

        // Show warnings (limited)
        for failure in warnings.iter().take(5) {
            println!(
                "🟡 {} {}/{}",
                format!("[{}]", failure.code).yellow(),
                failure.object_kind,
                failure.object_name
            );
            println!("   {}", failure.message);
            println!();
        }

        if warnings.len() > 5 {
            println!("   ... and {} more warnings\n", warnings.len() - 5);
        }
    }

    // Helm chart validation output
    if !helm_charts.is_empty() {
        println!("\n{}", "═".repeat(91).bright_cyan());
        println!(
            "{}",
            "📦 HELM CHART VALIDATION (helmlint)".bright_cyan().bold()
        );
        println!("{}\n", "═".repeat(91).bright_cyan());

        for chart in &chart_validations {
            if chart.issues.is_empty() {
                println!("{}  {} - No issues found", "✅".green(), chart.chart_name);
            } else {
                println!(
                    "{}  {} - {} issues found",
                    "⚠️".yellow(),
                    chart.chart_name,
                    chart.issues.len()
                );

                for issue in chart.issues.iter().take(3) {
                    println!(
                        "   {} {}",
                        format!("[{}]", issue.code).yellow(),
                        issue.message
                    );
                }
                if chart.issues.len() > 3 {
                    println!("   ... and {} more\n", chart.issues.len() - 3);
                }
            }
        }
        println!();
    }

    Ok(())
}

/// Calculate an overall health score based on all findings.
fn calculate_health_score(
    resource_result: &crate::analyzer::k8s_optimize::OptimizationResult,
    kubelint_result: &crate::analyzer::kubelint::LintResult,
    helm_validations: &[crate::analyzer::k8s_optimize::ChartValidation],
) -> u8 {
    let total_resources = resource_result.summary.resources_analyzed.max(1) as f32;
    let optimal_resources = resource_result.summary.optimal as f32;

    // Start with resource optimization score (40% weight)
    let resource_score = (optimal_resources / total_resources) * 40.0;

    // Security score (40% weight)
    let security_objects = kubelint_result.summary.objects_analyzed.max(1) as f32;
    let security_issues = kubelint_result.failures.len() as f32;
    let security_score =
        ((security_objects - security_issues.min(security_objects)) / security_objects) * 40.0;

    // Helm validation score (20% weight)
    let total_charts = helm_validations.len().max(1) as f32;
    let charts_with_issues = helm_validations
        .iter()
        .filter(|c| !c.issues.is_empty())
        .count() as f32;
    let helm_score = ((total_charts - charts_with_issues) / total_charts) * 20.0;

    (resource_score + security_score + helm_score).round() as u8
}

/// Find Helm charts in a directory.
fn find_helm_charts(path: &Path) -> Vec<std::path::PathBuf> {
    let mut charts = Vec::new();

    if path.join("Chart.yaml").exists() {
        charts.push(path.to_path_buf());
        return charts;
    }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                if entry_path.join("Chart.yaml").exists() {
                    charts.push(entry_path);
                } else {
                    // Check one level deeper
                    if let Ok(sub_entries) = std::fs::read_dir(&entry_path) {
                        for sub_entry in sub_entries.flatten() {
                            let sub_path = sub_entry.path();
                            if sub_path.is_dir() && sub_path.join("Chart.yaml").exists() {
                                charts.push(sub_path);
                            }
                        }
                    }
                }
            }
        }
    }

    charts
}

/// Generate optimized manifest files.
fn generate_fixes(
    result: &crate::analyzer::k8s_optimize::OptimizationResult,
    _base_path: &Path,
) -> Result<()> {
    if result.recommendations.is_empty() {
        println!("No fixes to generate - all resources are well-configured!");
        return Ok(());
    }

    println!("\n\u{1F4DD} Suggested fixes:\n");

    for rec in &result.recommendations {
        println!(
            "# {} ({}/{})",
            rec.resource_identifier(),
            rec.resource_kind,
            rec.container
        );
        println!("{}", rec.fix_yaml);
        println!();
    }

    println!("Apply these changes to your manifest files to optimize resource allocation.");

    Ok(())
}

/// Handle live cluster analysis (Phase 2) - connects to cluster for real metrics.
async fn handle_live_optimize(path: &Path, options: OptimizeOptions) -> Result<()> {
    use colored::Colorize;

    // Install rustls crypto provider (required for TLS connections to K8s API)
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cluster_context = options
        .cluster
        .clone()
        .unwrap_or_else(|| "current".to_string());
    let is_json = options.format.to_lowercase() == "json";

    if !is_json {
        println!("\n\u{2601}\u{FE0F}  Connecting to Kubernetes cluster...\n");
    }

    // Build live analyzer config
    let live_config = LiveAnalyzerConfig {
        prometheus_url: options.prometheus.clone(),
        history_period: options.period.clone(),
        safety_margin_pct: options.safety_margin.unwrap_or(20),
        min_samples: 100,
        waste_threshold_pct: options.threshold.map(|t| t as f32).unwrap_or(10.0),
        namespace: options.namespace.clone(),
        include_system: options.include_system,
    };

    // Create analyzer (with context or default)
    let analyzer = if cluster_context == "current" || cluster_context.is_empty() {
        LiveAnalyzer::new(live_config).await
    } else {
        LiveAnalyzer::with_context(&cluster_context, live_config).await
    }
    .map_err(|e| {
        crate::error::IaCGeneratorError::Io(std::io::Error::other(format!(
            "Failed to connect to cluster: {}",
            e
        )))
    })?;

    // Check available data sources
    let sources = analyzer.available_sources().await;

    if !is_json {
        println!("\u{1F4CA} Available data sources:");
        for source in &sources {
            let (icon, name) = match source {
                DataSource::MetricsServer => ("\u{1F4C8}", "metrics-server (real-time)"),
                DataSource::Prometheus => ("\u{1F4CA}", "Prometheus (historical)"),
                DataSource::Combined => ("\u{2728}", "Combined (highest accuracy)"),
                DataSource::Static => ("\u{1F4C4}", "Static (heuristics only)"),
            };
            println!("   {} {}", icon, name);
        }
        println!();
    }

    // Run analysis
    let result = analyzer.analyze().await.map_err(|e| {
        crate::error::IaCGeneratorError::Io(std::io::Error::other(format!(
            "Analysis failed: {}",
            e
        )))
    })?;

    // Display results (only in non-JSON mode)
    if !is_json {
        let source_name = match result.source {
            DataSource::Combined => "Combined (Prometheus + metrics-server)"
                .bright_green()
                .to_string(),
            DataSource::Prometheus => "Prometheus (historical data)".green().to_string(),
            DataSource::MetricsServer => "metrics-server (real-time snapshot)".yellow().to_string(),
            DataSource::Static => "Static heuristics (no cluster data)".red().to_string(),
        };

        println!("\n\u{1F50E} Analysis Results (Source: {})\n", source_name);
        println!("{}\n", "=".repeat(70).bright_blue());

        // Summary
        println!("\u{1F4CA} Summary:");
        println!(
            "   Resources analyzed: {}",
            result.summary.resources_analyzed
        );
        println!(
            "   Over-provisioned:   {} {}",
            result.summary.over_provisioned,
            if result.summary.over_provisioned > 0 {
                "\u{26A0}\u{FE0F}"
            } else {
                "\u{2705}"
            }
        );
        println!(
            "   Under-provisioned:  {} {}",
            result.summary.under_provisioned,
            if result.summary.under_provisioned > 0 {
                "\u{1F6A8}"
            } else {
                "\u{2705}"
            }
        );
        println!("   Optimal:            {}", result.summary.optimal);
        println!("   Confidence:         {}%", result.summary.confidence);

        // Waste summary
        if result.summary.total_cpu_waste_millicores > 0
            || result.summary.total_memory_waste_bytes > 0
        {
            println!("\n\u{1F4B8} Waste Summary:");
            if result.summary.total_cpu_waste_millicores > 0 {
                let cores = result.summary.total_cpu_waste_millicores as f64 / 1000.0;
                println!("   CPU wasted:    {:.2} cores", cores);
            }
            if result.summary.total_memory_waste_bytes > 0 {
                let gb =
                    result.summary.total_memory_waste_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                println!("   Memory wasted: {:.2} GB", gb);
            }
        }

        // Recommendations
        if !result.recommendations.is_empty() {
            println!("\n\u{1F4DD} Recommendations:\n");
            println!(
                "{:<40} {:>10} {:>10} {:>8} {:>8}",
                "Workload", "CPU Waste", "Mem Waste", "Conf", "Severity"
            );
            println!("{}", "-".repeat(80));

            for rec in &result.recommendations {
                let severity_str = match rec.severity {
                    Severity::Critical => "CRIT".red().bold().to_string(),
                    Severity::High => "HIGH".red().to_string(),
                    Severity::Medium => "MED".yellow().to_string(),
                    Severity::Low => "LOW".blue().to_string(),
                    Severity::Info => "INFO".dimmed().to_string(),
                };

                let workload = format!("{}/{}", rec.namespace, rec.workload_name);
                let workload_display = if workload.len() > 38 {
                    format!("...{}", &workload[workload.len() - 35..])
                } else {
                    workload
                };

                println!(
                    "{:<40} {:>9.0}% {:>9.0}% {:>7}% {:>8}",
                    workload_display,
                    rec.cpu_waste_pct,
                    rec.memory_waste_pct,
                    rec.confidence,
                    severity_str
                );

                // Show recommended values
                let cpu_rec = format_millicores(rec.recommended_cpu_millicores);
                let mem_rec = format_bytes(rec.recommended_memory_bytes);
                println!(
                    "   {} CPU: {} -> {} | Memory: {} -> {}",
                    "\u{27A1}\u{FE0F}".dimmed(),
                    rec.current_cpu_millicores
                        .map(format_millicores)
                        .unwrap_or_else(|| "none".to_string())
                        .red(),
                    cpu_rec.green(),
                    rec.current_memory_bytes
                        .map(format_bytes)
                        .unwrap_or_else(|| "none".to_string())
                        .red(),
                    mem_rec.green()
                );
            }
        }

        // Warnings
        for warning in &result.warnings {
            println!("\n\u{26A0}\u{FE0F}  {}", warning.yellow());
        }
    }

    // Also run static analysis on manifests if path provided
    if path.exists() && path.is_dir() {
        if options.full && is_json {
            // Run comprehensive analysis and output unified JSON with live data
            run_comprehensive_analysis_with_live(path, &result, &options)?;
        } else {
            if !is_json {
                println!(
                    "\n\u{1F4C1} Also checking local manifests in: {}\n",
                    path.display()
                );
            }
            let _ = handle_static_optimize(
                path,
                OptimizeOptions {
                    cluster: None,
                    prometheus: None,
                    namespace: None,
                    period: "7d".to_string(),
                    severity: options.severity.clone(),
                    threshold: options.threshold,
                    safety_margin: options.safety_margin,
                    include_info: options.include_info,
                    include_system: options.include_system,
                    format: options.format.clone(),
                    output: None,
                    fix: false,
                    full: options.full,
                    apply: false,
                    dry_run: options.dry_run,
                    backup_dir: None,
                    min_confidence: options.min_confidence,
                    cloud_provider: options.cloud_provider.clone(),
                    region: options.region.clone(),
                },
            );
        }
    } else if options.full && is_json {
        // Output live-only unified report
        run_live_only_unified_report(&result)?;
    }

    // Write to file if requested
    if let Some(output_path) = &options.output {
        let json = serde_json::to_string_pretty(&result).map_err(|e| {
            crate::error::IaCGeneratorError::Io(std::io::Error::other(format!(
                "Failed to serialize result: {}",
                e
            )))
        })?;
        std::fs::write(output_path, json)?;
        if !is_json {
            println!("\n\u{1F4BE} Report saved to: {}", output_path);
        }
    }

    Ok(())
}

/// Run comprehensive analysis with live cluster data and output unified JSON report.
fn run_comprehensive_analysis_with_live(
    path: &Path,
    live_result: &crate::analyzer::k8s_optimize::LiveAnalysisResult,
    options: &OptimizeOptions,
) -> Result<()> {
    use crate::analyzer::k8s_optimize::{
        ChartValidation, CloudProvider, HelmIssue, HelmValidationReport, HelmValidationSummary,
        LiveClusterSummary, ResourceOptimizationReport, ResourceOptimizationSummary,
        SecurityFinding, SecurityReport, SecuritySummary, UnifiedMetadata, UnifiedReport,
        UnifiedSummary, analyze_trends_from_live, calculate_from_live,
        locate_resources_from_static,
    };

    // Run static analysis with default config
    let static_config = K8sOptimizeConfig::default();
    let resource_result = analyze(path, &static_config);

    // Run kubelint with default config
    let kubelint_config = KubelintConfig::default().with_all_builtin();
    let kubelint_result = kubelint(path, &kubelint_config);

    // Run helmlint on all charts
    let helm_charts = find_helm_charts(path);
    let helmlint_config = HelmlintConfig::default();
    let mut chart_validations: Vec<ChartValidation> = Vec::new();

    for chart_path in &helm_charts {
        let chart_name = chart_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let helmlint_result = helmlint(chart_path, &helmlint_config);
        chart_validations.push(ChartValidation {
            chart_name,
            issues: helmlint_result
                .failures
                .iter()
                .map(|f| HelmIssue {
                    code: f.code.to_string(),
                    severity: format!("{:?}", f.severity).to_lowercase(),
                    message: f.message.clone(),
                })
                .collect(),
        });
    }

    // Build live cluster summary with P95 indicator
    let uses_prometheus = matches!(
        live_result.source,
        DataSource::Prometheus | DataSource::Combined
    );
    let live_summary = LiveClusterSummary {
        source: format!("{:?}", live_result.source),
        resources_analyzed: live_result.summary.resources_analyzed,
        over_provisioned: live_result.summary.over_provisioned,
        under_provisioned: live_result.summary.under_provisioned,
        optimal: live_result.summary.optimal,
        confidence: live_result.summary.confidence,
        uses_p95: if uses_prometheus { Some(true) } else { None },
        history_period: if uses_prometheus {
            Some(options.period.clone())
        } else {
            None
        },
    };

    // Deduplicate live vs static findings
    // Live findings take precedence but static findings that match increase confidence
    let (deduplicated_recs, dedup_stats) = deduplicate_recommendations(
        &live_result.recommendations,
        &resource_result.recommendations,
    );

    // Calculate totals using deduplicated data
    let live_analyzed = live_result.summary.resources_analyzed;
    let static_analyzed = resource_result.summary.resources_analyzed as usize;
    let total_resources = std::cmp::max(live_analyzed, static_analyzed);

    // Count issues from all sources (using deduplicated count)
    let resource_issues = deduplicated_recs.len();
    let security_issues = kubelint_result.failures.len();
    let helm_issues: usize = chart_validations.iter().map(|h| h.issues.len()).sum();
    let total_issues = resource_issues + security_issues + helm_issues;

    // Log deduplication stats
    if dedup_stats.duplicates_removed > 0 {
        eprintln!(
            "📊 Deduplication: {} duplicates removed, {} corroborated findings",
            dedup_stats.duplicates_removed, dedup_stats.corroborated
        );
    }

    // Count severities
    let mut critical = 0usize;
    let mut high = 0usize;
    let mut medium = 0usize;

    // Count from live recommendations
    for rec in &live_result.recommendations {
        match rec.severity {
            crate::analyzer::k8s_optimize::Severity::Critical => critical += 1,
            crate::analyzer::k8s_optimize::Severity::High => high += 1,
            crate::analyzer::k8s_optimize::Severity::Medium => medium += 1,
            _ => {}
        }
    }

    // Count from static recommendations
    for rec in &resource_result.recommendations {
        match rec.severity {
            crate::analyzer::k8s_optimize::Severity::Critical => critical += 1,
            crate::analyzer::k8s_optimize::Severity::High => high += 1,
            crate::analyzer::k8s_optimize::Severity::Medium => medium += 1,
            _ => {}
        }
    }

    // Count from security findings
    for f in &kubelint_result.failures {
        if f.severity == crate::analyzer::kubelint::Severity::Error {
            critical += 1;
        } else if f.severity == crate::analyzer::kubelint::Severity::Warning {
            medium += 1;
        }
    }

    // Use live confidence when available, otherwise calculate
    let confidence = if live_result.summary.confidence > 0 {
        live_result.summary.confidence
    } else {
        calculate_health_score(&resource_result, &kubelint_result, &chart_validations)
    };

    let health_score =
        calculate_health_score(&resource_result, &kubelint_result, &chart_validations);

    // Build unified report
    let report = UnifiedReport {
        summary: UnifiedSummary {
            total_resources,
            total_issues,
            critical_issues: critical,
            high_issues: high,
            medium_issues: medium,
            confidence,
            health_score,
        },
        live_analysis: Some(live_summary),
        resource_optimization: ResourceOptimizationReport {
            summary: ResourceOptimizationSummary {
                resources: resource_result.summary.resources_analyzed as usize,
                containers: resource_result.summary.containers_analyzed as usize,
                over_provisioned: resource_result.summary.over_provisioned as usize,
                missing_requests: resource_result.summary.missing_requests as usize,
                optimal: resource_result.summary.optimal as usize,
                estimated_waste_percent: resource_result.summary.total_waste_percentage,
            },
            recommendations: resource_result.recommendations.clone(),
        },
        security: SecurityReport {
            summary: SecuritySummary {
                objects_analyzed: kubelint_result.summary.objects_analyzed,
                checks_run: kubelint_result.summary.checks_run,
                critical: kubelint_result
                    .failures
                    .iter()
                    .filter(|f| f.severity == crate::analyzer::kubelint::Severity::Error)
                    .count(),
                warnings: kubelint_result.failures.len(),
            },
            findings: kubelint_result
                .failures
                .iter()
                .map(|f| SecurityFinding {
                    code: f.code.to_string(),
                    severity: format!("{:?}", f.severity).to_lowercase(),
                    object_kind: f.object_kind.clone(),
                    object_name: f.object_name.clone(),
                    message: f.message.clone(),
                    remediation: f.remediation.clone(),
                })
                .collect(),
        },
        helm_validation: HelmValidationReport {
            summary: HelmValidationSummary {
                charts_analyzed: chart_validations.len(),
                charts_with_issues: chart_validations
                    .iter()
                    .filter(|c| !c.issues.is_empty())
                    .count(),
                total_issues: helm_issues,
            },
            charts: chart_validations,
        },
        live_fixes: if live_result.recommendations.is_empty() {
            None
        } else {
            Some(
                live_result
                    .recommendations
                    .iter()
                    .map(|rec| crate::analyzer::k8s_optimize::LiveFix {
                        namespace: rec.namespace.clone(),
                        workload_name: rec.workload_name.clone(),
                        container_name: rec.container_name.clone(),
                        confidence: rec.confidence,
                        source: format!("{:?}", rec.data_source),
                        fix_yaml: rec.generate_fix_yaml(),
                    })
                    .collect(),
            )
        },
        trend_analysis: Some(analyze_trends_from_live(&live_result.recommendations)),
        cost_estimation: {
            // Parse cloud provider from options
            let provider = match options.cloud_provider.as_deref() {
                Some("aws") => CloudProvider::Aws,
                Some("gcp") => CloudProvider::Gcp,
                Some("azure") => CloudProvider::Azure,
                Some("onprem") => CloudProvider::OnPrem,
                _ => CloudProvider::Unknown,
            };
            Some(calculate_from_live(
                &live_result.recommendations,
                provider,
                &options.region,
            ))
        },
        precise_fixes: {
            let fixes = locate_resources_from_static(&resource_result.recommendations);
            if fixes.is_empty() { None } else { Some(fixes) }
        },
        metadata: UnifiedMetadata {
            path: path.display().to_string(),
            analysis_time_ms: resource_result.metadata.duration_ms,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    // Output JSON
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );

    Ok(())
}

/// Run live-only unified report (when no path is provided).
fn run_live_only_unified_report(
    live_result: &crate::analyzer::k8s_optimize::LiveAnalysisResult,
) -> Result<()> {
    use crate::analyzer::k8s_optimize::{
        HelmValidationReport, HelmValidationSummary, LiveClusterSummary,
        ResourceOptimizationReport, ResourceOptimizationSummary, SecurityReport, SecuritySummary,
        UnifiedMetadata, UnifiedReport, UnifiedSummary, analyze_trends_from_live,
    };

    let uses_prometheus = matches!(
        live_result.source,
        crate::analyzer::k8s_optimize::DataSource::Prometheus
            | crate::analyzer::k8s_optimize::DataSource::Combined
    );
    let live_summary = LiveClusterSummary {
        source: format!("{:?}", live_result.source),
        resources_analyzed: live_result.summary.resources_analyzed,
        over_provisioned: live_result.summary.over_provisioned,
        under_provisioned: live_result.summary.under_provisioned,
        optimal: live_result.summary.optimal,
        confidence: live_result.summary.confidence,
        uses_p95: if uses_prometheus { Some(true) } else { None },
        history_period: None, // Not tracked in live-only mode
    };

    // Count severities from live recommendations
    let mut critical = 0;
    let mut high = 0;
    let mut medium = 0;
    for rec in &live_result.recommendations {
        match rec.severity {
            crate::analyzer::k8s_optimize::Severity::Critical => critical += 1,
            crate::analyzer::k8s_optimize::Severity::High => high += 1,
            crate::analyzer::k8s_optimize::Severity::Medium => medium += 1,
            _ => {}
        }
    }

    let report = UnifiedReport {
        summary: UnifiedSummary {
            total_resources: live_result.summary.resources_analyzed,
            total_issues: live_result.recommendations.len(),
            critical_issues: critical,
            high_issues: high,
            medium_issues: medium,
            confidence: live_result.summary.confidence,
            health_score: if live_result.recommendations.is_empty() {
                100
            } else {
                (100 - std::cmp::min(critical * 15 + high * 10 + medium * 3, 100)) as u8
            },
        },
        live_analysis: Some(live_summary),
        resource_optimization: ResourceOptimizationReport {
            summary: ResourceOptimizationSummary {
                resources: live_result.summary.resources_analyzed,
                containers: live_result.recommendations.len(),
                over_provisioned: live_result.summary.over_provisioned,
                missing_requests: 0,
                optimal: live_result.summary.optimal,
                estimated_waste_percent: 0.0,
            },
            recommendations: vec![],
        },
        security: SecurityReport {
            summary: SecuritySummary {
                objects_analyzed: 0,
                checks_run: 0,
                critical: 0,
                warnings: 0,
            },
            findings: vec![],
        },
        helm_validation: HelmValidationReport {
            summary: HelmValidationSummary {
                charts_analyzed: 0,
                charts_with_issues: 0,
                total_issues: 0,
            },
            charts: vec![],
        },
        live_fixes: if live_result.recommendations.is_empty() {
            None
        } else {
            Some(
                live_result
                    .recommendations
                    .iter()
                    .map(|rec| crate::analyzer::k8s_optimize::LiveFix {
                        namespace: rec.namespace.clone(),
                        workload_name: rec.workload_name.clone(),
                        container_name: rec.container_name.clone(),
                        confidence: rec.confidence,
                        source: format!("{:?}", rec.data_source),
                        fix_yaml: rec.generate_fix_yaml(),
                    })
                    .collect(),
            )
        },
        trend_analysis: Some(analyze_trends_from_live(&live_result.recommendations)),
        cost_estimation: None, // No cloud provider info in live-only mode
        precise_fixes: None,   // No static files in live-only mode
        metadata: UnifiedMetadata {
            path: "cluster-only".to_string(),
            analysis_time_ms: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );

    Ok(())
}

/// Statistics about deduplication.
struct DeduplicationStats {
    duplicates_removed: usize,
    corroborated: usize,
}

/// Merged recommendation from live and/or static sources.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Used for deduplication tracking
struct MergedRecommendation {
    namespace: String,
    workload_name: String,
    container_name: String,
    severity: crate::analyzer::k8s_optimize::Severity,
    /// Confidence adjusted for corroboration
    confidence: u8,
    /// Source of the finding
    source: RecommendationSource,
    /// CPU waste percentage
    cpu_waste_pct: f32,
    /// Memory waste percentage
    memory_waste_pct: f32,
}

#[derive(Debug, Clone, PartialEq)]
enum RecommendationSource {
    LiveOnly,
    StaticOnly,
    Corroborated,
}

/// Deduplicate live vs static recommendations.
/// Live findings take precedence, but matching static findings increase confidence.
fn deduplicate_recommendations(
    live_recs: &[crate::analyzer::k8s_optimize::LiveRecommendation],
    static_recs: &[crate::analyzer::k8s_optimize::ResourceRecommendation],
) -> (Vec<MergedRecommendation>, DeduplicationStats) {
    use std::collections::HashMap;

    let mut merged: HashMap<(String, String, String), MergedRecommendation> = HashMap::new();
    let mut stats = DeduplicationStats {
        duplicates_removed: 0,
        corroborated: 0,
    };

    // First, add all live recommendations (highest priority)
    for rec in live_recs {
        let key = (
            rec.namespace.clone(),
            rec.workload_name.clone(),
            rec.container_name.clone(),
        );
        merged.insert(
            key,
            MergedRecommendation {
                namespace: rec.namespace.clone(),
                workload_name: rec.workload_name.clone(),
                container_name: rec.container_name.clone(),
                severity: rec.severity,
                confidence: rec.confidence,
                source: RecommendationSource::LiveOnly,
                cpu_waste_pct: rec.cpu_waste_pct,
                memory_waste_pct: rec.memory_waste_pct,
            },
        );
    }

    // Then check static recommendations
    for rec in static_recs {
        let ns = rec
            .namespace
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let key = (ns.clone(), rec.resource_name.clone(), rec.container.clone());

        if let Some(existing) = merged.get_mut(&key) {
            // Live finding exists - this is corroborated
            // Boost confidence by 10% (up to 100)
            existing.confidence = std::cmp::min(existing.confidence + 10, 100);
            existing.source = RecommendationSource::Corroborated;
            stats.duplicates_removed += 1;
            stats.corroborated += 1;
        } else {
            // Only static finding exists
            merged.insert(
                key,
                MergedRecommendation {
                    namespace: ns,
                    workload_name: rec.resource_name.clone(),
                    container_name: rec.container.clone(),
                    severity: rec.severity,
                    confidence: 50, // Lower confidence for static-only
                    source: RecommendationSource::StaticOnly,
                    cpu_waste_pct: 0.0, // Static analysis doesn't have precise waste metrics
                    memory_waste_pct: 0.0,
                },
            );
        }
    }

    (merged.into_values().collect(), stats)
}

/// Format millicores to human-readable string.
fn format_millicores(millicores: u64) -> String {
    if millicores >= 1000 {
        format!("{:.1}", millicores as f64 / 1000.0)
    } else {
        format!("{}m", millicores)
    }
}

/// Format bytes to human-readable string.
fn format_bytes(bytes: u64) -> String {
    const GI: u64 = 1024 * 1024 * 1024;
    const MI: u64 = 1024 * 1024;

    if bytes >= GI {
        format!("{:.1}Gi", bytes as f64 / GI as f64)
    } else {
        format!("{}Mi", bytes / MI)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_handle_optimize_nonexistent_path() {
        let result = handle_optimize(
            &PathBuf::from("/nonexistent/path"),
            OptimizeOptions::default(),
        )
        .await;
        // Should not panic, just return empty results
        assert!(result.is_ok());
    }
}
