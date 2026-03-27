//! Helper functions for display formatting

use crate::analyzer::display::BoxDrawer;
use crate::analyzer::{
    ArchitecturePattern, DetectedTechnology, DockerAnalysis, LibraryType, OrchestrationPattern,
    ProjectCategory, TechnologyCategory,
};
use colored::*;

/// Get emoji for project category
pub fn get_category_emoji(category: &ProjectCategory) -> &'static str {
    match category {
        ProjectCategory::Frontend => "🌐",
        ProjectCategory::Backend => "⚙️",
        ProjectCategory::Api => "🔌",
        ProjectCategory::Service => "🚀",
        ProjectCategory::Library => "📚",
        ProjectCategory::Tool => "🔧",
        ProjectCategory::Documentation => "📖",
        ProjectCategory::Infrastructure => "🏗️",
        ProjectCategory::Unknown => "❓",
    }
}

/// Format project category name
pub fn format_project_category(category: &ProjectCategory) -> &'static str {
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

/// Display architecture description
pub fn display_architecture_description(pattern: &ArchitecturePattern) {
    match pattern {
        ArchitecturePattern::Monolithic => {
            println!("   📦 This is a single, self-contained application");
        }
        ArchitecturePattern::Fullstack => {
            println!("   🌐 This is a full-stack application with separate frontend and backend");
        }
        ArchitecturePattern::Microservices => {
            println!(
                "   🔗 This is a microservices architecture with multiple independent services"
            );
        }
        ArchitecturePattern::ApiFirst => {
            println!("   🔌 This is an API-first architecture focused on service interfaces");
        }
        ArchitecturePattern::EventDriven => {
            println!("   📡 This is an event-driven architecture with decoupled components");
        }
        ArchitecturePattern::Mixed => {
            println!("   🔀 This is a mixed architecture combining multiple patterns");
        }
    }
}

/// Helper function for displaying architecture description - returns string
pub fn display_architecture_description_to_string(pattern: &ArchitecturePattern) -> String {
    match pattern {
        ArchitecturePattern::Monolithic => {
            "   📦 This is a single, self-contained application\n".to_string()
        }
        ArchitecturePattern::Fullstack => {
            "   🌐 This is a full-stack application with separate frontend and backend\n"
                .to_string()
        }
        ArchitecturePattern::Microservices => {
            "   🔗 This is a microservices architecture with multiple independent services\n"
                .to_string()
        }
        ArchitecturePattern::ApiFirst => {
            "   🔌 This is an API-first architecture focused on service interfaces\n".to_string()
        }
        ArchitecturePattern::EventDriven => {
            "   📡 This is an event-driven architecture with decoupled components\n".to_string()
        }
        ArchitecturePattern::Mixed => {
            "   🔀 This is a mixed architecture combining multiple patterns\n".to_string()
        }
    }
}

/// Get main technologies for display
pub fn get_main_technologies(technologies: &[DetectedTechnology]) -> String {
    let primary = technologies.iter().find(|t| t.is_primary);
    let frameworks: Vec<_> = technologies
        .iter()
        .filter(|t| {
            matches!(
                t.category,
                TechnologyCategory::FrontendFramework | TechnologyCategory::MetaFramework
            )
        })
        .take(2)
        .collect();

    let mut result = Vec::new();

    if let Some(p) = primary {
        result.push(p.name.clone());
    }

    for f in frameworks {
        if Some(&f.name) != primary.map(|p| &p.name) {
            result.push(f.name.clone());
        }
    }

    if result.is_empty() {
        "-".to_string()
    } else {
        result.join(", ")
    }
}

/// Add confidence score as a progress bar to the box drawer
pub fn add_confidence_bar_to_drawer(score: f32, box_drawer: &mut BoxDrawer) {
    let percentage = (score * 100.0) as u8;
    let bar_width = 20;
    let filled = ((score * bar_width as f32) as usize).min(bar_width);

    let bar = format!(
        "{}{}",
        "█".repeat(filled).green(),
        "░".repeat(bar_width - filled).dimmed()
    );

    let color = if percentage >= 80 {
        "green"
    } else if percentage >= 60 {
        "yellow"
    } else {
        "red"
    };

    let confidence_info = format!("{} {}", bar, format!("{:.0}%", percentage).color(color));
    box_drawer.add_line("Confidence:", &confidence_info, true);
}

