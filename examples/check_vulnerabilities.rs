use syncable_cli::analyzer::dependency_parser::{DependencyParser};
use syncable_cli::analyzer::vulnerability_checker::VulnerabilityChecker;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let project_path = Path::new(".");
    println!("🔍 Checking vulnerabilities in: {}", project_path.display());
    
    // Parse dependencies
    let parser = DependencyParser::new();
    let dependencies = parser.parse_all_dependencies(project_path)?;
    
    if dependencies.is_empty() {
        println!("No dependencies found.");
        return Ok(());
    }
    
    // Print found dependencies
    for (lang, deps) in &dependencies {
        println!("\n{:?} dependencies: {}", lang, deps.len());
        for dep in deps.iter().take(5) {
            println!("  - {} v{}", dep.name, dep.version);
        }
        if deps.len() > 5 {
            println!("  ... and {} more", deps.len() - 5);
        }
    }
    
    // Check vulnerabilities
    println!("\n🛡️ Checking for vulnerabilities...");
    let checker = VulnerabilityChecker::new();
    let report = checker.check_all_dependencies(&dependencies, project_path).await?;
    
    println!("\n📊 Vulnerability Report");
    println!("Checked at: {}", report.checked_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("Total vulnerabilities: {}", report.total_vulnerabilities);
    
    if report.total_vulnerabilities > 0 {
        println!("\nSeverity breakdown:");
        if report.critical_count > 0 {
            println!("  CRITICAL: {}", report.critical_count);
        }
        if report.high_count > 0 {
            println!("  HIGH: {}", report.high_count);
        }
        if report.medium_count > 0 {
            println!("  MEDIUM: {}", report.medium_count);
        }
        if report.low_count > 0 {
            println!("  LOW: {}", report.low_count);
        }
        
        println!("\nVulnerable dependencies:");
        for vuln_dep in &report.vulnerable_dependencies {
            println!("\n  📦 {} v{} ({:?})", vuln_dep.name, vuln_dep.version, vuln_dep.language);
            for vuln in &vuln_dep.vulnerabilities {
                println!("    ⚠️  {} [{:?}] - {}", vuln.id, vuln.severity, vuln.title);
                if let Some(ref cve) = vuln.cve {
                    println!("       CVE: {}", cve);
                }
                if let Some(ref patched) = vuln.patched_versions {
                    println!("       Fix: Upgrade to {}", patched);
                }
            }
        }
    } else {
        println!("\n✅ No known vulnerabilities found!");
    }
    
    Ok(())
} 