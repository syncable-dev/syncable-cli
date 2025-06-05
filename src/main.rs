use clap::Parser;
use syncable_cli::{
    analyzer::{self, vulnerability_checker::VulnerabilitySeverity},
    cli::{Cli, Commands, OutputFormat, SeverityThreshold},
    config,
    generator,
};
use std::process;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run() -> syncable_cli::Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    cli.init_logging();
    
    // Load configuration
    let _config = match config::load_config(cli.config.as_deref()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            process::exit(1);
        }
    };
    
    // Execute command
    let result = match cli.command {
        Commands::Analyze { path, json, detailed, only } => {
            handle_analyze(path, json, detailed, only)
        }
        Commands::Generate { 
            path, 
            output, 
            dockerfile, 
            compose, 
            terraform, 
            all,
            dry_run,
            force 
        } => {
            handle_generate(path, output, dockerfile, compose, terraform, all, dry_run, force)
        }
        Commands::Validate { path, types, fix } => {
            handle_validate(path, types, fix)
        }
        Commands::Support { languages, frameworks, detailed } => {
            handle_support(languages, frameworks, detailed)
        }
        Commands::Dependencies { path, licenses, vulnerabilities, prod_only, dev_only, format } => {
            handle_dependencies(path, licenses, vulnerabilities, prod_only, dev_only, format).await
        }
        Commands::Vulnerabilities { path, severity, format, output } => {
            handle_vulnerabilities(path, severity, format, output).await
        }
    };
    
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
    
    Ok(())
}

