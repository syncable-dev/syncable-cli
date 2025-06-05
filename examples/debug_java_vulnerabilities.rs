use env_logger;
use log::{info, debug, error};
use syncable_cli::analyzer::dependency_parser::{DependencyParser, Language};
use syncable_cli::analyzer::vulnerability_checker::VulnerabilityChecker;
use std::path::Path;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable debug logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    // Get project path from command line args or use current directory
    let args: Vec<String> = env::args().collect();
    let project_path = if args.len() > 1 {
        Path::new(&args[1])
    } else {
        Path::new(".")
    };
    
    info!("🔍 Debug Java vulnerability scanning in: {}", project_path.display());
    
    // Parse dependencies
    let parser = DependencyParser::new();
    info!("📦 Parsing dependencies...");
    let dependencies = parser.parse_all_dependencies(project_path)?;
    
    if dependencies.is_empty() {
        error!("❌ No dependencies found!");
        info!("Make sure you're in a Java project directory with:");
        info!("  - pom.xml (Maven project)");
        info!("  - build.gradle or build.gradle.kts (Gradle project)");
        return Ok(());
    }
    
    // Show detailed dependency information
    info!("📊 Found dependencies in {} languages:", dependencies.len());
    for (lang, deps) in &dependencies {
        info!("  {:?}: {} dependencies", lang, deps.len());
        if *lang == Language::Java {
            info!("    Java dependencies details:");
            for dep in deps.iter().take(10) {
                info!("      - {} v{} (source: {:?})", dep.name, dep.version, dep.source);
            }
            if deps.len() > 10 {
                info!("      ... and {} more", deps.len() - 10);
            }
        }
    }
    
    // Check if Java dependencies were found
    if !dependencies.contains_key(&Language::Java) {
        error!("❌ No Java dependencies detected!");
        info!("Troubleshooting steps:");
        info!("1. Make sure you're in a Java project directory");
        info!("2. For Maven projects: ensure pom.xml exists and has <dependencies> section");
        info!("3. For Gradle projects: ensure build.gradle exists with dependency declarations");
        info!("4. Run 'mvn dependency:resolve' or 'gradle build' to ensure dependencies are resolved");
        return Ok(());
    }
    
    // Check vulnerabilities
    info!("🛡️ Checking for vulnerabilities...");
    let checker = VulnerabilityChecker::new();
    
    match checker.check_all_dependencies(&dependencies, project_path).await {
        Ok(report) => {
            info!("✅ Vulnerability scan completed successfully!");
            info!("📊 Results:");
            info!("  Total vulnerabilities: {}", report.total_vulnerabilities);
            info!("  Critical: {}", report.critical_count);
            info!("  High: {}", report.high_count);
            info!("  Medium: {}", report.medium_count);
            info!("  Low: {}", report.low_count);
            
            if report.total_vulnerabilities > 0 {
                info!("🚨 Vulnerable dependencies:");
                for vuln_dep in &report.vulnerable_dependencies {
                    info!("  - {} v{} ({} vulnerabilities)", 
                          vuln_dep.name, vuln_dep.version, vuln_dep.vulnerabilities.len());
                    for vuln in &vuln_dep.vulnerabilities {
                        info!("    • {} [{:?}] - {}", vuln.id, vuln.severity, vuln.title);
                    }
                }
            } else {
                info!("✅ No vulnerabilities found!");
                info!("This could mean:");
                info!("  - Your dependencies are up to date and secure");
                info!("  - The vulnerability scanner (grype) didn't find any issues");
                info!("  - The dependency versions couldn't be matched with vulnerability databases");
            }
        }
        Err(e) => {
            error!("❌ Vulnerability scanning failed: {}", e);
            info!("Common issues:");
            info!("  - grype not installed: brew install grype");
            info!("  - Project not built: run 'mvn compile' or 'gradle build'");
            info!("  - Dependencies not resolved: run 'mvn dependency:resolve'");
        }
    }
    
    Ok(())
} 