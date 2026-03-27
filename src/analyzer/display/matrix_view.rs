//! Matrix/dashboard view display functionality

use crate::analyzer::display::{
    BoxDrawer, format_list_smart, format_ports_smart, get_color_adapter, get_terminal_width,
    helpers::{add_confidence_bar_to_drawer, format_project_category},
    smart_truncate, visual_width,
};
use crate::analyzer::{ArchitecturePattern, MonorepoAnalysis};

/// Display analysis in a compact matrix/dashboard format
pub fn display_matrix_view(analysis: &MonorepoAnalysis) {
    // Header
    let colors = get_color_adapter();
    println!("\n{}", colors.border(&"═".repeat(100)));
    println!("{}", colors.header_text("📊 PROJECT ANALYSIS DASHBOARD"));
    println!("{}", colors.border(&"═".repeat(100)));

    // Architecture Overview Box
    display_architecture_box(analysis);

    // Technology Stack Box
    display_technology_stack_box(analysis);

    // Projects Matrix
    if analysis.projects.len() > 1 {
        display_projects_matrix(analysis);
    } else {
        display_single_project_matrix(analysis);
    }

    // Docker Infrastructure Overview
    if analysis
        .projects
        .iter()
        .any(|p| p.analysis.docker_analysis.is_some())
    {
        display_docker_overview_matrix(analysis);
    }

    // Analysis Metrics Box
    display_metrics_box(analysis);

    // Footer
    println!("\n{}", colors.border(&"═".repeat(100)));
}

/// Display analysis in a compact matrix/dashboard format - returns string
pub fn display_matrix_view_to_string(analysis: &MonorepoAnalysis) -> String {
    let mut output = String::new();

    // Header
    let colors = get_color_adapter();
    output.push_str(&format!("\n{}\n", colors.border(&"═".repeat(100))));
    output.push_str(&format!(
        "{}\n",
        colors.header_text("📊 PROJECT ANALYSIS DASHBOARD")
    ));
    output.push_str(&format!("{}\n", colors.border(&"═".repeat(100))));

    // Architecture Overview Box
    output.push_str(&display_architecture_box_to_string(analysis));

    // Technology Stack Box
    output.push_str(&display_technology_stack_box_to_string(analysis));

    // Projects Matrix
    if analysis.projects.len() > 1 {
        output.push_str(&display_projects_matrix_to_string(analysis));
    } else {
        output.push_str(&display_single_project_matrix_to_string(analysis));
    }

    // Docker Infrastructure Overview
    if analysis
        .projects
        .iter()
        .any(|p| p.analysis.docker_analysis.is_some())
    {
        output.push_str(&display_docker_overview_matrix_to_string(analysis));
    }

    // Analysis Metrics Box
    output.push_str(&display_metrics_box_to_string(analysis));

    // Footer
    output.push_str(&format!("\n{}\n", colors.border(&"═".repeat(100))));

    output
}

/// Display architecture overview in a box
fn display_architecture_box(analysis: &MonorepoAnalysis) {
    let colors = get_color_adapter();
    let mut box_drawer = BoxDrawer::new("Architecture Overview");

    let arch_type = if analysis.is_monorepo {
        format!("Monorepo ({} projects)", analysis.projects.len())
    } else {
        "Single Project".to_string()
    };

    box_drawer.add_line("Type:", &colors.project_type(&arch_type), true);
    box_drawer.add_line(
        "Pattern:",
        &colors.architecture_pattern(&format!(
            "{:?}",
            analysis.technology_summary.architecture_pattern
        )),
        true,
    );

    // Pattern description
    let pattern_desc = match &analysis.technology_summary.architecture_pattern {
        ArchitecturePattern::Monolithic => "Single, self-contained application",
        ArchitecturePattern::Fullstack => "Full-stack app with frontend/backend separation",
        ArchitecturePattern::Microservices => "Multiple independent microservices",
        ArchitecturePattern::ApiFirst => "API-first architecture with service interfaces",
        ArchitecturePattern::EventDriven => "Event-driven with decoupled components",
        ArchitecturePattern::Mixed => "Mixed architecture patterns",
    };
    box_drawer.add_value_only(&colors.dimmed(pattern_desc));

    println!("\n{}", box_drawer.draw());
}

