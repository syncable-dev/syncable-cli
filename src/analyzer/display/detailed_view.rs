//! Detailed/legacy vertical view display functionality

use crate::analyzer::display::helpers::{
    display_architecture_description, display_architecture_description_to_string,
    display_docker_analysis_detailed_legacy, display_docker_analysis_detailed_legacy_to_string,
    display_technologies_detailed_legacy, display_technologies_detailed_legacy_to_string,
    format_project_category, get_category_emoji,
};
use crate::analyzer::{MonorepoAnalysis, ProjectCategory};

/// Display in detailed vertical format (legacy)
pub fn display_detailed_view(analysis: &MonorepoAnalysis) {
    // Use the legacy detailed display format
    println!("{}", "=".repeat(80));
    println!("\n📊 PROJECT ANALYSIS RESULTS");
    println!("{}", "=".repeat(80));

    // Overall project information
    if analysis.is_monorepo {
        println!(
            "\n🏗️  Architecture: Monorepo with {} projects",
            analysis.projects.len()
        );
        println!(
            "   Pattern: {:?}",
            analysis.technology_summary.architecture_pattern
        );

        display_architecture_description(&analysis.technology_summary.architecture_pattern);
    } else {
        println!("\n🏗️  Architecture: Single Project");
    }

    // Technology Summary
    println!("\n🌐 Technology Summary:");
    if !analysis.technology_summary.languages.is_empty() {
        println!(
            "   Languages: {}",
            analysis.technology_summary.languages.join(", ")
        );
    }
    if !analysis.technology_summary.frameworks.is_empty() {
        println!(
            "   Frameworks: {}",
            analysis.technology_summary.frameworks.join(", ")
        );
    }
    if !analysis.technology_summary.databases.is_empty() {
        println!(
            "   Databases: {}",
            analysis.technology_summary.databases.join(", ")
        );
    }

    // Individual project details
    println!("\n📁 Project Details:");
    println!("{}", "=".repeat(80));

    for (i, project) in analysis.projects.iter().enumerate() {
        println!(
            "\n{} {}. {} ({})",
            get_category_emoji(&project.project_category),
            i + 1,
            project.name,
            format_project_category(&project.project_category)
        );

        if analysis.is_monorepo {
            println!("   📂 Path: {}", project.path.display());
        }

        // Languages for this project
        if !project.analysis.languages.is_empty() {
            println!("   🌐 Languages:");
            for lang in &project.analysis.languages {
                print!(
                    "      • {} (confidence: {:.1}%)",
                    lang.name,
                    lang.confidence * 100.0
                );
                if let Some(version) = &lang.version {
                    print!(" - Version: {}", version);
                }
                println!();
            }
        }

        // Technologies for this project
        if !project.analysis.technologies.is_empty() {
            println!("   🚀 Technologies:");
            display_technologies_detailed_legacy(&project.analysis.technologies);
        }

        // Entry Points
        if !project.analysis.entry_points.is_empty() {
            println!(
                "   📍 Entry Points ({}):",
                project.analysis.entry_points.len()
            );
            for (j, entry) in project.analysis.entry_points.iter().enumerate() {
                println!("      {}. File: {}", j + 1, entry.file.display());
                if let Some(func) = &entry.function {
                    println!("         Function: {}", func);
                }
                if let Some(cmd) = &entry.command {
                    println!("         Command: {}", cmd);
                }
            }
        }

        // Ports
        if !project.analysis.ports.is_empty() {
            println!("   🔌 Exposed Ports ({}):", project.analysis.ports.len());
            for port in &project.analysis.ports {
                println!("      • Port {}: {:?}", port.number, port.protocol);
                if let Some(desc) = &port.description {
                    println!("        {}", desc);
                }
            }
        }

        // Environment Variables
        if !project.analysis.environment_variables.is_empty() {
            println!(
                "   🔐 Environment Variables ({}):",
                project.analysis.environment_variables.len()
            );
            let required_vars: Vec<_> = project
                .analysis
                .environment_variables
                .iter()
                .filter(|ev| ev.required)
                .collect();
            let optional_vars: Vec<_> = project
                .analysis
                .environment_variables
                .iter()
                .filter(|ev| !ev.required)
                .collect();

            if !required_vars.is_empty() {
                println!("      Required:");
                for var in required_vars {
                    println!(
                        "        • {} {}",
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
                println!("      Optional:");
                for var in optional_vars {
                    println!(
                        "        • {} = {:?}",
                        var.name,
                        var.default_value.as_deref().unwrap_or("no default")
                    );
                }
            }
        }

        // Build Scripts
        if !project.analysis.build_scripts.is_empty() {
            println!(
                "   🔨 Build Scripts ({}):",
                project.analysis.build_scripts.len()
            );
            let default_scripts: Vec<_> = project
                .analysis
                .build_scripts
                .iter()
                .filter(|bs| bs.is_default)
                .collect();
            let other_scripts: Vec<_> = project
                .analysis
                .build_scripts
                .iter()
                .filter(|bs| !bs.is_default)
                .collect();

            if !default_scripts.is_empty() {
                println!("      Default scripts:");
                for script in default_scripts {
                    println!("        • {}: {}", script.name, script.command);
                    if let Some(desc) = &script.description {
                        println!("          {}", desc);
                    }
                }
            }

            if !other_scripts.is_empty() {
                println!("      Other scripts:");
                for script in other_scripts {
                    println!("        • {}: {}", script.name, script.command);
                    if let Some(desc) = &script.description {
                        println!("          {}", desc);
                    }
                }
            }
        }

        // Dependencies (sample)
        if !project.analysis.dependencies.is_empty() {
            println!(
                "   📦 Dependencies ({}):",
                project.analysis.dependencies.len()
            );
            if project.analysis.dependencies.len() <= 5 {
                for (name, version) in &project.analysis.dependencies {
                    println!("      • {} v{}", name, version);
                }
            } else {
                // Show first 5
                for (name, version) in project.analysis.dependencies.iter().take(5) {
                    println!("      • {} v{}", name, version);
                }
                println!(
                    "      ... and {} more",
                    project.analysis.dependencies.len() - 5
                );
            }
        }

        // Docker Infrastructure Analysis
        if let Some(docker_analysis) = &project.analysis.docker_analysis {
            display_docker_analysis_detailed_legacy(docker_analysis);
        }

        // Project type
        println!("   🎯 Project Type: {:?}", project.analysis.project_type);

        if i < analysis.projects.len() - 1 {
            println!("{}", "-".repeat(40));
        }
    }

    // Summary
    println!("\n📋 ANALYSIS SUMMARY");
    println!("{}", "=".repeat(80));
    println!("✅ Project Analysis Complete!");

    if analysis.is_monorepo {
        println!("\n🏗️  Monorepo Architecture:");
        println!("   • Total projects: {}", analysis.projects.len());
        println!(
            "   • Architecture pattern: {:?}",
            analysis.technology_summary.architecture_pattern
        );

        let frontend_count = analysis
            .projects
            .iter()
            .filter(|p| p.project_category == ProjectCategory::Frontend)
            .count();
        let backend_count = analysis
            .projects
            .iter()
            .filter(|p| {
                matches!(
                    p.project_category,
                    ProjectCategory::Backend | ProjectCategory::Api
                )
            })
            .count();
        let service_count = analysis
            .projects
            .iter()
            .filter(|p| p.project_category == ProjectCategory::Service)
            .count();
        let lib_count = analysis
            .projects
            .iter()
            .filter(|p| p.project_category == ProjectCategory::Library)
            .count();

        if frontend_count > 0 {
            println!("   • Frontend projects: {}", frontend_count);
        }
        if backend_count > 0 {
            println!("   • Backend/API projects: {}", backend_count);
        }
        if service_count > 0 {
            println!("   • Service projects: {}", service_count);
        }
        if lib_count > 0 {
            println!("   • Library projects: {}", lib_count);
        }
    }

    println!("\n📈 Analysis Metadata:");
    println!(
        "   • Duration: {}ms",
        analysis.metadata.analysis_duration_ms
    );
    println!("   • Files analyzed: {}", analysis.metadata.files_analyzed);
    println!(
        "   • Confidence score: {:.1}%",
        analysis.metadata.confidence_score * 100.0
    );
    println!(
        "   • Analyzer version: {}",
        analysis.metadata.analyzer_version
    );
}

/// Display detailed view - returns string  
pub fn display_detailed_view_to_string(analysis: &MonorepoAnalysis) -> String {
    let mut output = String::new();

    output.push_str(&format!("{}\n", "=".repeat(80)));
    output.push_str("\n📊 PROJECT ANALYSIS RESULTS\n");
    output.push_str(&format!("{}\n", "=".repeat(80)));

    // Overall project information
    if analysis.is_monorepo {
        output.push_str(&format!(
            "\n🏗️  Architecture: Monorepo with {} projects\n",
            analysis.projects.len()
        ));
        output.push_str(&format!(
            "   Pattern: {:?}\n",
            analysis.technology_summary.architecture_pattern
        ));

        output.push_str(&display_architecture_description_to_string(
            &analysis.technology_summary.architecture_pattern,
        ));
    } else {
        output.push_str("\n🏗️  Architecture: Single Project\n");
    }

    // Technology Summary
    output.push_str("\n🌐 Technology Summary:\n");
    if !analysis.technology_summary.languages.is_empty() {
        output.push_str(&format!(
            "   Languages: {}\n",
            analysis.technology_summary.languages.join(", ")
        ));
    }
    if !analysis.technology_summary.frameworks.is_empty() {
        output.push_str(&format!(
            "   Frameworks: {}\n",
            analysis.technology_summary.frameworks.join(", ")
        ));
    }
    if !analysis.technology_summary.databases.is_empty() {
        output.push_str(&format!(
            "   Databases: {}\n",
            analysis.technology_summary.databases.join(", ")
        ));
    }

    // Individual project details
    output.push_str("\n📁 Project Details:\n");
    output.push_str(&format!("{}\n", "=".repeat(80)));

    for (i, project) in analysis.projects.iter().enumerate() {
        output.push_str(&format!(
            "\n{} {}. {} ({})\n",
            get_category_emoji(&project.project_category),
            i + 1,
            project.name,
            format_project_category(&project.project_category)
        ));

        if analysis.is_monorepo {
            output.push_str(&format!("   📂 Path: {}\n", project.path.display()));
        }

        // Languages for this project
        if !project.analysis.languages.is_empty() {
            output.push_str("   🌐 Languages:\n");
            for lang in &project.analysis.languages {
                output.push_str(&format!(
                    "      • {} (confidence: {:.1}%)",
                    lang.name,
                    lang.confidence * 100.0
                ));
                if let Some(version) = &lang.version {
                    output.push_str(&format!(" - Version: {}", version));
                }
                output.push('\n');
            }
        }

        // Technologies for this project
        if !project.analysis.technologies.is_empty() {
            output.push_str("   🚀 Technologies:\n");
            output.push_str(&display_technologies_detailed_legacy_to_string(
                &project.analysis.technologies,
            ));
        }

        // Entry Points
        if !project.analysis.entry_points.is_empty() {
            output.push_str(&format!(
                "   📍 Entry Points ({}):\n",
                project.analysis.entry_points.len()
            ));
            for (j, entry) in project.analysis.entry_points.iter().enumerate() {
                output.push_str(&format!(
                    "      {}. File: {}\n",
                    j + 1,
                    entry.file.display()
                ));
                if let Some(func) = &entry.function {
                    output.push_str(&format!("         Function: {}\n", func));
                }
                if let Some(cmd) = &entry.command {
                    output.push_str(&format!("         Command: {}\n", cmd));
                }
            }
        }

        // Ports
        if !project.analysis.ports.is_empty() {
            output.push_str(&format!(
                "   🔌 Exposed Ports ({}):\n",
                project.analysis.ports.len()
            ));
            for port in &project.analysis.ports {
                output.push_str(&format!(
                    "      • Port {}: {:?}\n",
                    port.number, port.protocol
                ));
                if let Some(desc) = &port.description {
                    output.push_str(&format!("        {}\n", desc));
                }
            }
        }

        // Environment Variables
        if !project.analysis.environment_variables.is_empty() {
            output.push_str(&format!(
                "   🔐 Environment Variables ({}):\n",
                project.analysis.environment_variables.len()
            ));
            let required_vars: Vec<_> = project
                .analysis
                .environment_variables
                .iter()
                .filter(|ev| ev.required)
                .collect();
            let optional_vars: Vec<_> = project
                .analysis
                .environment_variables
                .iter()
                .filter(|ev| !ev.required)
                .collect();

            if !required_vars.is_empty() {
                output.push_str("      Required:\n");
                for var in required_vars {
                    output.push_str(&format!(
                        "        • {} {}\n",
                        var.name,
                        if let Some(desc) = &var.description {
                            format!("({})", desc)
                        } else {
                            String::new()
                        }
                    ));
                }
            }

            if !optional_vars.is_empty() {
                output.push_str("      Optional:\n");
                for var in optional_vars {
                    output.push_str(&format!(
                        "        • {} = {:?}\n",
                        var.name,
                        var.default_value.as_deref().unwrap_or("no default")
                    ));
                }
            }
        }

        // Build Scripts
        if !project.analysis.build_scripts.is_empty() {
            output.push_str(&format!(
                "   🔨 Build Scripts ({}):\n",
                project.analysis.build_scripts.len()
            ));
            let default_scripts: Vec<_> = project
                .analysis
                .build_scripts
                .iter()
                .filter(|bs| bs.is_default)
                .collect();
            let other_scripts: Vec<_> = project
                .analysis
                .build_scripts
                .iter()
                .filter(|bs| !bs.is_default)
                .collect();

            if !default_scripts.is_empty() {
                output.push_str("      Default scripts:\n");
                for script in default_scripts {
                    output.push_str(&format!("        • {}: {}\n", script.name, script.command));
                    if let Some(desc) = &script.description {
                        output.push_str(&format!("          {}\n", desc));
                    }
                }
            }

            if !other_scripts.is_empty() {
                output.push_str("      Other scripts:\n");
                for script in other_scripts {
                    output.push_str(&format!("        • {}: {}\n", script.name, script.command));
                    if let Some(desc) = &script.description {
                        output.push_str(&format!("          {}\n", desc));
                    }
                }
            }
        }

        // Dependencies (sample)
        if !project.analysis.dependencies.is_empty() {
            output.push_str(&format!(
                "   📦 Dependencies ({}):\n",
                project.analysis.dependencies.len()
            ));
            if project.analysis.dependencies.len() <= 5 {
                for (name, version) in &project.analysis.dependencies {
                    output.push_str(&format!("      • {} v{}\n", name, version));
                }
            } else {
                // Show first 5
                for (name, version) in project.analysis.dependencies.iter().take(5) {
                    output.push_str(&format!("      • {} v{}\n", name, version));
                }
                output.push_str(&format!(
                    "      ... and {} more\n",
                    project.analysis.dependencies.len() - 5
                ));
            }
        }

        // Docker Infrastructure Analysis
        if let Some(docker_analysis) = &project.analysis.docker_analysis {
            output.push_str(&display_docker_analysis_detailed_legacy_to_string(
                docker_analysis,
            ));
        }

        // Project type
        output.push_str(&format!(
            "   🎯 Project Type: {:?}\n",
            project.analysis.project_type
        ));

        if i < analysis.projects.len() - 1 {
            output.push_str(&format!("{}\n", "-".repeat(40)));
        }
    }

    // Summary
    output.push_str("\n📋 ANALYSIS SUMMARY\n");
    output.push_str(&format!("{}\n", "=".repeat(80)));
    output.push_str("✅ Project Analysis Complete!\n");

    if analysis.is_monorepo {
        output.push_str("\n🏗️  Monorepo Architecture:\n");
        output.push_str(&format!(
            "   • Total projects: {}\n",
            analysis.projects.len()
        ));
        output.push_str(&format!(
            "   • Architecture pattern: {:?}\n",
            analysis.technology_summary.architecture_pattern
        ));

        let frontend_count = analysis
            .projects
            .iter()
            .filter(|p| p.project_category == ProjectCategory::Frontend)
            .count();
        let backend_count = analysis
            .projects
            .iter()
            .filter(|p| {
                matches!(
                    p.project_category,
                    ProjectCategory::Backend | ProjectCategory::Api
                )
            })
            .count();
        let service_count = analysis
            .projects
            .iter()
            .filter(|p| p.project_category == ProjectCategory::Service)
            .count();
        let lib_count = analysis
            .projects
            .iter()
            .filter(|p| p.project_category == ProjectCategory::Library)
            .count();

        if frontend_count > 0 {
            output.push_str(&format!("   • Frontend projects: {}\n", frontend_count));
        }
        if backend_count > 0 {
            output.push_str(&format!("   • Backend/API projects: {}\n", backend_count));
        }
        if service_count > 0 {
            output.push_str(&format!("   • Service projects: {}\n", service_count));
        }
        if lib_count > 0 {
            output.push_str(&format!("   • Library projects: {}\n", lib_count));
        }
    }

    output.push_str("\n📈 Analysis Metadata:\n");
    output.push_str(&format!(
        "   • Duration: {}ms\n",
        analysis.metadata.analysis_duration_ms
    ));
    output.push_str(&format!(
        "   • Files analyzed: {}\n",
        analysis.metadata.files_analyzed
    ));
    output.push_str(&format!(
        "   • Confidence score: {:.1}%\n",
        analysis.metadata.confidence_score * 100.0
    ));
    output.push_str(&format!(
        "   • Analyzer version: {}\n",
        analysis.metadata.analyzer_version
    ));

    output
}
