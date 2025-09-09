use crate::{
    analyzer::{self, analyze_monorepo, vulnerability::VulnerabilitySeverity},
    cli::OutputFormat,
};
use crate::handlers::utils::format_project_category;
use std::process;
use std::collections::HashMap;

pub async fn handle_dependencies(
    path: std::path::PathBuf,
    licenses: bool,
    vulnerabilities: bool,
    _prod_only: bool,
    _dev_only: bool,
    format: OutputFormat,
) -> crate::Result<String> {
    let project_path = path.canonicalize()
        .unwrap_or_else(|_| path.clone());
    
    let mut output = String::new();
    let header = format!("🔍 Analyzing dependencies: {}\n", project_path.display());
    println!("{}", header);
    output.push_str(&header);
    
    // First, analyze the project using monorepo analysis
    let monorepo_analysis = analyze_monorepo(&project_path)?;
    
    // Collect all languages from all projects
    let mut all_languages = Vec::new();
    for project in &monorepo_analysis.projects {
        all_languages.extend(project.analysis.languages.clone());
    }
    
    // Then perform detailed dependency analysis using the collected languages
    let dep_analysis = analyzer::dependency_parser::parse_detailed_dependencies(
        &project_path,
        &all_languages,
        &analyzer::AnalysisConfig::default(),
    ).await?;
    
    if format == OutputFormat::Table {
        let table_output = display_dependencies_table(&dep_analysis, &monorepo_analysis, licenses, vulnerabilities, &all_languages, &project_path).await?;
        output.push_str(&table_output);
    } else if format == OutputFormat::Json {
        // JSON output
        let json_data = serde_json::json!({
            "dependencies": dep_analysis.dependencies,
            "total": dep_analysis.dependencies.len(),
        });
        let json_output = serde_json::to_string_pretty(&json_data)?;
        println!("{}", json_output);
        output.push_str(&json_output);
    }
    
    Ok(output)
}

async fn display_dependencies_table(
    dep_analysis: &analyzer::dependency_parser::DependencyAnalysis,
    monorepo_analysis: &analyzer::MonorepoAnalysis,
    licenses: bool,
    vulnerabilities: bool,
    all_languages: &[analyzer::DetectedLanguage],
    project_path: &std::path::Path,
) -> crate::Result<String> {
    use termcolor::{ColorChoice, StandardStream, WriteColor, ColorSpec, Color};
    
    let mut output = String::new();
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    
    // Print summary
    let summary_header = format!("\n📦 Dependency Analysis Report\n{}\n", "=".repeat(80));
    println!("{}", summary_header);
    output.push_str(&summary_header);
    
    let total_deps_line = format!("Total dependencies: {}\n", dep_analysis.dependencies.len());
    println!("{}", total_deps_line);
    output.push_str(&total_deps_line);
    
    if monorepo_analysis.is_monorepo {
        let projects_line = format!("Projects analyzed: {}\n", monorepo_analysis.projects.len());
        println!("{}", projects_line);
        output.push_str(&projects_line);
        for project in &monorepo_analysis.projects {
            let project_line = format!("  • {} ({})\n", project.name, format_project_category(&project.project_category));
            println!("{}", project_line);
            output.push_str(&project_line);
        }
    }
    
    for (name, info) in &dep_analysis.dependencies {
        let dep_line = format!("  {} v{}", name, info.version);
        print!("{}", dep_line);
        output.push_str(&dep_line);
        
        // Color code by type
        stdout.set_color(ColorSpec::new().set_fg(Some(
            if info.is_dev { Color::Yellow } else { Color::Green }
        )))?;
        
        let type_tag = format!(" [{}]", if info.is_dev { "dev" } else { "prod" });
        print!("{}", type_tag);
        output.push_str(&type_tag);
        
        stdout.reset()?;
        
        if licenses && info.license.is_some() {
            let license_info = format!(" - License: {}", info.license.as_ref().unwrap_or(&"Unknown".to_string()));
            print!("{}", license_info);
            output.push_str(&license_info);
        }
        
        println!();
        output.push('\n');
    }
    
    if licenses {
        let license_output = display_license_summary(&dep_analysis.dependencies);
        output.push_str(&license_output);
    }
    
    if vulnerabilities {
        let vuln_output = check_and_display_vulnerabilities(dep_analysis, all_languages, project_path).await?;
        output.push_str(&vuln_output);
    }
    
    Ok(output)
}