/// Display architecture overview in a box - returns string
fn display_architecture_box_to_string(analysis: &MonorepoAnalysis) -> String {
    let colors = get_color_adapter();
    let mut box_drawer = BoxDrawer::new("Architecture Overview");

    let arch_type = if analysis.is_monorepo {
        format!("Monorepo ({} projects)", analysis.projects.len())
    } else {
        "Single Project".to_string()
    };

    box_drawer.add_line("Type:", &colors.project_type(&arch_type), true);
    box_drawer.add_line(
        "Pattern:",
        &colors.architecture_pattern(&format!(
            "{:?}",
            analysis.technology_summary.architecture_pattern
        )),
        true,
    );

    // Pattern description
    let pattern_desc = match &analysis.technology_summary.architecture_pattern {
        ArchitecturePattern::Monolithic => "Single, self-contained application",
        ArchitecturePattern::Fullstack => "Full-stack app with frontend/backend separation",
        ArchitecturePattern::Microservices => "Multiple independent microservices",
        ArchitecturePattern::ApiFirst => "API-first architecture with service interfaces",
        ArchitecturePattern::EventDriven => "Event-driven with decoupled components",
        ArchitecturePattern::Mixed => "Mixed architecture patterns",
    };
    box_drawer.add_value_only(&colors.dimmed(pattern_desc));

    format!("\n{}", box_drawer.draw())
}

/// Display technology stack overview
fn display_technology_stack_box(analysis: &MonorepoAnalysis) {
    let colors = get_color_adapter();
    let mut box_drawer = BoxDrawer::new("Technology Stack");
    let term_width = get_terminal_width();
    // Max value width for the Technology Stack box (leave room for label + borders)
    let max_value_width = term_width.saturating_sub(30).min(80);

    let mut has_content = false;

    // Languages - show up to 4 with truncation
    if !analysis.technology_summary.languages.is_empty() {
        let languages = format_list_smart(&analysis.technology_summary.languages, 4, 20);
        let languages = smart_truncate(&languages, max_value_width);
        box_drawer.add_line("Languages:", &colors.language(&languages), true);
        has_content = true;
    }

    // Frameworks - show up to 4 with truncation
    if !analysis.technology_summary.frameworks.is_empty() {
        let frameworks = format_list_smart(&analysis.technology_summary.frameworks, 4, 16);
        let frameworks = smart_truncate(&frameworks, max_value_width);
        box_drawer.add_line("Frameworks:", &colors.framework(&frameworks), true);
        has_content = true;
    }

    // Databases - show up to 3 with truncation
    if !analysis.technology_summary.databases.is_empty() {
        let databases = format_list_smart(&analysis.technology_summary.databases, 3, 15);
        let databases = smart_truncate(&databases, max_value_width);
        box_drawer.add_line("Databases:", &colors.database(&databases), true);
        has_content = true;
    }

    if !has_content {
        box_drawer.add_value_only("No technologies detected");
    }

    println!("\n{}", box_drawer.draw());
}

/// Display technology stack overview - returns string
fn display_technology_stack_box_to_string(analysis: &MonorepoAnalysis) -> String {
    let colors = get_color_adapter();
    let mut box_drawer = BoxDrawer::new("Technology Stack");
    let term_width = get_terminal_width();
    // Max value width for the Technology Stack box (leave room for label + borders)
    let max_value_width = term_width.saturating_sub(30).min(80);

    let mut has_content = false;

    // Languages - show up to 4 with truncation
    if !analysis.technology_summary.languages.is_empty() {
        let languages = format_list_smart(&analysis.technology_summary.languages, 4, 20);
        let languages = smart_truncate(&languages, max_value_width);
        box_drawer.add_line("Languages:", &colors.language(&languages), true);
        has_content = true;
    }

    // Frameworks - show up to 4 with truncation
    if !analysis.technology_summary.frameworks.is_empty() {
        let frameworks = format_list_smart(&analysis.technology_summary.frameworks, 4, 16);
        let frameworks = smart_truncate(&frameworks, max_value_width);
        box_drawer.add_line("Frameworks:", &colors.framework(&frameworks), true);
        has_content = true;
    }

    // Databases - show up to 3 with truncation
    if !analysis.technology_summary.databases.is_empty() {
        let databases = format_list_smart(&analysis.technology_summary.databases, 3, 15);
        let databases = smart_truncate(&databases, max_value_width);
        box_drawer.add_line("Databases:", &colors.database(&databases), true);
        has_content = true;
    }

    if !has_content {
        box_drawer.add_value_only("No technologies detected");
    }

    format!("\n{}", box_drawer.draw())
}

