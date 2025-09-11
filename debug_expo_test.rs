use syncable_cli::analyzer::{
    framework_detector::detect_frameworks,
    AnalysisConfig, DetectedLanguage, TechnologyCategory
};
use std::path::Path;

fn main() {
    // Test Expo React Native detection that should NOT detect Next.js
    let language = DetectedLanguage {
        name: "TypeScript".to_string(),
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
    
    println!("Detected technologies:");
    for tech in &technologies {
        println!("  - {} (confidence: {}, primary: {})", tech.name, tech.confidence, tech.is_primary);
    }
    
    // Should detect Expo as primary, not Next.js
    let expo = technologies.iter().find(|t| t.name == "Expo");
    let nextjs = technologies.iter().find(|t| t.name == "Next.js");
    
    println!("Expo detected: {:?}", expo.is_some());
    println!("Next.js detected: {:?}", nextjs.is_some());
    
    if let Some(expo_tech) = expo {
        println!("Expo is primary: {}", expo_tech.is_primary);
    }
}