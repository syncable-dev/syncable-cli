use clap::Parser;
use syncable_cli::{
    analyzer::{
        self, vulnerability_checker::VulnerabilitySeverity, DetectedTechnology, TechnologyCategory, LibraryType, 
        analyze_monorepo, ProjectCategory,
        // Import new modular security types
        security::{TurboSecurityAnalyzer, TurboConfig, ScanMode},
    },
    cli::{Cli, Commands, ToolsCommand, OutputFormat, SeverityThreshold, DisplayFormat, SecurityScanMode},
    config,
    generator,
};

// Use alias for the turbo SecuritySeverity to avoid conflicts
use syncable_cli::analyzer::security::SecuritySeverity as TurboSecuritySeverity;
use syncable_cli::analyzer::display::{display_analysis, DisplayMode, BoxDrawer};
use std::process;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, Duration};
use dirs::cache_dir;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run() -> syncable_cli::Result<()> {
    let cli = Cli::parse();
    
    // Handle update cache clearing
    if cli.clear_update_cache {
        clear_update_cache();
        println!("✅ Update cache cleared. Checking for updates now...");
    }
    
    check_for_update().await;
    
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
        Commands::Analyze { path, json, detailed, display, only } => {
            handle_analyze(path, json, detailed, display, only)
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
        Commands::Security { 
            path, 
            mode,
            include_low, 
            no_secrets, 
            no_code_patterns, 
            no_infrastructure, 
            no_compliance, 
            frameworks, 
            format, 
            output, 
            fail_on_findings 
        } => {
            handle_security(
                path, 
                mode,
                include_low, 
                no_secrets, 
                no_code_patterns, 
                no_infrastructure, 
                no_compliance, 
                frameworks, 
                format, 
                output, 
                fail_on_findings
            )
        }
        Commands::Tools { command } => {
            handle_tools(command).await
        }
    };
    
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
    
    Ok(())
}

fn clear_update_cache() {
    let cache_dir_path = cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("syncable-cli");
    let cache_file = cache_dir_path.join("version_cache.json");
    
    if cache_file.exists() {
        match fs::remove_file(&cache_file) {
            Ok(_) => {
                if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                    eprintln!("🗑️  Removed update cache file: {}", cache_file.display());
                }
            }
            Err(e) => {
                eprintln!("⚠️  Failed to remove update cache: {}", e);
            }
        }
    } else {
        if std::env::var("SYNC_CTL_DEBUG").is_ok() {
            eprintln!("🗑️  No update cache file found at: {}", cache_file.display());
        }
    }
}