fn display_license_summary(dependencies: &analyzer::dependency_parser::DetailedDependencyMap) -> String {
    let mut output = String::new();
    output.push_str(&format!("\n📋 License Summary\n{}\n", "-".repeat(80)));
    
    let mut license_counts: HashMap<String, usize> = HashMap::new();
    
    for (_name, info) in dependencies {
        if let Some(license) = &info.license {
            *license_counts.entry(license.clone()).or_insert(0) += 1;
        }
    }
    
    let mut licenses: Vec<_> = license_counts.into_iter().collect();
    licenses.sort_by(|a, b| b.1.cmp(&a.1));
    
    for (license, count) in licenses {
        output.push_str(&format!("  {}: {} packages\n", license, count));
    }
    
    println!("{}", output);
    output
}

async fn check_and_display_vulnerabilities(
    dep_analysis: &analyzer::dependency_parser::DependencyAnalysis,
    all_languages: &[analyzer::DetectedLanguage],
    project_path: &std::path::Path,
) -> crate::Result<String> {
    use termcolor::{ColorChoice, StandardStream, WriteColor, ColorSpec, Color};
    
    let mut output = String::new();
    
    println!("\n🔍 Checking for vulnerabilities...");
    output.push_str("\n🔍 Checking for vulnerabilities...\n");
    
    // Convert DetailedDependencyMap to the format expected by VulnerabilityChecker
    let mut deps_by_language: HashMap<analyzer::dependency_parser::Language, Vec<analyzer::dependency_parser::DependencyInfo>> = HashMap::new();
    
    // Group dependencies by detected languages
    for language in all_languages {
        let mut lang_deps = Vec::new();
        
        // Filter dependencies that belong to this language
        for (name, info) in &dep_analysis.dependencies {
            // Simple heuristic to determine language based on source
            let matches_language = match language.name.as_str() {
                "Rust" => info.source == "crates.io",
                "JavaScript" | "TypeScript" => info.source == "npm",
                "Python" => info.source == "pypi",
                "Go" => info.source == "go modules",
                "Java" | "Kotlin" => info.source == "maven" || info.source == "gradle",
                _ => false,
            };
            
            if matches_language {
                // Convert to new DependencyInfo format expected by vulnerability checker
                lang_deps.push(analyzer::dependency_parser::DependencyInfo {
                    name: name.clone(),
                    version: info.version.clone(),
                    dep_type: if info.is_dev { 
                        analyzer::dependency_parser::DependencyType::Dev 
                    } else { 
                        analyzer::dependency_parser::DependencyType::Production 
                    },
                    license: info.license.clone().unwrap_or_default(),
                    source: Some(info.source.clone()),
                    language: match language.name.as_str() {
                        "Rust" => analyzer::dependency_parser::Language::Rust,
                        "JavaScript" => analyzer::dependency_parser::Language::JavaScript,
                        "TypeScript" => analyzer::dependency_parser::Language::TypeScript,
                        "Python" => analyzer::dependency_parser::Language::Python,
                        "Go" => analyzer::dependency_parser::Language::Go,
                        "Java" => analyzer::dependency_parser::Language::Java,
                        "Kotlin" => analyzer::dependency_parser::Language::Kotlin,
                        _ => analyzer::dependency_parser::Language::Unknown,
                    },
                });
            }
        }
        
        if !lang_deps.is_empty() {
            let lang_enum = match language.name.as_str() {
                "Rust" => analyzer::dependency_parser::Language::Rust,
                "JavaScript" => analyzer::dependency_parser::Language::JavaScript,
                "TypeScript" => analyzer::dependency_parser::Language::TypeScript,
                "Python" => analyzer::dependency_parser::Language::Python,
                "Go" => analyzer::dependency_parser::Language::Go,
                "Java" => analyzer::dependency_parser::Language::Java,
                "Kotlin" => analyzer::dependency_parser::Language::Kotlin,
                _ => analyzer::dependency_parser::Language::Unknown,
            };
            deps_by_language.insert(lang_enum, lang_deps);
        }
    }
    
    let checker = analyzer::vulnerability::VulnerabilityChecker::new();
    match checker.check_all_dependencies(&deps_by_language, project_path).await {
        Ok(report) => {
            let mut stdout = StandardStream::stdout(ColorChoice::Always);
            
            let report_header = format!("\n🛡️ Vulnerability Report\n{}\nChecked at: {}\nTotal vulnerabilities: {}\n",
                "-".repeat(80),
                report.checked_at.format("%Y-%m-%d %H:%M:%S UTC"),
                report.total_vulnerabilities
            );
            println!("{}", report_header);
            output.push_str(&report_header);
            
            if report.total_vulnerabilities > 0 {
                let breakdown_output = display_vulnerability_breakdown(&report, &mut stdout)?;
                output.push_str(&breakdown_output);
                
                let deps_output = display_vulnerable_dependencies(&report, &mut stdout)?;
                output.push_str(&deps_output);
            } else {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                let no_vulns_message = "\n✅ No known vulnerabilities found!\n";
                println!("{}", no_vulns_message);
                output.push_str(no_vulns_message);
                stdout.reset()?;
            }
        }
        Err(e) => {
            eprintln!("Error checking vulnerabilities: {}", e);
            process::exit(1);
        }
    }
    
    Ok(output)
}

