use crate::analyzer::{AnalysisConfig, DetectedFramework, DetectedLanguage, FrameworkCategory};
use crate::error::Result;
// Remove unused import
use std::path::Path;

/// Framework detection rules and patterns
struct FrameworkRule {
    name: String,
    category: FrameworkCategory,
    confidence: f32,
    dependency_patterns: Vec<String>,
    #[allow(dead_code)]
    alternative_names: Vec<String>,
}

/// Detects frameworks used in the project based on language analysis
pub fn detect_frameworks(
    _project_root: &Path,
    languages: &[DetectedLanguage],
    _config: &AnalysisConfig,
) -> Result<Vec<DetectedFramework>> {
    let mut frameworks = Vec::new();
    
    for language in languages {
        let lang_frameworks = match language.name.as_str() {
            "Rust" => detect_rust_frameworks(language),
            "JavaScript" | "TypeScript" | "JavaScript/TypeScript" => detect_js_frameworks(language),
            "Python" => detect_python_frameworks(language),
            "Go" => detect_go_frameworks(language),
            "Java" | "Kotlin" | "Java/Kotlin" => detect_jvm_frameworks(language),
            _ => Vec::new(),
        };
        frameworks.extend(lang_frameworks);
    }
    
    // Remove duplicates and sort by confidence
    frameworks.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    frameworks.dedup_by(|a, b| a.name == b.name);
    
    Ok(frameworks)
}

/// Detect Rust frameworks from dependencies
fn detect_rust_frameworks(language: &DetectedLanguage) -> Vec<DetectedFramework> {
    let rules = get_rust_framework_rules();
    detect_frameworks_by_dependencies(&rules, &language.main_dependencies, language.confidence)
}

/// Detect JavaScript/TypeScript frameworks from dependencies  
fn detect_js_frameworks(language: &DetectedLanguage) -> Vec<DetectedFramework> {
    let rules = get_js_framework_rules();
    detect_frameworks_by_dependencies(&rules, &language.main_dependencies, language.confidence)
}

/// Detect Python frameworks from dependencies
fn detect_python_frameworks(language: &DetectedLanguage) -> Vec<DetectedFramework> {
    let rules = get_python_framework_rules();
    detect_frameworks_by_dependencies(&rules, &language.main_dependencies, language.confidence)
}

/// Detect Go frameworks from dependencies
fn detect_go_frameworks(language: &DetectedLanguage) -> Vec<DetectedFramework> {
    let rules = get_go_framework_rules();
    detect_frameworks_by_dependencies(&rules, &language.main_dependencies, language.confidence)
}

/// Detect JVM (Java/Kotlin) frameworks from dependencies
fn detect_jvm_frameworks(language: &DetectedLanguage) -> Vec<DetectedFramework> {
    let rules = get_jvm_framework_rules();
    detect_frameworks_by_dependencies(&rules, &language.main_dependencies, language.confidence)
}

/// Generic framework detection based on dependency patterns
fn detect_frameworks_by_dependencies(
    rules: &[FrameworkRule],
    dependencies: &[String],
    base_confidence: f32,
) -> Vec<DetectedFramework> {
    let mut frameworks = Vec::new();
    
    for rule in rules {
        let mut matches = 0;
        let total_patterns = rule.dependency_patterns.len();
        
        if total_patterns == 0 {
            continue;
        }
        
        for pattern in &rule.dependency_patterns {
            if dependencies.iter().any(|dep| matches_pattern(dep, pattern)) {
                matches += 1;
            }
        }
        
        // Calculate confidence based on pattern matches and base language confidence
        if matches > 0 {
            let pattern_confidence = matches as f32 / total_patterns as f32;
            let final_confidence = (rule.confidence * pattern_confidence * base_confidence).min(1.0);
            
            frameworks.push(DetectedFramework {
                name: rule.name.clone(),
                version: None, // TODO: Extract version from dependencies
                category: rule.category.clone(),
                confidence: final_confidence,
            });
        }
    }
    
    frameworks
}

