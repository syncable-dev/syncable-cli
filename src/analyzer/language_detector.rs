use crate::analyzer::{AnalysisConfig, DetectedLanguage};
use crate::common::file_utils;
use crate::error::Result;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;

/// Language detection results with detailed information
#[derive(Debug, Clone)]
pub struct LanguageInfo {
    pub name: String,
    pub version: Option<String>,
    pub edition: Option<String>,
    pub package_manager: Option<String>,
    pub main_dependencies: Vec<String>,
    pub dev_dependencies: Vec<String>,
    pub confidence: f32,
    pub source_files: Vec<PathBuf>,
    pub manifest_files: Vec<PathBuf>,
}

/// Detects programming languages with advanced manifest parsing
pub fn detect_languages(
    files: &[PathBuf],
    config: &AnalysisConfig,
) -> Result<Vec<DetectedLanguage>> {
    let mut language_info = HashMap::new();
    
    // First pass: collect files by extension and find manifests
    let mut source_files_by_lang = HashMap::new();
    let mut manifest_files = Vec::new();
    
    for file in files {
        if let Some(extension) = file.extension().and_then(|e| e.to_str()) {
            match extension {
                // Rust files
                "rs" => source_files_by_lang
                    .entry("rust")
                    .or_insert_with(Vec::new)
                    .push(file.clone()),
                    
                // JavaScript/TypeScript files
                "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => source_files_by_lang
                    .entry("javascript")
                    .or_insert_with(Vec::new)
                    .push(file.clone()),
                    
                // Python files
                "py" | "pyx" | "pyi" => source_files_by_lang
                    .entry("python")
                    .or_insert_with(Vec::new)
                    .push(file.clone()),
                    
                // Go files
                "go" => source_files_by_lang
                    .entry("go")
                    .or_insert_with(Vec::new)
                    .push(file.clone()),
                    
                // Java/Kotlin files
                "java" | "kt" | "kts" => source_files_by_lang
                    .entry("jvm")
                    .or_insert_with(Vec::new)
                    .push(file.clone()),
                    
                _ => {}
            }
        }
        
        // Check for manifest files
        if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
            if is_manifest_file(filename) {
                manifest_files.push(file.clone());
            }
        }
    }
    
    // Second pass: analyze each detected language with manifest parsing
    if source_files_by_lang.contains_key("rust") || has_manifest(&manifest_files, &["Cargo.toml"]) {
        if let Ok(info) = analyze_rust_project(&manifest_files, source_files_by_lang.get("rust"), config) {
            language_info.insert("rust", info);
        }
    }
    
    if source_files_by_lang.contains_key("javascript") || has_manifest(&manifest_files, &["package.json"]) {
        if let Ok(info) = analyze_javascript_project(&manifest_files, source_files_by_lang.get("javascript"), config) {
            language_info.insert("javascript", info);
        }
    }
    
    if source_files_by_lang.contains_key("python") || has_manifest(&manifest_files, &["requirements.txt", "Pipfile", "pyproject.toml", "setup.py"]) {
        if let Ok(info) = analyze_python_project(&manifest_files, source_files_by_lang.get("python"), config) {
            language_info.insert("python", info);
        }
    }
    
    if source_files_by_lang.contains_key("go") || has_manifest(&manifest_files, &["go.mod"]) {
        if let Ok(info) = analyze_go_project(&manifest_files, source_files_by_lang.get("go"), config) {
            language_info.insert("go", info);
        }
    }
    
    if source_files_by_lang.contains_key("jvm") || has_manifest(&manifest_files, &["pom.xml", "build.gradle", "build.gradle.kts"]) {
        if let Ok(info) = analyze_jvm_project(&manifest_files, source_files_by_lang.get("jvm"), config) {
            language_info.insert("jvm", info);
        }
    }
    
    // Convert to DetectedLanguage format
    let mut detected_languages = Vec::new();
    for (_, info) in language_info {
        detected_languages.push(DetectedLanguage {
            name: info.name,
            version: info.version,
            confidence: info.confidence,
            files: info.source_files,
            main_dependencies: info.main_dependencies,
            dev_dependencies: info.dev_dependencies,
            package_manager: info.package_manager,
        });
    }
    
    // Sort by confidence (highest first)
    detected_languages.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    
    Ok(detected_languages)
}

