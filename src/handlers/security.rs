use crate::{
    analyzer::security::{TurboSecurityAnalyzer, TurboConfig, ScanMode},
    analyzer::security::turbo::results::SecurityReport,
    analyzer::security::SecuritySeverity as TurboSecuritySeverity,
    analyzer::display::BoxDrawer,
    cli::{OutputFormat, SecurityScanMode},
};
use colored::*;
use std::path::PathBuf;

pub fn handle_security(
    path: PathBuf,
    mode: SecurityScanMode,
    include_low: bool,
    no_secrets: bool,
    no_code_patterns: bool,
    _no_infrastructure: bool,
    _no_compliance: bool,
    _frameworks: Vec<String>,
    format: OutputFormat,
    output: Option<PathBuf>,
    fail_on_findings: bool,
) -> crate::Result<String> {
    let project_path = path.canonicalize()
        .unwrap_or_else(|_| path.clone());
    
    // Build string output while also printing
    let mut result_output = String::new();
    
    // Print and collect header
    println!("🛡️  Running security analysis on: {}", project_path.display());
    result_output.push_str(&format!("🛡️  Running security analysis on: {}\n", project_path.display()));
    
    // Convert CLI mode to internal ScanMode, with flag overrides
    let scan_mode = determine_scan_mode(mode, include_low, no_secrets, no_code_patterns);
    
    // Configure turbo analyzer
    let config = create_turbo_config(scan_mode, fail_on_findings, no_secrets);
    
    // Initialize and run analyzer
    let analyzer = TurboSecurityAnalyzer::new(config)
        .map_err(|e| crate::error::IaCGeneratorError::Analysis(
            crate::error::AnalysisError::InvalidStructure(
                format!("Failed to create turbo security analyzer: {}", e)
            )
        ))?;
    
    let start_time = std::time::Instant::now();
    let security_report = analyzer.analyze_project(&project_path)
        .map_err(|e| crate::error::IaCGeneratorError::Analysis(
            crate::error::AnalysisError::InvalidStructure(
                format!("Turbo security analysis failed: {}", e)
            )
        ))?;
    let scan_duration = start_time.elapsed();
    
    // Print and collect scan completion
    println!("⚡ Scan completed in {:.2}s", scan_duration.as_secs_f64());
    result_output.push_str(&format!("⚡ Scan completed in {:.2}s\n", scan_duration.as_secs_f64()));
    
    // Format output
    let output_string = match format {
        OutputFormat::Table => format_security_table(&security_report, scan_mode, &path),
        OutputFormat::Json => serde_json::to_string_pretty(&security_report)?,
    };
    
    // Add formatted output to result string
    result_output.push_str(&output_string);
    
    // Output results
    if let Some(output_path) = output {
        std::fs::write(&output_path, &output_string)?;
        println!("Security report saved to: {}", output_path.display());
        result_output.push_str(&format!("\nSecurity report saved to: {}\n", output_path.display()));
    } else {
        print!("{}", output_string);
    }
    
    // Exit with error code if requested and findings exist
    if fail_on_findings && security_report.total_findings > 0 {
        handle_exit_codes(&security_report);
    }
    
    Ok(result_output)
}

fn determine_scan_mode(
    mode: SecurityScanMode,
    include_low: bool,
    no_secrets: bool,
    no_code_patterns: bool,
) -> ScanMode {
    if no_secrets && no_code_patterns {
        // Override: if both secrets and code patterns are disabled, use lightning
        ScanMode::Lightning
    } else if include_low {
        // Override: if including low findings, force paranoid mode
        ScanMode::Paranoid
    } else {
        // Use the requested mode from CLI
        match mode {
            SecurityScanMode::Lightning => ScanMode::Lightning,
            SecurityScanMode::Fast => ScanMode::Fast,
            SecurityScanMode::Balanced => ScanMode::Balanced,
            SecurityScanMode::Thorough => ScanMode::Thorough,
            SecurityScanMode::Paranoid => ScanMode::Paranoid,
        }
    }
}