/// Helper function for legacy detailed technology display
pub fn display_technologies_detailed_legacy(technologies: &[DetectedTechnology]) {
    // Group technologies by category
    let mut by_category: std::collections::HashMap<&TechnologyCategory, Vec<&DetectedTechnology>> =
        std::collections::HashMap::new();

    for tech in technologies {
        by_category.entry(&tech.category).or_default().push(tech);
    }

    // Find and display primary technology
    if let Some(primary) = technologies.iter().find(|t| t.is_primary) {
        println!("\n🛠️  Technology Stack:");
        println!(
            "   🎯 PRIMARY: {} (confidence: {:.1}%)",
            primary.name,
            primary.confidence * 100.0
        );
        println!("      Architecture driver for this project");
    }

    // Display categories in order
    let categories = [
        (TechnologyCategory::MetaFramework, "🏗️  Meta-Frameworks"),
        (
            TechnologyCategory::BackendFramework,
            "🖥️  Backend Frameworks",
        ),
        (
            TechnologyCategory::FrontendFramework,
            "🎨 Frontend Frameworks",
        ),
        (
            TechnologyCategory::Library(LibraryType::UI),
            "🎨 UI Libraries",
        ),
        (
            TechnologyCategory::Library(LibraryType::Utility),
            "📚 Core Libraries",
        ),
        (TechnologyCategory::BuildTool, "🔨 Build Tools"),
        (TechnologyCategory::PackageManager, "📦 Package Managers"),
        (TechnologyCategory::Database, "🗃️  Database & ORM"),
        (TechnologyCategory::Runtime, "⚡ Runtimes"),
        (TechnologyCategory::Testing, "🧪 Testing"),
    ];

    for (category, label) in &categories {
        if let Some(techs) = by_category.get(category)
            && !techs.is_empty()
        {
            println!("\n   {}:", label);
            for tech in techs {
                println!(
                    "      • {} (confidence: {:.1}%)",
                    tech.name,
                    tech.confidence * 100.0
                );
                if let Some(version) = &tech.version {
                    println!("        Version: {}", version);
                }
            }
        }
    }

    // Handle other Library types separately
    for (cat, techs) in &by_category {
        if let TechnologyCategory::Library(lib_type) = cat {
            let label = match lib_type {
                LibraryType::StateManagement => "🔄 State Management",
                LibraryType::DataFetching => "🔃 Data Fetching",
                LibraryType::Routing => "🗺️  Routing",
                LibraryType::Styling => "🎨 Styling",
                LibraryType::HttpClient => "🌐 HTTP Clients",
                LibraryType::Authentication => "🔐 Authentication",
                LibraryType::Other(_) => "📦 Other Libraries",
                _ => continue, // Skip already handled UI and Utility
            };

            // Only print if not already handled above
            if !matches!(lib_type, LibraryType::UI | LibraryType::Utility) && !techs.is_empty() {
                println!("\n   {}:", label);
                for tech in techs {
                    println!(
                        "      • {} (confidence: {:.1}%)",
                        tech.name,
                        tech.confidence * 100.0
                    );
                    if let Some(version) = &tech.version {
                        println!("        Version: {}", version);
                    }
                }
            }
        }
    }
}

