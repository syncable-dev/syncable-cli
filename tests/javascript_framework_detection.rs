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

#[test]
fn test_expo_react_native_detection_should_not_detect_nextjs() {
    // Test Expo React Native detection that should NOT detect Next.js
    let language = DetectedLanguage {
        name: "JavaScript".to_string(),
        version: Some("4.0.0".to_string()),
        confidence: 0.95,
        files: vec![
            std::path::PathBuf::from("app.json"),
            std::path::PathBuf::from("App.tsx"),
            std::path::PathBuf::from("android/build.gradle"),
            std::path::PathBuf::from("ios/Podfile"),
        ],
        main_dependencies: vec![
            "expo".to_string(),
            "react-native".to_string(),
            "react".to_string(),
            "next".to_string(), // This dependency should not cause Next.js to be detected
        ],
        dev_dependencies: vec![],
        package_manager: Some("npm".to_string()),
    };
    
    let config = AnalysisConfig::default();
    let project_root = Path::new(".");
    
    let technologies = detect_frameworks(project_root, &[language], &config).unwrap();
    
    // Should detect Expo as primary, not Next.js
    let expo = technologies.iter().find(|t| t.name == "Expo");
    let nextjs = technologies.iter().find(|t| t.name == "Next.js");
    
    assert!(expo.is_some(), "Should detect Expo");
    assert!(expo.unwrap().is_primary, "Expo should be primary");
    assert!(nextjs.is_none(), "Should not detect Next.js in Expo project");
}

#[test]
fn test_encore_backend_detection() {
    // Test Encore backend detection
    let language = DetectedLanguage {
        name: "TypeScript".to_string(),
        version: Some("4.0.0".to_string()),
        confidence: 0.95,
        files: vec![
            std::path::PathBuf::from("main.go"),
            std::path::PathBuf::from("service/user.go"),
            std::path::PathBuf::from("encore.app"),
        ],
        main_dependencies: vec![
            "encore.dev".to_string(),
        ],
        dev_dependencies: vec![],
        package_manager: Some("go mod".to_string()),
    };
    
    let config = AnalysisConfig::default();
    let project_root = Path::new(".");
    
    let technologies = detect_frameworks(project_root, &[language], &config).unwrap();
    
    // Should detect Encore as primary
    let encore = technologies.iter().find(|t| t.name == "Encore");
    
    assert!(encore.is_some(), "Should detect Encore");
    assert!(encore.unwrap().is_primary, "Encore should be primary");
}

#[test]
fn test_encore_detection_should_not_detect_nextjs() {
    // Test Encore detection that should NOT detect Next.js
    let language = DetectedLanguage {
        name: "JavaScript".to_string(),
        version: Some("4.0.0".to_string()),
        confidence: 0.95,
        files: vec![
            std::path::PathBuf::from("encore.app"),
            std::path::PathBuf::from("service/api.encore.service.ts"),
        ],
        main_dependencies: vec![
            "encore.dev".to_string(),
            "next".to_string(), // This dependency should not cause Next.js to be detected
        ],
        dev_dependencies: vec![],
        package_manager: Some("npm".to_string()),
    };
    
    let config = AnalysisConfig::default();
    let project_root = Path::new(".");
    
    let technologies = detect_frameworks(project_root, &[language], &config).unwrap();
    
    // Should detect Encore as primary, not Next.js
    let encore = technologies.iter().find(|t| t.name == "Encore");
    let nextjs = technologies.iter().find(|t| t.name == "Next.js");
    
    assert!(encore.is_some(), "Should detect Encore");
    assert!(encore.unwrap().is_primary, "Encore should be primary");
    assert!(nextjs.is_none(), "Should not detect Next.js in Encore project");
}