fn handle_analyze(
    path: std::path::PathBuf,
    json: bool,
    detailed: bool,
    _only: Option<Vec<String>>,
) -> syncable_cli::Result<()> {
    println!("🔍 Analyzing project: {}", path.display());
    
    let analysis = analyzer::analyze_project(&path)?;
    
    if json {
        println!("{}", serde_json::to_string_pretty(&analysis)?);
    } else if detailed {
        // Use the beautiful formatting from the example
        println!("{}", "=".repeat(60));
        println!("\n📊 PROJECT CONTEXT ANALYSIS RESULTS");
        println!("{}", "=".repeat(60));
        
        // Project Type
        println!("\n🎯 Project Type: {:?}", analysis.project_type);
        use analyzer::ProjectType;
        match analysis.project_type {
            ProjectType::WebApplication => println!("   This is a web application with UI"),
            ProjectType::ApiService => println!("   This is an API service without UI"),
            ProjectType::CliTool => println!("   This is a command-line tool"),
            ProjectType::Library => println!("   This is a library/package"),
            ProjectType::Microservice => println!("   This is a microservice"),
            ProjectType::StaticSite => println!("   This is a static website"),
            _ => println!("   Project type details not available"),
        }
        
        // Languages
        println!("\n🌐 Languages Detected ({}):", analysis.languages.len());
        for (i, lang) in analysis.languages.iter().enumerate() {
            println!("   {}. {} (confidence: {:.1}%)", 
                i + 1, 
                lang.name, 
                lang.confidence * 100.0
            );
            if let Some(version) = &lang.version {
                println!("      Version: {}", version);
            }
        }
        
        // Frameworks
        println!("\n🚀 Frameworks Detected ({}):", analysis.frameworks.len());
        for (i, framework) in analysis.frameworks.iter().enumerate() {
            println!("   {}. {} (confidence: {:.1}%)", 
                i + 1,
                framework.name, 
                framework.confidence * 100.0
            );
        }
        
        // Entry Points
        println!("\n📍 Entry Points ({}):", analysis.entry_points.len());
        if analysis.entry_points.is_empty() {
            println!("   No entry points detected");
        } else {
            for (i, entry) in analysis.entry_points.iter().enumerate() {
                println!("   {}. File: {}", i + 1, entry.file.display());
                if let Some(func) = &entry.function {
                    println!("      Function: {}", func);
                }
                if let Some(cmd) = &entry.command {
                    println!("      Command: {}", cmd);
                }
            }
        }
        
        // Ports
        println!("\n🔌 Exposed Ports ({}):", analysis.ports.len());
        if analysis.ports.is_empty() {
            println!("   No ports detected");
        } else {
            for port in &analysis.ports {
                println!("   - Port {}: {:?}", port.number, port.protocol);
                if let Some(desc) = &port.description {
                    println!("     {}", desc);
                }
            }
        }
        
        // Environment Variables
        println!("\n🔐 Environment Variables ({}):", analysis.environment_variables.len());
        let required_vars: Vec<_> = analysis.environment_variables.iter()
            .filter(|ev| ev.required)
            .collect();
        let optional_vars: Vec<_> = analysis.environment_variables.iter()
            .filter(|ev| !ev.required)
            .collect();
        
        if !required_vars.is_empty() {
            println!("   Required:");
            for var in required_vars {
                println!("     - {} {}", 
                    var.name,
                    if let Some(desc) = &var.description { 
                        format!("({})", desc) 
                    } else { 
                        String::new() 
                    }
                );
            }
        }
        
        if !optional_vars.is_empty() {
            println!("   Optional:");
            for var in optional_vars {
                println!("     - {} = {:?}", 
                    var.name, 
                    var.default_value.as_deref().unwrap_or("no default")
                );
            }
        }
        
        if analysis.environment_variables.is_empty() {
            println!("   No environment variables detected");
        }
        
        // Build Scripts
        println!("\n🔨 Build Scripts ({}):", analysis.build_scripts.len());
        let default_scripts: Vec<_> = analysis.build_scripts.iter()
            .filter(|bs| bs.is_default)
            .collect();
        let other_scripts: Vec<_> = analysis.build_scripts.iter()
            .filter(|bs| !bs.is_default)
            .collect();
        
        if !default_scripts.is_empty() {
            println!("   Default scripts:");
            for script in default_scripts {
                println!("     - {}: {}", script.name, script.command);
                if let Some(desc) = &script.description {
                    println!("       {}", desc);
                }
            }
        }
        
        if !other_scripts.is_empty() {
            println!("   Other scripts:");
            for script in other_scripts {
                println!("     - {}: {}", script.name, script.command);
                if let Some(desc) = &script.description {
                    println!("       {}", desc);
                }
            }
        }
        
        if analysis.build_scripts.is_empty() {
            println!("   No build scripts detected");
        }
        
        // Dependencies (sample)
        println!("\n📦 Dependencies ({}):", analysis.dependencies.len());
        if analysis.dependencies.is_empty() {
            println!("   No dependencies detected");
        } else if analysis.dependencies.len() <= 10 {
            for (name, version) in &analysis.dependencies {
                println!("   - {} v{}", name, version);
            }
        } else {
            // Show first 10
            for (name, version) in analysis.dependencies.iter().take(10) {
                println!("   - {} v{}", name, version);
            }
            println!("   ... and {} more", analysis.dependencies.len() - 10);
        }
        
        // Summary
        println!("\n📋 SUMMARY");
        println!("{}", "=".repeat(60));
        println!("✅ Project Context Analysis Complete!");
        println!("\nProject Context Components:");
        println!("   1. Entry points detected: {}", 
            if analysis.entry_points.is_empty() { "❌ None" } else { "✅ Yes" });
        println!("   2. Ports identified: {}", 
            if analysis.ports.is_empty() { "❌ None" } else { "✅ Yes" });
        println!("   3. Environment variables extracted: {}", 
            if analysis.environment_variables.is_empty() { "❌ None" } else { "✅ Yes" });
        println!("   4. Build scripts analyzed: {}", 
            if analysis.build_scripts.is_empty() { "❌ None" } else { "✅ Yes" });
        println!("   5. Project type determined: {}", 
            if matches!(analysis.project_type, ProjectType::Unknown) { "❌ Unknown" } else { "✅ Yes" });
        
        println!("\n📈 Analysis Metadata:");
        println!("   - Duration: {}ms", analysis.analysis_metadata.analysis_duration_ms);
        println!("   - Files analyzed: {}", analysis.analysis_metadata.files_analyzed);
        println!("   - Confidence score: {:.1}%", analysis.analysis_metadata.confidence_score * 100.0);
        
    } else {
        // Simple summary view (non-detailed)
        println!("\n📊 Analysis Results:");
        println!("├── Project: {}", analysis.project_root.display());
        println!("├── Languages detected: {}", analysis.languages.len());
        for lang in &analysis.languages {
            println!("│   ├── {} (confidence: {:.1}%)", lang.name, lang.confidence * 100.0);
        }
        println!("├── Frameworks detected: {}", analysis.frameworks.len());
        for framework in &analysis.frameworks {
            println!("│   ├── {} (confidence: {:.1}%)", framework.name, framework.confidence * 100.0);
        }
        println!("├── Dependencies found: {}", analysis.dependencies.len());
        println!("├── Entry points: {}", analysis.entry_points.len());
        println!("├── Ports detected: {}", analysis.ports.len());
        println!("├── Environment variables: {}", analysis.environment_variables.len());
        println!("└── Project type: {:?}", analysis.project_type);
        
        println!("\n📈 Analysis metadata:");
        println!("├── Duration: {}ms", analysis.analysis_metadata.analysis_duration_ms);
        println!("├── Files analyzed: {}", analysis.analysis_metadata.files_analyzed);
        println!("└── Confidence score: {:.1}%", analysis.analysis_metadata.confidence_score * 100.0);
    }
    
    Ok(())
}

