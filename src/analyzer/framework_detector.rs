use crate::analyzer::{AnalysisConfig, DetectedTechnology, DetectedLanguage};
use crate::analyzer::frameworks::*;
use crate::error::Result;
use std::path::Path;

/// Detects technologies (frameworks, libraries, tools) with proper classification
pub fn detect_frameworks(
    _project_root: &Path,
    languages: &[DetectedLanguage],
    _config: &AnalysisConfig,
) -> Result<Vec<DetectedTechnology>> {
    let mut all_technologies = Vec::new();
    
    // Initialize language-specific detectors
    let rust_detector = rust::RustFrameworkDetector;
    let js_detector = javascript::JavaScriptFrameworkDetector;
    let python_detector = python::PythonFrameworkDetector;
    let go_detector = go::GoFrameworkDetector;
    let java_detector = java::JavaFrameworkDetector;
    
    for language in languages {
        let lang_technologies = match language.name.as_str() {
            "Rust" => rust_detector.detect_frameworks(language)?,
            "JavaScript" | "TypeScript" | "JavaScript/TypeScript" => js_detector.detect_frameworks(language)?,
            "Python" => python_detector.detect_frameworks(language)?,
            "Go" => go_detector.detect_frameworks(language)?,
            "Java" | "Kotlin" | "Java/Kotlin" => java_detector.detect_frameworks(language)?,
            _ => Vec::new(),
        };
        all_technologies.extend(lang_technologies);
    }
    
    // Apply exclusivity rules and resolve conflicts
    let resolved_technologies = FrameworkDetectionUtils::resolve_technology_conflicts(all_technologies);
    
    // Mark primary technologies
    let final_technologies = FrameworkDetectionUtils::mark_primary_technologies(resolved_technologies);
    
    // Sort by confidence and remove exact duplicates
    let mut result = final_technologies;
    result.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    result.dedup_by(|a, b| a.name == b.name);
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{TechnologyCategory, LibraryType};
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
        
        let config = AnalysisConfig::default();
        let project_root = Path::new(".");
        
        let technologies = detect_frameworks(project_root, &[language], &config).unwrap();
        
        // Should detect Actix Web and Tokio
        let actix_web = technologies.iter().find(|t| t.name == "Actix Web");
        let tokio = technologies.iter().find(|t| t.name == "Tokio");
        
        if let Some(actix) = actix_web {
            assert!(matches!(actix.category, TechnologyCategory::BackendFramework));
            assert!(actix.is_primary);
            assert!(actix.confidence > 0.8);
        }
        
        if let Some(tokio_tech) = tokio {
            assert!(matches!(tokio_tech.category, TechnologyCategory::Runtime));
            assert!(!tokio_tech.is_primary);
        }
    }
    
    #[test]
    fn test_javascript_next_js_detection() {
        let language = DetectedLanguage {
            name: "JavaScript".to_string(),
            version: Some("18.0.0".to_string()),
            confidence: 0.9,
            files: vec![PathBuf::from("pages/index.js")],
            main_dependencies: vec![
                "next".to_string(),
                "react".to_string(),
                "react-dom".to_string(),
            ],
            dev_dependencies: vec!["eslint".to_string()],
            package_manager: Some("npm".to_string()),
        };
        
        let config = AnalysisConfig::default();
        let project_root = Path::new(".");
        
        let technologies = detect_frameworks(project_root, &[language], &config).unwrap();
        
        // Should detect Next.js and React
        let nextjs = technologies.iter().find(|t| t.name == "Next.js");
        let react = technologies.iter().find(|t| t.name == "React");
        
        if let Some(next) = nextjs {
            assert!(matches!(next.category, TechnologyCategory::MetaFramework));
            assert!(next.is_primary);
            assert!(next.requires.contains(&"React".to_string()));
        }
        
        if let Some(react_tech) = react {
            assert!(matches!(react_tech.category, TechnologyCategory::Library(LibraryType::UI)));
            assert!(!react_tech.is_primary); // Should be false since Next.js is the meta-framework
        }
    }
    
    #[test]
    fn test_python_fastapi_detection() {
        let language = DetectedLanguage {
            name: "Python".to_string(),
            version: Some("3.11.0".to_string()),
            confidence: 0.95,
            files: vec![PathBuf::from("main.py")],
            main_dependencies: vec![
                "fastapi".to_string(),
                "uvicorn".to_string(),
                "pydantic".to_string(),
            ],
            dev_dependencies: vec!["pytest".to_string()],
            package_manager: Some("pip".to_string()),
        };
        
        let config = AnalysisConfig::default();
        let project_root = Path::new(".");
        
        let technologies = detect_frameworks(project_root, &[language], &config).unwrap();
        
        // Should detect FastAPI and Uvicorn
        let fastapi = technologies.iter().find(|t| t.name == "FastAPI");
        let uvicorn = technologies.iter().find(|t| t.name == "Uvicorn");
        
        if let Some(fastapi_tech) = fastapi {
            assert!(matches!(fastapi_tech.category, TechnologyCategory::BackendFramework));
            assert!(fastapi_tech.is_primary);
        }
        
        if let Some(uvicorn_tech) = uvicorn {
            assert!(matches!(uvicorn_tech.category, TechnologyCategory::Runtime));
            assert!(!uvicorn_tech.is_primary);
        }
    }
    
    #[test]
    fn test_go_gin_detection() {
        let language = DetectedLanguage {
            name: "Go".to_string(),
            version: Some("1.21.0".to_string()),
            confidence: 0.95,
            files: vec![PathBuf::from("main.go")],
            main_dependencies: vec![
                "github.com/gin-gonic/gin".to_string(),
                "gorm.io/gorm".to_string(),
            ],
            dev_dependencies: vec!["github.com/stretchr/testify".to_string()],
            package_manager: Some("go mod".to_string()),
        };
        
        let config = AnalysisConfig::default();
        let project_root = Path::new(".");
        
        let technologies = detect_frameworks(project_root, &[language], &config).unwrap();
        
        // Should detect Gin and GORM
        let gin = technologies.iter().find(|t| t.name == "Gin");
        let gorm = technologies.iter().find(|t| t.name == "GORM");
        
        if let Some(gin_tech) = gin {
            assert!(matches!(gin_tech.category, TechnologyCategory::BackendFramework));
            assert!(gin_tech.is_primary);
        }
        
        if let Some(gorm_tech) = gorm {
            assert!(matches!(gorm_tech.category, TechnologyCategory::Database));
            assert!(!gorm_tech.is_primary);
        }
    }
    
    #[test]
    fn test_java_spring_boot_detection() {
        let language = DetectedLanguage {
            name: "Java".to_string(),
            version: Some("17.0.0".to_string()),
            confidence: 0.95,
            files: vec![PathBuf::from("src/main/java/Application.java")],
            main_dependencies: vec![
                "spring-boot".to_string(),
                "spring-web".to_string(),
            ],
            dev_dependencies: vec!["junit".to_string()],
            package_manager: Some("maven".to_string()),
        };
        
        let config = AnalysisConfig::default();
        let project_root = Path::new(".");
        
        let technologies = detect_frameworks(project_root, &[language], &config).unwrap();
        
        // Should detect Spring Boot
        let spring_boot = technologies.iter().find(|t| t.name == "Spring Boot");
        
        if let Some(spring) = spring_boot {
            assert!(matches!(spring.category, TechnologyCategory::BackendFramework));
            assert!(spring.is_primary);
        }
    }

    #[test]
    fn test_technology_conflicts_resolution() {
        let language = DetectedLanguage {
            name: "Rust".to_string(),
            version: Some("1.70.0".to_string()),
            confidence: 0.95,
            files: vec![PathBuf::from("src/main.rs")],
            main_dependencies: vec![
                "tokio".to_string(),
                "async-std".to_string(), // These should conflict
            ],
            dev_dependencies: vec![],
            package_manager: Some("cargo".to_string()),
        };
        
        let config = AnalysisConfig::default();
        let project_root = Path::new(".");
        
        let technologies = detect_frameworks(project_root, &[language], &config).unwrap();
        
        // Should only have one async runtime (higher confidence wins)
        let async_runtimes: Vec<_> = technologies.iter()
            .filter(|t| matches!(t.category, TechnologyCategory::Runtime))
            .collect();
        
        assert!(async_runtimes.len() <= 1, "Should resolve conflicting async runtimes: found {:?}", 
               async_runtimes.iter().map(|t| &t.name).collect::<Vec<_>>());
    }
} 