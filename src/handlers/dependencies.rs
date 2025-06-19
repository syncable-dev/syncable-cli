use crate::{
    analyzer::{self, analyze_monorepo, vulnerability_checker::VulnerabilitySeverity},
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
) -> crate::Result<()> {
    let project_path = path.canonicalize()
        .unwrap_or_else(|_| path.clone());
    
    println!("🔍 Analyzing dependencies: {}", project_path.display());
    
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
        display_dependencies_table(&dep_analysis, &monorepo_analysis, licenses, vulnerabilities, &all_languages, &project_path).await?;
    } else if format == OutputFormat::Json {
        // JSON output
        let output = serde_json::json!({
            "dependencies": dep_analysis.dependencies,
            "total": dep_analysis.dependencies.len(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    }
    
    Ok(())
}

async fn display_dependencies_table(
    dep_analysis: &analyzer::dependency_parser::DependencyAnalysis,
    monorepo_analysis: &analyzer::MonorepoAnalysis,
    licenses: bool,
    vulnerabilities: bool,
    all_languages: &[analyzer::DetectedLanguage],
    project_path: &std::path::Path,
) -> crate::Result<()> {
    use termcolor::{ColorChoice, StandardStream, WriteColor, ColorSpec, Color};
    
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    
    // Print summary
    println!("\n📦 Dependency Analysis Report");
    println!("{}", "=".repeat(80));
    
    let total_deps: usize = dep_analysis.dependencies.len();
    println!("Total dependencies: {}", total_deps);
    
    if monorepo_analysis.is_monorepo {
        println!("Projects analyzed: {}", monorepo_analysis.projects.len());
        for project in &monorepo_analysis.projects {
            println!("  • {} ({})", project.name, format_project_category(&project.project_category));
        }
    }
    
    for (name, info) in &dep_analysis.dependencies {
        print!("  {} v{}", name, info.version);
        
        // Color code by type
        stdout.set_color(ColorSpec::new().set_fg(Some(
            if info.is_dev { Color::Yellow } else { Color::Green }
        )))?;
        
        print!(" [{}]", if info.is_dev { "dev" } else { "prod" });
        
        stdout.reset()?;
        
        if licenses && info.license.is_some() {
            print!(" - License: {}", info.license.as_ref().unwrap_or(&"Unknown".to_string()));
        }
        
        println!();
    }
    
    if licenses {
        display_license_summary(&dep_analysis.dependencies);
    }
    
    if vulnerabilities {
        check_and_display_vulnerabilities(dep_analysis, all_languages, project_path).await?;
    }
    
    Ok(())
}

fn display_license_summary(dependencies: &analyzer::dependency_parser::DetailedDependencyMap) {
    println!("\n📋 License Summary");
    println!("{}", "-".repeat(80));
    
    let mut license_counts: HashMap<String, usize> = HashMap::new();
    
    for (_name, info) in dependencies {
        if let Some(license) = &info.license {
            *license_counts.entry(license.clone()).or_insert(0) += 1;
        }
    }
    
    let mut licenses: Vec<_> = license_counts.into_iter().collect();
    licenses.sort_by(|a, b| b.1.cmp(&a.1));
    
    for (license, count) in licenses {
        println!("  {}: {} packages", license, count);
    }
}

async fn check_and_display_vulnerabilities(
    dep_analysis: &analyzer::dependency_parser::DependencyAnalysis,
    all_languages: &[analyzer::DetectedLanguage],
    project_path: &std::path::Path,
) -> crate::Result<()> {
    use termcolor::{ColorChoice, StandardStream, WriteColor, ColorSpec, Color};
    
    println!("\n🔍 Checking for vulnerabilities...");
    
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
    
    let checker = analyzer::vulnerability_checker::VulnerabilityChecker::new();
    match checker.check_all_dependencies(&deps_by_language, project_path).await {
        Ok(report) => {
            let mut stdout = StandardStream::stdout(ColorChoice::Always);
            
            println!("\n🛡️ Vulnerability Report");
            println!("{}", "-".repeat(80));
            println!("Checked at: {}", report.checked_at.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("Total vulnerabilities: {}", report.total_vulnerabilities);
            
            if report.total_vulnerabilities > 0 {
                display_vulnerability_breakdown(&report, &mut stdout)?;
                display_vulnerable_dependencies(&report, &mut stdout)?;
            } else {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                println!("\n✅ No known vulnerabilities found!");
                stdout.reset()?;
            }
        }
        Err(e) => {
            eprintln!("Error checking vulnerabilities: {}", e);
            process::exit(1);
        }
    }
    
    Ok(())
}

fn display_vulnerability_breakdown(
    report: &analyzer::vulnerability_checker::VulnerabilityReport,
    stdout: &mut termcolor::StandardStream,
) -> crate::Result<()> {
    use termcolor::{WriteColor, ColorSpec, Color};
    
    println!("\nSeverity Breakdown:");
    if report.critical_count > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        println!("  CRITICAL: {}", report.critical_count);
        stdout.reset()?;
    }
    if report.high_count > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        println!("  HIGH: {}", report.high_count);
        stdout.reset()?;
    }
    if report.medium_count > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        println!("  MEDIUM: {}", report.medium_count);
        stdout.reset()?;
    }
    if report.low_count > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
        println!("  LOW: {}", report.low_count);
        stdout.reset()?;
    }
    
    Ok(())
}

fn display_vulnerable_dependencies(
    report: &analyzer::vulnerability_checker::VulnerabilityReport,
    stdout: &mut termcolor::StandardStream,
) -> crate::Result<()> {
    use termcolor::{WriteColor, ColorSpec, Color};
    
    println!("\nVulnerable Dependencies:");
    for vuln_dep in &report.vulnerable_dependencies {
        println!("\n  📦 {} v{} ({})", 
            vuln_dep.name, 
            vuln_dep.version,
            vuln_dep.language.as_str()
        );
        
        for vuln in &vuln_dep.vulnerabilities {
            print!("    ⚠️  {} ", vuln.id);
            
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
            
            print!("[{}]", match vuln.severity {
                VulnerabilitySeverity::Critical => "CRITICAL",
                VulnerabilitySeverity::High => "HIGH",
                VulnerabilitySeverity::Medium => "MEDIUM",
                VulnerabilitySeverity::Low => "LOW",
                VulnerabilitySeverity::Info => "INFO",
            });
            
            stdout.reset()?;
            
            println!(" - {}", vuln.title);
            
            if let Some(ref cve) = vuln.cve {
                println!("       CVE: {}", cve);
            }
            if let Some(ref patched) = vuln.patched_versions {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                println!("       Fix: Upgrade to {}", patched);
                stdout.reset()?;
            }
        }
    }
    
    Ok(())
} 