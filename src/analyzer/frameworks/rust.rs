use super::{LanguageFrameworkDetector, TechnologyRule, FrameworkDetectionUtils};
use crate::analyzer::{DetectedTechnology, DetectedLanguage, TechnologyCategory, LibraryType};
use crate::error::Result;

pub struct RustFrameworkDetector;

impl LanguageFrameworkDetector for RustFrameworkDetector {
    fn detect_frameworks(&self, language: &DetectedLanguage) -> Result<Vec<DetectedTechnology>> {
        let rules = get_rust_technology_rules();
        
        // Combine main and dev dependencies for comprehensive detection
        let all_deps: Vec<String> = language.main_dependencies.iter()
            .chain(language.dev_dependencies.iter())
            .cloned()
            .collect();
        
        let technologies = FrameworkDetectionUtils::detect_technologies_by_dependencies(
            &rules, &all_deps, language.confidence
        );
        
        Ok(technologies)
    }
    
    fn supported_languages(&self) -> Vec<&'static str> {
        vec!["Rust"]
    }
}

/// Rust technology detection rules with comprehensive framework coverage
fn get_rust_technology_rules() -> Vec<TechnologyRule> {
    vec![
        // WEB FRAMEWORKS
        TechnologyRule {
            name: "Actix Web".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["actix-web".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["actix".to_string()],
        },
        TechnologyRule {
            name: "Axum".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["axum".to_string()],
            requires: vec!["Tokio".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Rocket".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["rocket".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Warp".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["warp".to_string()],
            requires: vec!["Tokio".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Loco".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["loco-rs".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["loco".to_string()],
        },
        TechnologyRule {
            name: "Poem".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["poem".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Salvo".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["salvo".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Trillium".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["trillium".to_string()],
            requires: vec!["Tokio".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        
        // ASYNC RUNTIMES
        TechnologyRule {
            name: "Tokio".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.90,
            dependency_patterns: vec!["tokio".to_string()],
            requires: vec![],
            conflicts_with: vec!["async-std".to_string()],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "async-std".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.90,
            dependency_patterns: vec!["async-std".to_string()],
            requires: vec![],
            conflicts_with: vec!["Tokio".to_string()],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // DATABASE/ORM
        TechnologyRule {
            name: "SeaORM".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["sea-orm".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["sea_orm".to_string()],
        },
        TechnologyRule {
            name: "Diesel".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["diesel".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "SQLx".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["sqlx".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // SERIALIZATION
        TechnologyRule {
            name: "Serde".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.85,
            dependency_patterns: vec!["serde".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // TESTING
        TechnologyRule {
            name: "Criterion".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["criterion".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // GUI FRAMEWORKS (WASM/Desktop)
        TechnologyRule {
            name: "Leptos".to_string(),
            category: TechnologyCategory::FrontendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["leptos".to_string()],
            requires: vec![],
            conflicts_with: vec!["Yew".to_string(), "Dioxus".to_string()],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Yew".to_string(),
            category: TechnologyCategory::FrontendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["yew".to_string()],
            requires: vec![],
            conflicts_with: vec!["Leptos".to_string(), "Dioxus".to_string()],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Dioxus".to_string(),
            category: TechnologyCategory::FrontendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["dioxus".to_string()],
            requires: vec![],
            conflicts_with: vec!["Leptos".to_string(), "Yew".to_string()],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Tauri".to_string(),
            category: TechnologyCategory::Library(LibraryType::UI),
            confidence: 0.95,
            dependency_patterns: vec!["tauri".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "egui".to_string(),
            category: TechnologyCategory::Library(LibraryType::UI),
            confidence: 0.95,
            dependency_patterns: vec!["egui".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
    ]
} 