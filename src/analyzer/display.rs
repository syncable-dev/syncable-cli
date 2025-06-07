//! # Display Module
//! 
//! Provides improved CLI output formatting with matrix/dashboard views for better readability
//! and easier parsing by both humans and LLMs.

use crate::analyzer::{
    MonorepoAnalysis, ProjectCategory, ArchitecturePattern,
    DetectedTechnology, TechnologyCategory, LibraryType,
    DockerAnalysis, OrchestrationPattern,
};
use colored::*;

/// Content line for measuring and drawing
#[derive(Debug, Clone)]
struct ContentLine {
    label: String,
    value: String,
    label_colored: bool,
}

impl ContentLine {
    fn new(label: &str, value: &str, label_colored: bool) -> Self {
        Self {
            label: label.to_string(),
            value: value.to_string(),
            label_colored,
        }
    }
    
    
    fn separator() -> Self {
        Self {
            label: "SEPARATOR".to_string(),
            value: String::new(),
            label_colored: false,
        }
    }
    

}

/// Box drawer that pre-calculates optimal dimensions
pub struct BoxDrawer {
    title: String,
    lines: Vec<ContentLine>,
    min_width: usize,
    max_width: usize,
}

impl BoxDrawer {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            lines: Vec::new(),
            min_width: 60,
            max_width: 120, // Reduced from 150 for better terminal compatibility
        }
    }
    
    pub fn add_line(&mut self, label: &str, value: &str, label_colored: bool) {
        self.lines.push(ContentLine::new(label, value, label_colored));
    }
    
    pub fn add_value_only(&mut self, value: &str) {
        self.lines.push(ContentLine::new("", value, false));
    }
    
    pub fn add_separator(&mut self) {
        self.lines.push(ContentLine::separator());
    }
    
    /// Calculate optimal box width based on content
    fn calculate_optimal_width(&self) -> usize {
        let title_width = visual_width(&self.title) + 6; // "┌─ " + title + " " + extra padding
        let mut max_content_width = 0;
        
        // Calculate the actual rendered width for each line
        for line in &self.lines {
            if line.label == "SEPARATOR" {
                continue;
            }
            
            let rendered_width = self.calculate_rendered_line_width(line);
            max_content_width = max_content_width.max(rendered_width);
        }
        
        // Add reasonable buffer for content
        let content_width_with_buffer = max_content_width + 4; // More buffer for safety
        
        // Box needs padding: "│ " + content + " │" = content + 4
        let needed_width = content_width_with_buffer + 4;
        
        // Use the maximum of title width and content width
        let optimal_width = title_width.max(needed_width).max(self.min_width);
        optimal_width.min(self.max_width)
    }
    
    /// Calculate the actual rendered width of a line as it will appear
    fn calculate_rendered_line_width(&self, line: &ContentLine) -> usize {
        let label_width = visual_width(&line.label);
        let value_width = visual_width(&line.value);
        
        if !line.label.is_empty() && !line.value.is_empty() {
            // Label + value: need space between them
            // For colored labels, ensure minimum spacing
            let min_label_space = if line.label_colored { 25 } else { label_width };
            min_label_space + 2 + value_width // 2 spaces minimum between label and value
        } else if !line.value.is_empty() {
            // Value only
            value_width
        } else if !line.label.is_empty() {
            // Label only
            label_width
        } else {
            // Empty line
            0
        }
    }
    
    /// Draw the complete box
    pub fn draw(&self) -> String {
        let box_width = self.calculate_optimal_width();
        let content_width = box_width - 4; // Available space for content
        
        let mut output = Vec::new();
        
        // Top border
        output.push(self.draw_top(box_width));
        
        // Content lines
        for line in &self.lines {
            if line.label == "SEPARATOR" {
                output.push(self.draw_separator(box_width));
            } else if line.label.is_empty() && line.value.is_empty() {
                output.push(self.draw_empty_line(box_width));
            } else {
                output.push(self.draw_content_line(line, content_width));
            }
        }
        
        // Bottom border
        output.push(self.draw_bottom(box_width));
        
        output.join("\n")
    }
    
    fn draw_top(&self, width: usize) -> String {
        let title_colored = self.title.bright_cyan();
        let title_len = visual_width(&self.title);
        
        // "┌─ " + title + " " + remaining dashes + "┐"
        let prefix_len = 3; // "┌─ "
        let suffix_len = 1; // "┐"
        let title_space = 1; // space after title
        
        let remaining_space = width - prefix_len - title_len - title_space - suffix_len;
        
        format!("┌─ {} {}┐", 
            title_colored,
            "─".repeat(remaining_space)
        )
    }
    
    fn draw_bottom(&self, width: usize) -> String {
        format!("└{}┘", "─".repeat(width - 2))
    }
    
    fn draw_separator(&self, width: usize) -> String {
        format!("│ {} │", "─".repeat(width - 4).dimmed())
    }
    
    fn draw_empty_line(&self, width: usize) -> String {
        format!("│ {} │", " ".repeat(width - 4))
    }
    
    fn draw_content_line(&self, line: &ContentLine, content_width: usize) -> String {
        // Format the label with color if needed
        let formatted_label = if line.label_colored && !line.label.is_empty() {
            line.label.bright_white().to_string()
        } else {
            line.label.clone()
        };
        
        // Calculate actual display widths (use original label for width)
        let label_display_width = visual_width(&line.label);
        let value_display_width = visual_width(&line.value);
        
        // Build the content
        let content = if !line.label.is_empty() && !line.value.is_empty() {
            // Both label and value - ensure proper spacing
            let min_label_space = if line.label_colored { 25 } else { label_display_width };
            let label_padding = min_label_space.saturating_sub(label_display_width);
            let remaining_space = content_width.saturating_sub(min_label_space + 2); // 2 for spacing
            
            if value_display_width <= remaining_space {
                // Value fits - right align it
                let value_padding = remaining_space.saturating_sub(value_display_width);
                format!("{}{:<width$}  {}{}", 
                    formatted_label, 
                    "",
                    " ".repeat(value_padding),
                    line.value,
                    width = label_padding
                )
            } else {
                // Value too long - truncate it
                let truncated_value = truncate_to_width(&line.value, remaining_space.saturating_sub(3));
                format!("{}{:<width$}  {}", 
                    formatted_label, 
                    "",
                    truncated_value,
                    width = label_padding
                )
            }
        } else if !line.value.is_empty() {
            // Value only - left align
            if value_display_width <= content_width {
                format!("{:<width$}", line.value, width = content_width)
            } else {
                truncate_to_width(&line.value, content_width)
            }
        } else if !line.label.is_empty() {
            // Label only - left align
            if label_display_width <= content_width {
                format!("{:<width$}", formatted_label, width = content_width)
            } else {
                truncate_to_width(&formatted_label, content_width)
            }
        } else {
            // Empty line
            " ".repeat(content_width)
        };
        
        // Ensure final content is exactly the right width
        let actual_width = visual_width(&content);
        let final_content = if actual_width < content_width {
            format!("{}{}", content, " ".repeat(content_width - actual_width))
        } else if actual_width > content_width {
            truncate_to_width(&content, content_width)
        } else {
            content
        };
        
        format!("│ {} │", final_content)
    }
}