/// Analyze Rust project from Cargo.toml
fn analyze_rust_project(
    manifest_files: &[PathBuf],
    source_files: Option<&Vec<PathBuf>>,
    config: &AnalysisConfig,
) -> Result<LanguageInfo> {
    let mut info = LanguageInfo {
        name: "Rust".to_string(),
        version: None,
        edition: None,
        package_manager: Some("cargo".to_string()),
        main_dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        confidence: 0.5,
        source_files: source_files.map_or(Vec::new(), |f| f.clone()),
        manifest_files: Vec::new(),
    };
    
    // Find and parse Cargo.toml
    for manifest in manifest_files {
        if manifest.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml") {
            info.manifest_files.push(manifest.clone());
            
            if let Ok(content) = file_utils::read_file_safe(manifest, config.max_file_size) {
                if let Ok(cargo_toml) = toml::from_str::<toml::Value>(&content) {
                    // Extract edition
                    if let Some(package) = cargo_toml.get("package") {
                        if let Some(edition) = package.get("edition").and_then(|e| e.as_str()) {
                            info.edition = Some(edition.to_string());
                        }
                        
                        // Estimate Rust version from edition
                        info.version = match info.edition.as_deref() {
                            Some("2021") => Some("1.56+".to_string()),
                            Some("2018") => Some("1.31+".to_string()),
                            Some("2015") => Some("1.0+".to_string()),
                            _ => Some("unknown".to_string()),
                        };
                    }
                    
                    // Extract dependencies
                    if let Some(deps) = cargo_toml.get("dependencies") {
                        if let Some(deps_table) = deps.as_table() {
                            for (name, _) in deps_table {
                                info.main_dependencies.push(name.clone());
                            }
                        }
                    }
                    
                    // Extract dev dependencies if enabled
                    if config.include_dev_dependencies {
                        if let Some(dev_deps) = cargo_toml.get("dev-dependencies") {
                            if let Some(dev_deps_table) = dev_deps.as_table() {
                                for (name, _) in dev_deps_table {
                                    info.dev_dependencies.push(name.clone());
                                }
                            }
                        }
                    }
                    
                    info.confidence = 0.95; // High confidence with manifest
                }
            }
            break;
        }
    }
    
    // Boost confidence if we have source files
    if !info.source_files.is_empty() {
        info.confidence = (info.confidence + 0.9) / 2.0;
    }
    
    Ok(info)
}

/// Analyze JavaScript/TypeScript project from package.json
fn analyze_javascript_project(
    manifest_files: &[PathBuf],
    source_files: Option<&Vec<PathBuf>>,
    config: &AnalysisConfig,
) -> Result<LanguageInfo> {
    let mut info = LanguageInfo {
        name: "JavaScript/TypeScript".to_string(),
        version: None,
        edition: None,
        package_manager: None,
        main_dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        confidence: 0.5,
        source_files: source_files.map_or(Vec::new(), |f| f.clone()),
        manifest_files: Vec::new(),
    };
    
    // Detect package manager from lock files
    for manifest in manifest_files {
        if let Some(filename) = manifest.file_name().and_then(|n| n.to_str()) {
            match filename {
                "package-lock.json" => info.package_manager = Some("npm".to_string()),
                "yarn.lock" => info.package_manager = Some("yarn".to_string()),
                "pnpm-lock.yaml" => info.package_manager = Some("pnpm".to_string()),
                _ => {}
            }
        }
    }
    
    // Default to npm if no package manager detected
    if info.package_manager.is_none() {
        info.package_manager = Some("npm".to_string());
    }
    
    // Find and parse package.json
    for manifest in manifest_files {
        if manifest.file_name().and_then(|n| n.to_str()) == Some("package.json") {
            info.manifest_files.push(manifest.clone());
            
            if let Ok(content) = file_utils::read_file_safe(manifest, config.max_file_size) {
                if let Ok(package_json) = serde_json::from_str::<JsonValue>(&content) {
                    // Extract Node.js version from engines
                    if let Some(engines) = package_json.get("engines") {
                        if let Some(node_version) = engines.get("node").and_then(|v| v.as_str()) {
                            info.version = Some(node_version.to_string());
                        }
                    }
                    
                    // Extract dependencies
                    if let Some(deps) = package_json.get("dependencies") {
                        if let Some(deps_obj) = deps.as_object() {
                            for (name, _) in deps_obj {
                                info.main_dependencies.push(name.clone());
                            }
                        }
                    }
                    
                    // Extract dev dependencies if enabled
                    if config.include_dev_dependencies {
                        if let Some(dev_deps) = package_json.get("devDependencies") {
                            if let Some(dev_deps_obj) = dev_deps.as_object() {
                                for (name, _) in dev_deps_obj {
                                    info.dev_dependencies.push(name.clone());
                                }
                            }
                        }
                    }
                    
                    info.confidence = 0.95; // High confidence with manifest
                }
            }
            break;
        }
    }
    
    // Adjust name based on file types
    if let Some(files) = source_files {
        let has_typescript = files.iter().any(|f| {
            f.extension()
                .and_then(|e| e.to_str())
                .map_or(false, |ext| ext == "ts" || ext == "tsx")
        });
        
        if has_typescript {
            info.name = "TypeScript".to_string();
        } else {
            info.name = "JavaScript".to_string();
        }
    }
    
    // Boost confidence if we have source files
    if !info.source_files.is_empty() {
        info.confidence = (info.confidence + 0.9) / 2.0;
    }
    
    Ok(info)
}

