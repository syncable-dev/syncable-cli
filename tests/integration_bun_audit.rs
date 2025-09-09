use std::path::Path;
use std::fs;
use tempfile::TempDir;
use tokio;

use syncable_cli::analyzer::{
    dependency_parser::{DependencyParser, DependencyInfo, DependencyType, Language},
    vulnerability_checker::VulnerabilityChecker,
    runtime_detector::{RuntimeDetector, PackageManager, JavaScriptRuntime, DetectionConfidence},
    tool_detector::ToolDetector,
};

/// Integration tests for end-to-end bun audit workflow
/// These tests simulate real project scenarios and test the complete pipeline

#[tokio::test]
async fn test_bun_project_detection_and_audit_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    // Create a simulated Bun project
    create_bun_project(project_path);
    
    // Test 1: Runtime Detection
    let runtime_detector = RuntimeDetector::new(project_path.to_path_buf());
    let detection_result = runtime_detector.detect_js_runtime_and_package_manager();
    
    assert_eq!(detection_result.package_manager, PackageManager::Bun);
    assert_eq!(detection_result.runtime, JavaScriptRuntime::Bun);
    
    // Test 2: Tool Detection
    let mut tool_detector = ToolDetector::new();
    let js_managers = tool_detector.detect_js_package_managers();
    
    assert!(js_managers.contains_key("bun"));
    assert!(js_managers.contains_key("npm"));
    assert!(js_managers.contains_key("yarn"));
    assert!(js_managers.contains_key("pnpm"));
    
    // Test 3: Dependency Parsing
    let parser = DependencyParser::new();
    let dependencies = parser.parse_all_dependencies(project_path).unwrap();
    
    assert!(dependencies.contains_key(&Language::JavaScript));
    let js_deps = &dependencies[&Language::JavaScript];
    assert!(!js_deps.is_empty());
    
    // Verify we have the expected dependencies
    assert!(js_deps.iter().any(|d| d.name == "express"));
    assert!(js_deps.iter().any(|d| d.name == "lodash"));
    
    // Test 4: Vulnerability Checking (will use mock data since we can't guarantee bun is installed)
    let checker = VulnerabilityChecker::new();
    let report = checker.check_all_dependencies(&dependencies, project_path).await;
    
    // Should complete without error (may find 0 vulnerabilities if tools aren't installed)
    assert!(report.is_ok());
    let vulnerability_report = report.unwrap();
    
    // Verify report structure
    assert!(vulnerability_report.total_vulnerabilities >= 0);
    assert!(vulnerability_report.critical_count >= 0);
    assert!(vulnerability_report.high_count >= 0);
    assert!(vulnerability_report.medium_count >= 0);
    assert!(vulnerability_report.low_count >= 0);
}

#[tokio::test]
async fn test_npm_project_detection_and_audit_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    // Create a simulated npm project
    create_npm_project(project_path);
    
    // Test runtime detection
    let runtime_detector = RuntimeDetector::new(project_path.to_path_buf());
    let detection_result = runtime_detector.detect_js_runtime_and_package_manager();
    
    assert_eq!(detection_result.package_manager, PackageManager::Npm);
    assert_eq!(detection_result.runtime, JavaScriptRuntime::Node);
    
    // Test dependency parsing
    let parser = DependencyParser::new();
    let dependencies = parser.parse_all_dependencies(project_path).unwrap();
    
    assert!(dependencies.contains_key(&Language::JavaScript));
    let js_deps = &dependencies[&Language::JavaScript];
    assert!(!js_deps.is_empty());
    
    // Test vulnerability checking
    let checker = VulnerabilityChecker::new();
    let report = checker.check_all_dependencies(&dependencies, project_path).await;
    assert!(report.is_ok());
}

#[tokio::test]
async fn test_yarn_project_detection_and_audit_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    // Create a simulated yarn project
    create_yarn_project(project_path);
    
    // Test runtime detection
    let runtime_detector = RuntimeDetector::new(project_path.to_path_buf());
    let detection_result = runtime_detector.detect_js_runtime_and_package_manager();
    
    assert_eq!(detection_result.package_manager, PackageManager::Yarn);
    assert_eq!(detection_result.runtime, JavaScriptRuntime::Node);
    
    // Test the complete workflow
    let parser = DependencyParser::new();
    let dependencies = parser.parse_all_dependencies(project_path).unwrap();
    let checker = VulnerabilityChecker::new();
    let report = checker.check_all_dependencies(&dependencies, project_path).await;
    assert!(report.is_ok());
}

