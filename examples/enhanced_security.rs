//! Example: Enhanced Security Analysis
//! 
//! This example demonstrates the enhanced security analysis capabilities
//! including the new modular JavaScript/TypeScript security analyzer.

use std::path::Path;
use syncable_cli::analyzer::{analyze_project, SecurityAnalyzer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    // For this example, analyze the current directory or a provided path
    let project_path = std::env::args()
        .nth(1)
        .map(|p| Path::new(&p).to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    
    println!("🔍 Analyzing project security for: {}", project_path.display());
    
    // First, perform regular project analysis to detect languages
    let analysis = analyze_project(&project_path)?;
    
    println!("\n📋 Detected Languages:");
    for lang in &analysis.languages {
        println!("  • {} (confidence: {:.1}%)", lang.name, lang.confidence * 100.0);
    }
    
    println!("\n🔧 Detected Technologies:");
    for tech in &analysis.technologies {
        println!("  • {} v{} ({:?})", 
            tech.name, 
            tech.version.as_deref().unwrap_or("unknown"),
            tech.category
        );
    }
    
    // Check if this is a JavaScript/TypeScript project
    let has_js = analysis.languages.iter()
        .any(|lang| matches!(lang.name.as_str(), "JavaScript" | "TypeScript" | "JSX" | "TSX"));
    
    if has_js {
        println!("\n✅ JavaScript/TypeScript project detected! Using enhanced security analysis...");
    } else {
        println!("\n📄 Using general security analysis...");
    }
    
    // Run enhanced security analysis
    println!("\n🛡️  Starting enhanced security analysis...");
    
    let mut security_analyzer = SecurityAnalyzer::new()?;
    let security_report = security_analyzer.analyze_security_enhanced(&analysis)?;
    
    // Display results
    println!("\n📊 Security Analysis Results:");
    println!("  Overall Score: {:.1}/100", security_report.overall_score);
    println!("  Risk Level: {:?}", security_report.risk_level);
    println!("  Total Findings: {}", security_report.total_findings);
    
    if security_report.total_findings > 0 {
        println!("\n🚨 Security Findings:");
        
        // Group findings by severity
        for severity in [
            syncable_cli::analyzer::security::core::SecuritySeverity::Critical,
            syncable_cli::analyzer::security::core::SecuritySeverity::High,
            syncable_cli::analyzer::security::core::SecuritySeverity::Medium,
            syncable_cli::analyzer::security::core::SecuritySeverity::Low,
        ] {
            let findings: Vec<_> = security_report.findings.iter()
                .filter(|f| f.severity == severity)
                .collect();
            
            if !findings.is_empty() {
                let severity_icon = match severity {
                    syncable_cli::analyzer::security::core::SecuritySeverity::Critical => "🔴",
                    syncable_cli::analyzer::security::core::SecuritySeverity::High => "🟠",
                    syncable_cli::analyzer::security::core::SecuritySeverity::Medium => "🟡",
                    syncable_cli::analyzer::security::core::SecuritySeverity::Low => "🔵",
                    _ => "⚪",
                };
                
                println!("\n{} {:?} Severity ({} findings):", severity_icon, severity, findings.len());
                
                for finding in findings.iter().take(3) { // Show first 3 of each severity
                    println!("  📍 {}", finding.title);
                    if let Some(ref file_path) = finding.file_path {
                        let relative_path = file_path.strip_prefix(&project_path)
                            .unwrap_or(file_path);
                        print!("     📄 {}", relative_path.display());
                        if let Some(line) = finding.line_number {
                            print!(":{}", line);
                        }
                        println!();
                    }
                    println!("     💡 {}", finding.description);
                    
                    if !finding.remediation.is_empty() {
                        println!("     🔧 Remediation: {}", finding.remediation[0]);
                    }
                    println!();
                }
                
                if findings.len() > 3 {
                    println!("     ... and {} more findings", findings.len() - 3);
                }
            }
        }
        
        // Show recommendations
        if !security_report.recommendations.is_empty() {
            println!("\n💡 Recommendations:");
            for (i, recommendation) in security_report.recommendations.iter().enumerate() {
                println!("  {}. {}", i + 1, recommendation);
            }
        }
    } else {
        println!("✅ No security issues detected!");
    }
    
    println!("\n✨ Enhanced security analysis complete!");
    
    Ok(())
} 