/// Analyze Python project from various manifest files
fn analyze_python_project(
    manifest_files: &[PathBuf],
    source_files: Option<&Vec<PathBuf>>,
    config: &AnalysisConfig,
) -> Result<LanguageInfo> {
    let mut info = LanguageInfo {
        name: "Python".to_string(),
        version: None,
        edition: None,
        package_manager: None,
        main_dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        confidence: 0.5,
        source_files: source_files.map_or(Vec::new(), |f| f.clone()),
        manifest_files: Vec::new(),
    };
    
    // Detect package manager and parse manifest files
    for manifest in manifest_files {
        if let Some(filename) = manifest.file_name().and_then(|n| n.to_str()) {
            info.manifest_files.push(manifest.clone());
            
            match filename {
                "requirements.txt" => {
                    info.package_manager = Some("pip".to_string());
                    if let Ok(content) = file_utils::read_file_safe(manifest, config.max_file_size) {
                        parse_requirements_txt(&content, &mut info);
                        info.confidence = 0.85;
                    }
                }
                "Pipfile" => {
                    info.package_manager = Some("pipenv".to_string());
                    if let Ok(content) = file_utils::read_file_safe(manifest, config.max_file_size) {
                        parse_pipfile(&content, &mut info, config);
                        info.confidence = 0.90;
                    }
                }
                "pyproject.toml" => {
                    info.package_manager = Some("poetry/pip".to_string());
                    if let Ok(content) = file_utils::read_file_safe(manifest, config.max_file_size) {
                        parse_pyproject_toml(&content, &mut info, config);
                        info.confidence = 0.95;
                    }
                }
                "setup.py" => {
                    info.package_manager = Some("setuptools".to_string());
                    if let Ok(content) = file_utils::read_file_safe(manifest, config.max_file_size) {
                        parse_setup_py(&content, &mut info);
                        info.confidence = 0.80;
                    }
                }
                _ => {}
            }
        }
    }
    
    // Default to pip if no package manager detected
    if info.package_manager.is_none() && !info.source_files.is_empty() {
        info.package_manager = Some("pip".to_string());
        info.confidence = 0.75;
    }
    
    // Boost confidence if we have source files
    if !info.source_files.is_empty() {
        info.confidence = (info.confidence + 0.8) / 2.0;
    }
    
    Ok(info)
}

/// Parse requirements.txt file
fn parse_requirements_txt(content: &str, info: &mut LanguageInfo) {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Extract package name (before ==, >=, etc.)
        if let Some(package_name) = line.split(&['=', '>', '<', '!', '~', ';'][..]).next() {
            let clean_name = package_name.trim();
            if !clean_name.is_empty() && !clean_name.starts_with('-') {
                info.main_dependencies.push(clean_name.to_string());
            }
        }
    }
}

/// Parse Pipfile (TOML format)
fn parse_pipfile(content: &str, info: &mut LanguageInfo, config: &AnalysisConfig) {
    if let Ok(pipfile) = toml::from_str::<toml::Value>(content) {
        // Extract Python version requirement
        if let Some(requires) = pipfile.get("requires") {
            if let Some(python_version) = requires.get("python_version").and_then(|v| v.as_str()) {
                info.version = Some(format!("~={}", python_version));
            } else if let Some(python_full) = requires.get("python_full_version").and_then(|v| v.as_str()) {
                info.version = Some(format!("=={}", python_full));
            }
        }
        
        // Extract packages
        if let Some(packages) = pipfile.get("packages") {
            if let Some(packages_table) = packages.as_table() {
                for (name, _) in packages_table {
                    info.main_dependencies.push(name.clone());
                }
            }
        }
        
        // Extract dev packages if enabled
        if config.include_dev_dependencies {
            if let Some(dev_packages) = pipfile.get("dev-packages") {
                if let Some(dev_packages_table) = dev_packages.as_table() {
                    for (name, _) in dev_packages_table {
                        info.dev_dependencies.push(name.clone());
                    }
                }
            }
        }
    }
}