/// Calculate visual width of a string, handling ANSI color codes
fn visual_width(s: &str) -> usize {
    let mut width = 0;
    let mut chars = s.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip ANSI escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                while let Some(c) = chars.next() {
                    if c.is_ascii_alphabetic() {
                        break; // End of escape sequence
                    }
                }
            }
        } else {
            // Simple width calculation for common cases
            // Most characters are width 1, some are width 0 or 2
            width += char_width(ch);
        }
    }
    
    width
}

/// Simple character width calculation without external dependencies
fn char_width(ch: char) -> usize {
    match ch {
        // Control characters have width 0
        '\u{0000}'..='\u{001F}' | '\u{007F}' => 0,
        // Combining marks have width 0
        '\u{0300}'..='\u{036F}' => 0,
        // Emoji and symbols (width 2)
        '\u{2600}'..='\u{26FF}' |    // Miscellaneous Symbols
        '\u{2700}'..='\u{27BF}' |    // Dingbats
        '\u{1F000}'..='\u{1F02F}' |  // Mahjong Tiles
        '\u{1F030}'..='\u{1F09F}' |  // Domino Tiles
        '\u{1F0A0}'..='\u{1F0FF}' |  // Playing Cards
        '\u{1F100}'..='\u{1F1FF}' |  // Enclosed Alphanumeric Supplement
        '\u{1F200}'..='\u{1F2FF}' |  // Enclosed Ideographic Supplement
        '\u{1F300}'..='\u{1F5FF}' |  // Miscellaneous Symbols and Pictographs
        '\u{1F600}'..='\u{1F64F}' |  // Emoticons
        '\u{1F650}'..='\u{1F67F}' |  // Ornamental Dingbats
        '\u{1F680}'..='\u{1F6FF}' |  // Transport and Map Symbols
        '\u{1F700}'..='\u{1F77F}' |  // Alchemical Symbols
        '\u{1F780}'..='\u{1F7FF}' |  // Geometric Shapes Extended
        '\u{1F800}'..='\u{1F8FF}' |  // Supplemental Arrows-C
        '\u{1F900}'..='\u{1F9FF}' |  // Supplemental Symbols and Pictographs
        // Full-width characters (common CJK ranges)
        '\u{1100}'..='\u{115F}' |  // Hangul Jamo
        '\u{2E80}'..='\u{2EFF}' |  // CJK Radicals
        '\u{2F00}'..='\u{2FDF}' |  // Kangxi Radicals
        '\u{2FF0}'..='\u{2FFF}' |  // Ideographic Description
        '\u{3000}'..='\u{303E}' |  // CJK Symbols and Punctuation
        '\u{3041}'..='\u{3096}' |  // Hiragana
        '\u{30A1}'..='\u{30FA}' |  // Katakana
        '\u{3105}'..='\u{312D}' |  // Bopomofo
        '\u{3131}'..='\u{318E}' |  // Hangul Compatibility Jamo
        '\u{3190}'..='\u{31BA}' |  // Kanbun
        '\u{31C0}'..='\u{31E3}' |  // CJK Strokes
        '\u{31F0}'..='\u{31FF}' |  // Katakana Phonetic Extensions
        '\u{3200}'..='\u{32FF}' |  // Enclosed CJK Letters and Months
        '\u{3300}'..='\u{33FF}' |  // CJK Compatibility
        '\u{3400}'..='\u{4DBF}' |  // CJK Extension A
        '\u{4E00}'..='\u{9FFF}' |  // CJK Unified Ideographs
        '\u{A000}'..='\u{A48C}' |  // Yi Syllables
        '\u{A490}'..='\u{A4C6}' |  // Yi Radicals
        '\u{AC00}'..='\u{D7AF}' |  // Hangul Syllables
        '\u{F900}'..='\u{FAFF}' |  // CJK Compatibility Ideographs
        '\u{FE10}'..='\u{FE19}' |  // Vertical Forms
        '\u{FE30}'..='\u{FE6F}' |  // CJK Compatibility Forms
        '\u{FF00}'..='\u{FF60}' |  // Fullwidth Forms
        '\u{FFE0}'..='\u{FFE6}' => 2,
        // Most other printable characters have width 1
        _ => 1,
    }
}

