use std::path::Path;
use syncable_cli::analyzer::{analyze_project, SecurityAnalyzer, SecurityAnalysisConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    // Get project path from command line arguments or use current directory
    let project_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".".to_string());
    
    println!("🔍 Analyzing security for project: {}", project_path);
    
    // First perform a general project analysis
    let project_analysis = analyze_project(Path::new(&project_path))?;
    
    println!("📊 Project Analysis Summary:");
    println!("  Languages: {:?}", project_analysis.languages.iter().map(|l| &l.name).collect::<Vec<_>>());
    println!("  Technologies: {:?}", project_analysis.technologies.iter().map(|t| &t.name).collect::<Vec<_>>());
    println!("  Environment Variables: {}", project_analysis.environment_variables.len());
    
    // Create security analyzer with default configuration
    let security_config = SecurityAnalysisConfig {
        include_low_severity: true, // Include low severity findings for demonstration
        check_secrets: true,
        check_code_patterns: true,
        check_infrastructure: true,
        check_compliance: true,
        frameworks_to_check: vec![
            "SOC2".to_string(),
            "GDPR".to_string(),
            "OWASP".to_string(),
        ],
        ignore_patterns: vec![
            "node_modules".to_string(),
            ".git".to_string(),
            "target".to_string(),
        ],
    };
    
    let security_analyzer = SecurityAnalyzer::with_config(security_config)?;
    
    // Perform security analysis
    println!("\n🛡️  Running comprehensive security analysis...");
    let security_report = security_analyzer.analyze_security(&project_analysis)?;
    
    // Display results
    println!("\n📋 Security Analysis Report");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🏆 Overall Security Score: {:.1}/100", security_report.overall_score);
    println!("⚠️  Risk Level: {:?}", security_report.risk_level);
    println!("🔍 Total Findings: {}", security_report.total_findings);
    
    if !security_report.findings_by_severity.is_empty() {
        println!("\n📊 Findings by Severity:");
        for (severity, count) in &security_report.findings_by_severity {
            let emoji = match severity {
                syncable_cli::analyzer::SecuritySeverity::Critical => "🚨",
                syncable_cli::analyzer::SecuritySeverity::High => "⚠️ ",
                syncable_cli::analyzer::SecuritySeverity::Medium => "⚡",
                syncable_cli::analyzer::SecuritySeverity::Low => "ℹ️ ",
                syncable_cli::analyzer::SecuritySeverity::Info => "💡",
            };
            println!("  {} {:?}: {}", emoji, severity, count);
        }
    }
    
    if !security_report.findings_by_category.is_empty() {
        println!("\n🗂️  Findings by Category:");
        for (category, count) in &security_report.findings_by_category {
            let emoji = match category {
                syncable_cli::analyzer::SecurityCategory::SecretsExposure => "🔐",
                syncable_cli::analyzer::SecurityCategory::InsecureConfiguration => "⚙️ ",
                syncable_cli::analyzer::SecurityCategory::CodeSecurityPattern => "💻",
                syncable_cli::analyzer::SecurityCategory::InfrastructureSecurity => "🏗️ ",
                syncable_cli::analyzer::SecurityCategory::AuthenticationSecurity => "🔑",
                syncable_cli::analyzer::SecurityCategory::DataProtection => "🛡️ ",
                syncable_cli::analyzer::SecurityCategory::NetworkSecurity => "🌐",
                syncable_cli::analyzer::SecurityCategory::Compliance => "📜",
            };
            println!("  {} {:?}: {}", emoji, category, count);
        }
    }
    
    // Display detailed findings
    if !security_report.findings.is_empty() {
        println!("\n🔍 Detailed Security Findings:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        for (i, finding) in security_report.findings.iter().enumerate() {
            let severity_emoji = match finding.severity {
                syncable_cli::analyzer::SecuritySeverity::Critical => "🚨",
                syncable_cli::analyzer::SecuritySeverity::High => "⚠️ ",
                syncable_cli::analyzer::SecuritySeverity::Medium => "⚡",
                syncable_cli::analyzer::SecuritySeverity::Low => "ℹ️ ",
                syncable_cli::analyzer::SecuritySeverity::Info => "💡",
            };
            
            println!("\n{}. {} [{}] {}", i + 1, severity_emoji, finding.id, finding.title);
            println!("   📝 {}", finding.description);
            
            if let Some(file) = &finding.file_path {
                print!("   📁 File: {}", file.display());
                if let Some(line) = finding.line_number {
                    print!(" (line {})", line);
                }
                println!();
            }
            
            if let Some(evidence) = &finding.evidence {
                println!("   🔍 Evidence: {}", evidence);
            }
            
            if !finding.remediation.is_empty() {
                println!("   🔧 Remediation:");
                for remediation in &finding.remediation {
                    println!("      • {}", remediation);
                }
            }
            
            if let Some(cwe) = &finding.cwe_id {
                println!("   🏷️  CWE: {}", cwe);
            }
        }
    }
    
    // Display recommendations
    if !security_report.recommendations.is_empty() {
        println!("\n💡 Security Recommendations:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        for (i, recommendation) in security_report.recommendations.iter().enumerate() {
            println!("{}. {}", i + 1, recommendation);
        }
    }
    
    // Display compliance status
    if !security_report.compliance_status.is_empty() {
        println!("\n📜 Compliance Status:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        for (framework, status) in &security_report.compliance_status {
            println!("🏛️  {}: {:.1}% coverage", framework, status.coverage);
            if !status.missing_controls.is_empty() {
                println!("   Missing controls: {}", status.missing_controls.join(", "));
            }
        }
    }
    
    println!("\n✅ Security analysis completed!");
    
    // Exit with appropriate code based on findings
    if security_report.findings_by_severity.contains_key(&syncable_cli::analyzer::SecuritySeverity::Critical) {
        println!("❌ Critical security issues found. Please address immediately.");
        std::process::exit(1);
    } else if security_report.findings_by_severity.contains_key(&syncable_cli::analyzer::SecuritySeverity::High) {
        println!("⚠️  High severity security issues found. Review recommended.");
        std::process::exit(2);
    }
    
    Ok(())
} 