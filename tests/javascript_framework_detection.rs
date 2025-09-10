use syncable_cli::analyzer::{
    framework_detector::detect_frameworks,
    AnalysisConfig, DetectedLanguage, TechnologyCategory
};
use std::path::Path;

#[test]
fn test_javascript_framework_detection_with_file_indicators() {
    // Test Next.js detection through config file
    let language = DetectedLanguage {
        name: "JavaScript".to_string(),
        version: Some("18.0.0".to_string()),
        confidence: 0.9,
        files: vec![
            std::path::PathBuf::from("next.config.js"),
            std::path::PathBuf::from("pages/index.js"),
        ],
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

    // Should detect Next.js with high confidence due to config file
    let nextjs = technologies.iter().find(|t| t.name == "Next.js");
    
    assert!(nextjs.is_some(), "Next.js should be detected");
    let nextjs = nextjs.unwrap();
    
    assert!(matches!(nextjs.category, TechnologyCategory::MetaFramework));
    assert!(nextjs.is_primary);
    assert!(nextjs.confidence > 0.9); // High confidence from config file detection
    
    // Should also detect React as a dependency
    let react = technologies.iter().find(|t| t.name == "React");
    assert!(react.is_some(), "React should be detected");
}

#[test]
fn test_expo_detection_with_config_file() {
    // Test Expo detection through config file
    let language = DetectedLanguage {
        name: "JavaScript".to_string(),
        version: Some("18.0.0".to_string()),
        confidence: 0.9,
        files: vec![
            std::path::PathBuf::from("app.json"),
            std::path::PathBuf::from("App.js"),
        ],
        main_dependencies: vec![
            "expo".to_string(),
            "react".to_string(),
            "react-native".to_string(),
        ],
        dev_dependencies: vec![],
        package_manager: Some("npm".to_string()),
    };

    let config = AnalysisConfig::default();
    let project_root = Path::new(".");

    let technologies = detect_frameworks(project_root, &[language], &config).unwrap();

    // Should detect Expo with high confidence due to config file
    let expo = technologies.iter().find(|t| t.name == "Expo");
    
    assert!(expo.is_some(), "Expo should be detected");
    let expo = expo.unwrap();
    
    assert!(matches!(expo.category, TechnologyCategory::MetaFramework));
    assert!(expo.is_primary);
    assert!(expo.confidence > 0.9); // High confidence from config file detection
}

#[test]
fn test_tanstack_start_detection_with_config_file() {
    // Test TanStack Start detection through config file with specific content
    let language = DetectedLanguage {
        name: "JavaScript".to_string(),
        version: Some("18.0.0".to_string()),
        confidence: 0.9,
        files: vec![
            std::path::PathBuf::from("app.config.ts"),
        ],
        main_dependencies: vec![
            "@tanstack/react-start".to_string(),
            "react".to_string(),
            "vinxi".to_string(),
        ],
        dev_dependencies: vec![],
        package_manager: Some("npm".to_string()),
    };

    let config = AnalysisConfig::default();
    let project_root = Path::new(".");

    let technologies = detect_frameworks(project_root, &[language], &config).unwrap();

    // Should detect TanStack Start with high confidence
    let tanstack = technologies.iter().find(|t| t.name == "Tanstack Start");
    
    assert!(tanstack.is_some(), "Tanstack Start should be detected");
    let tanstack = tanstack.unwrap();
    
    assert!(matches!(tanstack.category, TechnologyCategory::MetaFramework));
    assert!(tanstack.is_primary);
    assert!(tanstack.confidence > 0.9); // High confidence from dependency + config file
}

#[test]
fn test_react_native_detection_with_config_file() {
    // Test React Native detection through config file
    let language = DetectedLanguage {
        name: "JavaScript".to_string(),
        version: Some("18.0.0".to_string()),
        confidence: 0.9,
        files: vec![
            std::path::PathBuf::from("react-native.config.js"),
        ],
        main_dependencies: vec![
            "react-native".to_string(),
            "react".to_string(),
        ],
        dev_dependencies: vec![],
        package_manager: Some("npm".to_string()),
    };

    let config = AnalysisConfig::default();
    let project_root = Path::new(".");

    let technologies = detect_frameworks(project_root, &[language], &config).unwrap();

    // Should detect React Native with high confidence due to config file
    let react_native = technologies.iter().find(|t| t.name == "React Native");
    
    assert!(react_native.is_some(), "React Native should be detected");
    let react_native = react_native.unwrap();
    
    assert!(matches!(react_native.category, TechnologyCategory::FrontendFramework));
    assert!(react_native.is_primary);
    assert!(react_native.confidence > 0.9); // High confidence from config file detection
}