async fn check_for_update() {
    let cache_dir_path = cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("syncable-cli");
    let cache_file = cache_dir_path.join("version_cache.json");
    let now = SystemTime::now();

    // Smart cache system: only cache when no update is available
    // Check every 2 hours when no update was found, immediately when an update might be available
    let should_check = if let Ok(metadata) = fs::metadata(&cache_file) {
        if let Ok(modified) = metadata.modified() {
            let cache_duration = now.duration_since(modified).unwrap_or(Duration::ZERO);
            
            // Read cached data to determine cache strategy
            if let Ok(cache_content) = fs::read_to_string(&cache_file) {
                if let Ok(cache_data) = serde_json::from_str::<serde_json::Value>(&cache_content) {
                    let cached_latest = cache_data["latest_version"].as_str().unwrap_or("");
                    let current = env!("CARGO_PKG_VERSION");
                    
                    // If cached version is newer than current, check immediately
                    if !cached_latest.is_empty() && is_version_newer(current, cached_latest) {
                        if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                            eprintln!("🔍 Update available in cache, showing immediately");
                        }
                        show_update_notification(current, cached_latest);
                        return;
                    }
                    
                    // If no update in cache, check every 2 hours
                    cache_duration >= Duration::from_secs(60 * 60 * 2)
                } else {
                    true // Invalid cache, check now
                }
            } else {
                true // Can't read cache, check now
            }
        } else {
            true // Can't get modified time, check now
        }
    } else {
        true // No cache file, check now
    };

    if !should_check {
        if std::env::var("SYNC_CTL_DEBUG").is_ok() {
            eprintln!("🔍 Update check skipped - checked recently and no update available");
        }
        return;
    }

    // Debug logging
    if std::env::var("SYNC_CTL_DEBUG").is_ok() {
        eprintln!("🔍 Checking for updates...");
    }

    // Query GitHub releases API
    let client = reqwest::Client::builder()
        .user_agent(format!("syncable-cli/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(5))
        .build();
    
    match client {
        Ok(client) => {
            let result = client
                .get("https://api.github.com/repos/syncable-dev/syncable-cli/releases/latest")
                .send()
                .await;
                
            match result {
                Ok(response) => {
                    if !response.status().is_success() {
                        if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                            eprintln!("⚠️  GitHub API returned status: {}", response.status());
                        }
                        return;
                    }
                    
                    match response.json::<serde_json::Value>().await {
                        Ok(json) => {
                            let latest = json["tag_name"].as_str().unwrap_or("")
                                .trim_start_matches('v'); // Remove 'v' prefix if present
                            let current = env!("CARGO_PKG_VERSION");
                            
                            if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                                eprintln!("📦 Current version: {}, Latest version: {}", current, latest);
                            }
                            
                            // Update cache with latest version info
                            let cache_data = serde_json::json!({
                                "latest_version": latest,
                                "current_version": current,
                                "checked_at": now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
                                "update_available": is_version_newer(current, latest)
                            });
                            
                            let _ = fs::create_dir_all(&cache_dir_path);
                            let _ = fs::write(&cache_file, serde_json::to_string_pretty(&cache_data).unwrap_or_default());
                            
                            // Show update notification if newer version is available
                            if !latest.is_empty() && latest != current && is_version_newer(current, latest) {
                                show_update_notification(current, latest);
                            }
                        }
                        Err(e) => {
                            if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                                eprintln!("⚠️  Failed to parse GitHub API response: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                        eprintln!("⚠️  Failed to check for updates: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                eprintln!("⚠️  Failed to create HTTP client: {}", e);
            }
        }
    }
}

fn show_update_notification(current: &str, latest: &str) {
    use colored::*;
    
    let mut box_drawer = BoxDrawer::new(&"UPDATE AVAILABLE".bright_red().bold().to_string());
    
    // Version info line with prominent colors
    let version_info = format!("New version: {} | Current: {}", 
                              latest.bright_green().bold(), 
                              current.bright_red());
    box_drawer.add_value_only(&version_info);
    
    // Empty line for spacing
    box_drawer.add_value_only("");
    
    // Instructions header with emphasis
    box_drawer.add_value_only(&"To update, run one of these commands:".bright_cyan().bold().to_string());
    box_drawer.add_value_only("");
    
    // Recommended method - highlighted as primary option
    box_drawer.add_line(&"RECOMMENDED".bright_green().bold().to_string(), &"(via Cargo)".green().to_string(), false);
    let cargo_cmd = "cargo install syncable-cli".bright_white().on_blue().bold().to_string();
    box_drawer.add_value_only(&format!("  {}", cargo_cmd));
    box_drawer.add_value_only("");
    
    // Alternative method - neutral coloring
    box_drawer.add_line(&"ALTERNATIVE".yellow().bold().to_string(), &"(direct download)".yellow().to_string(), false);
    let github_url = format!("  Visit: {}", 
                            format!("github.com/syncable-dev/syncable-cli/releases/v{}", latest).bright_blue().underline());
    box_drawer.add_value_only(&github_url);
    box_drawer.add_value_only("");
    
    // Install script method - secondary option
    box_drawer.add_line(&"SCRIPT".magenta().bold().to_string(), &"(automated installer)".magenta().to_string(), false);
    let script_cmd = "curl -sSL install.syncable.dev | sh".bright_white().on_magenta().bold().to_string();
    box_drawer.add_value_only(&format!("  {}", script_cmd));
    
    // Add a helpful note
    box_drawer.add_value_only("");
    box_drawer.add_value_only(&"Tip: The Cargo method is fastest for existing Rust users".dimmed().italic().to_string());
    
    println!("\n{}", box_drawer.draw());
}

// Helper function to compare semantic versions
fn is_version_newer(current: &str, latest: &str) -> bool {
    let current_parts: Vec<u32> = current.split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let latest_parts: Vec<u32> = latest.split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    for i in 0..3 {
        let current_part = current_parts.get(i).unwrap_or(&0);
        let latest_part = latest_parts.get(i).unwrap_or(&0);
        
        if latest_part > current_part {
            return true;
        } else if latest_part < current_part {
            return false;
        }
    }
    
    false
}

fn handle_analyze(
    path: std::path::PathBuf,
    json: bool,
    detailed: bool,
    display: Option<DisplayFormat>,
    _only: Option<Vec<String>>,
) -> syncable_cli::Result<()> {
    println!("🔍 Analyzing project: {}", path.display());
    
    let monorepo_analysis = analyze_monorepo(&path)?;
    
    if json {
        display_analysis(&monorepo_analysis, DisplayMode::Json);
    } else {
        // Determine display mode
        let mode = if detailed {
            // Legacy flag for backward compatibility
            DisplayMode::Detailed
        } else {
            match display {
                Some(DisplayFormat::Matrix) | None => DisplayMode::Matrix,
                Some(DisplayFormat::Detailed) => DisplayMode::Detailed,
                Some(DisplayFormat::Summary) => DisplayMode::Summary,
            }
        };
        
        display_analysis(&monorepo_analysis, mode);
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
    
    let monorepo_analysis = analyze_monorepo(&path)?;
    
    println!("✅ Analysis complete. Generating IaC files...");
    
    if monorepo_analysis.is_monorepo {
        println!("📦 Detected monorepo with {} projects", monorepo_analysis.projects.len());
        println!("🚧 Monorepo IaC generation is coming soon! For now, generating for the overall structure.");
        println!("💡 Tip: You can run generate commands on individual project directories for now.");
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
        // Table output
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
            for language in &all_languages {
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

/// Display technologies in detailed format with proper categorization
fn display_technologies_detailed(technologies: &[DetectedTechnology]) {
    if technologies.is_empty() {
        println!("\n🛠️  Technologies Detected: None");
        return;
    }

    // Group technologies by IaC-relevant categories
    let mut meta_frameworks = Vec::new();
    let mut backend_frameworks = Vec::new();
    let mut frontend_frameworks = Vec::new();
    let mut ui_libraries = Vec::new();
    let mut build_tools = Vec::new();
    let mut databases = Vec::new();
    let mut testing = Vec::new();
    let mut runtimes = Vec::new();
    let mut other_libraries = Vec::new();

    for tech in technologies {
        match &tech.category {
            TechnologyCategory::MetaFramework => meta_frameworks.push(tech),
            TechnologyCategory::BackendFramework => backend_frameworks.push(tech),
            TechnologyCategory::FrontendFramework => frontend_frameworks.push(tech),
            TechnologyCategory::Library(lib_type) => match lib_type {
                LibraryType::UI => ui_libraries.push(tech),
                _ => other_libraries.push(tech),
            },
            TechnologyCategory::BuildTool => build_tools.push(tech),
            TechnologyCategory::Database => databases.push(tech),
            TechnologyCategory::Testing => testing.push(tech),
            TechnologyCategory::Runtime => runtimes.push(tech),
            _ => other_libraries.push(tech),
        }
    }

    println!("\n🛠️  Technology Stack:");
    
    // Primary Framework (highlighted)
    if let Some(primary) = technologies.iter().find(|t| t.is_primary) {
        println!("   🎯 PRIMARY: {} (confidence: {:.1}%)", primary.name, primary.confidence * 100.0);
        println!("      Architecture driver for this project");
    }

    // Meta-frameworks
    if !meta_frameworks.is_empty() {
        println!("\n   🏗️  Meta-Frameworks:");
        for tech in meta_frameworks {
            println!("      • {} (confidence: {:.1}%)", tech.name, tech.confidence * 100.0);
        }
    }

    // Backend frameworks
    if !backend_frameworks.is_empty() {
        println!("\n   🖥️  Backend Frameworks:");
        for tech in backend_frameworks {
            println!("      • {} (confidence: {:.1}%)", tech.name, tech.confidence * 100.0);
        }
    }

    // Frontend frameworks
    if !frontend_frameworks.is_empty() {
        println!("\n   🌐 Frontend Frameworks:");
        for tech in frontend_frameworks {
            println!("      • {} (confidence: {:.1}%)", tech.name, tech.confidence * 100.0);
        }
    }

    // UI Libraries
    if !ui_libraries.is_empty() {
        println!("\n   🎨 UI Libraries:");
        for tech in ui_libraries {
            println!("      • {} (confidence: {:.1}%)", tech.name, tech.confidence * 100.0);
        }
    }

    // Note: Removed utility library categories (Data Fetching, Routing, State Management)
    // as they don't provide value for IaC generation

    // Build Tools
    if !build_tools.is_empty() {
        println!("\n   🔨 Build Tools:");
        for tech in build_tools {
            println!("      • {} (confidence: {:.1}%)", tech.name, tech.confidence * 100.0);
        }
    }

    // Databases
    if !databases.is_empty() {
        println!("\n   🗃️  Database & ORM:");
        for tech in databases {
            println!("      • {} (confidence: {:.1}%)", tech.name, tech.confidence * 100.0);
        }
    }

    // Testing
    if !testing.is_empty() {
        println!("\n   🧪 Testing:");
        for tech in testing {
            println!("      • {} (confidence: {:.1}%)", tech.name, tech.confidence * 100.0);
        }
    }

    // Runtimes
    if !runtimes.is_empty() {
        println!("\n   ⚡ Runtimes:");
        for tech in runtimes {
            println!("      • {} (confidence: {:.1}%)", tech.name, tech.confidence * 100.0);
        }
    }

    // Other Libraries
    if !other_libraries.is_empty() {
        println!("\n   📚 Other Libraries:");
        for tech in other_libraries {
            println!("      • {} (confidence: {:.1}%)", tech.name, tech.confidence * 100.0);
        }
    }
}

/// Display technologies in summary format for simple view
fn display_technologies_summary(technologies: &[DetectedTechnology]) {
    println!("├── Technologies detected: {}", technologies.len());
    
    // Show primary technology first
    if let Some(primary) = technologies.iter().find(|t| t.is_primary) {
        println!("│   ├── 🎯 {} (PRIMARY, {:.1}%)", primary.name, primary.confidence * 100.0);
    }
    
    // Show other technologies
    for tech in technologies.iter().filter(|t| !t.is_primary) {
        let icon = match &tech.category {
            TechnologyCategory::MetaFramework => "🏗️",
            TechnologyCategory::BackendFramework => "🖥️",
            TechnologyCategory::FrontendFramework => "🌐",
            TechnologyCategory::Library(LibraryType::UI) => "🎨",
            TechnologyCategory::BuildTool => "🔨",
            TechnologyCategory::Database => "🗃️",
            TechnologyCategory::Testing => "🧪",
            TechnologyCategory::Runtime => "⚡",
            _ => "📚",
        };
        println!("│   ├── {} {} (confidence: {:.1}%)", icon, tech.name, tech.confidence * 100.0);
    }
}

fn handle_security(
    path: std::path::PathBuf,
    mode: SecurityScanMode,
    include_low: bool,
    no_secrets: bool,
    no_code_patterns: bool,
    no_infrastructure: bool,
    no_compliance: bool,
    frameworks: Vec<String>,
    format: OutputFormat,
    output: Option<std::path::PathBuf>,
    fail_on_findings: bool,
) -> syncable_cli::Result<()> {
    let project_path = path.canonicalize()
        .unwrap_or_else(|_| path.clone());
    
    println!("🛡️  Running security analysis on: {}", project_path.display());
    
    // Convert CLI mode to internal ScanMode, with flag overrides
    let scan_mode = if no_secrets && no_code_patterns {
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
    };
    
    // Configure turbo analyzer
    let config = TurboConfig {
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
    };
    
    // Initialize and run analyzer
    let analyzer = TurboSecurityAnalyzer::new(config)
        .map_err(|e| syncable_cli::error::IaCGeneratorError::Analysis(
            syncable_cli::error::AnalysisError::InvalidStructure(
                format!("Failed to create turbo security analyzer: {}", e)
            )
        ))?;
    
    let start_time = std::time::Instant::now();
    let security_report = analyzer.analyze_project(&project_path)
        .map_err(|e| syncable_cli::error::IaCGeneratorError::Analysis(
            syncable_cli::error::AnalysisError::InvalidStructure(
                format!("Turbo security analysis failed: {}", e)
            )
        ))?;
    let scan_duration = start_time.elapsed();
    
    println!("⚡ Scan completed in {:.2}s", scan_duration.as_secs_f64());
    
    // Format output in the beautiful style requested
    let output_string = match format {
        OutputFormat::Table => {
            use syncable_cli::analyzer::display::BoxDrawer;
            use colored::*;
            
            let mut output = String::new();
            
            // Header
            output.push_str(&format!("\n{}\n", "🛡️  Security Analysis Results".bright_white().bold()));
            output.push_str(&format!("{}\n", "═".repeat(80).bright_blue()));
            
            // Security Score Box
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
            
            // Analysis scope
            let config_files = security_report.findings.iter()
                .filter_map(|f| f.file_path.as_ref())
                .collect::<std::collections::HashSet<_>>()
                .len();
            score_box.add_line("Files Analyzed:", &config_files.max(1).to_string().green(), true);
            score_box.add_line("Scan Mode:", &format!("{:?}", scan_mode).green(), true);
            
            output.push_str(&format!("\n{}\n", score_box.draw()));
            
            // Findings in Card Format  
            if !security_report.findings.is_empty() {
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
                    let file_display = if let Some(file_path) = &finding.file_path {
                        // Cross-platform path normalization
                        let canonical_file = file_path.canonicalize().unwrap_or_else(|_| file_path.clone());
                        let canonical_project = path.canonicalize().unwrap_or_else(|_| path.clone());
                        
                        // Try to calculate relative path from project root
                        if let Ok(relative_path) = canonical_file.strip_prefix(&canonical_project) {
                            // Use forward slashes for consistency across platforms
                            let relative_str = relative_path.to_string_lossy().replace('\\', "/");
                            format!("./{}", relative_str)
                        } else {
                            // Fallback: try to find any common ancestor or use absolute path
                            let path_str = file_path.to_string_lossy();
                            if path_str.starts_with('/') {
                                // For absolute paths, try to extract meaningful relative portion
                                if let Some(project_name) = path.file_name().and_then(|n| n.to_str()) {
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
                                        // Last resort: show the full path
                                        path_str.to_string()
                                    }
                                } else {
                                    // Show full path if we can't determine project context
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
                    } else {
                        "N/A".to_string()
                    };
                    
                    // Parse gitignore status from description (clean colored text)
                    let gitignore_status = if finding.description.contains("is tracked by git") {
                        "TRACKED".bright_red().bold()
                    } else if finding.description.contains("is NOT in .gitignore") {
                        "EXPOSED".yellow().bold()
                    } else if finding.description.contains("is protected") || finding.description.contains("properly ignored") {
                        "SAFE".bright_green().bold()
                    } else if finding.description.contains("appears safe") {
                        "OK".bright_blue().bold()
                    } else {
                        "UNKNOWN".dimmed()
                    };
                    
                    // Determine finding type
                    let finding_type = if finding.title.contains("Environment Variable") {
                        "ENV VAR"
                    } else if finding.title.contains("Secret File") {
                        "SECRET FILE"
                    } else if finding.title.contains("API Key") || finding.title.contains("Stripe") || finding.title.contains("Firebase") {
                        "API KEY"
                    } else if finding.title.contains("Configuration") {
                        "CONFIG"
                    } else {
                        "OTHER"
                    };
                    
                    // Format position as "line:column" or just "line" if no column info
                    let position_display = match (finding.line_number, finding.column_number) {
                        (Some(line), Some(col)) => format!("{}:{}", line, col),
                        (Some(line), None) => format!("{}", line),
                        _ => "—".to_string(),
                    };
                    
                    // Card format: File path with intelligent display based on terminal width
                    let box_margin = 6; // Account for box borders and padding
                    let available_width = terminal_width.saturating_sub(box_margin);
                    let max_path_width = available_width.saturating_sub(20); // Leave space for numbering and spacing
                    
                    if file_display.len() + 3 <= max_path_width {
                        // Path fits on one line with numbering
                        findings_box.add_value_only(&format!("{}. {}", 
                            format!("{}", i + 1).bright_white().bold(),
                            file_display.cyan().bold()
                        ));
                    } else if file_display.len() <= available_width.saturating_sub(4) {
                        // Path fits on its own line with indentation
                        findings_box.add_value_only(&format!("{}.", 
                            format!("{}", i + 1).bright_white().bold()
                        ));
                        findings_box.add_value_only(&format!("   {}", 
                            file_display.cyan().bold()
                        ));
                    } else {
                        // Path is extremely long - use smart wrapping
                        findings_box.add_value_only(&format!("{}.", 
                            format!("{}", i + 1).bright_white().bold()
                        ));
                        
                        // Smart path wrapping - prefer breaking at directory separators
                        let wrap_width = available_width.saturating_sub(4);
                        let mut remaining = file_display.as_str();
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
                
                output.push_str(&format!("\n{}\n", findings_box.draw()));
                
                // GitIgnore Status Legend  
                let mut legend_box = BoxDrawer::new("Git Status Legend");
                legend_box.add_line(&"TRACKED:".bright_red().bold().to_string(), "File is tracked by git - CRITICAL RISK", false);
                legend_box.add_line(&"EXPOSED:".yellow().bold().to_string(), "File contains secrets but not in .gitignore", false);
                legend_box.add_line(&"SAFE:".bright_green().bold().to_string(), "File is properly ignored by .gitignore", false);
                legend_box.add_line(&"OK:".bright_blue().bold().to_string(), "File appears safe for version control", false);
                output.push_str(&format!("\n{}\n", legend_box.draw()));
            } else {
                let mut no_findings_box = BoxDrawer::new("Security Status");
                no_findings_box.add_value_only(&"✅ No security issues detected".green());
                no_findings_box.add_value_only("💡 Regular security scanning recommended");
                output.push_str(&format!("\n{}\n", no_findings_box.draw()));
            }
            
            // Recommendations Box
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
            output.push_str(&format!("\n{}\n", rec_box.draw()));
            
            output
        }
        OutputFormat::Json => {
            serde_json::to_string_pretty(&security_report)?
        }
    };
    
    // Output results
    if let Some(output_path) = output {
        std::fs::write(&output_path, output_string)?;
        println!("Security report saved to: {}", output_path.display());
    } else {
        print!("{}", output_string);
    }
    
    // Exit with error code if requested and findings exist
    if fail_on_findings && security_report.total_findings > 0 {
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
    
    Ok(())
}

async fn handle_tools(command: ToolsCommand) -> syncable_cli::Result<()> {
    use syncable_cli::analyzer::{tool_installer::ToolInstaller, dependency_parser::Language};
    use std::collections::HashMap;
    use termcolor::{ColorChoice, StandardStream, WriteColor, ColorSpec, Color};
    
    match command {
        ToolsCommand::Status { format, languages } => {
            let installer = ToolInstaller::new();
            
            // Determine which languages to check
            let langs_to_check = if let Some(lang_names) = languages {
                lang_names.iter()
                    .filter_map(|name| Language::from_string(name))
                    .collect()
            } else {
                vec![
                    Language::Rust,
                    Language::JavaScript,
                    Language::TypeScript,
                    Language::Python,
                    Language::Go,
                    Language::Java,
                    Language::Kotlin,
                ]
            };
            
            println!("🔧 Checking vulnerability scanning tools status...\n");
            
            match format {
                OutputFormat::Table => {
                    let mut stdout = StandardStream::stdout(ColorChoice::Always);
                    
                    println!("📋 Vulnerability Scanning Tools Status");
                    println!("{}", "=".repeat(50));
                    
                    for language in &langs_to_check {
                        let (tool_name, is_available) = match language {
                            Language::Rust => ("cargo-audit", installer.test_tool_availability("cargo-audit")),
                            Language::JavaScript | Language::TypeScript => ("npm", installer.test_tool_availability("npm")),
                            Language::Python => ("pip-audit", installer.test_tool_availability("pip-audit")),
                            Language::Go => ("govulncheck", installer.test_tool_availability("govulncheck")),
                            Language::Java | Language::Kotlin => ("grype", installer.test_tool_availability("grype")),
                            _ => continue,
                        };
                        
                        print!("  {} {:?}: ", 
                               if is_available { "✅" } else { "❌" }, 
                               language);
                        
                        if is_available {
                            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                            print!("{} installed", tool_name);
                        } else {
                            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                            print!("{} missing", tool_name);
                        }
                        
                        stdout.reset()?;
                        println!();
                    }
                    
                    // Check universal tools
                    println!("\n🔍 Universal Scanners:");
                    let grype_available = installer.test_tool_availability("grype");
                    print!("  {} Grype: ", if grype_available { "✅" } else { "❌" });
                    if grype_available {
                        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                        println!("installed");
                    } else {
                        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                        println!("missing");
                    }
                    stdout.reset()?;
                }
                OutputFormat::Json => {
                    let mut status = HashMap::new();
                    
                    for language in &langs_to_check {
                        let (tool_name, is_available) = match language {
                            Language::Rust => ("cargo-audit", installer.test_tool_availability("cargo-audit")),
                            Language::JavaScript | Language::TypeScript => ("npm", installer.test_tool_availability("npm")),
                            Language::Python => ("pip-audit", installer.test_tool_availability("pip-audit")),
                            Language::Go => ("govulncheck", installer.test_tool_availability("govulncheck")),
                            Language::Java | Language::Kotlin => ("grype", installer.test_tool_availability("grype")),
                            _ => continue,
                        };
                        
                        status.insert(format!("{:?}", language), serde_json::json!({
                            "tool": tool_name,
                            "available": is_available
                        }));
                    }
                    
                    println!("{}", serde_json::to_string_pretty(&status)?);
                }
            }
        }
        
        ToolsCommand::Install { languages, include_owasp, dry_run, yes } => {
            let mut installer = ToolInstaller::new();
            
            // Determine which languages to install tools for
            let langs_to_install = if let Some(lang_names) = languages {
                lang_names.iter()
                    .filter_map(|name| Language::from_string(name))
                    .collect()
            } else {
                vec![
                    Language::Rust,
                    Language::JavaScript,
                    Language::TypeScript,
                    Language::Python,
                    Language::Go,
                    Language::Java,
                ]
            };
            
            if dry_run {
                println!("🔍 Dry run: Tools that would be installed:");
                println!("{}", "=".repeat(50));
                
                for language in &langs_to_install {
                    let (tool_name, is_available) = match language {
                        Language::Rust => ("cargo-audit", installer.test_tool_availability("cargo-audit")),
                        Language::JavaScript | Language::TypeScript => ("npm", installer.test_tool_availability("npm")),
                        Language::Python => ("pip-audit", installer.test_tool_availability("pip-audit")),
                        Language::Go => ("govulncheck", installer.test_tool_availability("govulncheck")),
                        Language::Java | Language::Kotlin => ("grype", installer.test_tool_availability("grype")),
                        _ => continue,
                    };
                    
                    if !is_available {
                        println!("  📦 Would install {} for {:?}", tool_name, language);
                    } else {
                        println!("  ✅ {} already installed for {:?}", tool_name, language);
                    }
                }
                
                if include_owasp && !installer.test_tool_availability("dependency-check") {
                    println!("  📦 Would install OWASP Dependency Check (large download)");
                }
                
                return Ok(());
            }
            
            if !yes {
                use std::io::{self, Write};
                print!("🔧 Install missing vulnerability scanning tools? [y/N]: ");
                io::stdout().flush()?;
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                
                if !input.trim().to_lowercase().starts_with('y') {
                    println!("Installation cancelled.");
                    return Ok(());
                }
            }
            
            println!("🛠️  Installing vulnerability scanning tools...");
            
            match installer.ensure_tools_for_languages(&langs_to_install) {
                Ok(()) => {
                    println!("✅ Tool installation completed!");
                    installer.print_tool_status(&langs_to_install);
                    
                    // Show PATH instructions if needed
                    println!("\n💡 Setup Instructions:");
                    println!("  • Add ~/.local/bin to your PATH for manually installed tools");
                    println!("  • Add ~/go/bin to your PATH for Go tools");
                    println!("  • Add to your shell profile (~/.bashrc, ~/.zshrc, etc.):");
                    println!("    export PATH=\"$HOME/.local/bin:$HOME/go/bin:$PATH\"");
                }
                Err(e) => {
                    eprintln!("❌ Tool installation failed: {}", e);
                    eprintln!("\n🔧 Manual installation may be required for some tools.");
                    eprintln!("   Run 'sync-ctl tools guide' for manual installation instructions.");
                    return Err(e);
                }
            }
        }
        
        ToolsCommand::Verify { languages, verbose } => {
            let installer = ToolInstaller::new();
            
            // Determine which languages to verify
            let langs_to_verify = if let Some(lang_names) = languages {
                lang_names.iter()
                    .filter_map(|name| Language::from_string(name))
                    .collect()
            } else {
                vec![
                    Language::Rust,
                    Language::JavaScript,
                    Language::TypeScript,
                    Language::Python,
                    Language::Go,
                    Language::Java,
                ]
            };
            
            println!("🔍 Verifying vulnerability scanning tools...\n");
            
            let mut all_working = true;
            
            for language in &langs_to_verify {
                let (tool_name, is_working) = match language {
                    Language::Rust => {
                        let working = installer.test_tool_availability("cargo-audit");
                        ("cargo-audit", working)
                    }
                    Language::JavaScript | Language::TypeScript => {
                        let working = installer.test_tool_availability("npm");
                        ("npm", working)
                    }
                    Language::Python => {
                        let working = installer.test_tool_availability("pip-audit");
                        ("pip-audit", working)
                    }
                    Language::Go => {
                        let working = installer.test_tool_availability("govulncheck");
                        ("govulncheck", working)
                    }
                    Language::Java | Language::Kotlin => {
                        let working = installer.test_tool_availability("grype");
                        ("grype", working)
                    }
                    _ => continue,
                };
                
                print!("  {} {:?}: {}", 
                       if is_working { "✅" } else { "❌" }, 
                       language,
                       tool_name);
                
                if is_working {
                    println!(" - working correctly");
                    
                    if verbose {
                        // Try to get version info
                        use std::process::Command;
                        let version_result = match tool_name {
                            "cargo-audit" => Command::new("cargo").args(&["audit", "--version"]).output(),
                            "npm" => Command::new("npm").arg("--version").output(),
                            "pip-audit" => Command::new("pip-audit").arg("--version").output(),
                            "govulncheck" => Command::new("govulncheck").arg("-version").output(),
                            "grype" => Command::new("grype").arg("version").output(),
                            _ => continue,
                        };
                        
                        if let Ok(output) = version_result {
                            if output.status.success() {
                                let version = String::from_utf8_lossy(&output.stdout);
                                println!("    Version: {}", version.trim());
                            }
                        }
                    }
                } else {
                    println!(" - not working or missing");
                    all_working = false;
                }
            }
            
            if all_working {
                println!("\n✅ All tools are working correctly!");
            } else {
                println!("\n❌ Some tools are missing or not working.");
                println!("   Run 'sync-ctl tools install' to install missing tools.");
            }
        }
        
        ToolsCommand::Guide { languages, platform } => {
            let target_platform = platform.unwrap_or_else(|| {
                match std::env::consts::OS {
                    "macos" => "macOS".to_string(),
                    "linux" => "Linux".to_string(),
                    "windows" => "Windows".to_string(),
                    other => other.to_string(),
                }
            });
            
            println!("📚 Vulnerability Scanning Tools Installation Guide");
            println!("Platform: {}", target_platform);
            println!("{}", "=".repeat(60));
            
            let langs_to_show = if let Some(lang_names) = languages {
                lang_names.iter()
                    .filter_map(|name| Language::from_string(name))
                    .collect()
            } else {
                vec![
                    Language::Rust,
                    Language::JavaScript,
                    Language::TypeScript,
                    Language::Python,
                    Language::Go,
                    Language::Java,
                ]
            };
            
            for language in &langs_to_show {
                match language {
                    Language::Rust => {
                        println!("\n🦀 Rust - cargo-audit");
                        println!("  Install: cargo install cargo-audit");
                        println!("  Usage: cargo audit");
                    }
                    Language::JavaScript | Language::TypeScript => {
                        println!("\n🌐 JavaScript/TypeScript - npm audit");
                        println!("  Install: Download Node.js from https://nodejs.org/");
                        match target_platform.as_str() {
                            "macOS" => println!("  Package manager: brew install node"),
                            "Linux" => println!("  Package manager: sudo apt install nodejs npm (Ubuntu/Debian)"),
                            _ => {}
                        }
                        println!("  Usage: npm audit");
                    }
                    Language::Python => {
                        println!("\n🐍 Python - pip-audit");
                        println!("  Install: pipx install pip-audit (recommended)");
                        println!("  Alternative: pip3 install --user pip-audit");
                        println!("  Also available: safety (pip install safety)");
                        println!("  Usage: pip-audit");
                    }
                    Language::Go => {
                        println!("\n🐹 Go - govulncheck");
                        println!("  Install: go install golang.org/x/vuln/cmd/govulncheck@latest");
                        println!("  Note: Make sure ~/go/bin is in your PATH");
                        println!("  Usage: govulncheck ./...");
                    }
                    Language::Java => {
                        println!("\n☕ Java - Multiple options");
                        println!("  Grype (recommended):");
                        match target_platform.as_str() {
                            "macOS" => println!("    Install: brew install anchore/grype/grype"),
                            "Linux" => println!("    Install: Download from https://github.com/anchore/grype/releases"),
                            _ => println!("    Install: Download from https://github.com/anchore/grype/releases"),
                        }
                        println!("    Usage: grype .");
                        println!("  OWASP Dependency Check:");
                        match target_platform.as_str() {
                            "macOS" => println!("    Install: brew install dependency-check"),
                            _ => println!("    Install: Download from https://github.com/jeremylong/DependencyCheck/releases"),
                        }
                        println!("    Usage: dependency-check --project myproject --scan .");
                    }
                    _ => {}
                }
            }
            
            println!("\n🔍 Universal Scanners:");
            println!("  Grype: Works with multiple ecosystems");
            println!("  Trivy: Container and filesystem scanning");
            println!("  Snyk: Commercial solution with free tier");
            
            println!("\n💡 Tips:");
            println!("  • Run 'sync-ctl tools status' to check current installation");
            println!("  • Run 'sync-ctl tools install' for automatic installation");
            println!("  • Add tool directories to your PATH for easier access");
        }
    }
    
    Ok(())
}

/// Format project category for display
fn format_project_category(category: &ProjectCategory) -> &'static str {
    match category {
        ProjectCategory::Frontend => "Frontend",
        ProjectCategory::Backend => "Backend",
        ProjectCategory::Api => "API",
        ProjectCategory::Service => "Service",
        ProjectCategory::Library => "Library",
        ProjectCategory::Tool => "Tool",
        ProjectCategory::Documentation => "Documentation",
        ProjectCategory::Infrastructure => "Infrastructure",
        ProjectCategory::Unknown => "Unknown",
    }
}
