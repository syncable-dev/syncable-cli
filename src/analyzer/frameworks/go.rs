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
            file_indicators: vec![],
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
            file_indicators: vec![],
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
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Chi".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/go-chi/chi".to_string(), "go-chi/chi".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["chi".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Gorilla Mux".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/gorilla/mux".to_string(), "gorilla/mux".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["mux".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "HttpRouter".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/julienschmidt/httprouter".to_string(), "julienschmidt/httprouter".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["httprouter".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Beego".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/beego/beego".to_string(), "beego/beego".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["beego".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Revel".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/revel/revel".to_string(), "revel/revel".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["revel".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Buffalo".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/gobuffalo/buffalo".to_string(), "gobuffalo/buffalo".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["buffalo".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Gin Web Framework".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/gin-gonic/gin".to_string(), "gin-gonic".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["gin".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Go Kit".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["github.com/go-kit/kit".to_string(), "go-kit".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["kit".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Micro".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["github.com/micro/micro".to_string(), "micro/micro".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["micro".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Go Micro".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["github.com/micro/go-micro".to_string(), "micro/go-micro".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["go-micro".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Go Frame".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/gogf/gf".to_string(), "gogf/gf".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["gf".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Iris".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/kataras/iris".to_string(), "kataras/iris".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["iris".to_string()],
            file_indicators: vec![],
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
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Hertz".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/cloudwego/hertz".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["cloudwego/hertz".to_string()],
            file_indicators: vec![],
        },
        
        // Encore (Go) - Cloud development platform
        TechnologyRule {
            name: "Encore".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["encore.dev".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
            file_indicators: vec!["encore.app".to_string()],
        },
        
        // DATABASE/ORM
        TechnologyRule {
            name: "GORM".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            // Only match the specific gorm.io path, not just "gorm"
            dependency_patterns: vec!["gorm.io/gorm".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
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
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Xorm".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["xorm.io/xorm".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Bun".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/uptrace/bun".to_string(), "uptrace/bun".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "SQLBoiler".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/volatiletech/sqlboiler".to_string(), "volatiletech/sqlboiler".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Squirrel".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/Masterminds/squirrel".to_string(), "Masterminds/squirrel".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        
        // TESTING
        TechnologyRule {
            name: "Testify".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/stretchr/testify".to_string(), "stretchr/testify".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["stretchr/testify".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Ginkgo".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/onsi/ginkgo".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["onsi/ginkgo".to_string()],
            file_indicators: vec![],
        },
        
        // CLI FRAMEWORKS
        TechnologyRule {
            name: "Cobra".to_string(),
            category: TechnologyCategory::Library(LibraryType::CLI),
            confidence: 0.85,
            dependency_patterns: vec!["github.com/spf13/cobra".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["spf13/cobra".to_string()],
            file_indicators: vec![],
        },
        
        // CONFIG MANAGEMENT
        TechnologyRule {
            name: "Viper".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.80,
            dependency_patterns: vec!["github.com/spf13/viper".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["spf13/viper".to_string()],
            file_indicators: vec![],
        },
        
        // LOGGING
        TechnologyRule {
            name: "Logrus".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.85,
            dependency_patterns: vec!["github.com/sirupsen/logrus".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Zap".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.85,
            dependency_patterns: vec!["go.uber.org/zap".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        
        // HTTP CLIENTS
        TechnologyRule {
            name: "Resty".to_string(),
            category: TechnologyCategory::Library(LibraryType::HttpClient),
            confidence: 0.85,
            dependency_patterns: vec!["github.com/go-resty/resty".to_string(), "go-resty/resty".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["resty".to_string()],
            file_indicators: vec![],
        },
        
        // MESSAGING
        TechnologyRule {
            name: "NATS".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["github.com/nats-io/nats.go".to_string(), "nats-io/nats.go".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["nats".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Kafka".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["github.com/Shopify/sarama".to_string(), "Shopify/sarama".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["sarama".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "RabbitMQ".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["github.com/streadway/amqp".to_string(), "streadway/amqp".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["amqp".to_string()],
            file_indicators: vec![],
        },
    ]
}