/// Helper function for legacy detailed technology display - returns string
pub fn display_technologies_detailed_legacy_to_string(
    technologies: &[DetectedTechnology],
) -> String {
    let mut output = String::new();

    // Group technologies by category
    let mut by_category: std::collections::HashMap<&TechnologyCategory, Vec<&DetectedTechnology>> =
        std::collections::HashMap::new();

    for tech in technologies {
        by_category.entry(&tech.category).or_default().push(tech);
    }

    // Find and display primary technology
    if let Some(primary) = technologies.iter().find(|t| t.is_primary) {
        output.push_str("\n🛠️  Technology Stack:\n");
        output.push_str(&format!(
            "   🎯 PRIMARY: {} (confidence: {:.1}%)\n",
            primary.name,
            primary.confidence * 100.0
        ));
        output.push_str("      Architecture driver for this project\n");
    }

    // Display categories in order
    let categories = [
        (TechnologyCategory::MetaFramework, "🏗️  Meta-Frameworks"),
        (
            TechnologyCategory::BackendFramework,
            "🖥️  Backend Frameworks",
        ),
        (
            TechnologyCategory::FrontendFramework,
            "🎨 Frontend Frameworks",
        ),
        (
            TechnologyCategory::Library(LibraryType::UI),
            "🎨 UI Libraries",
        ),
        (
            TechnologyCategory::Library(LibraryType::Utility),
            "📚 Core Libraries",
        ),
        (TechnologyCategory::BuildTool, "🔨 Build Tools"),
        (TechnologyCategory::PackageManager, "📦 Package Managers"),
        (TechnologyCategory::Database, "🗃️  Database & ORM"),
        (TechnologyCategory::Runtime, "⚡ Runtimes"),
        (TechnologyCategory::Testing, "🧪 Testing"),
    ];

    for (category, label) in &categories {
        if let Some(techs) = by_category.get(category)
            && !techs.is_empty()
        {
            output.push_str(&format!("\n   {}:\n", label));
            for tech in techs {
                output.push_str(&format!(
                    "      • {} (confidence: {:.1}%)\n",
                    tech.name,
                    tech.confidence * 100.0
                ));
                if let Some(version) = &tech.version {
                    output.push_str(&format!("        Version: {}\n", version));
                }
            }
        }
    }

    // Handle other Library types separately
    for (cat, techs) in &by_category {
        if let TechnologyCategory::Library(lib_type) = cat {
            let label = match lib_type {
                LibraryType::StateManagement => "🔄 State Management",
                LibraryType::DataFetching => "🔃 Data Fetching",
                LibraryType::Routing => "🗺️  Routing",
                LibraryType::Styling => "🎨 Styling",
                LibraryType::HttpClient => "🌐 HTTP Clients",
                LibraryType::Authentication => "🔐 Authentication",
                LibraryType::Other(_) => "📦 Other Libraries",
                _ => continue, // Skip already handled UI and Utility
            };

            // Only print if not already handled above
            if !matches!(lib_type, LibraryType::UI | LibraryType::Utility) && !techs.is_empty() {
                output.push_str(&format!("\n   {}:\n", label));
                for tech in techs {
                    output.push_str(&format!(
                        "      • {} (confidence: {:.1}%)\n",
                        tech.name,
                        tech.confidence * 100.0
                    ));
                    if let Some(version) = &tech.version {
                        output.push_str(&format!("        Version: {}\n", version));
                    }
                }
            }
        }
    }

    output
}

