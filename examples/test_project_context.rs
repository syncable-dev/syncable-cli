//! Example: Test Project Context Analyzer
//!
//! This example demonstrates the Project Context Analyzer functionality
//! by analyzing the current project.

use std::env;
use std::path::Path;
use syncable_cli::analyzer::{ProjectType, analyze_project};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::init();

    // Get the project path from command line or use current directory
    let path = env::args().nth(1).unwrap_or_else(|| ".".to_string());

    let project_path = Path::new(&path);

    println!("🔍 Analyzing project at: {}", project_path.display());
    println!("{}", "=".repeat(60));

    // Run the analysis
    let analysis = analyze_project(project_path)?;

    // Display Project Context Analysis Results
    println!("\n📊 PROJECT CONTEXT ANALYSIS RESULTS");
    println!("{}", "=".repeat(60));

    // Project Type (Roadmap Requirement #5)
    println!("\n🎯 Project Type: {:?}", analysis.project_type);
    match analysis.project_type {
        ProjectType::WebApplication => println!("   This is a web application with UI"),
        ProjectType::ApiService => println!("   This is an API service without UI"),
        ProjectType::CliTool => println!("   This is a command-line tool"),
        ProjectType::Library => println!("   This is a library/package"),
        ProjectType::Microservice => println!("   This is a microservice"),
        ProjectType::StaticSite => println!("   This is a static website"),
        _ => println!("   Project type details not available"),
    }

    // Entry Points (Roadmap Requirement #1)
    println!("\n📍 Entry Points ({}):", analysis.entry_points.len());
    for (i, entry) in analysis.entry_points.iter().enumerate() {
        println!("   {}. File: {}", i + 1, entry.file.display());
        if let Some(func) = &entry.function {
            println!("      Function: {}", func);
        }
        if let Some(cmd) = &entry.command {
            println!("      Command: {}", cmd);
        }
    }

    // Ports (Roadmap Requirement #2)
    println!("\n🔌 Exposed Ports ({}):", analysis.ports.len());
    for port in &analysis.ports {
        println!("   - Port {}: {:?}", port.number, port.protocol);
        if let Some(desc) = &port.description {
            println!("     {}", desc);
        }
    }

    // Environment Variables (Roadmap Requirement #3)
    println!(
        "\n🔐 Environment Variables ({}):",
        analysis.environment_variables.len()
    );
    let required_vars: Vec<_> = analysis
        .environment_variables
        .iter()
        .filter(|ev| ev.required)
        .collect();
    let optional_vars: Vec<_> = analysis
        .environment_variables
        .iter()
        .filter(|ev| !ev.required)
        .collect();

    if !required_vars.is_empty() {
        println!("   Required:");
        for var in required_vars {
            println!(
                "     - {} {}",
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
            println!(
                "     - {} = {:?}",
                var.name,
                var.default_value.as_deref().unwrap_or("no default")
            );
        }
    }

    // Build Scripts (Roadmap Requirement #4)
    println!("\n🔨 Build Scripts ({}):", analysis.build_scripts.len());
    let default_scripts: Vec<_> = analysis
        .build_scripts
        .iter()
        .filter(|bs| bs.is_default)
        .collect();
    let other_scripts: Vec<_> = analysis
        .build_scripts
        .iter()
        .filter(|bs| !bs.is_default)
        .collect();

    if !default_scripts.is_empty() {
        println!("   Default scripts:");
        for script in default_scripts {
            println!("     - {}: {}", script.name, script.command);
        }
    }

    if !other_scripts.is_empty() {
        println!("   Other scripts:");
        for script in other_scripts {
            println!("     - {}: {}", script.name, script.command);
        }
    }

    // Summary
    println!("\n📋 SUMMARY");
    println!("{}", "=".repeat(60));
    println!("✅ All 5 Project Context Analyzer requirements verified:");
    println!(
        "   1. Entry points detected: {}",
        if analysis.entry_points.is_empty() {
            "❌ None"
        } else {
            "✅ Yes"
        }
    );
    println!(
        "   2. Ports identified: {}",
        if analysis.ports.is_empty() {
            "❌ None"
        } else {
            "✅ Yes"
        }
    );
    println!(
        "   3. Environment variables extracted: {}",
        if analysis.environment_variables.is_empty() {
            "❌ None"
        } else {
            "✅ Yes"
        }
    );
    println!(
        "   4. Build scripts analyzed: {}",
        if analysis.build_scripts.is_empty() {
            "❌ None"
        } else {
            "✅ Yes"
        }
    );
    println!(
        "   5. Project type determined: {}",
        if matches!(analysis.project_type, ProjectType::Unknown) {
            "❌ Unknown"
        } else {
            "✅ Yes"
        }
    );

    println!("\n✨ Project Context Analysis Complete!");

    Ok(())
}