/// Parse pyproject.toml file
fn parse_pyproject_toml(content: &str, info: &mut LanguageInfo, config: &AnalysisConfig) {
    if let Ok(pyproject) = toml::from_str::<toml::Value>(content) {
        // Extract Python version from project metadata
        if let Some(project) = pyproject.get("project") {
            if let Some(requires_python) = project.get("requires-python").and_then(|v| v.as_str()) {
                info.version = Some(requires_python.to_string());
            }
            
            // Extract dependencies
            if let Some(dependencies) = project.get("dependencies") {
                if let Some(deps_array) = dependencies.as_array() {
                    for dep in deps_array {
                        if let Some(dep_str) = dep.as_str() {
                            if let Some(package_name) = dep_str.split(&['=', '>', '<', '!', '~', ';'][..]).next() {
                                let clean_name = package_name.trim();
                                if !clean_name.is_empty() {
                                    info.main_dependencies.push(clean_name.to_string());
                                }
                            }
                        }
                    }
                }
            }
            
            // Extract optional dependencies (dev dependencies)
            if config.include_dev_dependencies {
                if let Some(optional_deps) = project.get("optional-dependencies") {
                    if let Some(optional_table) = optional_deps.as_table() {
                        for (_, deps) in optional_table {
                            if let Some(deps_array) = deps.as_array() {
                                for dep in deps_array {
                                    if let Some(dep_str) = dep.as_str() {
                                        if let Some(package_name) = dep_str.split(&['=', '>', '<', '!', '~', ';'][..]).next() {
                                            let clean_name = package_name.trim();
                                            if !clean_name.is_empty() {
                                                info.dev_dependencies.push(clean_name.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Check for Poetry configuration
        if pyproject.get("tool").and_then(|t| t.get("poetry")).is_some() {
            info.package_manager = Some("poetry".to_string());
            
            // Extract Poetry dependencies
            if let Some(tool) = pyproject.get("tool") {
                if let Some(poetry) = tool.get("poetry") {
                    if let Some(dependencies) = poetry.get("dependencies") {
                        if let Some(deps_table) = dependencies.as_table() {
                            for (name, _) in deps_table {
                                if name != "python" {
                                    info.main_dependencies.push(name.clone());
                                }
                            }
                        }
                    }
                    
                    if config.include_dev_dependencies {
                        if let Some(dev_dependencies) = poetry.get("group")
                            .and_then(|g| g.get("dev"))
                            .and_then(|d| d.get("dependencies")) 
                        {
                            if let Some(dev_deps_table) = dev_dependencies.as_table() {
                                for (name, _) in dev_deps_table {
                                    info.dev_dependencies.push(name.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Parse setup.py file (basic extraction)
fn parse_setup_py(content: &str, info: &mut LanguageInfo) {
    // Basic regex-based parsing for common patterns
    for line in content.lines() {
        let line = line.trim();
        
        // Look for python_requires
        if line.contains("python_requires") {
            if let Some(start) = line.find("\"") {
                if let Some(end) = line[start + 1..].find("\"") {
                    let version = &line[start + 1..start + 1 + end];
                    info.version = Some(version.to_string());
                }
            } else if let Some(start) = line.find("'") {
                if let Some(end) = line[start + 1..].find("'") {
                    let version = &line[start + 1..start + 1 + end];
                    info.version = Some(version.to_string());
                }
            }
        }
        
        // Look for install_requires (basic pattern)
        if line.contains("install_requires") && line.contains("[") {
            // This is a simplified parser - could be enhanced
            info.main_dependencies.push("setuptools-detected".to_string());
        }
    }
}

/// Analyze Go project from go.mod
fn analyze_go_project(
    manifest_files: &[PathBuf],
    source_files: Option<&Vec<PathBuf>>,
    config: &AnalysisConfig,
) -> Result<LanguageInfo> {
    let mut info = LanguageInfo {
        name: "Go".to_string(),
        version: None,
        edition: None,
        package_manager: Some("go mod".to_string()),
        main_dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        confidence: 0.5,
        source_files: source_files.map_or(Vec::new(), |f| f.clone()),
        manifest_files: Vec::new(),
    };
    
    // Find and parse go.mod
    for manifest in manifest_files {
        if let Some(filename) = manifest.file_name().and_then(|n| n.to_str()) {
            match filename {
                "go.mod" => {
                    info.manifest_files.push(manifest.clone());
                    if let Ok(content) = file_utils::read_file_safe(manifest, config.max_file_size) {
                        parse_go_mod(&content, &mut info);
                        info.confidence = 0.95;
                    }
                }
                "go.sum" => {
                    info.manifest_files.push(manifest.clone());
                    // go.sum contains checksums, indicates a real Go project
                    info.confidence = (info.confidence + 0.9) / 2.0;
                }
                _ => {}
            }
        }
    }
    
    // Boost confidence if we have source files
    if !info.source_files.is_empty() {
        info.confidence = (info.confidence + 0.85) / 2.0;
    }
    
    Ok(info)
}

/// Parse go.mod file
fn parse_go_mod(content: &str, info: &mut LanguageInfo) {
    for line in content.lines() {
        let line = line.trim();
        
        // Parse go version directive
        if line.starts_with("go ") {
            let version = line[3..].trim();
            info.version = Some(version.to_string());
        }
        
        // Parse require block
        if line.starts_with("require ") {
            // Single line require
            let require_line = &line[8..].trim();
            if let Some(module_name) = require_line.split_whitespace().next() {
                info.main_dependencies.push(module_name.to_string());
            }
        }
    }
    
    // Parse multi-line require blocks
    let mut in_require_block = false;
    for line in content.lines() {
        let line = line.trim();
        
        if line == "require (" {
            in_require_block = true;
            continue;
        }
        
        if in_require_block {
            if line == ")" {
                in_require_block = false;
                continue;
            }
            
            // Parse dependency line
            if !line.is_empty() && !line.starts_with("//") {
                if let Some(module_name) = line.split_whitespace().next() {
                    info.main_dependencies.push(module_name.to_string());
                }
            }
        }
    }
}

/// Analyze JVM project (Java/Kotlin) from build files
fn analyze_jvm_project(
    manifest_files: &[PathBuf],
    source_files: Option<&Vec<PathBuf>>,
    config: &AnalysisConfig,
) -> Result<LanguageInfo> {
    let mut info = LanguageInfo {
        name: "Java/Kotlin".to_string(),
        version: None,
        edition: None,
        package_manager: None,
        main_dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        confidence: 0.5,
        source_files: source_files.map_or(Vec::new(), |f| f.clone()),
        manifest_files: Vec::new(),
    };
    
    // Detect build tool and parse manifest files
    for manifest in manifest_files {
        if let Some(filename) = manifest.file_name().and_then(|n| n.to_str()) {
            info.manifest_files.push(manifest.clone());
            
            match filename {
                "pom.xml" => {
                    info.package_manager = Some("maven".to_string());
                    if let Ok(content) = file_utils::read_file_safe(manifest, config.max_file_size) {
                        parse_maven_pom(&content, &mut info, config);
                        info.confidence = 0.90;
                    }
                }
                "build.gradle" => {
                    info.package_manager = Some("gradle".to_string());
                    if let Ok(content) = file_utils::read_file_safe(manifest, config.max_file_size) {
                        parse_gradle_build(&content, &mut info, config);
                        info.confidence = 0.85;
                    }
                }
                "build.gradle.kts" => {
                    info.package_manager = Some("gradle".to_string());
                    if let Ok(content) = file_utils::read_file_safe(manifest, config.max_file_size) {
                        parse_gradle_kts_build(&content, &mut info, config);
                        info.confidence = 0.85;
                    }
                }
                _ => {}
            }
        }
    }
    
    // Adjust name based on file types
    if let Some(files) = source_files {
        let has_kotlin = files.iter().any(|f| {
            f.extension()
                .and_then(|e| e.to_str())
                .map_or(false, |ext| ext == "kt" || ext == "kts")
        });
        
        if has_kotlin {
            info.name = "Kotlin".to_string();
        } else {
            info.name = "Java".to_string();
        }
    }
    
    // Boost confidence if we have source files
    if !info.source_files.is_empty() {
        info.confidence = (info.confidence + 0.8) / 2.0;
    }
    
    Ok(info)
}

/// Parse Maven pom.xml file (basic XML parsing)
fn parse_maven_pom(content: &str, info: &mut LanguageInfo, config: &AnalysisConfig) {
    // Simple regex-based XML parsing for common Maven patterns
    
    // Extract Java version from maven.compiler.source or java.version
    for line in content.lines() {
        let line = line.trim();
        
        // Look for Java version in properties
        if line.contains("<maven.compiler.source>") {
            if let Some(version) = extract_xml_content(line, "maven.compiler.source") {
                info.version = Some(version);
            }
        } else if line.contains("<java.version>") {
            if let Some(version) = extract_xml_content(line, "java.version") {
                info.version = Some(version);
            }
        } else if line.contains("<maven.compiler.target>") && info.version.is_none() {
            if let Some(version) = extract_xml_content(line, "maven.compiler.target") {
                info.version = Some(version);
            }
        }
        
        // Extract dependencies
        if line.contains("<groupId>") && line.contains("<artifactId>") {
            // This is a simplified approach - real XML parsing would be better
            if let Some(group_id) = extract_xml_content(line, "groupId") {
                if let Some(artifact_id) = extract_xml_content(line, "artifactId") {
                    let dependency = format!("{}:{}", group_id, artifact_id);
                    info.main_dependencies.push(dependency);
                }
            }
        } else if line.contains("<artifactId>") && !line.contains("<groupId>") {
            if let Some(artifact_id) = extract_xml_content(line, "artifactId") {
                info.main_dependencies.push(artifact_id);
            }
        }
    }
    
    // Look for dependencies in a more structured way
    let mut in_dependencies = false;
    let mut in_test_dependencies = false;
    
    for line in content.lines() {
        let line = line.trim();
        
        if line.contains("<dependencies>") {
            in_dependencies = true;
            continue;
        }
        
        if line.contains("</dependencies>") {
            in_dependencies = false;
            in_test_dependencies = false;
            continue;
        }
        
        if in_dependencies && line.contains("<scope>test</scope>") {
            in_test_dependencies = true;
        }
        
        if in_dependencies && line.contains("<artifactId>") {
            if let Some(artifact_id) = extract_xml_content(line, "artifactId") {
                if in_test_dependencies && config.include_dev_dependencies {
                    info.dev_dependencies.push(artifact_id);
                } else if !in_test_dependencies {
                    info.main_dependencies.push(artifact_id);
                }
            }
        }
    }
}

/// Parse Gradle build.gradle file (Groovy syntax)
fn parse_gradle_build(content: &str, info: &mut LanguageInfo, config: &AnalysisConfig) {
    for line in content.lines() {
        let line = line.trim();
        
        // Look for Java version
        if line.contains("sourceCompatibility") || line.contains("targetCompatibility") {
            if let Some(version) = extract_gradle_version(line) {
                info.version = Some(version);
            }
        } else if line.contains("JavaVersion.VERSION_") {
            if let Some(pos) = line.find("VERSION_") {
                let version_part = &line[pos + 8..];
                if let Some(end) = version_part.find(|c: char| !c.is_numeric() && c != '_') {
                    let version = &version_part[..end].replace('_', ".");
                    info.version = Some(version.to_string());
                }
            }
        }
        
        // Look for dependencies
        if line.starts_with("implementation ") || line.starts_with("compile ") {
            if let Some(dep) = extract_gradle_dependency(line) {
                info.main_dependencies.push(dep);
            }
        } else if (line.starts_with("testImplementation ") || line.starts_with("testCompile ")) && config.include_dev_dependencies {
            if let Some(dep) = extract_gradle_dependency(line) {
                info.dev_dependencies.push(dep);
            }
        }
    }
}

/// Parse Gradle build.gradle.kts file (Kotlin syntax)
fn parse_gradle_kts_build(content: &str, info: &mut LanguageInfo, config: &AnalysisConfig) {
    // Kotlin DSL is similar to Groovy but with some syntax differences
    parse_gradle_build(content, info, config); // Reuse the same logic for now
}

/// Extract content from XML tags
fn extract_xml_content(line: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);
    
    if let Some(start) = line.find(&open_tag) {
        if let Some(end) = line.find(&close_tag) {
            let content_start = start + open_tag.len();
            if content_start < end {
                return Some(line[content_start..end].trim().to_string());
            }
        }
    }
    None
}

/// Extract version from Gradle configuration line
fn extract_gradle_version(line: &str) -> Option<String> {
    // Look for patterns like sourceCompatibility = '11' or sourceCompatibility = "11"
    if let Some(equals_pos) = line.find('=') {
        let value_part = line[equals_pos + 1..].trim();
        if let Some(start_quote) = value_part.find(['\'', '"']) {
            let quote_char = value_part.chars().nth(start_quote).unwrap();
            if let Some(end_quote) = value_part[start_quote + 1..].find(quote_char) {
                let version = &value_part[start_quote + 1..start_quote + 1 + end_quote];
                return Some(version.to_string());
            }
        }
    }
    None
}

/// Extract dependency from Gradle dependency line
fn extract_gradle_dependency(line: &str) -> Option<String> {
    // Look for patterns like implementation 'group:artifact:version' or implementation("group:artifact:version")
    if let Some(start_quote) = line.find(['\'', '"']) {
        let quote_char = line.chars().nth(start_quote).unwrap();
        if let Some(end_quote) = line[start_quote + 1..].find(quote_char) {
            let dependency = &line[start_quote + 1..start_quote + 1 + end_quote];
            // Extract just the artifact name for simplicity
            if let Some(last_colon) = dependency.rfind(':') {
                if let Some(first_colon) = dependency[..last_colon].rfind(':') {
                    return Some(dependency[first_colon + 1..last_colon].to_string());
                }
            }
            return Some(dependency.to_string());
        }
    }
    None
}

/// Check if a filename is a known manifest file
fn is_manifest_file(filename: &str) -> bool {
    matches!(
        filename,
        "Cargo.toml" | "Cargo.lock" |
        "package.json" | "package-lock.json" | "yarn.lock" | "pnpm-lock.yaml" |
        "requirements.txt" | "Pipfile" | "Pipfile.lock" | "pyproject.toml" | "setup.py" |
        "go.mod" | "go.sum" |
        "pom.xml" | "build.gradle" | "build.gradle.kts"
    )
}

/// Check if any of the specified manifest files exist
fn has_manifest(manifest_files: &[PathBuf], target_files: &[&str]) -> bool {
    manifest_files.iter().any(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map_or(false, |name| target_files.contains(&name))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_rust_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create Cargo.toml
        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
tokio = "1.0"

[dev-dependencies]
assert_cmd = "2.0"
"#;
        fs::write(root.join("Cargo.toml"), cargo_toml).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        
        let config = AnalysisConfig::default();
        let files = vec![
            root.join("Cargo.toml"),
            root.join("src/main.rs"),
        ];
        
        let languages = detect_languages(&files, &config).unwrap();
        assert_eq!(languages.len(), 1);
        assert_eq!(languages[0].name, "Rust");
        assert_eq!(languages[0].version, Some("1.56+".to_string()));
        assert!(languages[0].confidence > 0.9);
    }
    
    #[test]
    fn test_javascript_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create package.json
        let package_json = r#"
{
  "name": "test-project",
  "version": "1.0.0",
  "engines": {
    "node": ">=16.0.0"
  },
  "dependencies": {
    "express": "^4.18.0",
    "lodash": "^4.17.21"
  },
  "devDependencies": {
    "jest": "^29.0.0"
  }
}
"#;
        fs::write(root.join("package.json"), package_json).unwrap();
        fs::write(root.join("index.js"), "console.log('hello');").unwrap();
        
        let config = AnalysisConfig::default();
        let files = vec![
            root.join("package.json"),
            root.join("index.js"),
        ];
        
        let languages = detect_languages(&files, &config).unwrap();
        assert_eq!(languages.len(), 1);
        assert_eq!(languages[0].name, "JavaScript");
        assert_eq!(languages[0].version, Some(">=16.0.0".to_string()));
        assert!(languages[0].confidence > 0.9);
    }
    
    #[test]
    fn test_python_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create pyproject.toml
        let pyproject_toml = r#"
[project]
name = "test-project"
version = "0.1.0"
requires-python = ">=3.8"
dependencies = [
    "flask>=2.0.0",
    "requests>=2.25.0",
    "pandas>=1.3.0"
]

[project.optional-dependencies]
dev = [
    "pytest>=6.0.0",
    "black>=21.0.0"
]
"#;
        fs::write(root.join("pyproject.toml"), pyproject_toml).unwrap();
        fs::write(root.join("app.py"), "print('Hello, World!')").unwrap();
        
        let config = AnalysisConfig::default();
        let files = vec![
            root.join("pyproject.toml"),
            root.join("app.py"),
        ];
        
        let languages = detect_languages(&files, &config).unwrap();
        assert_eq!(languages.len(), 1);
        assert_eq!(languages[0].name, "Python");
        assert_eq!(languages[0].version, Some(">=3.8".to_string()));
        assert!(languages[0].confidence > 0.8);
    }
    
    #[test]
    fn test_go_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create go.mod
        let go_mod = r#"
module example.com/myproject

go 1.21

require (
    github.com/gin-gonic/gin v1.9.1
    github.com/stretchr/testify v1.8.4
    golang.org/x/time v0.3.0
)
"#;
        fs::write(root.join("go.mod"), go_mod).unwrap();
        fs::write(root.join("main.go"), "package main\n\nfunc main() {}").unwrap();
        
        let config = AnalysisConfig::default();
        let files = vec![
            root.join("go.mod"),
            root.join("main.go"),
        ];
        
        let languages = detect_languages(&files, &config).unwrap();
        assert_eq!(languages.len(), 1);
        assert_eq!(languages[0].name, "Go");
        assert_eq!(languages[0].version, Some("1.21".to_string()));
        assert!(languages[0].confidence > 0.8);
    }
    
    #[test]
    fn test_java_maven_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create pom.xml
        let pom_xml = r#"
<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    
    <groupId>com.example</groupId>
    <artifactId>test-project</artifactId>
    <version>1.0.0</version>
    
    <properties>
        <maven.compiler.source>17</maven.compiler.source>
        <maven.compiler.target>17</maven.compiler.target>
    </properties>
    
    <dependencies>
        <dependency>
            <groupId>org.springframework</groupId>
            <artifactId>spring-core</artifactId>
            <version>5.3.21</version>
        </dependency>
        <dependency>
            <groupId>junit</groupId>
            <artifactId>junit</artifactId>
            <version>4.13.2</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>
"#;
        fs::create_dir_all(root.join("src/main/java")).unwrap();
        fs::write(root.join("pom.xml"), pom_xml).unwrap();
        fs::write(root.join("src/main/java/App.java"), "public class App {}").unwrap();
        
        let config = AnalysisConfig::default();
        let files = vec![
            root.join("pom.xml"),
            root.join("src/main/java/App.java"),
        ];
        
        let languages = detect_languages(&files, &config).unwrap();
        assert_eq!(languages.len(), 1);
        assert_eq!(languages[0].name, "Java");
        assert_eq!(languages[0].version, Some("17".to_string()));
        assert!(languages[0].confidence > 0.8);
    }
    
    #[test]
    fn test_kotlin_gradle_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create build.gradle.kts
        let build_gradle_kts = r#"
plugins {
    kotlin("jvm") version "1.9.0"
    application
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

dependencies {
    implementation("org.jetbrains.kotlin:kotlin-stdlib")
    implementation("io.ktor:ktor-server-core:2.3.2")
    testImplementation("org.jetbrains.kotlin:kotlin-test")
}
"#;
        fs::create_dir_all(root.join("src/main/kotlin")).unwrap();
        fs::write(root.join("build.gradle.kts"), build_gradle_kts).unwrap();
        fs::write(root.join("src/main/kotlin/Main.kt"), "fun main() {}").unwrap();
        
        let config = AnalysisConfig::default();
        let files = vec![
            root.join("build.gradle.kts"),
            root.join("src/main/kotlin/Main.kt"),
        ];
        
        let languages = detect_languages(&files, &config).unwrap();
        assert_eq!(languages.len(), 1);
        assert_eq!(languages[0].name, "Kotlin");
        assert!(languages[0].confidence > 0.8);
    }
    
    #[test]
    fn test_python_requirements_txt_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create requirements.txt
        let requirements_txt = r#"
Flask==2.3.2
requests>=2.28.0
pandas==1.5.3
pytest==7.4.0
black>=23.0.0
"#;
        fs::write(root.join("requirements.txt"), requirements_txt).unwrap();
        fs::write(root.join("app.py"), "import flask").unwrap();
        
        let config = AnalysisConfig::default();
        let files = vec![
            root.join("requirements.txt"),
            root.join("app.py"),
        ];
        
        let languages = detect_languages(&files, &config).unwrap();
        assert_eq!(languages.len(), 1);
        assert_eq!(languages[0].name, "Python");
        assert!(languages[0].confidence > 0.8);
    }
} 