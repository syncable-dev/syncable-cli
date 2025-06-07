use crate::analyzer::{AnalysisConfig, DetectedLanguage, DependencyMap};
use crate::analyzer::vulnerability_checker::{VulnerabilityChecker, VulnerabilityInfo};
use crate::error::{Result, AnalysisError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use log::{debug, info, warn};

/// Detailed dependency information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
    pub dep_type: DependencyType,
    pub license: String,
    pub source: Option<String>,
    pub language: Language,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyType {
    Production,
    Dev,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Go,
    Java,
    Kotlin,
    Unknown,
}

impl Language {
    pub fn as_str(&self) -> &str {
        match self {
            Language::Rust => "Rust",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Python => "Python",
            Language::Go => "Go",
            Language::Java => "Java",
            Language::Kotlin => "Kotlin",
            Language::Unknown => "Unknown",
        }
    }

    pub fn from_string(s: &str) -> Option<Language> {
        match s.to_lowercase().as_str() {
            "rust" => Some(Language::Rust),
            "javascript" | "js" => Some(Language::JavaScript),
            "typescript" | "ts" => Some(Language::TypeScript),
            "python" | "py" => Some(Language::Python),
            "go" | "golang" => Some(Language::Go),
            "java" => Some(Language::Java),
            "kotlin" => Some(Language::Kotlin),
            _ => None,
        }
    }
}

/// Vulnerability information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vulnerability {
    pub id: String,
    pub severity: VulnerabilitySeverity,
    pub description: String,
    pub fixed_in: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VulnerabilitySeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Legacy dependency info for existing code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegacyDependencyInfo {
    pub version: String,
    pub is_dev: bool,
    pub license: Option<String>,
    pub vulnerabilities: Vec<Vulnerability>,
    pub source: String, // npm, crates.io, pypi, etc.
}

/// Enhanced dependency map with detailed information
pub type DetailedDependencyMap = HashMap<String, LegacyDependencyInfo>;

/// Result of dependency analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DependencyAnalysis {
    pub dependencies: DetailedDependencyMap,
    pub total_count: usize,
    pub production_count: usize,
    pub dev_count: usize,
    pub vulnerable_count: usize,
    pub license_summary: HashMap<String, usize>,
}

/// New dependency parser for vulnerability checking
pub struct DependencyParser;

impl DependencyParser {
    pub fn new() -> Self {
        Self
    }
    
    /// Check vulnerabilities for dependencies using the vulnerability checker
    async fn check_vulnerabilities_for_dependencies(
        &self,
        dependencies: &HashMap<Language, Vec<DependencyInfo>>,
        project_path: &Path,
    ) -> HashMap<String, Vec<VulnerabilityInfo>> {
        let mut vulnerability_map = HashMap::new();
        
        let checker = VulnerabilityChecker::new();
        
        match checker.check_all_dependencies(dependencies, project_path).await {
            Ok(report) => {
                info!("Found {} total vulnerabilities across all dependencies", report.total_vulnerabilities);
                
                // Map vulnerabilities by dependency name
                for vuln_dep in report.vulnerable_dependencies {
                    vulnerability_map.insert(vuln_dep.name, vuln_dep.vulnerabilities);
                }
            }
            Err(e) => {
                warn!("Failed to check vulnerabilities: {}", e);
            }
        }
        
        vulnerability_map
    }
    
    /// Convert VulnerabilityInfo to legacy Vulnerability format
    fn convert_vulnerability_info(vuln_info: &VulnerabilityInfo) -> Vulnerability {
        Vulnerability {
            id: vuln_info.id.clone(),
            severity: match vuln_info.severity {
                crate::analyzer::vulnerability_checker::VulnerabilitySeverity::Critical => VulnerabilitySeverity::Critical,
                crate::analyzer::vulnerability_checker::VulnerabilitySeverity::High => VulnerabilitySeverity::High,
                crate::analyzer::vulnerability_checker::VulnerabilitySeverity::Medium => VulnerabilitySeverity::Medium,
                crate::analyzer::vulnerability_checker::VulnerabilitySeverity::Low => VulnerabilitySeverity::Low,
                crate::analyzer::vulnerability_checker::VulnerabilitySeverity::Info => VulnerabilitySeverity::Info,
            },
            description: vuln_info.description.clone(),
            fixed_in: vuln_info.patched_versions.clone(),
        }
    }
    
    pub fn parse_all_dependencies(&self, project_root: &Path) -> Result<HashMap<Language, Vec<DependencyInfo>>> {
        let mut dependencies = HashMap::new();
        
        // Check for Rust
        if project_root.join("Cargo.toml").exists() {
            let rust_deps = self.parse_rust_deps(project_root)?;
            if !rust_deps.is_empty() {
                dependencies.insert(Language::Rust, rust_deps);
            }
        }
        
        // Check for JavaScript/TypeScript
        if project_root.join("package.json").exists() {
            let js_deps = self.parse_js_deps(project_root)?;
            if !js_deps.is_empty() {
                dependencies.insert(Language::JavaScript, js_deps);
            }
        }
        
        // Check for Python
        if project_root.join("requirements.txt").exists() || 
           project_root.join("pyproject.toml").exists() ||
           project_root.join("Pipfile").exists() {
            let py_deps = self.parse_python_deps(project_root)?;
            if !py_deps.is_empty() {
                dependencies.insert(Language::Python, py_deps);
            }
        }
        
        // Check for Go
        if project_root.join("go.mod").exists() {
            let go_deps = self.parse_go_deps(project_root)?;
            if !go_deps.is_empty() {
                dependencies.insert(Language::Go, go_deps);
            }
        }
        
        // Check for Java/Kotlin
        if project_root.join("pom.xml").exists() || project_root.join("build.gradle").exists() {
            let java_deps = self.parse_java_deps(project_root)?;
            if !java_deps.is_empty() {
                dependencies.insert(Language::Java, java_deps);
            }
        }
        
        Ok(dependencies)
    }
    
    fn parse_rust_deps(&self, project_root: &Path) -> Result<Vec<DependencyInfo>> {
        let cargo_lock = project_root.join("Cargo.lock");
        let cargo_toml = project_root.join("Cargo.toml");
        
        let mut deps = Vec::new();
        
        // First try to parse from Cargo.lock (complete dependency tree)
        if cargo_lock.exists() {
            let content = fs::read_to_string(&cargo_lock)?;
            let parsed: toml::Value = toml::from_str(&content)
                .map_err(|e| AnalysisError::DependencyParsing {
                    file: "Cargo.lock".to_string(),
                    reason: e.to_string(),
                })?;
            
            // Parse package list from Cargo.lock
            if let Some(packages) = parsed.get("package").and_then(|p| p.as_array()) {
                for package in packages {
                    if let Some(package_table) = package.as_table() {
                        if let (Some(name), Some(version)) = (
                            package_table.get("name").and_then(|n| n.as_str()),
                            package_table.get("version").and_then(|v| v.as_str())
                        ) {
                            // Determine if it's a direct dependency by checking Cargo.toml
                            let dep_type = self.get_rust_dependency_type(name, &cargo_toml);
                            
                            deps.push(DependencyInfo {
                                name: name.to_string(),
                                version: version.to_string(),
                                dep_type,
                                license: detect_rust_license(name).unwrap_or_else(|| "Unknown".to_string()),
                                source: Some("crates.io".to_string()),
                                language: Language::Rust,
                            });
                        }
                    }
                }
            }
        } else if cargo_toml.exists() {
            // Fallback to Cargo.toml if Cargo.lock doesn't exist
            let content = fs::read_to_string(&cargo_toml)?;
            let parsed: toml::Value = toml::from_str(&content)
                .map_err(|e| AnalysisError::DependencyParsing {
                    file: "Cargo.toml".to_string(),
                    reason: e.to_string(),
                })?;
            
            // Parse regular dependencies
            if let Some(dependencies) = parsed.get("dependencies").and_then(|d| d.as_table()) {
                for (name, value) in dependencies {
                    let version = extract_version_from_toml_value(value);
                    deps.push(DependencyInfo {
                        name: name.clone(),
                        version,
                        dep_type: DependencyType::Production,
                        license: detect_rust_license(name).unwrap_or_else(|| "Unknown".to_string()),
                        source: Some("crates.io".to_string()),
                        language: Language::Rust,
                    });
                }
            }
            
            // Parse dev dependencies
            if let Some(dev_deps) = parsed.get("dev-dependencies").and_then(|d| d.as_table()) {
                for (name, value) in dev_deps {
                    let version = extract_version_from_toml_value(value);
                    deps.push(DependencyInfo {
                        name: name.clone(),
                        version,
                        dep_type: DependencyType::Dev,
                        license: detect_rust_license(name).unwrap_or_else(|| "Unknown".to_string()),
                        source: Some("crates.io".to_string()),
                        language: Language::Rust,
                    });
                }
            }
        }
        
        Ok(deps)
    }
    