fn create_turbo_config(scan_mode: ScanMode, fail_on_findings: bool, no_secrets: bool) -> TurboConfig {
    TurboConfig {
        scan_mode,
        max_file_size: 10 * 1024 * 1024, // 10MB
        worker_threads: 0, // Auto-detect
        use_mmap: true,
        enable_cache: true,
        cache_size_mb: 100,
        max_critical_findings: if fail_on_findings { Some(1) } else { None },
        timeout_seconds: Some(60),
        skip_gitignored: true,
        priority_extensions: vec![
            "env".to_string(), "key".to_string(), "pem".to_string(),
            "json".to_string(), "yml".to_string(), "yaml".to_string(),
            "toml".to_string(), "ini".to_string(), "conf".to_string(),
            "config".to_string(), "js".to_string(), "ts".to_string(),
            "py".to_string(), "rs".to_string(), "go".to_string(),
        ],
        pattern_sets: if no_secrets {
            vec![]
        } else {
            vec!["default".to_string(), "aws".to_string(), "gcp".to_string()]
        },
    }
}

fn format_security_table(
    security_report: &SecurityReport,
    scan_mode: ScanMode,
    path: &std::path::Path,
) -> String {
    let mut output = String::new();
    
    // Header
    output.push_str(&format!("\n{}\n", "🛡️  Security Analysis Results".bright_white().bold()));
    output.push_str(&format!("{}\n", "═".repeat(80).bright_blue()));
    
    // Security Score Box
    output.push_str(&format_security_summary_box(security_report, scan_mode));
    
    // Findings
    if !security_report.findings.is_empty() {
        output.push_str(&format_security_findings_box(security_report, path));
        output.push_str(&format_gitignore_legend());
    } else {
        output.push_str(&format_no_findings_box(security_report.files_scanned));
    }
    
    // Recommendations
    output.push_str(&format_recommendations_box(security_report));
    
    output
}

fn format_security_summary_box(
    security_report: &SecurityReport,
    scan_mode: ScanMode,
) -> String {
    let mut score_box = BoxDrawer::new("Security Summary");
    score_box.add_line("Overall Score:", &format!("{:.0}/100", security_report.overall_score).bright_yellow(), true);
    score_box.add_line("Risk Level:", &format!("{:?}", security_report.risk_level).color(match security_report.risk_level {
        TurboSecuritySeverity::Critical => "bright_red",
        TurboSecuritySeverity::High => "red", 
        TurboSecuritySeverity::Medium => "yellow",
        TurboSecuritySeverity::Low => "green",
        TurboSecuritySeverity::Info => "blue",
    }), true);
    score_box.add_line("Total Findings:", &security_report.total_findings.to_string().cyan(), true);
    score_box.add_line("Files Scanned:", &security_report.files_scanned.to_string().green(), true);
    score_box.add_line("Scan Mode:", &format!("{:?}", scan_mode).green(), true);
    
    format!("\n{}\n", score_box.draw())
}

fn format_security_findings_box(
    security_report: &SecurityReport,
    project_path: &std::path::Path,
) -> String {
    // Get terminal width to determine optimal display width
    let terminal_width = if let Some((width, _)) = term_size::dimensions() {
        width.saturating_sub(10) // Leave some margin
    } else {
        120 // Fallback width
    };
    
    let mut findings_box = BoxDrawer::new("Security Findings");
    
    for (i, finding) in security_report.findings.iter().enumerate() {
        let severity_color = match finding.severity {
            TurboSecuritySeverity::Critical => "bright_red",
            TurboSecuritySeverity::High => "red",
            TurboSecuritySeverity::Medium => "yellow", 
            TurboSecuritySeverity::Low => "blue",
            TurboSecuritySeverity::Info => "green",
        };
        
        // Extract relative file path from project root
        let file_display = calculate_relative_path(finding.file_path.as_ref(), project_path);
        
        // Parse gitignore status from description
        let gitignore_status = determine_gitignore_status(&finding.description);
        
        // Determine finding type
        let finding_type = determine_finding_type(&finding.title);
        
        // Format position
        let position_display = format_position(finding.line_number, finding.column_number);
        
        // Display file path with intelligent wrapping
        format_file_path(&mut findings_box, i + 1, &file_display, terminal_width);
        
        findings_box.add_value_only(&format!("   {} {} | {} {} | {} {} | {} {}", 
            "Type:".dimmed(),
            finding_type.yellow(),
            "Severity:".dimmed(),
            format!("{:?}", finding.severity).color(severity_color).bold(),
            "Position:".dimmed(),
            position_display.bright_cyan(),
            "Status:".dimmed(),
            gitignore_status
        ));
        
        // Add spacing between findings (except for the last one)
        if i < security_report.findings.len() - 1 {
            findings_box.add_value_only("");
        }
    }
    
    format!("\n{}\n", findings_box.draw())
}

