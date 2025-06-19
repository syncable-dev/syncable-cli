use crate::analyzer::ProjectCategory;

pub fn handle_support(
    languages: bool,
    frameworks: bool,
    _detailed: bool,
) -> crate::Result<()> {
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

/// Format project category for display
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