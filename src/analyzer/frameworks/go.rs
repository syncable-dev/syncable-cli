use super::{LanguageFrameworkDetector, TechnologyRule, FrameworkDetectionUtils};
use crate::analyzer::{DetectedTechnology, DetectedLanguage, TechnologyCategory, LibraryType};
use crate::error::Result;

pub struct GoFrameworkDetector;

impl LanguageFrameworkDetector for GoFrameworkDetector {
    fn detect_frameworks(&self, language: &DetectedLanguage) -> Result<Vec<DetectedTechnology>> {
        let rules = get_go_technology_rules();
        
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
        vec!["Go"]
    }
}

/// Go technology detection rules with comprehensive framework coverage
fn get_go_technology_rules() -> Vec<TechnologyRule> {
    vec![
        // WEB FRAMEWORKS
        TechnologyRule {
            name: "Gin".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/gin-gonic/gin".to_string(), "gin-gonic".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["gin-gonic".to_string()],
        },
        TechnologyRule {
            name: "Echo".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/labstack/echo".to_string(), "labstack/echo".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["labstack/echo".to_string()],
        },
        TechnologyRule {
            name: "Fiber".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/gofiber/fiber".to_string(), "gofiber".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["gofiber".to_string()],
        },
        TechnologyRule {
            name: "Beego".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/beego/beego".to_string(), "beego".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Chi".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/go-chi/chi".to_string(), "go-chi".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["go-chi".to_string()],
        },
        TechnologyRule {
            name: "Gorilla Mux".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/gorilla/mux".to_string(), "gorilla/mux".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["mux".to_string(), "gorilla".to_string()],
        },
        TechnologyRule {
            name: "Revel".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/revel/revel".to_string(), "revel".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Buffalo".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/gobuffalo/buffalo".to_string(), "gobuffalo".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["gobuffalo".to_string()],
        },
        TechnologyRule {
            name: "Iris".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/kataras/iris".to_string(), "kataras/iris".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "FastHTTP".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/valyala/fasthttp".to_string(), "fasthttp".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["valyala/fasthttp".to_string()],
        },
        TechnologyRule {
            name: "Hertz".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/cloudwego/hertz".to_string(), "cloudwego/hertz".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["cloudwego".to_string()],
        },
        
        // DATABASE/ORM
        TechnologyRule {
            name: "GORM".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["gorm.io/gorm".to_string(), "gorm".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Ent".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["entgo.io/ent".to_string(), "facebook/ent".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["entgo".to_string()],
        },
        TechnologyRule {
            name: "Xorm".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["xorm.io/xorm".to_string(), "xorm".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // MICROSERVICES
        TechnologyRule {
            name: "Go Kit".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["github.com/go-kit/kit".to_string(), "go-kit".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["kit".to_string()],
        },
        TechnologyRule {
            name: "Kratos".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/go-kratos/kratos".to_string(), "go-kratos".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["go-kratos".to_string()],
        },
        
        // MESSAGE QUEUES
        TechnologyRule {
            name: "Sarama".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.85,
            dependency_patterns: vec!["github.com/shopify/sarama".to_string(), "sarama".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["shopify/sarama".to_string()],
        },
        
        // TESTING
        TechnologyRule {
            name: "Testify".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/stretchr/testify".to_string(), "testify".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["stretchr/testify".to_string()],
        },
        TechnologyRule {
            name: "Ginkgo".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/onsi/ginkgo".to_string(), "ginkgo".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["onsi/ginkgo".to_string()],
        },
        
        // CLI FRAMEWORKS
        TechnologyRule {
            name: "Cobra".to_string(),
            category: TechnologyCategory::Library(LibraryType::CLI),
            confidence: 0.85,
            dependency_patterns: vec!["github.com/spf13/cobra".to_string(), "cobra".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["spf13/cobra".to_string()],
        },
        
        // CONFIG MANAGEMENT
        TechnologyRule {
            name: "Viper".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.80,
            dependency_patterns: vec!["github.com/spf13/viper".to_string(), "viper".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["spf13/viper".to_string()],
        },
    ]
} 