/// Column width constraints for responsive display
struct ColumnConfig {
    max_width: usize,
    min_width: usize,
}

/// Display projects in a matrix table format with smart truncation
fn display_projects_matrix(analysis: &MonorepoAnalysis) {
    let term_width = get_terminal_width();
    let mut box_drawer = BoxDrawer::new("Projects Matrix");

    // Column configuration: max widths to prevent explosion
    // Adjusted based on terminal width
    let is_wide = term_width >= 120;
    let col_configs = [
        ColumnConfig {
            max_width: if is_wide { 24 } else { 18 },
            min_width: 7,
        }, // Project
        ColumnConfig {
            max_width: 10,
            min_width: 4,
        }, // Type
        ColumnConfig {
            max_width: if is_wide { 20 } else { 16 },
            min_width: 9,
        }, // Languages (wider for 2-3 items)
        ColumnConfig {
            max_width: if is_wide { 22 } else { 18 },
            min_width: 9,
        }, // Main Tech (wider for 2 items)
        ColumnConfig {
            max_width: if is_wide { 16 } else { 12 },
            min_width: 5,
        }, // Ports
        ColumnConfig {
            max_width: 6,
            min_width: 6,
        }, // Docker
        ColumnConfig {
            max_width: 4,
            min_width: 4,
        }, // Deps
    ];

    // Collect all data first, applying smart formatting
    let mut project_data = Vec::new();
    for project in &analysis.projects {
        let name = smart_truncate(&project.name, col_configs[0].max_width);
        let proj_type = smart_truncate(
            format_project_category(&project.project_category),
            col_configs[1].max_width,
        );

        // Languages: show 2-3 with "+N" for extras (wider terminals get 3)
        let lang_names: Vec<String> = project
            .analysis
            .languages
            .iter()
            .map(|l| l.name.clone())
            .collect();
        let max_langs = if is_wide { 3 } else { 2 };
        let languages = format_list_smart(&lang_names, max_langs, 12);

        // Main tech: show 2 with "+N" for extras
        let tech_names: Vec<String> = project
            .analysis
            .technologies
            .iter()
            .map(|t| t.name.clone())
            .collect();
        let main_tech = format_list_smart(&tech_names, 2, 14);

        // Smart ports: deduplicate and limit to 3
        let port_numbers: Vec<u16> = project.analysis.ports.iter().map(|p| p.number).collect();
        let ports = format_ports_smart(&port_numbers, 3);

        let docker = if project.analysis.docker_analysis.is_some() {
            "Yes"
        } else {
            "No"
        };
        let deps_count = project.analysis.dependencies.len().to_string();

        project_data.push((
            name,
            proj_type,
            languages,
            main_tech,
            ports,
            docker.to_string(),
            deps_count,
        ));
    }

    // Calculate column widths based on content (capped by max_width)
    let headers = [
        "Project",
        "Type",
        "Languages",
        "Main Tech",
        "Ports",
        "Docker",
        "Deps",
    ];
    let mut col_widths: Vec<usize> = headers
        .iter()
        .zip(&col_configs)
        .map(|(h, cfg)| visual_width(h).clamp(cfg.min_width, cfg.max_width))
        .collect();

    for (name, proj_type, languages, main_tech, ports, docker, deps_count) in &project_data {
        col_widths[0] = col_widths[0]
            .max(visual_width(name))
            .min(col_configs[0].max_width);
        col_widths[1] = col_widths[1]
            .max(visual_width(proj_type))
            .min(col_configs[1].max_width);
        col_widths[2] = col_widths[2]
            .max(visual_width(languages))
            .min(col_configs[2].max_width);
        col_widths[3] = col_widths[3]
            .max(visual_width(main_tech))
            .min(col_configs[3].max_width);
        col_widths[4] = col_widths[4]
            .max(visual_width(ports))
            .min(col_configs[4].max_width);
        col_widths[5] = col_widths[5]
            .max(visual_width(docker))
            .min(col_configs[5].max_width);
        col_widths[6] = col_widths[6]
            .max(visual_width(deps_count))
            .min(col_configs[6].max_width);
    }

    // Create header row
    let header_parts: Vec<String> = headers
        .iter()
        .zip(&col_widths)
        .map(|(h, &w)| format!("{:<width$}", h, width = w))
        .collect();
    let header_line = header_parts.join(" │ ");
    box_drawer.add_value_only(&header_line);

    // Add separator
    let separator_parts: Vec<String> = col_widths.iter().map(|&w| "─".repeat(w)).collect();
    let separator_line = separator_parts.join("─┼─");
    box_drawer.add_value_only(&separator_line);

    // Add data rows
    for (name, proj_type, languages, main_tech, ports, docker, deps_count) in project_data {
        let row_parts = [
            format!("{:<width$}", name, width = col_widths[0]),
            format!("{:<width$}", proj_type, width = col_widths[1]),
            format!("{:<width$}", languages, width = col_widths[2]),
            format!("{:<width$}", main_tech, width = col_widths[3]),
            format!("{:<width$}", ports, width = col_widths[4]),
            format!("{:<width$}", docker, width = col_widths[5]),
            format!("{:<width$}", deps_count, width = col_widths[6]),
        ];
        let row_line = row_parts.join(" │ ");
        box_drawer.add_value_only(&row_line);
    }

    println!("\n{}", box_drawer.draw());
}