fn display_vulnerability_breakdown(
    report: &analyzer::vulnerability::VulnerabilityReport,
    stdout: &mut termcolor::StandardStream,
) -> crate::Result<String> {
    use termcolor::{WriteColor, ColorSpec, Color};
    
    let mut output = String::new();
    
    output.push_str("\nSeverity Breakdown:\n");
    if report.critical_count > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        let critical_line = format!("  CRITICAL: {}\n", report.critical_count);
        output.push_str(&critical_line);
        print!("{}", critical_line);
        stdout.reset()?;
    }
    if report.high_count > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        let high_line = format!("  HIGH: {}\n", report.high_count);
        output.push_str(&high_line);
        print!("{}", high_line);
        stdout.reset()?;
    }
    if report.medium_count > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        let medium_line = format!("  MEDIUM: {}\n", report.medium_count);
        output.push_str(&medium_line);
        print!("{}", medium_line);
        stdout.reset()?;
    }
    if report.low_count > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
        let low_line = format!("  LOW: {}\n", report.low_count);
        output.push_str(&low_line);
        print!("{}", low_line);
        stdout.reset()?;
    }
    
    Ok(output)
}

fn display_vulnerable_dependencies(
    report: &analyzer::vulnerability::VulnerabilityReport,
    stdout: &mut termcolor::StandardStream,
) -> crate::Result<String> {
    use termcolor::{WriteColor, ColorSpec, Color};
    
    let mut output = String::new();
    
    output.push_str("\nVulnerable Dependencies:\n");
    for vuln_dep in &report.vulnerable_dependencies {
        let dep_line = format!("\n  📦 {} v{} ({})\n", 
            vuln_dep.name, 
            vuln_dep.version,
            vuln_dep.language.as_str()
        );
        output.push_str(&dep_line);
        print!("{}", dep_line);
        
        for vuln in &vuln_dep.vulnerabilities {
            let vuln_id_line = format!("    ⚠️  {} ", vuln.id);
            output.push_str(&vuln_id_line);
            print!("{}", vuln_id_line);
            
            // Color by severity
            stdout.set_color(ColorSpec::new().set_fg(Some(
                match vuln.severity {
                    VulnerabilitySeverity::Critical => Color::Red,
                    VulnerabilitySeverity::High => Color::Red,
                    VulnerabilitySeverity::Medium => Color::Yellow,
                    VulnerabilitySeverity::Low => Color::Blue,
                    VulnerabilitySeverity::Info => Color::Cyan,
                }
            )).set_bold(vuln.severity == VulnerabilitySeverity::Critical))?;
            
            let severity_tag = match vuln.severity {
                VulnerabilitySeverity::Critical => "[CRITICAL]",
                VulnerabilitySeverity::High => "[HIGH]",
                VulnerabilitySeverity::Medium => "[MEDIUM]",
                VulnerabilitySeverity::Low => "[LOW]",
                VulnerabilitySeverity::Info => "[INFO]",
            };
            output.push_str(severity_tag);
            print!("{}", severity_tag);
            
            stdout.reset()?;
            
            let title_line = format!(" - {}\n", vuln.title);
            output.push_str(&title_line);
            print!("{}", title_line);
            
            if let Some(ref cve) = vuln.cve {
                let cve_line = format!("       CVE: {}\n", cve);
                output.push_str(&cve_line);
                println!("{}", cve_line.trim_end());
            }
            if let Some(ref patched) = vuln.patched_versions {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                let fix_line = format!("       Fix: Upgrade to {}\n", patched);
                output.push_str(&fix_line);
                println!("{}", fix_line.trim_end());
                stdout.reset()?;
            }
        }
    }
    
    Ok(output)
} 