    fn get_rust_dependency_type(&self, dep_name: &str, cargo_toml_path: &Path) -> DependencyType {
        if !cargo_toml_path.exists() {
            return DependencyType::Production;
        }
        
        if let Ok(content) = fs::read_to_string(cargo_toml_path) {
            if let Ok(parsed) = toml::from_str::<toml::Value>(&content) {
                // Check if it's in dev-dependencies
                if let Some(dev_deps) = parsed.get("dev-dependencies").and_then(|d| d.as_table()) {
                    if dev_deps.contains_key(dep_name) {
                        return DependencyType::Dev;
                    }
                }
                
                // Check if it's in regular dependencies
                if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_table()) {
                    if deps.contains_key(dep_name) {
                        return DependencyType::Production;
                    }
                }
            }
        }
        
        // Default to production for transitive dependencies
        DependencyType::Production
    }
    
    fn parse_js_deps(&self, project_root: &Path) -> Result<Vec<DependencyInfo>> {
        let package_json = project_root.join("package.json");
        let content = fs::read_to_string(&package_json)?;
        let parsed: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| AnalysisError::DependencyParsing {
                file: "package.json".to_string(),
                reason: e.to_string(),
            })?;
        
        let mut deps = Vec::new();
        
        // Parse regular dependencies
        if let Some(dependencies) = parsed.get("dependencies").and_then(|d| d.as_object()) {
            for (name, version) in dependencies {
                if let Some(ver_str) = version.as_str() {
                    deps.push(DependencyInfo {
                        name: name.clone(),
                        version: ver_str.to_string(),
                        dep_type: DependencyType::Production,
                        license: detect_npm_license(name).unwrap_or_else(|| "Unknown".to_string()),
                        source: Some("npm".to_string()),
                        language: Language::JavaScript,
                    });
                }
            }
        }
        
        // Parse dev dependencies
        if let Some(dev_deps) = parsed.get("devDependencies").and_then(|d| d.as_object()) {
            for (name, version) in dev_deps {
                if let Some(ver_str) = version.as_str() {
                    deps.push(DependencyInfo {
                        name: name.clone(),
                        version: ver_str.to_string(),
                        dep_type: DependencyType::Dev,
                        license: detect_npm_license(name).unwrap_or_else(|| "Unknown".to_string()),
                        source: Some("npm".to_string()),
                        language: Language::JavaScript,
                    });
                }
            }
        }
        
        Ok(deps)
    }
    
    fn parse_python_deps(&self, project_root: &Path) -> Result<Vec<DependencyInfo>> {
        let mut deps = Vec::new();
        
        // Try pyproject.toml first (modern Python packaging)
        let pyproject = project_root.join("pyproject.toml");
        if pyproject.exists() {
            debug!("Found pyproject.toml, parsing Python dependencies");
            let content = fs::read_to_string(&pyproject)?;
            if let Ok(parsed) = toml::from_str::<toml::Value>(&content) {
                // Poetry dependencies
                if let Some(poetry_deps) = parsed
                    .get("tool")
                    .and_then(|t| t.get("poetry"))
                    .and_then(|p| p.get("dependencies"))
                    .and_then(|d| d.as_table())
                {
                    debug!("Found Poetry dependencies in pyproject.toml");
                    for (name, value) in poetry_deps {
                        if name != "python" {
                            let version = extract_version_from_toml_value(value);
                            deps.push(DependencyInfo {
                                name: name.clone(),
                                version,
                                dep_type: DependencyType::Production,
                                license: detect_pypi_license(name).unwrap_or_else(|| "Unknown".to_string()),
                                source: Some("pypi".to_string()),
                                language: Language::Python,
                            });
                        }
                    }
                }
                
                // Poetry dev dependencies
                if let Some(poetry_dev_deps) = parsed
                    .get("tool")
                    .and_then(|t| t.get("poetry"))
                    .and_then(|p| p.get("group"))
                    .and_then(|g| g.get("dev"))
                    .and_then(|d| d.get("dependencies"))
                    .and_then(|d| d.as_table())
                    .or_else(|| {
                        // Fallback to older Poetry format
                        parsed
                            .get("tool")
                            .and_then(|t| t.get("poetry"))
                            .and_then(|p| p.get("dev-dependencies"))
                            .and_then(|d| d.as_table())
                    })
                {
                    debug!("Found Poetry dev dependencies in pyproject.toml");
                    for (name, value) in poetry_dev_deps {
                        let version = extract_version_from_toml_value(value);
                        deps.push(DependencyInfo {
                            name: name.clone(),
                            version,
                            dep_type: DependencyType::Dev,
                            license: detect_pypi_license(name).unwrap_or_else(|| "Unknown".to_string()),
                            source: Some("pypi".to_string()),
                            language: Language::Python,
                        });
                    }
                }
                
                // PEP 621 dependencies (setuptools, flit, hatch, pdm)
                if let Some(project_deps) = parsed
                    .get("project")
                    .and_then(|p| p.get("dependencies"))
                    .and_then(|d| d.as_array())
                {
                    debug!("Found PEP 621 dependencies in pyproject.toml");
                    for dep in project_deps {
                        if let Some(dep_str) = dep.as_str() {
                            let (name, version) = self.parse_python_requirement_spec(dep_str);
                            deps.push(DependencyInfo {
                                name: name.clone(),
                                version,
                                dep_type: DependencyType::Production,
                                license: detect_pypi_license(&name).unwrap_or_else(|| "Unknown".to_string()),
                                source: Some("pypi".to_string()),
                                language: Language::Python,
                            });
                        }
                    }
                }
                
                // PEP 621 optional dependencies (test, dev, etc.)
                if let Some(optional_deps) = parsed
                    .get("project")
                    .and_then(|p| p.get("optional-dependencies"))
                    .and_then(|d| d.as_table())
                {
                    debug!("Found PEP 621 optional dependencies in pyproject.toml");
                    for (group_name, group_deps) in optional_deps {
                        if let Some(deps_array) = group_deps.as_array() {
                            let is_dev = group_name.contains("dev") || group_name.contains("test");
                            for dep in deps_array {
                                if let Some(dep_str) = dep.as_str() {
                                    let (name, version) = self.parse_python_requirement_spec(dep_str);
                                    deps.push(DependencyInfo {
                                        name: name.clone(),
                                        version,
                                        dep_type: if is_dev { DependencyType::Dev } else { DependencyType::Optional },
                                        license: detect_pypi_license(&name).unwrap_or_else(|| "Unknown".to_string()),
                                        source: Some("pypi".to_string()),
                                        language: Language::Python,
                                    });
                                }
                            }
                        }
                    }
                }
                
                // PDM dependencies
                if let Some(pdm_deps) = parsed
                    .get("tool")
                    .and_then(|t| t.get("pdm"))
                    .and_then(|p| p.get("dev-dependencies"))
                    .and_then(|d| d.as_table())
                {
                    debug!("Found PDM dev dependencies in pyproject.toml");
                    for (_group_name, group_deps) in pdm_deps {
                        if let Some(deps_array) = group_deps.as_array() {
                            for dep in deps_array {
                                if let Some(dep_str) = dep.as_str() {
                                    let (name, version) = self.parse_python_requirement_spec(dep_str);
                                    deps.push(DependencyInfo {
                                        name: name.clone(),
                                        version,
                                        dep_type: DependencyType::Dev,
                                        license: detect_pypi_license(&name).unwrap_or_else(|| "Unknown".to_string()),
                                        source: Some("pypi".to_string()),
                                        language: Language::Python,
                                    });
                                }
                            }
                        }
                    }
                }
                
                // Setuptools dependencies (legacy)
                if let Some(setuptools_deps) = parsed
                    .get("tool")
                    .and_then(|t| t.get("setuptools"))
                    .and_then(|s| s.get("dynamic"))
                    .and_then(|d| d.get("dependencies"))
                    .and_then(|d| d.as_array())
                {
                    debug!("Found setuptools dependencies in pyproject.toml");
                    for dep in setuptools_deps {
                        if let Some(dep_str) = dep.as_str() {
                            let (name, version) = self.parse_python_requirement_spec(dep_str);
                            deps.push(DependencyInfo {
                                name: name.clone(),
                                version,
                                dep_type: DependencyType::Production,
                                license: detect_pypi_license(&name).unwrap_or_else(|| "Unknown".to_string()),
                                source: Some("pypi".to_string()),
                                language: Language::Python,
                            });
                        }
                    }
                }
            }
        }
        
        // Try Pipfile (pipenv)
        let pipfile = project_root.join("Pipfile");
        if pipfile.exists() && deps.is_empty() {
            debug!("Found Pipfile, parsing pipenv dependencies");
            let content = fs::read_to_string(&pipfile)?;
            if let Ok(parsed) = toml::from_str::<toml::Value>(&content) {
                // Production dependencies
                if let Some(packages) = parsed.get("packages").and_then(|p| p.as_table()) {
                    for (name, value) in packages {
                        let version = extract_version_from_toml_value(value);
                        deps.push(DependencyInfo {
                            name: name.clone(),
                            version,
                            dep_type: DependencyType::Production,
                            license: detect_pypi_license(name).unwrap_or_else(|| "Unknown".to_string()),
                            source: Some("pypi".to_string()),
                            language: Language::Python,
                        });
                    }
                }
                
                // Dev dependencies
                if let Some(dev_packages) = parsed.get("dev-packages").and_then(|p| p.as_table()) {
                    for (name, value) in dev_packages {
                        let version = extract_version_from_toml_value(value);
                        deps.push(DependencyInfo {
                            name: name.clone(),
                            version,
                            dep_type: DependencyType::Dev,
                            license: detect_pypi_license(name).unwrap_or_else(|| "Unknown".to_string()),
                            source: Some("pypi".to_string()),
                            language: Language::Python,
                        });
                    }
                }
            }
        }
        
        // Try requirements.txt (legacy, but still widely used)
        let requirements_txt = project_root.join("requirements.txt");
        if requirements_txt.exists() && deps.is_empty() {
            debug!("Found requirements.txt, parsing legacy Python dependencies");
            let content = fs::read_to_string(&requirements_txt)?;
            for line in content.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') && !line.starts_with('-') {
                    let (name, version) = self.parse_python_requirement_spec(line);
                    deps.push(DependencyInfo {
                        name: name.clone(),
                        version,
                        dep_type: DependencyType::Production,
                        license: detect_pypi_license(&name).unwrap_or_else(|| "Unknown".to_string()),
                        source: Some("pypi".to_string()),
                        language: Language::Python,
                    });
                }
            }
        }
        
        // Try requirements-dev.txt
        let requirements_dev = project_root.join("requirements-dev.txt");
        if requirements_dev.exists() {
            debug!("Found requirements-dev.txt, parsing dev dependencies");
            let content = fs::read_to_string(&requirements_dev)?;
            for line in content.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') && !line.starts_with('-') {
                    let (name, version) = self.parse_python_requirement_spec(line);
                    deps.push(DependencyInfo {
                        name: name.clone(),
                        version,
                        dep_type: DependencyType::Dev,
                        license: detect_pypi_license(&name).unwrap_or_else(|| "Unknown".to_string()),
                        source: Some("pypi".to_string()),
                        language: Language::Python,
                    });
                }
            }
        }
        
        debug!("Parsed {} Python dependencies", deps.len());
        if !deps.is_empty() {
            debug!("Sample Python dependencies:");
            for dep in deps.iter().take(5) {
                debug!("  - {} v{} ({:?})", dep.name, dep.version, dep.dep_type);
            }
        }
        
        Ok(deps)
    }
    
    fn parse_go_deps(&self, project_root: &Path) -> Result<Vec<DependencyInfo>> {
        let go_mod = project_root.join("go.mod");
        let content = fs::read_to_string(&go_mod)?;
        let mut deps = Vec::new();
        let mut in_require_block = false;
        
        for line in content.lines() {
            let trimmed = line.trim();
            
            if trimmed.starts_with("require (") {
                in_require_block = true;
                continue;
            }
            
            if in_require_block && trimmed == ")" {
                in_require_block = false;
                continue;
            }
            
            if in_require_block || trimmed.starts_with("require ") {
                let parts: Vec<&str> = trimmed
                    .trim_start_matches("require ")
                    .split_whitespace()
                    .collect();
                
                if parts.len() >= 2 {
                    let name = parts[0];
                    let version = parts[1];
                    
                    deps.push(DependencyInfo {
                        name: name.to_string(),
                        version: version.to_string(),
                        dep_type: DependencyType::Production,
                        license: detect_go_license(name).unwrap_or("Unknown".to_string()),
                        source: Some("go modules".to_string()),
                        language: Language::Go,
                    });
                }
            }
        }
        
        Ok(deps)
    }
    
    /// Parse a Python requirement specification string (e.g., "package>=1.0.0")
    fn parse_python_requirement_spec(&self, spec: &str) -> (String, String) {
        // Handle requirement specification formats like:
        // - package==1.0.0
        // - package>=1.0.0,<2.0.0
        // - package~=1.0.0
        // - package[extra]>=1.0.0
        // - package
        
        let spec = spec.trim();
        
        // Remove any index URLs or other options
        let spec = if let Some(index) = spec.find("--") {
            &spec[..index]
        } else {
            spec
        }.trim();
        
        // Find the package name (before any version operators)
        let version_operators = ['=', '>', '<', '~', '!'];
        let version_start = spec.find(&version_operators[..]);
        
        if let Some(pos) = version_start {
            // Extract package name (including any extras)
            let package_part = spec[..pos].trim();
            let version_part = spec[pos..].trim();
            
            // Handle extras like package[extra] - keep them as part of the name
            let package_name = if package_part.contains('[') && package_part.contains(']') {
                // For packages with extras, extract just the base name
                if let Some(bracket_start) = package_part.find('[') {
                    package_part[..bracket_start].trim().to_string()
                } else {
                    package_part.to_string()
                }
            } else {
                package_part.to_string()
            };
            
            (package_name, version_part.to_string())
        } else {
            // No version specified - handle potential extras
            let package_name = if spec.contains('[') && spec.contains(']') {
                if let Some(bracket_start) = spec.find('[') {
                    spec[..bracket_start].trim().to_string()
                } else {
                    spec.to_string()
                }
            } else {
                spec.to_string()
            };
            
            (package_name, "*".to_string())
        }
    }
    
    fn parse_java_deps(&self, project_root: &Path) -> Result<Vec<DependencyInfo>> {
        let mut deps = Vec::new();
        
        debug!("Parsing Java dependencies in: {}", project_root.display());
        
        // Check for Maven pom.xml
        let pom_xml = project_root.join("pom.xml");
        if pom_xml.exists() {
            debug!("Found pom.xml, parsing Maven dependencies");
            let content = fs::read_to_string(&pom_xml)?;
            
            // Try to use the dependency:list Maven command first for accurate results
            if let Ok(maven_deps) = self.parse_maven_dependencies_with_command(project_root) {
                if !maven_deps.is_empty() {
                    debug!("Successfully parsed {} Maven dependencies using mvn command", maven_deps.len());
                    deps.extend(maven_deps);
                }
            }
            
            // If no deps from command, fall back to XML parsing
            if deps.is_empty() {
                debug!("Falling back to XML parsing for Maven dependencies");
                let xml_deps = self.parse_pom_xml(&content)?;
                debug!("Parsed {} dependencies from pom.xml", xml_deps.len());
                deps.extend(xml_deps);
            }
        }
        
        // Check for Gradle build.gradle or build.gradle.kts
        let build_gradle = project_root.join("build.gradle");
        let build_gradle_kts = project_root.join("build.gradle.kts");
        
        if (build_gradle.exists() || build_gradle_kts.exists()) && deps.is_empty() {
            debug!("Found Gradle build file, parsing Gradle dependencies");
            
                         // Try to use the dependencies Gradle command first
             if let Ok(gradle_deps) = self.parse_gradle_dependencies_with_command(project_root) {
                if !gradle_deps.is_empty() {
                    debug!("Successfully parsed {} Gradle dependencies using gradle command", gradle_deps.len());
                    deps.extend(gradle_deps);
                }
            }
            
            // If no deps from command, fall back to build file parsing
            if deps.is_empty() {
                if build_gradle.exists() {
                    debug!("Falling back to build.gradle parsing");
                    let content = fs::read_to_string(&build_gradle)?;
                    let gradle_deps = self.parse_gradle_build(&content)?;
                    debug!("Parsed {} dependencies from build.gradle", gradle_deps.len());
                    deps.extend(gradle_deps);
                }
                
                if build_gradle_kts.exists() && deps.is_empty() {
                    debug!("Falling back to build.gradle.kts parsing");
                    let content = fs::read_to_string(&build_gradle_kts)?;
                    let gradle_deps = self.parse_gradle_build(&content)?; // Same logic works for .kts
                    debug!("Parsed {} dependencies from build.gradle.kts", gradle_deps.len());
                    deps.extend(gradle_deps);
                }
            }
        }
        
        debug!("Total Java dependencies found: {}", deps.len());
        if !deps.is_empty() {
            debug!("Sample dependencies:");
            for dep in deps.iter().take(5) {
                debug!("  - {} v{}", dep.name, dep.version);
            }
        }
        
        Ok(deps)
    }
    
    /// Parse Maven dependencies using mvn dependency:list command
    fn parse_maven_dependencies_with_command(&self, project_root: &Path) -> Result<Vec<DependencyInfo>> {
        use std::process::Command;
        
        let output = Command::new("mvn")
            .args(&["dependency:list", "-DoutputFile=deps.txt", "-DappendOutput=false", "-DincludeScope=compile"])
            .current_dir(project_root)
            .output();
            
        match output {
            Ok(result) if result.status.success() => {
                // Read the generated deps.txt file
                let deps_file = project_root.join("deps.txt");
                if deps_file.exists() {
                    let content = fs::read_to_string(&deps_file)?;
                    let deps = self.parse_maven_dependency_list(&content)?;
                    
                    // Clean up
                    let _ = fs::remove_file(&deps_file);
                    
                    return Ok(deps);
                }
            }
            _ => {
                debug!("Maven command failed or not available, falling back to XML parsing");
            }
        }
        
        Ok(vec![])
    }
    
    /// Parse Gradle dependencies using gradle dependencies command
    fn parse_gradle_dependencies_with_command(&self, project_root: &Path) -> Result<Vec<DependencyInfo>> {
        use std::process::Command;
        
        // Try gradle first, then gradlew
        let gradle_cmds = vec!["gradle", "./gradlew"];
        
        for gradle_cmd in gradle_cmds {
            let output = Command::new(gradle_cmd)
                .args(&["dependencies", "--configuration=runtimeClasspath", "--console=plain"])
                .current_dir(project_root)
                .output();
                
            match output {
                Ok(result) if result.status.success() => {
                    let output_str = String::from_utf8_lossy(&result.stdout);
                    let deps = self.parse_gradle_dependency_tree(&output_str)?;
                    if !deps.is_empty() {
                        return Ok(deps);
                    }
                }
                _ => {
                    debug!("Gradle command '{}' failed, trying next", gradle_cmd);
                    continue;
                }
            }
        }
        
        debug!("All Gradle commands failed, falling back to build file parsing");
        Ok(vec![])
    }
    
    /// Parse Maven dependency list output
    fn parse_maven_dependency_list(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        let mut deps = Vec::new();
        
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("The following") || trimmed.starts_with("---") {
                continue;
            }
            
            // Format: groupId:artifactId:type:version:scope
            let parts: Vec<&str> = trimmed.split(':').collect();
            if parts.len() >= 4 {
                let group_id = parts[0];
                let artifact_id = parts[1];
                let version = parts[3];
                let scope = if parts.len() > 4 { parts[4] } else { "compile" };
                
                let name = format!("{}:{}", group_id, artifact_id);
                let dep_type = match scope {
                    "test" | "provided" => DependencyType::Dev,
                    _ => DependencyType::Production,
                };
                
                deps.push(DependencyInfo {
                    name,
                    version: version.to_string(),
                    dep_type,
                    license: "Unknown".to_string(),
                    source: Some("maven".to_string()),
                    language: Language::Java,
                });
            }
        }
        
        Ok(deps)
    }
    
    /// Parse Gradle dependency tree output
    fn parse_gradle_dependency_tree(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        let mut deps = Vec::new();
        
        for line in content.lines() {
            let trimmed = line.trim();
            
            // Look for dependency lines that match pattern: +--- group:artifact:version
            if (trimmed.starts_with("+---") || trimmed.starts_with("\\---") || trimmed.starts_with("|")) 
                && trimmed.contains(':') {
                
                // Extract the dependency part
                let dep_part = if let Some(pos) = trimmed.find(' ') {
                    &trimmed[pos + 1..]
                } else {
                    trimmed
                };
                
                // Remove additional markers and get clean dependency string
                let clean_dep = dep_part
                    .replace(" (*)", "")
                    .replace(" (c)", "")
                    .replace(" (n)", "")
                    .replace("(*)", "")
                    .trim()
                    .to_string();
                
                let parts: Vec<&str> = clean_dep.split(':').collect();
                if parts.len() >= 3 {
                    let group_id = parts[0];
                    let artifact_id = parts[1];
                    let version = parts[2];
                    
                    let name = format!("{}:{}", group_id, artifact_id);
                    
                    deps.push(DependencyInfo {
                        name,
                        version: version.to_string(),
                        dep_type: DependencyType::Production,
                        license: "Unknown".to_string(),
                        source: Some("gradle".to_string()),
                        language: Language::Java,
                    });
                }
            }
        }
        
        Ok(deps)
    }
    
    /// Parse pom.xml file directly (fallback method)
    fn parse_pom_xml(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        let mut deps = Vec::new();
        let mut in_dependencies = false;
        let mut in_dependency = false;
        let mut current_group_id = String::new();
        let mut current_artifact_id = String::new();
        let mut current_version = String::new();
        let mut current_scope = String::new();
        
        for line in content.lines() {
            let trimmed = line.trim();
            
            if trimmed.contains("<dependencies>") {
                in_dependencies = true;
                continue;
            }
            
            if trimmed.contains("</dependencies>") {
                in_dependencies = false;
                continue;
            }
            
            if in_dependencies {
                if trimmed.contains("<dependency>") {
                    in_dependency = true;
                    current_group_id.clear();
                    current_artifact_id.clear();
                    current_version.clear();
                    current_scope.clear();
                    continue;
                }
                
                if trimmed.contains("</dependency>") && in_dependency {
                    in_dependency = false;
                    
                    if !current_group_id.is_empty() && !current_artifact_id.is_empty() {
                        let name = format!("{}:{}", current_group_id, current_artifact_id);
                        let version = if current_version.is_empty() { 
                            "unknown".to_string() 
                        } else { 
                            current_version.clone() 
                        };
                        
                        let dep_type = match current_scope.as_str() {
                            "test" | "provided" => DependencyType::Dev,
                            _ => DependencyType::Production,
                        };
                        
                        deps.push(DependencyInfo {
                            name,
                            version,
                            dep_type,
                            license: "Unknown".to_string(),
                            source: Some("maven".to_string()),
                            language: Language::Java,
                        });
                    }
                    continue;
                }
                
                if in_dependency {
                    if trimmed.contains("<groupId>") {
                        current_group_id = extract_xml_value(trimmed, "groupId").to_string();
                    } else if trimmed.contains("<artifactId>") {
                        current_artifact_id = extract_xml_value(trimmed, "artifactId").to_string();
                    } else if trimmed.contains("<version>") {
                        current_version = extract_xml_value(trimmed, "version").to_string();
                    } else if trimmed.contains("<scope>") {
                        current_scope = extract_xml_value(trimmed, "scope").to_string();
                    }
                }
            }
        }
        
        Ok(deps)
    }
    
    /// Parse Gradle build file directly (fallback method)
    fn parse_gradle_build(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        let mut deps = Vec::new();
        
        for line in content.lines() {
            let trimmed = line.trim();
            
            // Look for dependency declarations
            if (trimmed.starts_with("implementation ") || 
                trimmed.starts_with("compile ") ||
                trimmed.starts_with("api ") ||
                trimmed.starts_with("runtimeOnly ") ||
                trimmed.starts_with("testImplementation ") ||
                trimmed.starts_with("testCompile ")) {
                
                if let Some(dep_str) = extract_gradle_dependency(trimmed) {
                    let parts: Vec<&str> = dep_str.split(':').collect();
                    if parts.len() >= 3 {
                        let group_id = parts[0];
                        let artifact_id = parts[1];
                        let version = parts[2].trim_matches('"').trim_matches('\'');
                        
                        let name = format!("{}:{}", group_id, artifact_id);
                        let dep_type = if trimmed.starts_with("test") {
                            DependencyType::Dev
                        } else {
                            DependencyType::Production
                        };
                        
                        deps.push(DependencyInfo {
                            name,
                            version: version.to_string(),
                            dep_type,
                            license: "Unknown".to_string(),
                            source: Some("gradle".to_string()),
                            language: Language::Java,
                        });
                    }
                }
            }
        }
        
        Ok(deps)
    }
}