/// Display projects in a matrix table format - returns string (with smart truncation)
fn display_projects_matrix_to_string(analysis: &MonorepoAnalysis) -> String {
    let term_width = get_terminal_width();
    let mut box_drawer = BoxDrawer::new("Projects Matrix");

    // Column configuration: max widths to prevent explosion
    let is_wide = term_width >= 120;
    let col_configs = [
        ColumnConfig {
            max_width: if is_wide { 24 } else { 18 },
            min_width: 7,
        }, // Project
        ColumnConfig {
            max_width: 10,
            min_width: 4,
        }, // Type
        ColumnConfig {
            max_width: if is_wide { 20 } else { 16 },
            min_width: 9,
        }, // Languages (wider for 2-3 items)
        ColumnConfig {
            max_width: if is_wide { 22 } else { 18 },
            min_width: 9,
        }, // Main Tech (wider for 2 items)
        ColumnConfig {
            max_width: if is_wide { 16 } else { 12 },
            min_width: 5,
        }, // Ports
        ColumnConfig {
            max_width: 6,
            min_width: 6,
        }, // Docker
        ColumnConfig {
            max_width: 4,
            min_width: 4,
        }, // Deps
    ];

    // Collect all data first, applying smart formatting
    let mut project_data = Vec::new();
    for project in &analysis.projects {
        let name = smart_truncate(&project.name, col_configs[0].max_width);
        let proj_type = smart_truncate(
            format_project_category(&project.project_category),
            col_configs[1].max_width,
        );

        // Languages: show 2-3 with "+N" for extras (wider terminals get 3)
        let lang_names: Vec<String> = project
            .analysis
            .languages
            .iter()
            .map(|l| l.name.clone())
            .collect();
        let max_langs = if is_wide { 3 } else { 2 };
        let languages = format_list_smart(&lang_names, max_langs, 12);

        // Main tech: show 2 with "+N" for extras
        let tech_names: Vec<String> = project
            .analysis
            .technologies
            .iter()
            .map(|t| t.name.clone())
            .collect();
        let main_tech = format_list_smart(&tech_names, 2, 14);

        // Smart ports: deduplicate and limit to 3
        let port_numbers: Vec<u16> = project.analysis.ports.iter().map(|p| p.number).collect();
        let ports = format_ports_smart(&port_numbers, 3);

        let docker = if project.analysis.docker_analysis.is_some() {
            "Yes"
        } else {
            "No"
        };
        let deps_count = project.analysis.dependencies.len().to_string();

        project_data.push((
            name,
            proj_type,
            languages,
            main_tech,
            ports,
            docker.to_string(),
            deps_count,
        ));
    }

    // Calculate column widths based on content (capped by max_width)
    let headers = [
        "Project",
        "Type",
        "Languages",
        "Main Tech",
        "Ports",
        "Docker",
        "Deps",
    ];
    let mut col_widths: Vec<usize> = headers
        .iter()
        .zip(&col_configs)
        .map(|(h, cfg)| visual_width(h).clamp(cfg.min_width, cfg.max_width))
        .collect();

    for (name, proj_type, languages, main_tech, ports, docker, deps_count) in &project_data {
        col_widths[0] = col_widths[0]
            .max(visual_width(name))
            .min(col_configs[0].max_width);
        col_widths[1] = col_widths[1]
            .max(visual_width(proj_type))
            .min(col_configs[1].max_width);
        col_widths[2] = col_widths[2]
            .max(visual_width(languages))
            .min(col_configs[2].max_width);
        col_widths[3] = col_widths[3]
            .max(visual_width(main_tech))
            .min(col_configs[3].max_width);
        col_widths[4] = col_widths[4]
            .max(visual_width(ports))
            .min(col_configs[4].max_width);
        col_widths[5] = col_widths[5]
            .max(visual_width(docker))
            .min(col_configs[5].max_width);
        col_widths[6] = col_widths[6]
            .max(visual_width(deps_count))
            .min(col_configs[6].max_width);
    }

    // Create header row
    let header_parts: Vec<String> = headers
        .iter()
        .zip(&col_widths)
        .map(|(h, &w)| format!("{:<width$}", h, width = w))
        .collect();
    let header_line = header_parts.join(" │ ");
    box_drawer.add_value_only(&header_line);

    // Add separator
    let separator_parts: Vec<String> = col_widths.iter().map(|&w| "─".repeat(w)).collect();
    let separator_line = separator_parts.join("─┼─");
    box_drawer.add_value_only(&separator_line);

    // Add data rows
    for (name, proj_type, languages, main_tech, ports, docker, deps_count) in project_data {
        let row_parts = [
            format!("{:<width$}", name, width = col_widths[0]),
            format!("{:<width$}", proj_type, width = col_widths[1]),
            format!("{:<width$}", languages, width = col_widths[2]),
            format!("{:<width$}", main_tech, width = col_widths[3]),
            format!("{:<width$}", ports, width = col_widths[4]),
            format!("{:<width$}", docker, width = col_widths[5]),
            format!("{:<width$}", deps_count, width = col_widths[6]),
        ];
        let row_line = row_parts.join(" │ ");
        box_drawer.add_value_only(&row_line);
    }

    format!("\n{}", box_drawer.draw())
}