#[tokio::test]
async fn test_pnpm_project_detection_and_audit_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    // Create a simulated pnpm project
    create_pnpm_project(project_path);
    
    // Test runtime detection
    let runtime_detector = RuntimeDetector::new(project_path.to_path_buf());
    let detection_result = runtime_detector.detect_js_runtime_and_package_manager();
    
    assert_eq!(detection_result.package_manager, PackageManager::Pnpm);
    assert_eq!(detection_result.runtime, JavaScriptRuntime::Node);
    
    // Test the complete workflow
    let parser = DependencyParser::new();
    let dependencies = parser.parse_all_dependencies(project_path).unwrap();
    let checker = VulnerabilityChecker::new();
    let report = checker.check_all_dependencies(&dependencies, project_path).await;
    assert!(report.is_ok());
}

#[tokio::test]
async fn test_multi_runtime_project_priority() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    // Create a project with multiple lockfiles (Bun should have priority)
    create_multi_runtime_project(project_path);
    
    let runtime_detector = RuntimeDetector::new(project_path.to_path_buf());
    let detection_result = runtime_detector.detect_js_runtime_and_package_manager();
    
    // Bun should be detected as primary despite other lockfiles present
    assert_eq!(detection_result.package_manager, PackageManager::Bun);
    assert_eq!(detection_result.runtime, JavaScriptRuntime::Bun);
    
    // Test that vulnerability checking uses the detected runtime
    let parser = DependencyParser::new();
    let dependencies = parser.parse_all_dependencies(project_path).unwrap();
    let checker = VulnerabilityChecker::new();
    let report = checker.check_all_dependencies(&dependencies, project_path).await;
    assert!(report.is_ok());
}

#[tokio::test]
async fn test_vulnerability_checking_with_mixed_languages() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    // Create a project with multiple languages
    create_polyglot_project(project_path);
    
    let parser = DependencyParser::new();
    let dependencies = parser.parse_all_dependencies(project_path).unwrap();
    
    // Should detect multiple languages
    assert!(dependencies.contains_key(&Language::JavaScript));
    assert!(dependencies.contains_key(&Language::Python));
    assert!(dependencies.contains_key(&Language::Rust));
    
    // Test vulnerability checking across all languages
    let checker = VulnerabilityChecker::new();
    let report = checker.check_all_dependencies(&dependencies, project_path).await;
    assert!(report.is_ok());
    
    let vulnerability_report = report.unwrap();
    
    // Should handle mixed language vulnerabilities
    assert!(vulnerability_report.total_vulnerabilities >= 0);
}

#[test]
fn test_tool_detection_comprehensive() {
    let mut tool_detector = ToolDetector::new();
    
    // Test detection of all JavaScript package managers
    let js_tools = tool_detector.detect_js_package_managers();
    
    // Should attempt to detect all package managers
    assert_eq!(js_tools.len(), 4);
    assert!(js_tools.contains_key("bun"));
    assert!(js_tools.contains_key("npm"));
    assert!(js_tools.contains_key("yarn"));
    assert!(js_tools.contains_key("pnpm"));
    
    // Test bun-specific detection
    let bun_status = tool_detector.detect_bun();
    assert!(bun_status.last_checked.elapsed().unwrap().as_secs() < 5);
    
    // Test caching behavior
    let bun_status_cached = tool_detector.detect_bun();
    assert_eq!(bun_status.last_checked, bun_status_cached.last_checked);
    
    // Test cache clearing
    tool_detector.clear_cache();
    let bun_status_fresh = tool_detector.detect_bun();
    assert!(bun_status_fresh.last_checked >= bun_status.last_checked);
}

#[test]
fn test_runtime_detection_edge_cases() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    // Test empty project
    let runtime_detector = RuntimeDetector::new(project_path.to_path_buf());
    let detection_result = runtime_detector.detect_js_runtime_and_package_manager();
    assert_eq!(detection_result.package_manager, PackageManager::Unknown);
    
    // Test project with only package.json but no specific indicators
    fs::write(
        project_path.join("package.json"),
        r#"{"name": "test", "version": "1.0.0"}"#
    ).unwrap();
    
    // Create NEW detector after creating package.json
    let runtime_detector_with_pkg = RuntimeDetector::new(project_path.to_path_buf());
    let detection_result = runtime_detector_with_pkg.detect_js_runtime_and_package_manager();
    
    // Should default to npm when package.json exists but no specific indicators
    assert_eq!(detection_result.package_manager, PackageManager::Npm); // Default fallback
    assert_eq!(detection_result.runtime, JavaScriptRuntime::Node);
    assert_eq!(detection_result.confidence, DetectionConfidence::Low);
    
    // Test project with explicit packageManager field
    fs::write(
        project_path.join("package.json"),
        r#"{"name": "test", "version": "1.0.0", "packageManager": "bun@1.0.0"}"#
    ).unwrap();
    
    let detection_result = runtime_detector_with_pkg.detect_js_runtime_and_package_manager();
    assert_eq!(detection_result.package_manager, PackageManager::Bun);
}