fn handle_generate(
    path: std::path::PathBuf,
    _output: Option<std::path::PathBuf>,
    dockerfile: bool,
    compose: bool,
    terraform: bool,
    all: bool,
    dry_run: bool,
    _force: bool,
) -> syncable_cli::Result<()> {
    println!("🔍 Analyzing project for generation: {}", path.display());
    
    let analysis = analyzer::analyze_project(&path)?;
    
    println!("✅ Analysis complete. Generating IaC files...");
    
    let generate_all = all || (!dockerfile && !compose && !terraform);
    
    if generate_all || dockerfile {
        println!("\n🐳 Generating Dockerfile...");
        let dockerfile_content = generator::generate_dockerfile(&analysis)?;
        
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
        let compose_content = generator::generate_compose(&analysis)?;
        
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
        let terraform_content = generator::generate_terraform(&analysis)?;
        
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
    }
    
    Ok(())
}

fn handle_validate(
    _path: std::path::PathBuf,
    _types: Option<Vec<String>>,
    _fix: bool,
) -> syncable_cli::Result<()> {
    println!("🔍 Validating IaC files...");
    println!("⚠️  Validation feature is not yet implemented.");
    Ok(())
}

fn handle_support(
    languages: bool,
    frameworks: bool,
    _detailed: bool,
) -> syncable_cli::Result<()> {
    if languages || (!languages && !frameworks) {
        println!("🌐 Supported Languages:");
        println!("├── Rust");
        println!("├── JavaScript/TypeScript");
        println!("├── Python");
        println!("├── Go");
        println!("├── Java");
        println!("└── (More coming soon...)");
    }
    
    if frameworks || (!languages && !frameworks) {
        println!("\n🚀 Supported Frameworks:");
        println!("├── Web: Express.js, Next.js, React, Vue.js, Actix Web");
        println!("├── Database: PostgreSQL, MySQL, MongoDB, Redis");
        println!("├── Build Tools: npm, yarn, cargo, maven, gradle");
        println!("└── (More coming soon...)");
    }
    
    Ok(())
}