/// Parses project dependencies from various manifest files
pub fn parse_dependencies(
    project_root: &Path,
    languages: &[DetectedLanguage],
    _config: &AnalysisConfig,
) -> Result<DependencyMap> {
    let mut all_dependencies = DependencyMap::new();
    
    for language in languages {
        let deps = match language.name.as_str() {
            "Rust" => parse_rust_dependencies(project_root)?,
            "JavaScript" | "TypeScript" | "JavaScript/TypeScript" => parse_js_dependencies(project_root)?,
            "Python" => parse_python_dependencies(project_root)?,
            "Go" => parse_go_dependencies(project_root)?,
            "Java" | "Kotlin" | "Java/Kotlin" => parse_jvm_dependencies(project_root)?,
            _ => DependencyMap::new(),
        };
        all_dependencies.extend(deps);
    }
    
    Ok(all_dependencies)
}

/// Parse detailed dependencies with vulnerability and license information
pub async fn parse_detailed_dependencies(
    project_root: &Path,
    languages: &[DetectedLanguage],
    _config: &AnalysisConfig,
) -> Result<DependencyAnalysis> {
    let mut detailed_deps = DetailedDependencyMap::new();
    let mut license_summary = HashMap::new();
    
    // First, get all dependencies without vulnerabilities
    for language in languages {
        let deps = match language.name.as_str() {
            "Rust" => parse_rust_dependencies_detailed(project_root)?,
            "JavaScript" | "TypeScript" | "JavaScript/TypeScript" => parse_js_dependencies_detailed(project_root)?,
            "Python" => parse_python_dependencies_detailed(project_root)?,
            "Go" => parse_go_dependencies_detailed(project_root)?,
            "Java" | "Kotlin" | "Java/Kotlin" => parse_jvm_dependencies_detailed(project_root)?,
            _ => DetailedDependencyMap::new(),
        };
        
        // Update license summary
        for (_, dep_info) in &deps {
            if let Some(license) = &dep_info.license {
                *license_summary.entry(license.clone()).or_insert(0) += 1;
            }
        }
        
        detailed_deps.extend(deps);
    }
    
    // Check vulnerabilities for all dependencies
    let parser = DependencyParser::new();
    let all_deps = parser.parse_all_dependencies(project_root)?;
    let vulnerability_map = parser.check_vulnerabilities_for_dependencies(&all_deps, project_root).await;
    
    // Update dependencies with vulnerability information
    for (dep_name, dep_info) in detailed_deps.iter_mut() {
        if let Some(vulns) = vulnerability_map.get(dep_name) {
            dep_info.vulnerabilities = vulns.iter()
                .map(|v| DependencyParser::convert_vulnerability_info(v))
                .collect();
        }
    }
    
    let total_count = detailed_deps.len();
    let production_count = detailed_deps.values().filter(|d| !d.is_dev).count();
    let dev_count = detailed_deps.values().filter(|d| d.is_dev).count();
    let vulnerable_count = detailed_deps.values().filter(|d| !d.vulnerabilities.is_empty()).count();
    
    Ok(DependencyAnalysis {
        dependencies: detailed_deps,
        total_count,
        production_count,
        dev_count,
        vulnerable_count,
        license_summary,
    })
}