#[test]
fn test_false_positive_expo_detection_in_pure_typescript_project() {
    // Test case for the false positive Expo detection in a pure TypeScript project
    // This reproduces the issue reported by the user
    let language = DetectedLanguage {
        name: "TypeScript".to_string(),
        version: Some(">=20.0.0".to_string()),
        confidence: 0.92499995,
        files: vec![
            std::path::PathBuf::from("eslint.config.js"),
            std::path::PathBuf::from("src/tools/write-file.ts"),
            std::path::PathBuf::from("src/tools/read-file.ts"),
            std::path::PathBuf::from("src/tools/insert.ts"),
            std::path::PathBuf::from("src/tools/docker.ts"),
            std::path::PathBuf::from("src/tools/directoryContext.ts"),
            std::path::PathBuf::from("src/tools/ls.ts"),
            std::path::PathBuf::from("src/tools/grep.ts"),
            std::path::PathBuf::from("src/tools/edit.ts"),
            std::path::PathBuf::from("src/tools/index.ts"),
            std::path::PathBuf::from("src/tools/read-many-files.ts"),
            std::path::PathBuf::from("src/agents/repo-analysis-agent.ts"),
            std::path::PathBuf::from("src/agents/docker-agent.ts"),
            std::path::PathBuf::from("src/agents/infra-agent.ts"),
            std::path::PathBuf::from("src/toolkits/generateFileToolkit.ts"),
            std::path::PathBuf::from("src/toolkits/fullRepoToolkit.ts"),
            std::path::PathBuf::from("src/toolkits/dockerToolkit.ts"),
            std::path::PathBuf::from("src/toolkits/minimalRepoToolkit.ts"),
            std::path::PathBuf::from("src/supervisors/tech-lead-supervisor.ts"),
            std::path::PathBuf::from("src/supervisors/application-supervisor.ts"),
            std::path::PathBuf::from("src/supervisors/devops-supervisor.ts"),
            std::path::PathBuf::from("src/index.ts"),
        ],
        main_dependencies: vec![
            "@ai-sdk/anthropic".to_string(),
            "@types/glob".to_string(),
            "@voltagent/cli".to_string(),
            "@voltagent/core".to_string(),
            "@voltagent/langfuse-exporter".to_string(),
            "@voltagent/logger".to_string(),
            "@voltagent/vercel-ai".to_string(),
            "dockerode".to_string(),
            "dotenv".to_string(),
            "glob".to_string(),
            "tar-fs".to_string(),
            "zod".to_string(),
        ],
        dev_dependencies: vec![],
        package_manager: Some("npm".to_string()),
    };
    
    let config = AnalysisConfig::default();
    let project_root = Path::new(".");
    
    let technologies = detect_frameworks(project_root, &[language], &config).unwrap();
    
    // Print all detected technologies for debugging
    println!("Detected technologies:");
    for tech in &technologies {
        println!("  - {} (confidence: {:.2}, primary: {})", tech.name, tech.confidence, tech.is_primary);
    }
    
    // Should NOT detect Expo in this pure TypeScript project
    let expo = technologies.iter().find(|t| t.name == "Expo");
    
    if let Some(expo_tech) = expo {
        println!("ERROR: Expo incorrectly detected!");
        println!("  Confidence: {}", expo_tech.confidence);
        println!("  Is primary: {}", expo_tech.is_primary);
        panic!("Expo should NOT be detected in a pure TypeScript project without Expo dependencies");
    } else {
        println!("SUCCESS: Expo not detected (as expected)");
    }
}

#[test]
fn test_legitimate_expo_detection_still_works() {
    // Test that legitimate Expo detection still works after our fix
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

    // Should detect Expo with high confidence due to config file and proper dependencies
    let expo = technologies.iter().find(|t| t.name == "Expo");
    
    assert!(expo.is_some(), "Expo should be detected with proper dependencies");
    let expo = expo.unwrap();
    
    assert!(matches!(expo.category, TechnologyCategory::MetaFramework));
    assert!(expo.is_primary);
    assert!(expo.confidence > 0.9); // High confidence from config file and dependencies
    
    println!("SUCCESS: Expo correctly detected with legitimate dependencies");
    println!("  Confidence: {}", expo.confidence);
    println!("  Is primary: {}", expo.is_primary);
}