/// Truncate string to specified visual width, preserving color codes
fn truncate_to_width(s: &str, max_width: usize) -> String {
    let current_visual_width = visual_width(s);
    if current_visual_width <= max_width {
        return s.to_string();
    }
    
    // For strings with ANSI codes, we need to be more careful
    if s.contains('\x1b') {
        // Simple approach: strip ANSI codes, truncate, then re-apply if needed
        let stripped = strip_ansi_codes(s);
        if visual_width(&stripped) <= max_width {
            return s.to_string();
        }
        
        // Truncate the stripped version
        let mut result = String::new();
        let mut width = 0;
        for ch in stripped.chars() {
            let ch_width = char_width(ch);
            if width + ch_width > max_width.saturating_sub(3) {
                result.push_str("...");
                break;
            }
            result.push(ch);
            width += ch_width;
        }
        return result;
    }
    
    // No ANSI codes - simple truncation
    let mut result = String::new();
    let mut width = 0;
    
    for ch in s.chars() {
        let ch_width = char_width(ch);
        if width + ch_width > max_width.saturating_sub(3) {
            result.push_str("...");
            break;
        }
        result.push(ch);
        width += ch_width;
    }
    
    result
}

/// Strip ANSI escape codes from a string
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip ANSI escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                while let Some(c) = chars.next() {
                    if c.is_ascii_alphabetic() {
                        break; // End of escape sequence
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// Display mode for analysis output
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    /// Compact matrix view (default)
    Matrix,
    /// Detailed vertical view (legacy)
    Detailed,
    /// Summary only
    Summary,
    /// JSON output
    Json,
}

/// Main display function that routes to appropriate formatter
pub fn display_analysis(analysis: &MonorepoAnalysis, mode: DisplayMode) {
    match mode {
        DisplayMode::Matrix => display_matrix_view(analysis),
        DisplayMode::Detailed => display_detailed_view(analysis),
        DisplayMode::Summary => display_summary_view(analysis),
        DisplayMode::Json => display_json_view(analysis),
    }
}

/// Display analysis in a compact matrix/dashboard format
pub fn display_matrix_view(analysis: &MonorepoAnalysis) {
    // Header
    println!("\n{}", "═".repeat(100).bright_blue());
    println!("{}", "📊 PROJECT ANALYSIS DASHBOARD".bright_white().bold());
    println!("{}", "═".repeat(100).bright_blue());
    
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
    if analysis.projects.iter().any(|p| p.analysis.docker_analysis.is_some()) {
        display_docker_overview_matrix(analysis);
    }
    
    // Analysis Metrics Box
    display_metrics_box(analysis);
    
    // Footer
    println!("\n{}", "═".repeat(100).bright_blue());
}

/// Display architecture overview in a box
fn display_architecture_box(analysis: &MonorepoAnalysis) {
    let mut box_drawer = BoxDrawer::new("Architecture Overview");
    
    let arch_type = if analysis.is_monorepo {
        format!("Monorepo ({} projects)", analysis.projects.len())
    } else {
        "Single Project".to_string()
    };
    
    box_drawer.add_line("Type:", &arch_type.yellow(), true);
    box_drawer.add_line("Pattern:", &format!("{:?}", analysis.technology_summary.architecture_pattern).green(), true);
    
    // Pattern description
    let pattern_desc = match &analysis.technology_summary.architecture_pattern {
        ArchitecturePattern::Monolithic => "Single, self-contained application",
        ArchitecturePattern::Fullstack => "Full-stack app with frontend/backend separation",
        ArchitecturePattern::Microservices => "Multiple independent microservices",
        ArchitecturePattern::ApiFirst => "API-first architecture with service interfaces",
        ArchitecturePattern::EventDriven => "Event-driven with decoupled components",
        ArchitecturePattern::Mixed => "Mixed architecture patterns",
    };
    box_drawer.add_value_only(&pattern_desc.dimmed());
    
    println!("\n{}", box_drawer.draw());
}

/// Display technology stack overview
fn display_technology_stack_box(analysis: &MonorepoAnalysis) {
    let mut box_drawer = BoxDrawer::new("Technology Stack");
    
    let mut has_content = false;
    
    // Languages
    if !analysis.technology_summary.languages.is_empty() {
        let languages = analysis.technology_summary.languages.join(", ");
        box_drawer.add_line("Languages:", &languages.blue(), true);
        has_content = true;
    }
    
    // Frameworks
    if !analysis.technology_summary.frameworks.is_empty() {
        let frameworks = analysis.technology_summary.frameworks.join(", ");
        box_drawer.add_line("Frameworks:", &frameworks.magenta(), true);
        has_content = true;
    }
    
    // Databases
    if !analysis.technology_summary.databases.is_empty() {
        let databases = analysis.technology_summary.databases.join(", ");
        box_drawer.add_line("Databases:", &databases.cyan(), true);
        has_content = true;
    }
    
    if !has_content {
        box_drawer.add_value_only("No technologies detected");
    }
    
    println!("\n{}", box_drawer.draw());
}

/// Display projects in a matrix table format
fn display_projects_matrix(analysis: &MonorepoAnalysis) {
    let mut box_drawer = BoxDrawer::new("Projects Matrix");
    
    // Collect all data first to calculate optimal column widths
    let mut project_data = Vec::new();
    for project in &analysis.projects {
        let name = project.name.clone(); // Remove emoji to avoid width calculation issues
        let proj_type = format_project_category(&project.project_category);
        
        let languages = project.analysis.languages.iter()
            .map(|l| l.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        
        let main_tech = get_main_technologies(&project.analysis.technologies);
        
        let ports = if project.analysis.ports.is_empty() {
            "-".to_string()
        } else {
            project.analysis.ports.iter()
                .map(|p| p.number.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        
        let docker = if project.analysis.docker_analysis.is_some() {
            "Yes"
        } else {
            "No"
        };
        
        let deps_count = project.analysis.dependencies.len().to_string();
        
        project_data.push((name, proj_type.to_string(), languages, main_tech, ports, docker.to_string(), deps_count));
    }
    
    // Calculate column widths based on content
    let headers = vec!["Project", "Type", "Languages", "Main Tech", "Ports", "Docker", "Deps"];
    let mut col_widths = headers.iter().map(|h| visual_width(h)).collect::<Vec<_>>();
    
    for (name, proj_type, languages, main_tech, ports, docker, deps_count) in &project_data {
        col_widths[0] = col_widths[0].max(visual_width(name));
        col_widths[1] = col_widths[1].max(visual_width(proj_type));
        col_widths[2] = col_widths[2].max(visual_width(languages));
        col_widths[3] = col_widths[3].max(visual_width(main_tech));
        col_widths[4] = col_widths[4].max(visual_width(ports));
        col_widths[5] = col_widths[5].max(visual_width(docker));
        col_widths[6] = col_widths[6].max(visual_width(deps_count));
    }
    

    // Create header row
    let header_parts: Vec<String> = headers.iter().zip(&col_widths)
        .map(|(h, &w)| format!("{:<width$}", h, width = w))
        .collect();
    let header_line = header_parts.join(" │ ");
    box_drawer.add_value_only(&header_line);
    
    // Add separator
    let separator_parts: Vec<String> = col_widths.iter()
        .map(|&w| "─".repeat(w))
        .collect();
    let separator_line = separator_parts.join("─┼─");
    box_drawer.add_value_only(&separator_line);
    
    // Add data rows
    for (name, proj_type, languages, main_tech, ports, docker, deps_count) in project_data {
        let row_parts = vec![
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

/// Display single project in matrix format
fn display_single_project_matrix(analysis: &MonorepoAnalysis) {
    if let Some(project) = analysis.projects.first() {
        let mut box_drawer = BoxDrawer::new("Project Overview");
        
        // Basic info
        box_drawer.add_line("Name:", &project.name.yellow(), true);
        box_drawer.add_line("Type:", &format_project_category(&project.project_category).green(), true);
        
        // Languages 
        if !project.analysis.languages.is_empty() {
            let lang_info = project.analysis.languages.iter()
                .map(|l| l.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            box_drawer.add_line("Languages:", &lang_info.blue(), true);
        }
        
        // Technologies by category
        add_technologies_to_drawer(&project.analysis.technologies, &mut box_drawer);
        
        // Key metrics
        box_drawer.add_separator();
        box_drawer.add_line("Key Metrics:", "", true);
        
        // Display metrics on two lines to fit properly
        box_drawer.add_value_only(&format!("Entry Points: {} │ Exposed Ports: {} │ Env Variables: {}", 
            project.analysis.entry_points.len(),
            project.analysis.ports.len(),
            project.analysis.environment_variables.len()
        ).cyan());
        
        box_drawer.add_value_only(&format!("Build Scripts: {} │ Dependencies: {}", 
            project.analysis.build_scripts.len(),
            project.analysis.dependencies.len()
        ).cyan());
        
        // Confidence score with progress bar
        add_confidence_bar_to_drawer(project.analysis.analysis_metadata.confidence_score, &mut box_drawer);
        
        println!("\n{}", box_drawer.draw());
    }
}

/// Add technologies organized by category to the box drawer
fn add_technologies_to_drawer(technologies: &[DetectedTechnology], box_drawer: &mut BoxDrawer) {
    let mut by_category: std::collections::HashMap<&TechnologyCategory, Vec<&DetectedTechnology>> = std::collections::HashMap::new();
    
    for tech in technologies {
        by_category.entry(&tech.category).or_insert_with(Vec::new).push(tech);
    }
    
    // Display primary technology first
    if let Some(primary) = technologies.iter().find(|t| t.is_primary) {
        let primary_info = primary.name.bright_yellow().bold().to_string();
        box_drawer.add_line("Primary Stack:", &primary_info, true);
    }
    
    // Display other categories
    let categories = [
        (TechnologyCategory::FrontendFramework, "Frameworks"),
        (TechnologyCategory::BuildTool, "Build Tools"),
        (TechnologyCategory::Database, "Databases"),
        (TechnologyCategory::Testing, "Testing"),
    ];
    
    for (category, label) in &categories {
        if let Some(techs) = by_category.get(category) {
            let tech_names = techs.iter()
                .map(|t| t.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            
            if !tech_names.is_empty() {
                let label_with_colon = format!("{}:", label);
                box_drawer.add_line(&label_with_colon, &tech_names.magenta(), true);
            }
        }
    }
    
    // Handle Library category separately since it's parameterized - use vertical layout for many items
    let mut all_libraries: Vec<&DetectedTechnology> = Vec::new();
    for (cat, techs) in &by_category {
        if matches!(cat, TechnologyCategory::Library(_)) {
            all_libraries.extend(techs.iter().copied());
        }
    }
    
    if !all_libraries.is_empty() {
        // Sort libraries by confidence for better display
        all_libraries.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        
        if all_libraries.len() <= 3 {
            // For few libraries, keep horizontal layout
            let tech_names = all_libraries.iter()
                .map(|t| t.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            box_drawer.add_line("Libraries:", &tech_names.magenta(), true);
        } else {
            // For many libraries, use vertical layout with multiple rows
            box_drawer.add_line("Libraries:", "", true);
            
            // Group libraries into rows of 3-4 items each
            let items_per_row = 3;
            for chunk in all_libraries.chunks(items_per_row) {
                let row_items = chunk.iter()
                    .map(|t| t.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                
                // Add indented row
                let indented_row = format!("  {}", row_items);
                box_drawer.add_value_only(&indented_row.magenta());
            }
        }
    }
}

/// Display Docker infrastructure overview in matrix format
fn display_docker_overview_matrix(analysis: &MonorepoAnalysis) {
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
    
    box_drawer.add_line("Dockerfiles:", &total_dockerfiles.to_string().yellow(), true);
    box_drawer.add_line("Compose Files:", &total_compose_files.to_string().yellow(), true);
    box_drawer.add_line("Total Services:", &total_services.to_string().yellow(), true);
    
    let patterns = orchestration_patterns.iter()
        .map(|p| format!("{:?}", p))
        .collect::<Vec<_>>()
        .join(", ");
    box_drawer.add_line("Orchestration Patterns:", &patterns.green(), true);
    
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
                        let port_info = service.ports.iter()
                            .filter_map(|p| p.host_port.map(|hp| format!("{}:{}", hp, p.container_port)))
                            .collect::<Vec<_>>()
                            .join(", ");
                        
                        let deps_info = if service.depends_on.is_empty() {
                            String::new()
                        } else {
                            format!(" → {}", service.depends_on.join(", "))
                        };
                        
                        let info = format!("  {}: {}{}", service.name, port_info, deps_info);
                        box_drawer.add_value_only(&info.cyan());
                    }
                }
            }
        }
    }
    
    println!("\n{}", box_drawer.draw());
}

/// Display analysis metrics
fn display_metrics_box(analysis: &MonorepoAnalysis) {
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
        "Duration: {} | Files: {} | Score: {}% | Version: {}",
        duration_str,
        analysis.metadata.files_analyzed,
        format!("{:.0}", analysis.metadata.confidence_score * 100.0),
        analysis.metadata.analyzer_version
    );
    
    // Apply single color to the entire line for consistency
    let colored_metrics = metrics_line.cyan();
    box_drawer.add_value_only(&colored_metrics.to_string());
    
    println!("\n{}", box_drawer.draw());
}

/// Add confidence score as a progress bar to the box drawer
fn add_confidence_bar_to_drawer(score: f32, box_drawer: &mut BoxDrawer) {
    let percentage = (score * 100.0) as u8;
    let bar_width = 20;
    let filled = ((score * bar_width as f32) as usize).min(bar_width);
    
    let bar = format!("{}{}",
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

/// Get main technologies for display
fn get_main_technologies(technologies: &[DetectedTechnology]) -> String {
    let primary = technologies.iter().find(|t| t.is_primary);
    let frameworks: Vec<_> = technologies.iter()
        .filter(|t| matches!(t.category, TechnologyCategory::FrontendFramework | TechnologyCategory::MetaFramework))
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

/// Display in detailed vertical format (legacy)
pub fn display_detailed_view(analysis: &MonorepoAnalysis) {
    // Use the legacy detailed display format
    println!("{}", "=".repeat(80));
    println!("\n📊 PROJECT ANALYSIS RESULTS");
    println!("{}", "=".repeat(80));
    
    // Overall project information
    if analysis.is_monorepo {
        println!("\n🏗️  Architecture: Monorepo with {} projects", analysis.projects.len());
        println!("   Pattern: {:?}", analysis.technology_summary.architecture_pattern);
        
        display_architecture_description(&analysis.technology_summary.architecture_pattern);
    } else {
        println!("\n🏗️  Architecture: Single Project");
    }
    
    // Technology Summary
    println!("\n🌐 Technology Summary:");
    if !analysis.technology_summary.languages.is_empty() {
        println!("   Languages: {}", analysis.technology_summary.languages.join(", "));
    }
    if !analysis.technology_summary.frameworks.is_empty() {
        println!("   Frameworks: {}", analysis.technology_summary.frameworks.join(", "));
    }
    if !analysis.technology_summary.databases.is_empty() {
        println!("   Databases: {}", analysis.technology_summary.databases.join(", "));
    }
    
    // Individual project details
    println!("\n📁 Project Details:");
    println!("{}", "=".repeat(80));
    
    for (i, project) in analysis.projects.iter().enumerate() {
        println!("\n{} {}. {} ({})", 
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
                print!("      • {} (confidence: {:.1}%)", lang.name, lang.confidence * 100.0);
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
            println!("   📍 Entry Points ({}):", project.analysis.entry_points.len());
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
            println!("   🔐 Environment Variables ({}):", project.analysis.environment_variables.len());
            let required_vars: Vec<_> = project.analysis.environment_variables.iter()
                .filter(|ev| ev.required)
                .collect();
            let optional_vars: Vec<_> = project.analysis.environment_variables.iter()
                .filter(|ev| !ev.required)
                .collect();
            
            if !required_vars.is_empty() {
                println!("      Required:");
                for var in required_vars {
                    println!("        • {} {}", 
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
                    println!("        • {} = {:?}", 
                        var.name, 
                        var.default_value.as_deref().unwrap_or("no default")
                    );
                }
            }
        }
        
        // Build Scripts
        if !project.analysis.build_scripts.is_empty() {
            println!("   🔨 Build Scripts ({}):", project.analysis.build_scripts.len());
            let default_scripts: Vec<_> = project.analysis.build_scripts.iter()
                .filter(|bs| bs.is_default)
                .collect();
            let other_scripts: Vec<_> = project.analysis.build_scripts.iter()
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
            println!("   📦 Dependencies ({}):", project.analysis.dependencies.len());
            if project.analysis.dependencies.len() <= 5 {
                for (name, version) in &project.analysis.dependencies {
                    println!("      • {} v{}", name, version);
                }
            } else {
                // Show first 5
                for (name, version) in project.analysis.dependencies.iter().take(5) {
                    println!("      • {} v{}", name, version);
                }
                println!("      ... and {} more", project.analysis.dependencies.len() - 5);
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
        println!("   • Architecture pattern: {:?}", analysis.technology_summary.architecture_pattern);
        
        let frontend_count = analysis.projects.iter().filter(|p| p.project_category == ProjectCategory::Frontend).count();
        let backend_count = analysis.projects.iter().filter(|p| matches!(p.project_category, ProjectCategory::Backend | ProjectCategory::Api)).count();
        let service_count = analysis.projects.iter().filter(|p| p.project_category == ProjectCategory::Service).count();
        let lib_count = analysis.projects.iter().filter(|p| p.project_category == ProjectCategory::Library).count();
        
        if frontend_count > 0 { println!("   • Frontend projects: {}", frontend_count); }
        if backend_count > 0 { println!("   • Backend/API projects: {}", backend_count); }
        if service_count > 0 { println!("   • Service projects: {}", service_count); }
        if lib_count > 0 { println!("   • Library projects: {}", lib_count); }
    }
    
    println!("\n📈 Analysis Metadata:");
    println!("   • Duration: {}ms", analysis.metadata.analysis_duration_ms);
    println!("   • Files analyzed: {}", analysis.metadata.files_analyzed);
    println!("   • Confidence score: {:.1}%", analysis.metadata.confidence_score * 100.0);
    println!("   • Analyzer version: {}", analysis.metadata.analyzer_version);
}

/// Helper function for legacy detailed technology display
fn display_technologies_detailed_legacy(technologies: &[DetectedTechnology]) {
    // Group technologies by category
    let mut by_category: std::collections::HashMap<&TechnologyCategory, Vec<&DetectedTechnology>> = std::collections::HashMap::new();
    
    for tech in technologies {
        by_category.entry(&tech.category).or_insert_with(Vec::new).push(tech);
    }
    
    // Find and display primary technology
    if let Some(primary) = technologies.iter().find(|t| t.is_primary) {
        println!("\n🛠️  Technology Stack:");
        println!("   🎯 PRIMARY: {} (confidence: {:.1}%)", primary.name, primary.confidence * 100.0);
        println!("      Architecture driver for this project");
    }
    
    // Display categories in order
    let categories = [
        (TechnologyCategory::MetaFramework, "🏗️  Meta-Frameworks"),
        (TechnologyCategory::BackendFramework, "🖥️  Backend Frameworks"),
        (TechnologyCategory::FrontendFramework, "🎨 Frontend Frameworks"),
        (TechnologyCategory::Library(LibraryType::UI), "🎨 UI Libraries"),
        (TechnologyCategory::Library(LibraryType::Utility), "📚 Core Libraries"),
        (TechnologyCategory::BuildTool, "🔨 Build Tools"),
        (TechnologyCategory::PackageManager, "📦 Package Managers"),
        (TechnologyCategory::Database, "🗃️  Database & ORM"),
        (TechnologyCategory::Runtime, "⚡ Runtimes"),
        (TechnologyCategory::Testing, "🧪 Testing"),
    ];
    
    for (category, label) in &categories {
        if let Some(techs) = by_category.get(category) {
            if !techs.is_empty() {
                println!("\n   {}:", label);
                for tech in techs {
                    println!("      • {} (confidence: {:.1}%)", tech.name, tech.confidence * 100.0);
                    if let Some(version) = &tech.version {
                        println!("        Version: {}", version);
                    }
                }
            }
        }
    }
    
    // Handle other Library types separately
    for (cat, techs) in &by_category {
        match cat {
            TechnologyCategory::Library(lib_type) => {
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
                        println!("      • {} (confidence: {:.1}%)", tech.name, tech.confidence * 100.0);
                        if let Some(version) = &tech.version {
                            println!("        Version: {}", version);
                        }
                    }
                }
            }
            _ => {} // Other categories already handled in the array
        }
    }
}

/// Helper function for legacy Docker analysis display
fn display_docker_analysis_detailed_legacy(docker_analysis: &DockerAnalysis) {
    println!("\n   🐳 Docker Infrastructure Analysis:");
    
    // Dockerfiles
    if !docker_analysis.dockerfiles.is_empty() {
        println!("      📄 Dockerfiles ({}):", docker_analysis.dockerfiles.len());
        for dockerfile in &docker_analysis.dockerfiles {
            println!("         • {}", dockerfile.path.display());
            if let Some(env) = &dockerfile.environment {
                println!("           Environment: {}", env);
            }
            if let Some(base_image) = &dockerfile.base_image {
                println!("           Base image: {}", base_image);
            }
            if !dockerfile.exposed_ports.is_empty() {
                println!("           Exposed ports: {}", 
                    dockerfile.exposed_ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", "));
            }
            if dockerfile.is_multistage {
                println!("           Multi-stage build: {} stages", dockerfile.build_stages.len());
            }
            println!("           Instructions: {}", dockerfile.instruction_count);
        }
    }
    
    // Compose files
    if !docker_analysis.compose_files.is_empty() {
        println!("      📋 Compose Files ({}):", docker_analysis.compose_files.len());
        for compose_file in &docker_analysis.compose_files {
            println!("         • {}", compose_file.path.display());
            if let Some(env) = &compose_file.environment {
                println!("           Environment: {}", env);
            }
            if let Some(version) = &compose_file.version {
                println!("           Version: {}", version);
            }
            if !compose_file.service_names.is_empty() {
                println!("           Services: {}", compose_file.service_names.join(", "));
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
    println!("      🏗️  Orchestration Pattern: {:?}", docker_analysis.orchestration_pattern);
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

/// Display architecture description
fn display_architecture_description(pattern: &ArchitecturePattern) {
    match pattern {
        ArchitecturePattern::Monolithic => {
            println!("   📦 This is a single, self-contained application");
        }
        ArchitecturePattern::Fullstack => {
            println!("   🌐 This is a full-stack application with separate frontend and backend");
        }
        ArchitecturePattern::Microservices => {
            println!("   🔗 This is a microservices architecture with multiple independent services");
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

/// Display summary view only
pub fn display_summary_view(analysis: &MonorepoAnalysis) {
    println!("\n{} {}", "▶".bright_blue(), "PROJECT ANALYSIS SUMMARY".bright_white().bold());
    println!("{}", "─".repeat(50).dimmed());
    
    println!("{} Architecture: {}", "│".dimmed(), 
        if analysis.is_monorepo {
            format!("Monorepo ({} projects)", analysis.projects.len()).yellow()
        } else {
            "Single Project".to_string().yellow()
        }
    );
    
    println!("{} Pattern: {}", "│".dimmed(), format!("{:?}", analysis.technology_summary.architecture_pattern).green());
    println!("{} Stack: {}", "│".dimmed(), analysis.technology_summary.languages.join(", ").blue());
    
    if !analysis.technology_summary.frameworks.is_empty() {
        println!("{} Frameworks: {}", "│".dimmed(), analysis.technology_summary.frameworks.join(", ").magenta());
    }
    
    println!("{} Analysis Time: {}ms", "│".dimmed(), analysis.metadata.analysis_duration_ms);
    println!("{} Confidence: {:.0}%", "│".dimmed(), analysis.metadata.confidence_score * 100.0);
    
    println!("{}", "─".repeat(50).dimmed());
}

/// Display JSON output
pub fn display_json_view(analysis: &MonorepoAnalysis) {
    match serde_json::to_string_pretty(analysis) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing to JSON: {}", e),
    }
}

/// Get emoji for project category
fn get_category_emoji(category: &ProjectCategory) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_display_modes() {
        // Test that display modes are properly defined
        assert_eq!(DisplayMode::Matrix, DisplayMode::Matrix);
        assert_ne!(DisplayMode::Matrix, DisplayMode::Detailed);
    }
} 