/// Parse Rust dependencies from Cargo.toml
fn parse_rust_dependencies(project_root: &Path) -> Result<DependencyMap> {
    let cargo_toml = project_root.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(DependencyMap::new());
    }
    
    let content = fs::read_to_string(&cargo_toml)?;
    let parsed: toml::Value = toml::from_str(&content)
        .map_err(|e| AnalysisError::DependencyParsing {
            file: "Cargo.toml".to_string(),
            reason: e.to_string(),
        })?;
    
    let mut deps = DependencyMap::new();
    
    // Parse regular dependencies
    if let Some(dependencies) = parsed.get("dependencies").and_then(|d| d.as_table()) {
        for (name, value) in dependencies {
            let version = extract_version_from_toml_value(value);
            deps.insert(name.clone(), version);
        }
    }
    
    // Parse dev dependencies
    if let Some(dev_deps) = parsed.get("dev-dependencies").and_then(|d| d.as_table()) {
        for (name, value) in dev_deps {
            let version = extract_version_from_toml_value(value);
            deps.insert(format!("{} (dev)", name), version);
        }
    }
    
    Ok(deps)
}

/// Parse detailed Rust dependencies
fn parse_rust_dependencies_detailed(project_root: &Path) -> Result<DetailedDependencyMap> {
    let cargo_toml = project_root.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(DetailedDependencyMap::new());
    }
    
    let content = fs::read_to_string(&cargo_toml)?;
    let parsed: toml::Value = toml::from_str(&content)
        .map_err(|e| AnalysisError::DependencyParsing {
            file: "Cargo.toml".to_string(),
            reason: e.to_string(),
        })?;
    
    let mut deps = DetailedDependencyMap::new();
    
    // Parse regular dependencies
    if let Some(dependencies) = parsed.get("dependencies").and_then(|d| d.as_table()) {
        for (name, value) in dependencies {
            let version = extract_version_from_toml_value(value);
            deps.insert(name.clone(), LegacyDependencyInfo {
                version,
                is_dev: false,
                license: detect_rust_license(name),
                vulnerabilities: vec![], // Populated by vulnerability checker in parse_detailed_dependencies
                source: "crates.io".to_string(),
            });
        }
    }
    
    // Parse dev dependencies
    if let Some(dev_deps) = parsed.get("dev-dependencies").and_then(|d| d.as_table()) {
        for (name, value) in dev_deps {
            let version = extract_version_from_toml_value(value);
            deps.insert(name.clone(), LegacyDependencyInfo {
                version,
                is_dev: true,
                license: detect_rust_license(name),
                vulnerabilities: vec![], // Populated by vulnerability checker in parse_detailed_dependencies
                source: "crates.io".to_string(),
            });
        }
    }
    
    Ok(deps)
}