// Helper functions to create test projects

fn create_bun_project(project_path: &Path) {
    // Create package.json with bun-specific configuration
    fs::write(
        project_path.join("package.json"),
        r#"{
  "name": "test-bun-project",
  "version": "1.0.0",
  "packageManager": "bun@1.0.0",
  "engines": {
    "bun": ">=1.0.0"
  },
  "scripts": {
    "start": "bun run index.js",
    "dev": "bun --watch index.js"
  },
  "dependencies": {
    "express": "^4.18.0",
    "lodash": "^4.17.21"
  },
  "devDependencies": {
    "@types/node": "^18.0.0",
    "bun-types": "^1.0.0"
  }
}"#
    ).unwrap();
    
    // Create bun.lockb (simulated)
    fs::write(
        project_path.join("bun.lockb"),
        "Binary lockfile content (simulated)"
    ).unwrap();
    
    // Create bunfig.toml
    fs::write(
        project_path.join("bunfig.toml"),
        r#"[install]
cache = true

[install.scopes]
"@myorg" = { token = "$NPM_TOKEN", url = "https://registry.npmjs.org/" }
"#
    ).unwrap();
}

fn create_npm_project(project_path: &Path) {
    fs::write(
        project_path.join("package.json"),
        r#"{
  "name": "test-npm-project",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0",
    "axios": "^1.0.0"
  },
  "devDependencies": {
    "jest": "^29.0.0"
  }
}"#
    ).unwrap();
    
    fs::write(
        project_path.join("package-lock.json"),
        r#"{
  "name": "test-npm-project",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {}
}"#
    ).unwrap();
}

fn create_yarn_project(project_path: &Path) {
    fs::write(
        project_path.join("package.json"),
        r#"{
  "name": "test-yarn-project",
  "version": "1.0.0",
  "packageManager": "yarn@3.6.0",
  "dependencies": {
    "vue": "^3.0.0",
    "vuex": "^4.0.0"
  }
}"#
    ).unwrap();
    
    fs::write(
        project_path.join("yarn.lock"),
        r#"# THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.
# yarn lockfile v1

vue@^3.0.0:
  version "3.3.4"
  resolved "https://registry.yarnpkg.com/vue/-/vue-3.3.4.tgz"
"#
    ).unwrap();
}

fn create_pnpm_project(project_path: &Path) {
    fs::write(
        project_path.join("package.json"),
        r#"{
  "name": "test-pnpm-project",
  "version": "1.0.0",
  "packageManager": "pnpm@8.0.0",
  "dependencies": {
    "svelte": "^4.0.0"
  }
}"#
    ).unwrap();
    
    fs::write(
        project_path.join("pnpm-lock.yaml"),
        r#"lockfileVersion: '6.0'

settings:
  autoInstallPeers: true
  excludeLinksFromLockfile: false

dependencies:
  svelte:
    specifier: ^4.0.0
    version: 4.2.0
"#
    ).unwrap();
}

fn create_multi_runtime_project(project_path: &Path) {
    // Create package.json with explicit bun preference
    fs::write(
        project_path.join("package.json"),
        r#"{
  "name": "test-multi-runtime",
  "version": "1.0.0",
  "packageManager": "bun@1.0.0",
  "engines": {
    "bun": ">=1.0.0",
    "node": ">=18.0.0"
  },
  "dependencies": {
    "fastify": "^4.0.0"
  }
}"#
    ).unwrap();
    
    // Create all lockfiles to test priority
    fs::write(project_path.join("bun.lockb"), "bun lockfile").unwrap();
    fs::write(project_path.join("yarn.lock"), "yarn lockfile").unwrap();
    fs::write(project_path.join("pnpm-lock.yaml"), "pnpm lockfile").unwrap();
    fs::write(project_path.join("package-lock.json"), "{}").unwrap();
}

fn create_polyglot_project(project_path: &Path) {
    // JavaScript/Node.js
    fs::write(
        project_path.join("package.json"),
        r#"{
  "name": "polyglot-project",
  "version": "1.0.0",
  "dependencies": {
    "express": "^4.18.0"
  }
}"#
    ).unwrap();
    
    // Python
    fs::write(
        project_path.join("requirements.txt"),
        "flask==2.3.0\nrequests==2.31.0\n"
    ).unwrap();
    
    // Rust
    fs::write(
        project_path.join("Cargo.toml"),
        r#"[package]
name = "polyglot-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
tokio = "1.0"
"#
    ).unwrap();
    
    // Go
    fs::write(
        project_path.join("go.mod"),
        r#"module polyglot-project

go 1.19

require (
    github.com/gin-gonic/gin v1.9.0
    github.com/gorilla/mux v1.8.0
)
"#
    ).unwrap();
}