fn calculate_relative_path(file_path: Option<&PathBuf>, project_path: &std::path::Path) -> String {
    if let Some(file_path) = file_path {
        // Cross-platform path normalization
        let canonical_file = file_path.canonicalize().unwrap_or_else(|_| file_path.clone());
        let canonical_project = project_path.canonicalize().unwrap_or_else(|_| project_path.to_path_buf());
        
        // Try to calculate relative path from project root
        if let Ok(relative_path) = canonical_file.strip_prefix(&canonical_project) {
            // Use forward slashes for consistency across platforms
            let relative_str = relative_path.to_string_lossy().replace('\\', "/");
            format!("./{}", relative_str)
        } else {
            // Fallback logic for complex paths
            format_fallback_path(file_path, project_path)
        }
    } else {
        "N/A".to_string()
    }
}

fn format_fallback_path(file_path: &PathBuf, project_path: &std::path::Path) -> String {
    let path_str = file_path.to_string_lossy();
    if path_str.starts_with('/') {
        // For absolute paths, try to extract meaningful relative portion
        if let Some(project_name) = project_path.file_name().and_then(|n| n.to_str()) {
            if let Some(project_idx) = path_str.rfind(project_name) {
                let relative_part = &path_str[project_idx + project_name.len()..];
                if relative_part.starts_with('/') {
                    format!(".{}", relative_part)
                } else if !relative_part.is_empty() {
                    format!("./{}", relative_part)
                } else {
                    format!("./{}", file_path.file_name().unwrap_or_default().to_string_lossy())
                }
            } else {
                path_str.to_string()
            }
        } else {
            path_str.to_string()
        }
    } else {
        // For relative paths that don't strip properly, use as-is
        if path_str.starts_with("./") {
            path_str.to_string()
        } else {
            format!("./{}", path_str)
        }
    }
}

fn determine_gitignore_status(description: &str) -> ColoredString {
    if description.contains("is tracked by git") {
        "TRACKED".bright_red().bold()
    } else if description.contains("is NOT in .gitignore") {
        "EXPOSED".yellow().bold()
    } else if description.contains("is protected") || description.contains("properly ignored") {
        "SAFE".bright_green().bold()
    } else if description.contains("appears safe") {
        "OK".bright_blue().bold()
    } else {
        "UNKNOWN".dimmed()
    }
}

fn determine_finding_type(title: &str) -> &'static str {
    if title.contains("Environment Variable") {
        "ENV VAR"
    } else if title.contains("Secret File") {
        "SECRET FILE"
    } else if title.contains("API Key") || title.contains("Stripe") || title.contains("Firebase") {
        "API KEY"
    } else if title.contains("Configuration") {
        "CONFIG"
    } else {
        "OTHER"
    }
}

fn format_position(line_number: Option<usize>, column_number: Option<usize>) -> String {
    match (line_number, column_number) {
        (Some(line), Some(col)) => format!("{}:{}", line, col),
        (Some(line), None) => format!("{}", line),
        _ => "—".to_string(),
    }
}

fn format_file_path(findings_box: &mut BoxDrawer, index: usize, file_display: &str, terminal_width: usize) {
    let box_margin = 6; // Account for box borders and padding
    let available_width = terminal_width.saturating_sub(box_margin);
    let max_path_width = available_width.saturating_sub(20); // Leave space for numbering and spacing
    
    if file_display.len() + 3 <= max_path_width {
        // Path fits on one line with numbering
        findings_box.add_value_only(&format!("{}. {}", 
            format!("{}", index).bright_white().bold(),
            file_display.cyan().bold()
        ));
    } else if file_display.len() <= available_width.saturating_sub(4) {
        // Path fits on its own line with indentation
        findings_box.add_value_only(&format!("{}.", 
            format!("{}", index).bright_white().bold()
        ));
        findings_box.add_value_only(&format!("   {}", 
            file_display.cyan().bold()
        ));
    } else {
        // Path is extremely long - use smart wrapping
        format_long_path(findings_box, index, file_display, available_width);
    }
}