/// Parse JavaScript/Node.js dependencies from package.json
fn parse_js_dependencies(project_root: &Path) -> Result<DependencyMap> {
    let package_json = project_root.join("package.json");
    if !package_json.exists() {
        return Ok(DependencyMap::new());
    }
    
    let content = fs::read_to_string(&package_json)?;
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| AnalysisError::DependencyParsing {
            file: "package.json".to_string(),
            reason: e.to_string(),
        })?;
    
    let mut deps = DependencyMap::new();
    
    // Parse regular dependencies
    if let Some(dependencies) = parsed.get("dependencies").and_then(|d| d.as_object()) {
        for (name, version) in dependencies {
            if let Some(ver_str) = version.as_str() {
                deps.insert(name.clone(), ver_str.to_string());
            }
        }
    }
    
    // Parse dev dependencies
    if let Some(dev_deps) = parsed.get("devDependencies").and_then(|d| d.as_object()) {
        for (name, version) in dev_deps {
            if let Some(ver_str) = version.as_str() {
                deps.insert(format!("{} (dev)", name), ver_str.to_string());
            }
        }
    }
    
    Ok(deps)
}

/// Parse detailed JavaScript dependencies
fn parse_js_dependencies_detailed(project_root: &Path) -> Result<DetailedDependencyMap> {
    let package_json = project_root.join("package.json");
    if !package_json.exists() {
        return Ok(DetailedDependencyMap::new());
    }
    
    let content = fs::read_to_string(&package_json)?;
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| AnalysisError::DependencyParsing {
            file: "package.json".to_string(),
            reason: e.to_string(),
        })?;
    
    let mut deps = DetailedDependencyMap::new();
    
    // Parse regular dependencies
    if let Some(dependencies) = parsed.get("dependencies").and_then(|d| d.as_object()) {
        for (name, version) in dependencies {
            if let Some(ver_str) = version.as_str() {
                deps.insert(name.clone(), LegacyDependencyInfo {
                    version: ver_str.to_string(),
                    is_dev: false,
                    license: detect_npm_license(name),
                    vulnerabilities: vec![], // Populated by vulnerability checker in parse_detailed_dependencies
                    source: "npm".to_string(),
                });
            }
        }
    }
    
    // Parse dev dependencies
    if let Some(dev_deps) = parsed.get("devDependencies").and_then(|d| d.as_object()) {
        for (name, version) in dev_deps {
            if let Some(ver_str) = version.as_str() {
                deps.insert(name.clone(), LegacyDependencyInfo {
                    version: ver_str.to_string(),
                    is_dev: true,
                    license: detect_npm_license(name),
                    vulnerabilities: vec![], // Populated by vulnerability checker in parse_detailed_dependencies
                    source: "npm".to_string(),
                });
            }
        }
    }
    
    Ok(deps)
}