/// Helper function for legacy Docker analysis display
pub fn display_docker_analysis_detailed_legacy(docker_analysis: &DockerAnalysis) {
    println!("\n   🐳 Docker Infrastructure Analysis:");

    // Dockerfiles
    if !docker_analysis.dockerfiles.is_empty() {
        println!(
            "      📄 Dockerfiles ({}):",
            docker_analysis.dockerfiles.len()
        );
        for dockerfile in &docker_analysis.dockerfiles {
            println!("         • {}", dockerfile.path.display());
            if let Some(env) = &dockerfile.environment {
                println!("           Environment: {}", env);
            }
            if let Some(base_image) = &dockerfile.base_image {
                println!("           Base image: {}", base_image);
            }
            if !dockerfile.exposed_ports.is_empty() {
                println!(
                    "           Exposed ports: {}",
                    dockerfile
                        .exposed_ports
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            if dockerfile.is_multistage {
                println!(
                    "           Multi-stage build: {} stages",
                    dockerfile.build_stages.len()
                );
            }
            println!("           Instructions: {}", dockerfile.instruction_count);
        }
    }

    // Compose files
    if !docker_analysis.compose_files.is_empty() {
        println!(
            "      📋 Compose Files ({}):",
            docker_analysis.compose_files.len()
        );
        for compose_file in &docker_analysis.compose_files {
            println!("         • {}", compose_file.path.display());
            if let Some(env) = &compose_file.environment {
                println!("           Environment: {}", env);
            }
            if let Some(version) = &compose_file.version {
                println!("           Version: {}", version);
            }
            if !compose_file.service_names.is_empty() {
                println!(
                    "           Services: {}",
                    compose_file.service_names.join(", ")
                );
            }
            if !compose_file.networks.is_empty() {
                println!("           Networks: {}", compose_file.networks.join(", "));
            }
            if !compose_file.volumes.is_empty() {
                println!("           Volumes: {}", compose_file.volumes.join(", "));
            }
        }
    }

    // Rest of the detailed Docker display...
    println!(
        "      🏗️  Orchestration Pattern: {:?}",
        docker_analysis.orchestration_pattern
    );
    match docker_analysis.orchestration_pattern {
        OrchestrationPattern::SingleContainer => {
            println!("         Simple containerized application");
        }
        OrchestrationPattern::DockerCompose => {
            println!("         Multi-service Docker Compose setup");
        }
        OrchestrationPattern::Microservices => {
            println!("         Microservices architecture with service discovery");
        }
        OrchestrationPattern::EventDriven => {
            println!("         Event-driven architecture with message queues");
        }
        OrchestrationPattern::ServiceMesh => {
            println!("         Service mesh for advanced service communication");
        }
        OrchestrationPattern::Mixed => {
            println!("         Mixed/complex orchestration pattern");
        }
    }
}

/// Helper function for legacy Docker analysis display - returns string
pub fn display_docker_analysis_detailed_legacy_to_string(
    docker_analysis: &DockerAnalysis,
) -> String {
    let mut output = String::new();

    output.push_str("\n   🐳 Docker Infrastructure Analysis:\n");

    // Dockerfiles
    if !docker_analysis.dockerfiles.is_empty() {
        output.push_str(&format!(
            "      📄 Dockerfiles ({}):\n",
            docker_analysis.dockerfiles.len()
        ));
        for dockerfile in &docker_analysis.dockerfiles {
            output.push_str(&format!("         • {}\n", dockerfile.path.display()));
            if let Some(env) = &dockerfile.environment {
                output.push_str(&format!("           Environment: {}\n", env));
            }
            if let Some(base_image) = &dockerfile.base_image {
                output.push_str(&format!("           Base image: {}\n", base_image));
            }
            if !dockerfile.exposed_ports.is_empty() {
                output.push_str(&format!(
                    "           Exposed ports: {}\n",
                    dockerfile
                        .exposed_ports
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if dockerfile.is_multistage {
                output.push_str(&format!(
                    "           Multi-stage build: {} stages\n",
                    dockerfile.build_stages.len()
                ));
            }
            output.push_str(&format!(
                "           Instructions: {}\n",
                dockerfile.instruction_count
            ));
        }
    }

    // Compose files
    if !docker_analysis.compose_files.is_empty() {
        output.push_str(&format!(
            "      📋 Compose Files ({}):\n",
            docker_analysis.compose_files.len()
        ));
        for compose_file in &docker_analysis.compose_files {
            output.push_str(&format!("         • {}\n", compose_file.path.display()));
            if let Some(env) = &compose_file.environment {
                output.push_str(&format!("           Environment: {}\n", env));
            }
            if let Some(version) = &compose_file.version {
                output.push_str(&format!("           Version: {}\n", version));
            }
            if !compose_file.service_names.is_empty() {
                output.push_str(&format!(
                    "           Services: {}\n",
                    compose_file.service_names.join(", ")
                ));
            }
            if !compose_file.networks.is_empty() {
                output.push_str(&format!(
                    "           Networks: {}\n",
                    compose_file.networks.join(", ")
                ));
            }
            if !compose_file.volumes.is_empty() {
                output.push_str(&format!(
                    "           Volumes: {}\n",
                    compose_file.volumes.join(", ")
                ));
            }
        }
    }

    // Rest of the detailed Docker display...
    output.push_str(&format!(
        "      🏗️  Orchestration Pattern: {:?}\n",
        docker_analysis.orchestration_pattern
    ));
    match docker_analysis.orchestration_pattern {
        OrchestrationPattern::SingleContainer => {
            output.push_str("         Simple containerized application\n");
        }
        OrchestrationPattern::DockerCompose => {
            output.push_str("         Multi-service Docker Compose setup\n");
        }
        OrchestrationPattern::Microservices => {
            output.push_str("         Microservices architecture with service discovery\n");
        }
        OrchestrationPattern::EventDriven => {
            output.push_str("         Event-driven architecture with message queues\n");
        }
        OrchestrationPattern::ServiceMesh => {
            output.push_str("         Service mesh for advanced service communication\n");
        }
        OrchestrationPattern::Mixed => {
            output.push_str("         Mixed/complex orchestration pattern\n");
        }
    }

    output
}