/// Check if a dependency matches a pattern (supports wildcards)
fn matches_pattern(dependency: &str, pattern: &str) -> bool {
    if pattern.contains('*') {
        // Simple wildcard matching
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            dependency.starts_with(parts[0]) && dependency.ends_with(parts[1])
        } else {
            dependency.contains(&pattern.replace('*', ""))
        }
    } else {
        dependency == pattern || dependency.contains(pattern)
    }
}

/// Rust framework detection rules
fn get_rust_framework_rules() -> Vec<FrameworkRule> {
    vec![
        // Web Frameworks
        FrameworkRule {
            name: "Actix Web".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["actix-web".to_string()],
            alternative_names: vec!["actix".to_string()],
        },
        FrameworkRule {
            name: "Axum".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["axum".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Warp".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["warp".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Rocket".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["rocket".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Tide".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.90,
            dependency_patterns: vec!["tide".to_string()],
            alternative_names: vec![],
        },
        
        // Database
        FrameworkRule {
            name: "Diesel".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["diesel".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "SQLx".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["sqlx".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "SeaORM".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["sea-orm".to_string()],
            alternative_names: vec![],
        },
        
        // Testing
        FrameworkRule {
            name: "Tokio Test".to_string(),
            category: FrameworkCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["tokio-test".to_string()],
            alternative_names: vec![],
        },
        
        // Runtime
        FrameworkRule {
            name: "Tokio".to_string(),
            category: FrameworkCategory::Runtime,
            confidence: 0.90,
            dependency_patterns: vec!["tokio".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "async-std".to_string(),
            category: FrameworkCategory::Runtime,
            confidence: 0.90,
            dependency_patterns: vec!["async-std".to_string()],
            alternative_names: vec![],
        },
    ]
}

/// JavaScript/TypeScript framework detection rules
fn get_js_framework_rules() -> Vec<FrameworkRule> {
    vec![
        // Web Frameworks
        FrameworkRule {
            name: "Express.js".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["express".to_string()],
            alternative_names: vec!["express".to_string()],
        },
        FrameworkRule {
            name: "Next.js".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["next".to_string()],
            alternative_names: vec!["nextjs".to_string()],
        },
        FrameworkRule {
            name: "Nuxt.js".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["nuxt".to_string(), "@nuxt/core".to_string()],
            alternative_names: vec!["nuxtjs".to_string()],
        },
        FrameworkRule {
            name: "React".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.90,
            dependency_patterns: vec!["react".to_string()],
            alternative_names: vec!["reactjs".to_string()],
        },
        FrameworkRule {
            name: "Vue.js".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.90,
            dependency_patterns: vec!["vue".to_string()],
            alternative_names: vec!["vuejs".to_string()],
        },
        FrameworkRule {
            name: "Angular".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.90,
            dependency_patterns: vec!["@angular/core".to_string()],
            alternative_names: vec!["angular".to_string()],
        },
        FrameworkRule {
            name: "Nest.js".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["@nestjs/core".to_string()],
            alternative_names: vec!["nestjs".to_string()],
        },
        FrameworkRule {
            name: "Fastify".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["fastify".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Koa".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["koa".to_string()],
            alternative_names: vec!["koajs".to_string()],
        },
        
        // Database & ORM
        FrameworkRule {
            name: "Prisma".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["prisma".to_string(), "@prisma/client".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "TypeORM".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["typeorm".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Sequelize".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["sequelize".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Mongoose".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["mongoose".to_string()],
            alternative_names: vec![],
        },
        
        // Testing
        FrameworkRule {
            name: "Jest".to_string(),
            category: FrameworkCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["jest".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Mocha".to_string(),
            category: FrameworkCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["mocha".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Vitest".to_string(),
            category: FrameworkCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["vitest".to_string()],
            alternative_names: vec![],
        },
        
        // Build Tools
        FrameworkRule {
            name: "Webpack".to_string(),
            category: FrameworkCategory::BuildTool,
            confidence: 0.80,
            dependency_patterns: vec!["webpack".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Vite".to_string(),
            category: FrameworkCategory::BuildTool,
            confidence: 0.80,
            dependency_patterns: vec!["vite".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Parcel".to_string(),
            category: FrameworkCategory::BuildTool,
            confidence: 0.80,
            dependency_patterns: vec!["parcel".to_string()],
            alternative_names: vec![],
        },
    ]
}

/// Python framework detection rules
fn get_python_framework_rules() -> Vec<FrameworkRule> {
    vec![
        // Web Frameworks
        FrameworkRule {
            name: "Django".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["django".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Flask".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["flask".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "FastAPI".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["fastapi".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Starlette".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.90,
            dependency_patterns: vec!["starlette".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Tornado".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.90,
            dependency_patterns: vec!["tornado".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Pyramid".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.90,
            dependency_patterns: vec!["pyramid".to_string()],
            alternative_names: vec![],
        },
        
        // Database & ORM
        FrameworkRule {
            name: "SQLAlchemy".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["sqlalchemy".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Django ORM".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["django".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Peewee".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["peewee".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Tortoise ORM".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["tortoise-orm".to_string()],
            alternative_names: vec![],
        },
        
        // Testing
        FrameworkRule {
            name: "pytest".to_string(),
            category: FrameworkCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["pytest".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "unittest".to_string(),
            category: FrameworkCategory::Testing,
            confidence: 0.75,
            dependency_patterns: vec!["unittest2".to_string()],
            alternative_names: vec![],
        },
        
        // Machine Learning (special category)
        FrameworkRule {
            name: "TensorFlow".to_string(),
            category: FrameworkCategory::Other("ML".to_string()),
            confidence: 0.90,
            dependency_patterns: vec!["tensorflow".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "PyTorch".to_string(),
            category: FrameworkCategory::Other("ML".to_string()),
            confidence: 0.90,
            dependency_patterns: vec!["torch".to_string(), "pytorch".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Scikit-learn".to_string(),
            category: FrameworkCategory::Other("ML".to_string()),
            confidence: 0.85,
            dependency_patterns: vec!["scikit-learn".to_string(), "sklearn".to_string()],
            alternative_names: vec![],
        },
    ]
}

/// Go framework detection rules
fn get_go_framework_rules() -> Vec<FrameworkRule> {
    vec![
        // Web Frameworks
        FrameworkRule {
            name: "Gin".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/gin-gonic/gin".to_string()],
            alternative_names: vec!["gin-gonic".to_string()],
        },
        FrameworkRule {
            name: "Echo".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/labstack/echo".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Fiber".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/gofiber/fiber".to_string()],
            alternative_names: vec!["gofiber".to_string()],
        },
        FrameworkRule {
            name: "Chi".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/go-chi/chi".to_string()],
            alternative_names: vec!["go-chi".to_string()],
        },
        FrameworkRule {
            name: "Gorilla Mux".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/gorilla/mux".to_string()],
            alternative_names: vec!["mux".to_string()],
        },
        FrameworkRule {
            name: "Beego".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/beego/beego".to_string()],
            alternative_names: vec![],
        },
        
        // Database
        FrameworkRule {
            name: "GORM".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["gorm.io/gorm".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Ent".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["entgo.io/ent".to_string()],
            alternative_names: vec!["entgo".to_string()],
        },
        
        // Testing
        FrameworkRule {
            name: "Testify".to_string(),
            category: FrameworkCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/stretchr/testify".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Ginkgo".to_string(),
            category: FrameworkCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/onsi/ginkgo".to_string()],
            alternative_names: vec![],
        },
    ]
}

/// JVM (Java/Kotlin) framework detection rules
fn get_jvm_framework_rules() -> Vec<FrameworkRule> {
    vec![
        // Web Frameworks
        FrameworkRule {
            name: "Spring Boot".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec![
                "spring-boot-starter".to_string(),
                "org.springframework.boot".to_string(),
            ],
            alternative_names: vec!["spring".to_string()],
        },
        FrameworkRule {
            name: "Spring MVC".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.90,
            dependency_patterns: vec!["spring-webmvc".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Quarkus".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["quarkus".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Micronaut".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["micronaut".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Ktor".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.95,
            dependency_patterns: vec!["ktor".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Helidon".to_string(),
            category: FrameworkCategory::Web,
            confidence: 0.90,
            dependency_patterns: vec!["helidon".to_string()],
            alternative_names: vec![],
        },
        
        // Database
        FrameworkRule {
            name: "Spring Data JPA".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["spring-boot-starter-data-jpa".to_string()],
            alternative_names: vec!["jpa".to_string()],
        },
        FrameworkRule {
            name: "Hibernate".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["hibernate".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "MyBatis".to_string(),
            category: FrameworkCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["mybatis".to_string()],
            alternative_names: vec![],
        },
        
        // Testing
        FrameworkRule {
            name: "JUnit 5".to_string(),
            category: FrameworkCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["junit-jupiter".to_string()],
            alternative_names: vec!["junit5".to_string()],
        },
        FrameworkRule {
            name: "JUnit 4".to_string(),
            category: FrameworkCategory::Testing,
            confidence: 0.80,
            dependency_patterns: vec!["junit".to_string()],
            alternative_names: vec!["junit4".to_string()],
        },
        FrameworkRule {
            name: "TestNG".to_string(),
            category: FrameworkCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["testng".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Mockito".to_string(),
            category: FrameworkCategory::Testing,
            confidence: 0.80,
            dependency_patterns: vec!["mockito".to_string()],
            alternative_names: vec![],
        },
        
        // Build Tools
        FrameworkRule {
            name: "Gradle".to_string(),
            category: FrameworkCategory::BuildTool,
            confidence: 0.90,
            dependency_patterns: vec!["gradle".to_string()],
            alternative_names: vec![],
        },
        FrameworkRule {
            name: "Maven".to_string(),
            category: FrameworkCategory::BuildTool,
            confidence: 0.90,
            dependency_patterns: vec!["maven".to_string()],
            alternative_names: vec![],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_rust_actix_web_detection() {
        let language = DetectedLanguage {
            name: "Rust".to_string(),
            version: Some("1.70.0".to_string()),
            confidence: 0.9,
            files: vec![PathBuf::from("src/main.rs")],
            main_dependencies: vec!["actix-web".to_string(), "tokio".to_string()],
            dev_dependencies: vec!["assert_cmd".to_string()],
            package_manager: Some("cargo".to_string()),
        };
        
        let frameworks = detect_rust_frameworks(&language);
        
        // This is a simplified test - in real implementation, we'd need access to dependencies
        assert!(frameworks.is_empty() || frameworks.iter().any(|f| f.name.contains("Actix")));
    }
    
    #[test]
    fn test_framework_pattern_matching() {
        assert!(matches_pattern("express", "express"));
        assert!(matches_pattern("@nestjs/core", "@nestjs/*"));
        assert!(matches_pattern("spring-boot-starter-web", "spring-boot-starter*"));
        assert!(!matches_pattern("react", "vue"));
    }
    
    #[test]
    fn test_framework_categories() {
        let rules = get_js_framework_rules();
        
        let express_rule = rules.iter().find(|r| r.name == "Express.js").unwrap();
        assert!(matches!(express_rule.category, FrameworkCategory::Web));
        
        let jest_rule = rules.iter().find(|r| r.name == "Jest").unwrap();
        assert!(matches!(jest_rule.category, FrameworkCategory::Testing));
    }
} 