/// Parse Python dependencies from requirements.txt, Pipfile, or pyproject.toml
fn parse_python_dependencies(project_root: &Path) -> Result<DependencyMap> {
    let mut deps = DependencyMap::new();
    
    // Try requirements.txt first
    let requirements_txt = project_root.join("requirements.txt");
    if requirements_txt.exists() {
        let content = fs::read_to_string(&requirements_txt)?;
        for line in content.lines() {
            if !line.trim().is_empty() && !line.starts_with('#') {
                let parts: Vec<&str> = line.split(&['=', '>', '<', '~', '!'][..]).collect();
                if !parts.is_empty() {
                    let name = parts[0].trim();
                    let version = if parts.len() > 1 {
                        line[name.len()..].trim().to_string()
                    } else {
                        "*".to_string()
                    };
                    deps.insert(name.to_string(), version);
                }
            }
        }
    }
    
    // Try pyproject.toml
    let pyproject = project_root.join("pyproject.toml");
    if pyproject.exists() {
        let content = fs::read_to_string(&pyproject)?;
        if let Ok(parsed) = toml::from_str::<toml::Value>(&content) {
            // Poetry dependencies
            if let Some(poetry_deps) = parsed
                .get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("dependencies"))
                .and_then(|d| d.as_table())
            {
                for (name, value) in poetry_deps {
                    if name != "python" {
                        let version = extract_version_from_toml_value(value);
                        deps.insert(name.clone(), version);
                    }
                }
            }
            
            // Poetry dev dependencies
            if let Some(poetry_dev_deps) = parsed
                .get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("dev-dependencies"))
                .and_then(|d| d.as_table())
            {
                for (name, value) in poetry_dev_deps {
                    let version = extract_version_from_toml_value(value);
                    deps.insert(format!("{} (dev)", name), version);
                }
            }
            
            // PEP 621 dependencies
            if let Some(project_deps) = parsed
                .get("project")
                .and_then(|p| p.get("dependencies"))
                .and_then(|d| d.as_array())
            {
                for dep in project_deps {
                    if let Some(dep_str) = dep.as_str() {
                        let parts: Vec<&str> = dep_str.split(&['=', '>', '<', '~', '!'][..]).collect();
                        if !parts.is_empty() {
                            let name = parts[0].trim();
                            let version = if parts.len() > 1 {
                                dep_str[name.len()..].trim().to_string()
                            } else {
                                "*".to_string()
                            };
                            deps.insert(name.to_string(), version);
                        }
                    }
                }
            }
        }
    }
    
    Ok(deps)
}

/// Parse detailed Python dependencies
fn parse_python_dependencies_detailed(project_root: &Path) -> Result<DetailedDependencyMap> {
    let mut deps = DetailedDependencyMap::new();
    
    // Try requirements.txt first
    let requirements_txt = project_root.join("requirements.txt");
    if requirements_txt.exists() {
        let content = fs::read_to_string(&requirements_txt)?;
        for line in content.lines() {
            if !line.trim().is_empty() && !line.starts_with('#') {
                let parts: Vec<&str> = line.split(&['=', '>', '<', '~', '!'][..]).collect();
                if !parts.is_empty() {
                    let name = parts[0].trim();
                    let version = if parts.len() > 1 {
                        line[name.len()..].trim().to_string()
                    } else {
                        "*".to_string()
                    };
                    deps.insert(name.to_string(), LegacyDependencyInfo {
                        version,
                        is_dev: false,
                        license: detect_pypi_license(name),
                        vulnerabilities: vec![], // Populated by vulnerability checker in parse_detailed_dependencies
                        source: "pypi".to_string(),
                    });
                }
            }
        }
    }
    
    // Try pyproject.toml for more detailed info
    let pyproject = project_root.join("pyproject.toml");
    if pyproject.exists() {
        let content = fs::read_to_string(&pyproject)?;
        if let Ok(parsed) = toml::from_str::<toml::Value>(&content) {
            // Poetry dependencies
            if let Some(poetry_deps) = parsed
                .get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("dependencies"))
                .and_then(|d| d.as_table())
            {
                for (name, value) in poetry_deps {
                    if name != "python" {
                        let version = extract_version_from_toml_value(value);
                        deps.insert(name.clone(), LegacyDependencyInfo {
                            version,
                            is_dev: false,
                            license: detect_pypi_license(name),
                            vulnerabilities: vec![],
                            source: "pypi".to_string(),
                        });
                    }
                }
            }
            
            // Poetry dev dependencies
            if let Some(poetry_dev_deps) = parsed
                .get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("dev-dependencies"))
                .and_then(|d| d.as_table())
            {
                for (name, value) in poetry_dev_deps {
                    let version = extract_version_from_toml_value(value);
                    deps.insert(name.clone(), LegacyDependencyInfo {
                        version,
                        is_dev: true,
                        license: detect_pypi_license(name),
                        vulnerabilities: vec![],
                        source: "pypi".to_string(),
                    });
                }
            }
        }
    }
    
    Ok(deps)
}

/// Parse Go dependencies from go.mod
fn parse_go_dependencies(project_root: &Path) -> Result<DependencyMap> {
    let go_mod = project_root.join("go.mod");
    if !go_mod.exists() {
        return Ok(DependencyMap::new());
    }
    
    let content = fs::read_to_string(&go_mod)?;
    let mut deps = DependencyMap::new();
    let mut in_require_block = false;
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        if trimmed.starts_with("require (") {
            in_require_block = true;
            continue;
        }
        
        if in_require_block && trimmed == ")" {
            in_require_block = false;
            continue;
        }
        
        if in_require_block || trimmed.starts_with("require ") {
            let parts: Vec<&str> = trimmed
                .trim_start_matches("require ")
                .split_whitespace()
                .collect();
            
            if parts.len() >= 2 {
                let name = parts[0];
                let version = parts[1];
                deps.insert(name.to_string(), version.to_string());
            }
        }
    }
    
    Ok(deps)
}

/// Parse detailed Go dependencies
fn parse_go_dependencies_detailed(project_root: &Path) -> Result<DetailedDependencyMap> {
    let go_mod = project_root.join("go.mod");
    if !go_mod.exists() {
        return Ok(DetailedDependencyMap::new());
    }
    
    let content = fs::read_to_string(&go_mod)?;
    let mut deps = DetailedDependencyMap::new();
    let mut in_require_block = false;
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        if trimmed.starts_with("require (") {
            in_require_block = true;
            continue;
        }
        
        if in_require_block && trimmed == ")" {
            in_require_block = false;
            continue;
        }
        
        if in_require_block || trimmed.starts_with("require ") {
            let parts: Vec<&str> = trimmed
                .trim_start_matches("require ")
                .split_whitespace()
                .collect();
            
            if parts.len() >= 2 {
                let name = parts[0];
                let version = parts[1];
                let is_indirect = parts.len() > 2 && parts.contains(&"//") && parts.contains(&"indirect");
                
                deps.insert(name.to_string(), LegacyDependencyInfo {
                    version: version.to_string(),
                    is_dev: is_indirect,
                    license: detect_go_license(name),
                    vulnerabilities: vec![], // Populated by vulnerability checker in parse_detailed_dependencies
                    source: "go modules".to_string(),
                });
            }
        }
    }
    
    Ok(deps)
}