/// Display single project in matrix format
fn display_single_project_matrix(analysis: &MonorepoAnalysis) {
    if let Some(project) = analysis.projects.first() {
        let colors = get_color_adapter();
        let mut box_drawer = BoxDrawer::new("Project Overview");

        // Basic info
        box_drawer.add_line("Name:", &colors.primary(&project.name), true);
        box_drawer.add_line(
            "Type:",
            &colors.secondary(format_project_category(&project.project_category)),
            true,
        );

        // Languages
        if !project.analysis.languages.is_empty() {
            let lang_info = project
                .analysis
                .languages
                .iter()
                .map(|l| l.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            box_drawer.add_line("Languages:", &colors.language(&lang_info), true);
        }

        // Technologies by category (simplified for string version)
        if !project.analysis.technologies.is_empty() {
            let tech_names = project
                .analysis
                .technologies
                .iter()
                .take(3)
                .map(|t| t.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            box_drawer.add_line("Technologies:", &colors.technology(&tech_names), true);
        }

        // Key metrics
        box_drawer.add_separator();
        box_drawer.add_line("Key Metrics:", "", true);

        // Display metrics on two lines to fit properly
        box_drawer.add_value_only(&colors.info(&format!(
            "Entry Points: {} │ Exposed Ports: {} │ Env Variables: {}",
            project.analysis.entry_points.len(),
            project.analysis.ports.len(),
            project.analysis.environment_variables.len()
        )));

        box_drawer.add_value_only(&colors.info(&format!(
            "Build Scripts: {} │ Dependencies: {}",
            project.analysis.build_scripts.len(),
            project.analysis.dependencies.len()
        )));

        // Confidence score with progress bar
        add_confidence_bar_to_drawer(
            project.analysis.analysis_metadata.confidence_score,
            &mut box_drawer,
        );

        println!("\n{}", box_drawer.draw());
    }
}

/// Display single project in matrix format - returns string
fn display_single_project_matrix_to_string(analysis: &MonorepoAnalysis) -> String {
    if let Some(project) = analysis.projects.first() {
        let colors = get_color_adapter();
        let mut box_drawer = BoxDrawer::new("Project Overview");

        // Basic info
        box_drawer.add_line("Name:", &colors.primary(&project.name), true);
        box_drawer.add_line(
            "Type:",
            &colors.secondary(format_project_category(&project.project_category)),
            true,
        );

        // Languages
        if !project.analysis.languages.is_empty() {
            let lang_info = project
                .analysis
                .languages
                .iter()
                .map(|l| l.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            box_drawer.add_line("Languages:", &colors.language(&lang_info), true);
        }

        // Technologies by category (simplified for string version)
        if !project.analysis.technologies.is_empty() {
            let tech_names = project
                .analysis
                .technologies
                .iter()
                .take(3)
                .map(|t| t.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            box_drawer.add_line("Technologies:", &colors.technology(&tech_names), true);
        }

        // Key metrics
        box_drawer.add_separator();
        box_drawer.add_line("Key Metrics:", "", true);

        // Display metrics on two lines to fit properly
        box_drawer.add_value_only(&colors.info(&format!(
            "Entry Points: {} │ Exposed Ports: {} │ Env Variables: {}",
            project.analysis.entry_points.len(),
            project.analysis.ports.len(),
            project.analysis.environment_variables.len()
        )));

        box_drawer.add_value_only(&colors.info(&format!(
            "Build Scripts: {} │ Dependencies: {}",
            project.analysis.build_scripts.len(),
            project.analysis.dependencies.len()
        )));

        // Confidence score with progress bar
        add_confidence_bar_to_drawer(
            project.analysis.analysis_metadata.confidence_score,
            &mut box_drawer,
        );

        format!("\n{}", box_drawer.draw())
    } else {
        String::new()
    }
}

/// Display Docker infrastructure overview in matrix format
fn display_docker_overview_matrix(analysis: &MonorepoAnalysis) {
    let colors = get_color_adapter();
    let mut box_drawer = BoxDrawer::new("Docker Infrastructure");

    let mut total_dockerfiles = 0;
    let mut total_compose_files = 0;
    let mut total_services = 0;
    let mut orchestration_patterns = std::collections::HashSet::new();

    for project in &analysis.projects {
        if let Some(docker) = &project.analysis.docker_analysis {
            total_dockerfiles += docker.dockerfiles.len();
            total_compose_files += docker.compose_files.len();
            total_services += docker.services.len();
            orchestration_patterns.insert(&docker.orchestration_pattern);
        }
    }

    box_drawer.add_line(
        "Dockerfiles:",
        &colors.metric(&total_dockerfiles.to_string()),
        true,
    );
    box_drawer.add_line(
        "Compose Files:",
        &colors.metric(&total_compose_files.to_string()),
        true,
    );
    box_drawer.add_line(
        "Total Services:",
        &colors.metric(&total_services.to_string()),
        true,
    );

    let patterns = orchestration_patterns
        .iter()
        .map(|p| format!("{:?}", p))
        .collect::<Vec<_>>()
        .join(", ");
    box_drawer.add_line(
        "Orchestration Patterns:",
        &colors.secondary(&patterns),
        true,
    );

    // Service connectivity summary
    let mut has_services = false;
    for project in &analysis.projects {
        if let Some(docker) = &project.analysis.docker_analysis {
            for service in &docker.services {
                if !service.ports.is_empty() || !service.depends_on.is_empty() {
                    has_services = true;
                    break;
                }
            }
        }
    }

    if has_services {
        box_drawer.add_separator();
        box_drawer.add_line("Service Connectivity:", "", true);

        for project in &analysis.projects {
            if let Some(docker) = &project.analysis.docker_analysis {
                for service in &docker.services {
                    if !service.ports.is_empty() || !service.depends_on.is_empty() {
                        let port_info = service
                            .ports
                            .iter()
                            .filter_map(|p| {
                                p.host_port.map(|hp| format!("{}:{}", hp, p.container_port))
                            })
                            .collect::<Vec<_>>()
                            .join(", ");

                        let deps_info = if service.depends_on.is_empty() {
                            String::new()
                        } else {
                            format!(" → {}", service.depends_on.join(", "))
                        };

                        let info = format!("  {}: {}{}", service.name, port_info, deps_info);
                        box_drawer.add_value_only(&colors.info(&info));
                    }
                }
            }
        }
    }

    println!("\n{}", box_drawer.draw());
}

/// Display docker overview matrix - returns string
fn display_docker_overview_matrix_to_string(analysis: &MonorepoAnalysis) -> String {
    let colors = get_color_adapter();
    let mut box_drawer = BoxDrawer::new("Docker Infrastructure");

    let mut total_dockerfiles = 0;
    let mut total_compose_files = 0;
    let mut total_services = 0;

    for project in &analysis.projects {
        if let Some(docker) = &project.analysis.docker_analysis {
            total_dockerfiles += docker.dockerfiles.len();
            total_compose_files += docker.compose_files.len();
            total_services += docker.services.len();
        }
    }

    box_drawer.add_line(
        "Dockerfiles:",
        &colors.metric(&total_dockerfiles.to_string()),
        true,
    );
    box_drawer.add_line(
        "Compose Files:",
        &colors.metric(&total_compose_files.to_string()),
        true,
    );
    box_drawer.add_line(
        "Total Services:",
        &colors.metric(&total_services.to_string()),
        true,
    );

    format!("\n{}", box_drawer.draw())
}

/// Display analysis metrics
fn display_metrics_box(analysis: &MonorepoAnalysis) {
    let colors = get_color_adapter();
    let mut box_drawer = BoxDrawer::new("Analysis Metrics");

    // Performance metrics
    let duration_ms = analysis.metadata.analysis_duration_ms;
    let duration_str = if duration_ms < 1000 {
        format!("{}ms", duration_ms)
    } else {
        format!("{:.1}s", duration_ms as f64 / 1000.0)
    };

    // Create metrics line without emojis first to avoid width calculation issues
    let metrics_line = format!(
        "Duration: {} | Files: {} | Score: {:.0}% | Version: {}",
        duration_str,
        analysis.metadata.files_analyzed,
        analysis.metadata.confidence_score * 100.0,
        analysis.metadata.analyzer_version
    );

    // Apply single color to the entire line for consistency
    let colored_metrics = colors.info(&metrics_line);
    box_drawer.add_value_only(&colored_metrics.to_string());

    println!("\n{}", box_drawer.draw());
}

/// Display analysis metrics - returns string
fn display_metrics_box_to_string(analysis: &MonorepoAnalysis) -> String {
    let colors = get_color_adapter();
    let mut box_drawer = BoxDrawer::new("Analysis Metrics");

    // Performance metrics
    let duration_ms = analysis.metadata.analysis_duration_ms;
    let duration_str = if duration_ms < 1000 {
        format!("{}ms", duration_ms)
    } else {
        format!("{:.1}s", duration_ms as f64 / 1000.0)
    };

    // Create metrics line
    let metrics_line = format!(
        "Duration: {} | Files: {} | Score: {:.0}% | Version: {}",
        duration_str,
        analysis.metadata.files_analyzed,
        analysis.metadata.confidence_score * 100.0,
        analysis.metadata.analyzer_version
    );

    box_drawer.add_value_only(&colors.info(&metrics_line));

    format!("\n{}", box_drawer.draw())
}