fn format_long_path(findings_box: &mut BoxDrawer, index: usize, file_display: &str, available_width: usize) {
    findings_box.add_value_only(&format!("{}.", 
        format!("{}", index).bright_white().bold()
    ));
    
    // Smart path wrapping - prefer breaking at directory separators
    let wrap_width = available_width.saturating_sub(4);
    let mut remaining = file_display;
    let mut first_line = true;
    
    while !remaining.is_empty() {
        let prefix = if first_line { "   " } else { "     " };
        let line_width = wrap_width.saturating_sub(prefix.len());
        
        if remaining.len() <= line_width {
            // Last chunk fits entirely
            findings_box.add_value_only(&format!("{}{}", 
                prefix, remaining.cyan().bold()
            ));
            break;
        } else {
            // Find a good break point (prefer directory separator)
            let chunk = &remaining[..line_width];
            let break_point = chunk.rfind('/').unwrap_or(line_width.saturating_sub(1));
            
            findings_box.add_value_only(&format!("{}{}", 
                prefix, chunk[..break_point].cyan().bold()
            ));
            remaining = &remaining[break_point..];
            if remaining.starts_with('/') {
                remaining = &remaining[1..]; // Skip the separator
            }
        }
        first_line = false;
    }
}

fn format_gitignore_legend() -> String {
    let mut legend_box = BoxDrawer::new("Git Status Legend");
    legend_box.add_line(&"TRACKED:".bright_red().bold().to_string(), "File is tracked by git - CRITICAL RISK", false);
    legend_box.add_line(&"EXPOSED:".yellow().bold().to_string(), "File contains secrets but not in .gitignore", false);
    legend_box.add_line(&"SAFE:".bright_green().bold().to_string(), "File is properly ignored by .gitignore", false);
    legend_box.add_line(&"OK:".bright_blue().bold().to_string(), "File appears safe for version control", false);
    format!("\n{}\n", legend_box.draw())
}

fn format_no_findings_box(files_scanned: usize) -> String {
    let mut no_findings_box = BoxDrawer::new("Security Status");
    if files_scanned == 0 {
        no_findings_box.add_value_only(&"⚠️  No files were scanned".yellow());
        no_findings_box.add_value_only("This may indicate that all files were filtered out or the scan failed.");
        no_findings_box.add_value_only("💡 Try running with --mode thorough or --mode paranoid for a deeper scan");
    } else {
        no_findings_box.add_value_only(&"✅ No security issues detected".green());
        no_findings_box.add_value_only("💡 Regular security scanning recommended");
    }
    format!("\n{}\n", no_findings_box.draw())
}

fn format_recommendations_box(security_report: &SecurityReport) -> String {
    let mut rec_box = BoxDrawer::new("Key Recommendations");
    if !security_report.recommendations.is_empty() {
        for (i, rec) in security_report.recommendations.iter().take(5).enumerate() {
            // Clean up recommendation text
            let clean_rec = rec.replace("Add these patterns to your .gitignore:", "Add to .gitignore:");
            rec_box.add_value_only(&format!("{}. {}", i + 1, clean_rec));
        }
        if security_report.recommendations.len() > 5 {
            rec_box.add_value_only(&format!("... and {} more recommendations", 
                security_report.recommendations.len() - 5).dimmed());
        }
    } else {
        rec_box.add_value_only("✅ No immediate security concerns detected");
        rec_box.add_value_only("💡 Consider implementing dependency scanning");
        rec_box.add_value_only("💡 Review environment variable security practices");
    }
    format!("\n{}\n", rec_box.draw())
}

fn handle_exit_codes(security_report: &SecurityReport) -> ! {
    let critical_count = security_report.findings_by_severity
        .get(&TurboSecuritySeverity::Critical)
        .unwrap_or(&0);
    let high_count = security_report.findings_by_severity
        .get(&TurboSecuritySeverity::High)
        .unwrap_or(&0);
    
    if *critical_count > 0 {
        eprintln!("❌ Critical security issues found. Please address immediately.");
        std::process::exit(1);
    } else if *high_count > 0 {
        eprintln!("⚠️  High severity security issues found. Review recommended.");
        std::process::exit(2);
    } else {
        eprintln!("ℹ️  Security issues found but none are critical or high severity.");
        std::process::exit(3);
    }
} 