/// Parse JVM dependencies from pom.xml or build.gradle
fn parse_jvm_dependencies(project_root: &Path) -> Result<DependencyMap> {
    let mut deps = DependencyMap::new();
    
    // Try pom.xml (Maven)
    let pom_xml = project_root.join("pom.xml");
    if pom_xml.exists() {
        // Simple XML parsing for demonstration
        // In production, use a proper XML parser
        let content = fs::read_to_string(&pom_xml)?;
        let lines: Vec<&str> = content.lines().collect();
        
        for i in 0..lines.len() {
            if lines[i].contains("<dependency>") {
                let mut group_id = "";
                let mut artifact_id = "";
                let mut version = "";
                
                for j in i..lines.len() {
                    if lines[j].contains("</dependency>") {
                        break;
                    }
                    if lines[j].contains("<groupId>") {
                        group_id = extract_xml_value(lines[j], "groupId");
                    }
                    if lines[j].contains("<artifactId>") {
                        artifact_id = extract_xml_value(lines[j], "artifactId");
                    }
                    if lines[j].contains("<version>") {
                        version = extract_xml_value(lines[j], "version");
                    }
                }
                
                if !group_id.is_empty() && !artifact_id.is_empty() {
                    let name = format!("{}:{}", group_id, artifact_id);
                    deps.insert(name, version.to_string());
                }
            }
        }
    }
    
    // Try build.gradle (Gradle)
    let build_gradle = project_root.join("build.gradle");
    if build_gradle.exists() {
        let content = fs::read_to_string(&build_gradle)?;
        
        // Simple pattern matching for Gradle dependencies
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("implementation") || 
               trimmed.starts_with("compile") ||
               trimmed.starts_with("testImplementation") ||
               trimmed.starts_with("testCompile") {
                
                if let Some(dep_str) = extract_gradle_dependency(trimmed) {
                    let parts: Vec<&str> = dep_str.split(':').collect();
                    if parts.len() >= 3 {
                        let name = format!("{}:{}", parts[0], parts[1]);
                        let version = parts[2];
                        let is_test = trimmed.starts_with("test");
                        let key = if is_test { format!("{} (test)", name) } else { name };
                        deps.insert(key, version.to_string());
                    }
                }
            }
        }
    }
    
    Ok(deps)
}

/// Parse detailed JVM dependencies
fn parse_jvm_dependencies_detailed(project_root: &Path) -> Result<DetailedDependencyMap> {
    let mut deps = DetailedDependencyMap::new();
    
    // Try pom.xml (Maven)
    let pom_xml = project_root.join("pom.xml");
    if pom_xml.exists() {
        let content = fs::read_to_string(&pom_xml)?;
        let lines: Vec<&str> = content.lines().collect();
        
        for i in 0..lines.len() {
            if lines[i].contains("<dependency>") {
                let mut group_id = "";
                let mut artifact_id = "";
                let mut version = "";
                let mut scope = "compile";
                
                for j in i..lines.len() {
                    if lines[j].contains("</dependency>") {
                        break;
                    }
                    if lines[j].contains("<groupId>") {
                        group_id = extract_xml_value(lines[j], "groupId");
                    }
                    if lines[j].contains("<artifactId>") {
                        artifact_id = extract_xml_value(lines[j], "artifactId");
                    }
                    if lines[j].contains("<version>") {
                        version = extract_xml_value(lines[j], "version");
                    }
                    if lines[j].contains("<scope>") {
                        scope = extract_xml_value(lines[j], "scope");
                    }
                }
                
                if !group_id.is_empty() && !artifact_id.is_empty() {
                    let name = format!("{}:{}", group_id, artifact_id);
                    deps.insert(name.clone(), LegacyDependencyInfo {
                        version: version.to_string(),
                        is_dev: scope == "test" || scope == "provided",
                        license: detect_maven_license(&name),
                        vulnerabilities: vec![], // Populated by vulnerability checker in parse_detailed_dependencies
                        source: "maven".to_string(),
                    });
                }
            }
        }
    }
    
    // Try build.gradle (Gradle)
    let build_gradle = project_root.join("build.gradle");
    if build_gradle.exists() {
        let content = fs::read_to_string(&build_gradle)?;
        
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("implementation") || 
               trimmed.starts_with("compile") ||
               trimmed.starts_with("testImplementation") ||
               trimmed.starts_with("testCompile") {
                
                if let Some(dep_str) = extract_gradle_dependency(trimmed) {
                    let parts: Vec<&str> = dep_str.split(':').collect();
                    if parts.len() >= 3 {
                        let name = format!("{}:{}", parts[0], parts[1]);
                        let version = parts[2];
                        let is_test = trimmed.starts_with("test");
                        
                        deps.insert(name.clone(), LegacyDependencyInfo {
                            version: version.to_string(),
                            is_dev: is_test,
                            license: detect_maven_license(&name),
                            vulnerabilities: vec![],
                            source: "gradle".to_string(),
                        });
                    }
                }
            }
        }
    }
    
    Ok(deps)
}

// Helper functions

fn extract_version_from_toml_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Table(t) => {
            t.get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("*")
                .to_string()
        }
        _ => "*".to_string(),
    }
}

fn extract_xml_value<'a>(line: &'a str, tag: &str) -> &'a str {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);
    
    if let Some(start) = line.find(&start_tag) {
        if let Some(end) = line.find(&end_tag) {
            return &line[start + start_tag.len()..end];
        }
    }
    ""
}

fn extract_gradle_dependency(line: &str) -> Option<&str> {
    // Handle various Gradle dependency formats
    if let Some(start) = line.find('\'') {
        if let Some(end) = line.rfind('\'') {
            if start < end {
                return Some(&line[start + 1..end]);
            }
        }
    }
    if let Some(start) = line.find('"') {
        if let Some(end) = line.rfind('"') {
            if start < end {
                return Some(&line[start + 1..end]);
            }
        }
    }
    None
}

// License detection helpers (simplified - in production, use a proper license database)

fn detect_rust_license(crate_name: &str) -> Option<String> {
    // Common Rust crates and their licenses
    match crate_name {
        "serde" | "serde_json" | "tokio" | "clap" => Some("MIT OR Apache-2.0".to_string()),
        "actix-web" => Some("MIT OR Apache-2.0".to_string()),
        _ => Some("Unknown".to_string()),
    }
}

fn detect_npm_license(package_name: &str) -> Option<String> {
    // Common npm packages and their licenses
    match package_name {
        "react" | "vue" | "angular" => Some("MIT".to_string()),
        "express" => Some("MIT".to_string()),
        "webpack" => Some("MIT".to_string()),
        _ => Some("Unknown".to_string()),
    }
}

fn detect_pypi_license(package_name: &str) -> Option<String> {
    // Common Python packages and their licenses
    match package_name {
        "django" => Some("BSD-3-Clause".to_string()),
        "flask" => Some("BSD-3-Clause".to_string()),
        "requests" => Some("Apache-2.0".to_string()),
        "numpy" | "pandas" => Some("BSD-3-Clause".to_string()),
        _ => Some("Unknown".to_string()),
    }
}

fn detect_go_license(module_name: &str) -> Option<String> {
    // Common Go modules and their licenses
    if module_name.starts_with("github.com/gin-gonic/") {
        Some("MIT".to_string())
    } else if module_name.starts_with("github.com/gorilla/") {
        Some("BSD-3-Clause".to_string())
    } else {
        Some("Unknown".to_string())
    }
}