async fn handle_dependencies(
    path: std::path::PathBuf,
    licenses: bool,
    vulnerabilities: bool,
    _prod_only: bool,
    _dev_only: bool,
    format: OutputFormat,
) -> syncable_cli::Result<()> {
    let project_path = path.canonicalize()
        .unwrap_or_else(|_| path.clone());
    
    println!("🔍 Analyzing dependencies: {}", project_path.display());
    
    // First, analyze the project to detect languages
    let analysis = analyzer::analyze_project(&project_path)?;
    
    // Then perform detailed dependency analysis
    let dep_analysis = analyzer::dependency_parser::parse_detailed_dependencies(
        &project_path,
        &analysis.languages,
        &analyzer::AnalysisConfig::default(),
    )?;
    
    if format == OutputFormat::Table {
        // Table output
        use termcolor::{ColorChoice, StandardStream, WriteColor, ColorSpec, Color};
        
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        
        // Print summary
        println!("\n📦 Dependency Analysis Report");
        println!("{}", "=".repeat(80));
        
        let total_deps: usize = dep_analysis.dependencies.len();
        println!("Total dependencies: {}", total_deps);
        
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
            // License summary
            println!("\n📋 License Summary");
            println!("{}", "-".repeat(80));
            
            use std::collections::HashMap;
            let mut license_counts: HashMap<String, usize> = HashMap::new();
            
            for (_name, info) in &dep_analysis.dependencies {
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
        
        if vulnerabilities {
            println!("\n🔍 Checking for vulnerabilities...");
            
            // Convert DetailedDependencyMap to the format expected by VulnerabilityChecker
            let mut deps_by_language: HashMap<analyzer::dependency_parser::Language, Vec<analyzer::dependency_parser::DependencyInfo>> = HashMap::new();
            
            // Group dependencies by detected languages
            for language in &analysis.languages {
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
            match checker.check_all_dependencies(&deps_by_language, &project_path).await {
                Ok(report) => {
                    println!("\n🛡️ Vulnerability Report");
                    println!("{}", "-".repeat(80));
                    println!("Checked at: {}", report.checked_at.format("%Y-%m-%d %H:%M:%S UTC"));
                    println!("Total vulnerabilities: {}", report.total_vulnerabilities);
                    
                    if report.total_vulnerabilities > 0 {
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
        }
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

async fn handle_vulnerabilities(
    path: std::path::PathBuf,
    severity: Option<SeverityThreshold>,
    format: OutputFormat,
    output: Option<std::path::PathBuf>,
) -> syncable_cli::Result<()> {
    let project_path = path.canonicalize()
        .unwrap_or_else(|_| path.clone());
    
    println!("🔍 Scanning for vulnerabilities in: {}", project_path.display());
    
    // Parse dependencies
    let dependencies = analyzer::dependency_parser::DependencyParser::new().parse_all_dependencies(&project_path)?;
    
    if dependencies.is_empty() {
        println!("No dependencies found to check.");
        return Ok(());
    }
    
    // Check vulnerabilities
    let checker = analyzer::vulnerability_checker::VulnerabilityChecker::new();
    let report = checker.check_all_dependencies(&dependencies, &project_path).await
        .map_err(|e| syncable_cli::error::IaCGeneratorError::Analysis(
            syncable_cli::error::AnalysisError::DependencyParsing {
                file: "vulnerability check".to_string(),
                reason: e.to_string(),
            }
        ))?;
    
    // Filter by severity if requested
    let filtered_report = if let Some(threshold) = severity {
        let min_severity = match threshold {
            SeverityThreshold::Low => VulnerabilitySeverity::Low,
            SeverityThreshold::Medium => VulnerabilitySeverity::Medium,
            SeverityThreshold::High => VulnerabilitySeverity::High,
            SeverityThreshold::Critical => VulnerabilitySeverity::Critical,
        };
        
        let filtered_deps: Vec<_> = report.vulnerable_dependencies
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
        
        use analyzer::vulnerability_checker::VulnerabilityReport;
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
                    VulnerabilitySeverity::Info => {},
                }
            }
        }
        
        filtered
    } else {
        report
    };
    
    // Format output
    let output_string = match format {
        OutputFormat::Table => {
            // Color formatting for output

            
            let mut output = String::new();
            
            output.push_str(&format!("\n🛡️  Vulnerability Scan Report\n"));
            output.push_str(&format!("{}\n", "=".repeat(80)));
            output.push_str(&format!("Scanned at: {}\n", filtered_report.checked_at.format("%Y-%m-%d %H:%M:%S UTC")));
            output.push_str(&format!("Path: {}\n", project_path.display()));
            
            if let Some(threshold) = severity {
                output.push_str(&format!("Severity filter: >= {:?}\n", threshold));
            }
            
            output.push_str(&format!("\nSummary:\n"));
            output.push_str(&format!("Total vulnerabilities: {}\n", filtered_report.total_vulnerabilities));
            
            if filtered_report.total_vulnerabilities > 0 {
                output.push_str("\nBy Severity:\n");
                if filtered_report.critical_count > 0 {
                    output.push_str(&format!("  🔴 CRITICAL: {}\n", filtered_report.critical_count));
                }
                if filtered_report.high_count > 0 {
                    output.push_str(&format!("  🔴 HIGH: {}\n", filtered_report.high_count));
                }
                if filtered_report.medium_count > 0 {
                    output.push_str(&format!("  🟡 MEDIUM: {}\n", filtered_report.medium_count));
                }
                if filtered_report.low_count > 0 {
                    output.push_str(&format!("  🔵 LOW: {}\n", filtered_report.low_count));
                }
                
                output.push_str(&format!("\n{}\n", "-".repeat(80)));
                output.push_str("Vulnerable Dependencies:\n\n");
                
                for vuln_dep in &filtered_report.vulnerable_dependencies {
                    output.push_str(&format!("📦 {} v{} ({})\n", 
                        vuln_dep.name, 
                        vuln_dep.version,
                        vuln_dep.language.as_str()
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
                    output.push_str("\n");
                }
            } else {
                output.push_str("\n✅ No vulnerabilities found!\n");
            }
            
            output
        }
        OutputFormat::Json => {
            serde_json::to_string_pretty(&filtered_report)?
        }
    };
    
    // Output results
    if let Some(output_path) = output {
        std::fs::write(&output_path, output_string)?;
        println!("Report saved to: {}", output_path.display());
    } else {
        println!("{}", output_string);
    }
    
    // Exit with non-zero code if critical/high vulnerabilities found
    if filtered_report.critical_count > 0 || filtered_report.high_count > 0 {
        std::process::exit(1);
    }
    
    Ok(())
}