fn detect_maven_license(artifact: &str) -> Option<String> {
    // Common Maven artifacts and their licenses
    if artifact.starts_with("org.springframework") {
        Some("Apache-2.0".to_string())
    } else if artifact.starts_with("junit:junit") {
        Some("EPL-1.0".to_string())
    } else {
        Some("Unknown".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_parse_rust_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
assert_cmd = "2.0"
"#;
        
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();
        
        let deps = parse_rust_dependencies(temp_dir.path()).unwrap();
        assert_eq!(deps.get("serde"), Some(&"1.0".to_string()));
        assert_eq!(deps.get("tokio"), Some(&"1.0".to_string()));
        assert_eq!(deps.get("assert_cmd (dev)"), Some(&"2.0".to_string()));
    }
    
    #[test]
    fn test_parse_js_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = r#"{
  "name": "test",
  "version": "1.0.0",
  "dependencies": {
    "express": "^4.18.0",
    "react": "^18.0.0"
  },
  "devDependencies": {
    "jest": "^29.0.0"
  }
}"#;
        
        fs::write(temp_dir.path().join("package.json"), package_json).unwrap();
        
        let deps = parse_js_dependencies(temp_dir.path()).unwrap();
        assert_eq!(deps.get("express"), Some(&"^4.18.0".to_string()));
        assert_eq!(deps.get("react"), Some(&"^18.0.0".to_string()));
        assert_eq!(deps.get("jest (dev)"), Some(&"^29.0.0".to_string()));
    }
    
    #[test]
    fn test_vulnerability_severity() {
        let vuln = Vulnerability {
            id: "CVE-2023-1234".to_string(),
            severity: VulnerabilitySeverity::High,
            description: "Test vulnerability".to_string(),
            fixed_in: Some("1.0.1".to_string()),
        };
        
        assert!(matches!(vuln.severity, VulnerabilitySeverity::High));
    }
    
    #[test]
    fn test_parse_python_requirement_spec() {
        let parser = DependencyParser::new();
        
        // Test basic package name
        let (name, version) = parser.parse_python_requirement_spec("requests");
        assert_eq!(name, "requests");
        assert_eq!(version, "*");
        
        // Test package with exact version
        let (name, version) = parser.parse_python_requirement_spec("requests==2.28.0");
        assert_eq!(name, "requests");
        assert_eq!(version, "==2.28.0");
        
        // Test package with version constraint
        let (name, version) = parser.parse_python_requirement_spec("requests>=2.25.0,<3.0.0");
        assert_eq!(name, "requests");
        assert_eq!(version, ">=2.25.0,<3.0.0");
        
        // Test package with extras
        let (name, version) = parser.parse_python_requirement_spec("fastapi[all]>=0.95.0");
        assert_eq!(name, "fastapi");
        assert_eq!(version, ">=0.95.0");
        
        // Test package with tilde operator
        let (name, version) = parser.parse_python_requirement_spec("django~=4.1.0");
        assert_eq!(name, "django");
        assert_eq!(version, "~=4.1.0");
    }

    #[test]
    fn test_parse_pyproject_toml_poetry() {
        use std::fs;
        use tempfile::tempdir;
        
        let dir = tempdir().unwrap();
        let pyproject_path = dir.path().join("pyproject.toml");
        
        let pyproject_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.9"
fastapi = "^0.95.0"
uvicorn = {extras = ["standard"], version = "^0.21.0"}

[tool.poetry.group.dev.dependencies]
pytest = "^7.0.0"
black = "^23.0.0"
"#;
        
        fs::write(&pyproject_path, pyproject_content).unwrap();
        
        let parser = DependencyParser::new();
        let deps = parser.parse_python_deps(dir.path()).unwrap();
        
        assert!(!deps.is_empty());
        
        // Check that we found FastAPI and Uvicorn as production dependencies
        let fastapi = deps.iter().find(|d| d.name == "fastapi");
        assert!(fastapi.is_some());
        assert!(matches!(fastapi.unwrap().dep_type, DependencyType::Production));
        
        let uvicorn = deps.iter().find(|d| d.name == "uvicorn");
        assert!(uvicorn.is_some());
        assert!(matches!(uvicorn.unwrap().dep_type, DependencyType::Production));
        
        // Check that we found pytest and black as dev dependencies
        let pytest = deps.iter().find(|d| d.name == "pytest");
        assert!(pytest.is_some());
        assert!(matches!(pytest.unwrap().dep_type, DependencyType::Dev));
        
        let black = deps.iter().find(|d| d.name == "black");
        assert!(black.is_some());
        assert!(matches!(black.unwrap().dep_type, DependencyType::Dev));
        
        // Make sure we didn't include python as a dependency
        assert!(deps.iter().find(|d| d.name == "python").is_none());
    }

    #[test]
    fn test_parse_pyproject_toml_pep621() {
        use std::fs;
        use tempfile::tempdir;
        
        let dir = tempdir().unwrap();
        let pyproject_path = dir.path().join("pyproject.toml");
        
        let pyproject_content = r#"
[project]
name = "test-project"
version = "0.1.0"
dependencies = [
    "fastapi>=0.95.0",
    "uvicorn[standard]>=0.21.0",
    "pydantic>=1.10.0"
]

[project.optional-dependencies]
test = [
    "pytest>=7.0.0",
    "pytest-cov>=4.0.0"
]
dev = [
    "black>=23.0.0",
    "mypy>=1.0.0"
]
"#;
        
        fs::write(&pyproject_path, pyproject_content).unwrap();
        
        let parser = DependencyParser::new();
        let deps = parser.parse_python_deps(dir.path()).unwrap();
        
        assert!(!deps.is_empty());
        
        // Check production dependencies
        let prod_deps: Vec<_> = deps.iter().filter(|d| matches!(d.dep_type, DependencyType::Production)).collect();
        assert_eq!(prod_deps.len(), 3);
        assert!(prod_deps.iter().any(|d| d.name == "fastapi"));
        assert!(prod_deps.iter().any(|d| d.name == "uvicorn"));
        assert!(prod_deps.iter().any(|d| d.name == "pydantic"));
        
        // Check dev/test dependencies
        let dev_deps: Vec<_> = deps.iter().filter(|d| matches!(d.dep_type, DependencyType::Dev)).collect();
        assert!(dev_deps.iter().any(|d| d.name == "pytest"));
        assert!(dev_deps.iter().any(|d| d.name == "black"));
        assert!(dev_deps.iter().any(|d| d.name == "mypy"));
        
        // Check optional dependencies (test group is treated as dev)
        let test_deps: Vec<_> = deps.iter().filter(|d| d.name == "pytest-cov").collect();
        assert_eq!(test_deps.len(), 1);
        assert!(matches!(test_deps[0].dep_type, DependencyType::Dev));
    }

    #[test]
    fn test_parse_pipfile() {
        use std::fs;
        use tempfile::tempdir;
        
        let dir = tempdir().unwrap();
        let pipfile_path = dir.path().join("Pipfile");
        
        let pipfile_content = r#"
[[source]]
url = "https://pypi.org/simple"
verify_ssl = true
name = "pypi"

[packages]
django = "~=4.1.0"
django-rest-framework = "*"
psycopg2 = ">=2.9.0"

[dev-packages]
pytest = "*"
flake8 = "*"
black = ">=22.0.0"
"#;
        
        fs::write(&pipfile_path, pipfile_content).unwrap();
        
        let parser = DependencyParser::new();
        let deps = parser.parse_python_deps(dir.path()).unwrap();
        
        assert!(!deps.is_empty());
        
        // Check production dependencies
        let prod_deps: Vec<_> = deps.iter().filter(|d| matches!(d.dep_type, DependencyType::Production)).collect();
        assert_eq!(prod_deps.len(), 3);
        assert!(prod_deps.iter().any(|d| d.name == "django"));
        assert!(prod_deps.iter().any(|d| d.name == "django-rest-framework"));
        assert!(prod_deps.iter().any(|d| d.name == "psycopg2"));
        
        // Check dev dependencies
        let dev_deps: Vec<_> = deps.iter().filter(|d| matches!(d.dep_type, DependencyType::Dev)).collect();
        assert_eq!(dev_deps.len(), 3);
        assert!(dev_deps.iter().any(|d| d.name == "pytest"));
        assert!(dev_deps.iter().any(|d| d.name == "flake8"));
        assert!(dev_deps.iter().any(|d| d.name == "black"));
    }

    #[test]
    fn test_dependency_analysis_summary() {
        let mut deps = DetailedDependencyMap::new();
        deps.insert("prod-dep".to_string(), LegacyDependencyInfo {
            version: "1.0.0".to_string(),
            is_dev: false,
            license: Some("MIT".to_string()),
            vulnerabilities: vec![],
            source: "npm".to_string(),
        });
        deps.insert("dev-dep".to_string(), LegacyDependencyInfo {
            version: "2.0.0".to_string(),
            is_dev: true,
            license: Some("MIT".to_string()),
            vulnerabilities: vec![],
            source: "npm".to_string(),
        });
        
        let analysis = DependencyAnalysis {
            dependencies: deps,
            total_count: 2,
            production_count: 1,
            dev_count: 1,
            vulnerable_count: 0,
            license_summary: {
                let mut map = HashMap::new();
                map.insert("MIT".to_string(), 2);
                map
            },
        };
        
        assert_eq!(analysis.total_count, 2);
        assert_eq!(analysis.production_count, 1);
        assert_eq!(analysis.dev_count, 1);
        assert_eq!(analysis.license_summary.get("MIT"), Some(&2));
    }
} 