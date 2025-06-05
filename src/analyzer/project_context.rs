#[allow(unused_imports)]
use crate::analyzer::{AnalysisConfig, DetectedTechnology, DetectedLanguage, EntryPoint, EnvVar, Port, Protocol, ProjectType, BuildScript, TechnologyCategory, LibraryType};
use crate::error::{Result, AnalysisError};
use crate::common::file_utils::{read_file_safe, is_readable_file};
use std::path::{Path, PathBuf};
use std::collections::{HashSet, HashMap};
use regex::Regex;
use serde_json::Value;

/// Project context information
pub struct ProjectContext {
    pub entry_points: Vec<EntryPoint>,
    pub ports: Vec<Port>,
    pub environment_variables: Vec<EnvVar>,
    pub project_type: ProjectType,
    pub build_scripts: Vec<BuildScript>,
}

/// Helper function to create a regex with proper error handling
fn create_regex(pattern: &str) -> Result<Regex> {
    Regex::new(pattern)
        .map_err(|e| AnalysisError::InvalidStructure(format!("Invalid regex pattern '{}': {}", pattern, e)).into())
}

/// Analyzes project context including entry points, ports, and environment variables
pub fn analyze_context(
    project_root: &Path,
    languages: &[DetectedLanguage],
    technologies: &[DetectedTechnology],
    config: &AnalysisConfig,
) -> Result<ProjectContext> {
    log::info!("Analyzing project context");
    
    let mut entry_points = Vec::new();
    let mut ports = HashSet::new();
    let mut env_vars = HashMap::new();
    let mut build_scripts = Vec::new();
    
    // Analyze based on detected languages
    for language in languages {
        match language.name.as_str() {
            "JavaScript" | "TypeScript" => {
                analyze_node_project(project_root, &mut entry_points, &mut ports, &mut env_vars, &mut build_scripts, config)?;
            }
            "Python" => {
                analyze_python_project(project_root, &mut entry_points, &mut ports, &mut env_vars, &mut build_scripts, config)?;
            }
            "Rust" => {
                analyze_rust_project(project_root, &mut entry_points, &mut ports, &mut env_vars, &mut build_scripts, config)?;
            }
            "Go" => {
                analyze_go_project(project_root, &mut entry_points, &mut ports, &mut env_vars, &mut build_scripts, config)?;
            }
            "Java" | "Kotlin" => {
                analyze_jvm_project(project_root, &mut entry_points, &mut ports, &mut env_vars, &mut build_scripts, config)?;
            }
            _ => {}
        }
    }
    
    // Analyze common configuration files
    analyze_docker_files(project_root, &mut ports, &mut env_vars)?;
    analyze_env_files(project_root, &mut env_vars)?;
    analyze_makefile(project_root, &mut build_scripts)?;
    
    // Technology-specific analysis
    for technology in technologies {
        analyze_technology_specifics(technology, project_root, &mut entry_points, &mut ports)?;
    }
    
    // Detect microservices structure
    let microservices = detect_microservices_structure(project_root)?;
    
    // Determine project type
    let ports_vec: Vec<Port> = ports.iter().cloned().collect();
    let project_type = determine_project_type_with_structure(
        languages, 
        technologies, 
        &entry_points, 
        &ports_vec, 
        &microservices
    );
    
    // Convert collections to vectors
    let ports: Vec<Port> = ports.into_iter().collect();
    let environment_variables: Vec<EnvVar> = env_vars.into_iter()
        .map(|(name, (default, required, desc))| EnvVar {
            name,
            default_value: default,
            required,
            description: desc,
        })
        .collect();
    
    Ok(ProjectContext {
        entry_points,
        ports,
        environment_variables,
        project_type,
        build_scripts,
    })
}

/// Represents a detected microservice within the project
#[derive(Debug)]
struct MicroserviceInfo {
    name: String,
    has_db: bool,
    has_api: bool,
}

/// Detects microservice structure based on directory patterns
fn detect_microservices_structure(project_root: &Path) -> Result<Vec<MicroserviceInfo>> {
    let mut microservices = Vec::new();
    
    // Common patterns for microservice directories
    let service_indicators = ["api", "service", "encore.service.ts", "main.ts", "main.go", "main.py"];
    let db_indicators = ["db", "database", "migrations", "schema", "models"];
    
    // Check root-level directories
    if let Ok(entries) = std::fs::read_dir(project_root) {
        for entry in entries.flatten() {
            if entry.file_type()?.is_dir() {
                let dir_name = entry.file_name().to_string_lossy().to_string();
                let dir_path = entry.path();
                
                // Skip common non-service directories
                if dir_name.starts_with('.') || 
                   ["node_modules", "target", "dist", "build", "__pycache__", "vendor"].contains(&dir_name.as_str()) {
                    continue;
                }
                
                // Check if this directory looks like a service
                let mut has_api = false;
                let mut has_db = false;
                
                if let Ok(sub_entries) = std::fs::read_dir(&dir_path) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_name = sub_entry.file_name().to_string_lossy().to_string();
                        
                        // Check for API indicators
                        if service_indicators.iter().any(|&ind| sub_name.contains(ind)) {
                            has_api = true;
                        }
                        
                        // Check for DB indicators
                        if db_indicators.iter().any(|&ind| sub_name.contains(ind)) {
                            has_db = true;
                        }
                    }
                }
                
                // If it has service characteristics, add it as a microservice
                if has_api || has_db {
                    microservices.push(MicroserviceInfo {
                        name: dir_name,
                        has_db,
                        has_api,
                    });
                }
            }
        }
    }
    
    Ok(microservices)
}

/// Enhanced project type determination including microservice structure analysis
fn determine_project_type_with_structure(
    languages: &[DetectedLanguage],
    technologies: &[DetectedTechnology],
    entry_points: &[EntryPoint],
    ports: &[Port],
    microservices: &[MicroserviceInfo],
) -> ProjectType {
    // If we have multiple services with databases, it's likely a microservice architecture
    let services_with_db = microservices.iter().filter(|s| s.has_db).count();
    if services_with_db >= 2 || microservices.len() >= 3 {
        return ProjectType::Microservice;
    }
    
    // Fall back to original determination logic
    determine_project_type(languages, technologies, entry_points, ports)
}

/// Analyzes Node.js/JavaScript/TypeScript projects
fn analyze_node_project(
    root: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    build_scripts: &mut Vec<BuildScript>,
    config: &AnalysisConfig,
) -> Result<()> {
    let package_json_path = root.join("package.json");
    
    if is_readable_file(&package_json_path) {
        let content = read_file_safe(&package_json_path, config.max_file_size)?;
        let package_json: Value = serde_json::from_str(&content)?;
        
        // Extract scripts
        if let Some(scripts) = package_json.get("scripts").and_then(|s| s.as_object()) {
            for (name, command) in scripts {
                if let Some(cmd) = command.as_str() {
                    build_scripts.push(BuildScript {
                        name: name.clone(),
                        command: cmd.to_string(),
                        description: get_script_description(name),
                        is_default: name == "start" || name == "dev",
                    });
                    
                    // Look for ports in scripts
                    extract_ports_from_command(cmd, ports);
                }
            }
        }
        
        // Find main entry point
        if let Some(main) = package_json.get("main").and_then(|m| m.as_str()) {
            entry_points.push(EntryPoint {
                file: root.join(main),
                function: None,
                command: Some(format!("node {}", main)),
            });
        }
        
        // Check common entry files
        let common_entries = ["index.js", "index.ts", "app.js", "app.ts", "server.js", "server.ts", "main.js", "main.ts"];
        for entry in &common_entries {
            let path = root.join(entry);
            if is_readable_file(&path) {
                scan_js_file_for_context(&path, entry_points, ports, env_vars, config)?;
            }
        }
        
        // Check src directory
        let src_dir = root.join("src");
        if src_dir.is_dir() {
            for entry in &common_entries {
                let path = src_dir.join(entry);
                if is_readable_file(&path) {
                    scan_js_file_for_context(&path, entry_points, ports, env_vars, config)?;
                }
            }
        }
    }
    
    Ok(())
}

/// Scans JavaScript/TypeScript files for context information
fn scan_js_file_for_context(
    path: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    config: &AnalysisConfig,
) -> Result<()> {
    let content = read_file_safe(path, config.max_file_size)?;
    
    // Look for port assignments
    let port_regex = Regex::new(r"(?:PORT|port)\s*[=:]\s*(?:process\.env\.PORT\s*\|\|\s*)?(\d{1,5})")
        .map_err(|e| AnalysisError::InvalidStructure(format!("Invalid regex: {}", e)))?;
    for cap in port_regex.captures_iter(&content) {
        if let Some(port_str) = cap.get(1) {
            if let Ok(port) = port_str.as_str().parse::<u16>() {
                ports.insert(Port {
                    number: port,
                    protocol: Protocol::Http,
                    description: Some("HTTP server port".to_string()),
                });
            }
        }
    }
    
    // Look for app.listen() calls
    let listen_regex = Regex::new(r"\.listen\s*\(\s*(\d{1,5})")
        .map_err(|e| AnalysisError::InvalidStructure(format!("Invalid regex: {}", e)))?;
    for cap in listen_regex.captures_iter(&content) {
        if let Some(port_str) = cap.get(1) {
            if let Ok(port) = port_str.as_str().parse::<u16>() {
                ports.insert(Port {
                    number: port,
                    protocol: Protocol::Http,
                    description: Some("Express/HTTP server".to_string()),
                });
            }
        }
    }
    
    // Look for environment variable usage
    let env_regex = Regex::new(r"process\.env\.([A-Z_][A-Z0-9_]*)")
        .map_err(|e| AnalysisError::InvalidStructure(format!("Invalid regex: {}", e)))?;
    for cap in env_regex.captures_iter(&content) {
        if let Some(var_name) = cap.get(1) {
            let name = var_name.as_str().to_string();
            if !name.starts_with("NODE_") { // Skip Node.js internal vars
                env_vars.entry(name.clone()).or_insert((None, false, None));
            }
        }
    }
    
    // Look for Encore.dev imports and patterns
    if content.contains("encore.dev") {
        // Encore uses specific patterns for config and database
        let encore_patterns = [
            (r#"secret\s*\(\s*['"]([A-Z_][A-Z0-9_]*)['"]"#, "Encore secret configuration"),
            (r#"SQLDatabase\s*\(\s*['"](\w+)['"]"#, "Encore database"),
        ];
        
        for (pattern, description) in &encore_patterns {
            let regex = Regex::new(pattern).unwrap_or_else(|_| Regex::new(r"").unwrap());
            for cap in regex.captures_iter(&content) {
                if let Some(match_str) = cap.get(1) {
                    let name = match_str.as_str();
                    if pattern.contains("secret") {
                        env_vars.entry(name.to_string())
                            .or_insert((None, true, Some(description.to_string())));
                    }
                }
            }
        }
    }
    
    // Check if this is a main entry point
    if content.contains("createServer") || content.contains(".listen(") || 
       content.contains("app.listen") || content.contains("server.listen") ||
       content.contains("encore.dev") && content.contains("api.") {
        entry_points.push(EntryPoint {
            file: path.to_path_buf(),
            function: Some("main".to_string()),
            command: Some(format!("node {}", path.file_name().unwrap().to_string_lossy())),
        });
    }
    
    Ok(())
}

/// Analyzes Python projects
fn analyze_python_project(
    root: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    build_scripts: &mut Vec<BuildScript>,
    config: &AnalysisConfig,
) -> Result<()> {
    // Check for common Python entry points
    let common_entries = ["main.py", "app.py", "wsgi.py", "asgi.py", "manage.py", "run.py", "__main__.py"];
    
    for entry in &common_entries {
        let path = root.join(entry);
        if is_readable_file(&path) {
            scan_python_file_for_context(&path, entry_points, ports, env_vars, config)?;
        }
    }
    
    // Check setup.py for entry points
    let setup_py = root.join("setup.py");
    if is_readable_file(&setup_py) {
        let content = read_file_safe(&setup_py, config.max_file_size)?;
        
        // Look for console_scripts
        let console_regex = create_regex(r#"console_scripts['"]\s*:\s*\[(.*?)\]"#)?;
        if let Some(cap) = console_regex.captures(&content) {
            if let Some(scripts) = cap.get(1) {
                let script_regex = create_regex(r#"['"](\w+)\s*=\s*([\w\.]+):(\w+)"#)?;
                for script_cap in script_regex.captures_iter(scripts.as_str()) {
                    if let (Some(name), Some(module), Some(func)) = 
                        (script_cap.get(1), script_cap.get(2), script_cap.get(3)) {
                        entry_points.push(EntryPoint {
                            file: PathBuf::from(format!("{}.py", module.as_str().replace('.', "/"))),
                            function: Some(func.as_str().to_string()),
                            command: Some(name.as_str().to_string()),
                        });
                    }
                }
            }
        }
    }
    
    // Check pyproject.toml for scripts
    let pyproject = root.join("pyproject.toml");
    if is_readable_file(&pyproject) {
        let content = read_file_safe(&pyproject, config.max_file_size)?;
        if let Ok(toml_value) = toml::from_str::<toml::Value>(&content) {
            // Extract build scripts from poetry
            if let Some(scripts) = toml_value.get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("scripts"))
                .and_then(|s| s.as_table()) {
                for (name, cmd) in scripts {
                    if let Some(command) = cmd.as_str() {
                        build_scripts.push(BuildScript {
                            name: name.clone(),
                            command: command.to_string(),
                            description: None,
                            is_default: name == "start" || name == "run",
                        });
                    }
                }
            }
        }
    }
    
    // Common Python build commands
    build_scripts.push(BuildScript {
        name: "install".to_string(),
        command: "pip install -r requirements.txt".to_string(),
        description: Some("Install dependencies".to_string()),
        is_default: false,
    });
    
    Ok(())
}

/// Scans Python files for context information
fn scan_python_file_for_context(
    path: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    config: &AnalysisConfig,
) -> Result<()> {
    let content = read_file_safe(path, config.max_file_size)?;
    
    // Look for Flask/FastAPI/Django port configurations
    let port_patterns = [
        r"port\s*=\s*(\d{1,5})",
        r"PORT\s*=\s*(\d{1,5})",
        r"\.run\s*\([^)]*port\s*=\s*(\d{1,5})",
        r"uvicorn\.run\s*\([^)]*port\s*=\s*(\d{1,5})",
    ];
    
    for pattern in &port_patterns {
        let regex = create_regex(pattern)?;
        for cap in regex.captures_iter(&content) {
            if let Some(port_str) = cap.get(1) {
                if let Ok(port) = port_str.as_str().parse::<u16>() {
                    ports.insert(Port {
                        number: port,
                        protocol: Protocol::Http,
                        description: Some("Python web server".to_string()),
                    });
                }
            }
        }
    }
    
    // Look for environment variable usage
    let env_patterns = [
        r#"os\.environ\.get\s*\(\s*["']([A-Z_][A-Z0-9_]*)["']"#,
        r#"os\.environ\s*\[\s*["']([A-Z_][A-Z0-9_]*)["']\s*\]"#,
        r#"os\.getenv\s*\(\s*["']([A-Z_][A-Z0-9_]*)["']"#,
    ];
    
    for pattern in &env_patterns {
        let regex = create_regex(pattern)?;
        for cap in regex.captures_iter(&content) {
            if let Some(var_name) = cap.get(1) {
                let name = var_name.as_str().to_string();
                env_vars.entry(name.clone()).or_insert((None, false, None));
            }
        }
    }
    
    // Check if this is a main entry point
    if content.contains("if __name__ == '__main__':") ||
       content.contains("if __name__ == \"__main__\":") {
        entry_points.push(EntryPoint {
            file: path.to_path_buf(),
            function: Some("main".to_string()),
            command: Some(format!("python {}", path.file_name().unwrap().to_string_lossy())),
        });
    }
    
    Ok(())
}

/// Analyzes Rust projects
fn analyze_rust_project(
    root: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    build_scripts: &mut Vec<BuildScript>,
    config: &AnalysisConfig,
) -> Result<()> {
    let cargo_toml = root.join("Cargo.toml");
    
    if is_readable_file(&cargo_toml) {
        let content = read_file_safe(&cargo_toml, config.max_file_size)?;
        if let Ok(toml_value) = toml::from_str::<toml::Value>(&content) {
            // Check for binary targets
            if let Some(bins) = toml_value.get("bin").and_then(|b| b.as_array()) {
                for bin in bins {
                    if let Some(name) = bin.get("name").and_then(|n| n.as_str()) {
                        let path = bin.get("path")
                            .and_then(|p| p.as_str())
                            .map(PathBuf::from)
                            .unwrap_or_else(|| root.join("src").join("bin").join(format!("{}.rs", name)));
                        
                        entry_points.push(EntryPoint {
                            file: path,
                            function: Some("main".to_string()),
                            command: Some(format!("cargo run --bin {}", name)),
                        });
                    }
                }
            }
            
            // Default binary
            if let Some(_package_name) = toml_value.get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str()) {
                let main_rs = root.join("src").join("main.rs");
                if is_readable_file(&main_rs) {
                    entry_points.push(EntryPoint {
                        file: main_rs.clone(),
                        function: Some("main".to_string()),
                        command: Some("cargo run".to_string()),
                    });
                    
                    // Scan main.rs for context
                    scan_rust_file_for_context(&main_rs, ports, env_vars, config)?;
                }
            }
        }
    }
    
    // Common Rust build commands
    build_scripts.extend(vec![
        BuildScript {
            name: "build".to_string(),
            command: "cargo build".to_string(),
            description: Some("Build the project".to_string()),
            is_default: false,
        },
        BuildScript {
            name: "build-release".to_string(),
            command: "cargo build --release".to_string(),
            description: Some("Build optimized release version".to_string()),
            is_default: false,
        },
        BuildScript {
            name: "test".to_string(),
            command: "cargo test".to_string(),
            description: Some("Run tests".to_string()),
            is_default: false,
        },
        BuildScript {
            name: "run".to_string(),
            command: "cargo run".to_string(),
            description: Some("Run the application".to_string()),
            is_default: true,
        },
    ]);
    
    Ok(())
}

/// Scans Rust files for context information
fn scan_rust_file_for_context(
    path: &Path,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    config: &AnalysisConfig,
) -> Result<()> {
    let content = read_file_safe(path, config.max_file_size)?;
    
    // Look for port bindings
    let port_patterns = [
        r#"bind\s*\(\s*"[^"]*:(\d{1,5})"\s*\)"#,
        r#"bind\s*\(\s*\([^,]+,\s*(\d{1,5})\)\s*\)"#,
        r#"listen\s*\(\s*"[^"]*:(\d{1,5})"\s*\)"#,
        r"PORT[^=]*=\s*(\d{1,5})",
    ];
    
    for pattern in &port_patterns {
        let regex = create_regex(pattern)?;
        for cap in regex.captures_iter(&content) {
            if let Some(port_str) = cap.get(1) {
                if let Ok(port) = port_str.as_str().parse::<u16>() {
                    ports.insert(Port {
                        number: port,
                        protocol: Protocol::Http,
                        description: Some("Rust web server".to_string()),
                    });
                }
            }
        }
    }
    
    // Look for environment variable usage
    let env_patterns = [
        r#"env::var\s*\(\s*"([A-Z_][A-Z0-9_]*)"\s*\)"#,
        r#"std::env::var\s*\(\s*"([A-Z_][A-Z0-9_]*)"\s*\)"#,
        r#"env!\s*\(\s*"([A-Z_][A-Z0-9_]*)"\s*\)"#,
    ];
    
    for pattern in &env_patterns {
        let regex = create_regex(pattern)?;
        for cap in regex.captures_iter(&content) {
            if let Some(var_name) = cap.get(1) {
                let name = var_name.as_str().to_string();
                env_vars.entry(name.clone()).or_insert((None, false, None));
            }
        }
    }
    
    Ok(())
}

/// Analyzes Go projects
fn analyze_go_project(
    root: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    build_scripts: &mut Vec<BuildScript>,
    config: &AnalysisConfig,
) -> Result<()> {
    // Check for main.go
    let main_go = root.join("main.go");
    if is_readable_file(&main_go) {
        entry_points.push(EntryPoint {
            file: main_go.clone(),
            function: Some("main".to_string()),
            command: Some("go run main.go".to_string()),
        });
        
        scan_go_file_for_context(&main_go, ports, env_vars, config)?;
    }
    
    // Check cmd directory for multiple binaries
    let cmd_dir = root.join("cmd");
    if cmd_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&cmd_dir) {
            for entry in entries.flatten() {
                if entry.file_type()?.is_dir() {
                    let main_file = entry.path().join("main.go");
                    if is_readable_file(&main_file) {
                        let cmd_name = entry.file_name().to_string_lossy().to_string();
                        entry_points.push(EntryPoint {
                            file: main_file.clone(),
                            function: Some("main".to_string()),
                            command: Some(format!("go run ./cmd/{}", cmd_name)),
                        });
                        
                        scan_go_file_for_context(&main_file, ports, env_vars, config)?;
                    }
                }
            }
        }
    }
    
    // Common Go build commands
    build_scripts.extend(vec![
        BuildScript {
            name: "build".to_string(),
            command: "go build".to_string(),
            description: Some("Build the project".to_string()),
            is_default: false,
        },
        BuildScript {
            name: "test".to_string(),
            command: "go test ./...".to_string(),
            description: Some("Run tests".to_string()),
            is_default: false,
        },
        BuildScript {
            name: "run".to_string(),
            command: "go run .".to_string(),
            description: Some("Run the application".to_string()),
            is_default: true,
        },
        BuildScript {
            name: "mod-download".to_string(),
            command: "go mod download".to_string(),
            description: Some("Download dependencies".to_string()),
            is_default: false,
        },
    ]);
    
    Ok(())
}

/// Scans Go files for context information
fn scan_go_file_for_context(
    path: &Path,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    config: &AnalysisConfig,
) -> Result<()> {
    let content = read_file_safe(path, config.max_file_size)?;
    
    // Look for port bindings
    let port_patterns = [
        r#"Listen\s*\(\s*":(\d{1,5})"\s*\)"#,
        r#"ListenAndServe\s*\(\s*":(\d{1,5})"\s*,"#,
        r#"Addr:\s*":(\d{1,5})""#,
        r"PORT[^=]*=\s*(\d{1,5})",
    ];
    
    for pattern in &port_patterns {
        let regex = create_regex(pattern)?;
        for cap in regex.captures_iter(&content) {
            if let Some(port_str) = cap.get(1) {
                if let Ok(port) = port_str.as_str().parse::<u16>() {
                    ports.insert(Port {
                        number: port,
                        protocol: Protocol::Http,
                        description: Some("Go web server".to_string()),
                    });
                }
            }
        }
    }
    
    // Look for environment variable usage
    let env_patterns = [
        r#"os\.Getenv\s*\(\s*"([A-Z_][A-Z0-9_]*)"\s*\)"#,
        r#"os\.LookupEnv\s*\(\s*"([A-Z_][A-Z0-9_]*)"\s*\)"#,
    ];
    
    for pattern in &env_patterns {
        let regex = create_regex(pattern)?;
        for cap in regex.captures_iter(&content) {
            if let Some(var_name) = cap.get(1) {
                let name = var_name.as_str().to_string();
                env_vars.entry(name.clone()).or_insert((None, false, None));
            }
        }
    }
    
    Ok(())
}

/// Analyzes JVM projects (Java/Kotlin)
fn analyze_jvm_project(
    root: &Path,
    _entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    build_scripts: &mut Vec<BuildScript>,
    config: &AnalysisConfig,
) -> Result<()> {
    // Check for Maven
    let pom_xml = root.join("pom.xml");
    if is_readable_file(&pom_xml) {
        build_scripts.extend(vec![
            BuildScript {
                name: "build".to_string(),
                command: "mvn clean package".to_string(),
                description: Some("Build with Maven".to_string()),
                is_default: false,
            },
            BuildScript {
                name: "test".to_string(),
                command: "mvn test".to_string(),
                description: Some("Run tests".to_string()),
                is_default: false,
            },
            BuildScript {
                name: "run".to_string(),
                command: "mvn spring-boot:run".to_string(),
                description: Some("Run Spring Boot application".to_string()),
                is_default: true,
            },
        ]);
    }
    
    // Check for Gradle
    let gradle_files = ["build.gradle", "build.gradle.kts"];
    for gradle_file in &gradle_files {
        if is_readable_file(&root.join(gradle_file)) {
            build_scripts.extend(vec![
                BuildScript {
                    name: "build".to_string(),
                    command: "./gradlew build".to_string(),
                    description: Some("Build with Gradle".to_string()),
                    is_default: false,
                },
                BuildScript {
                    name: "test".to_string(),
                    command: "./gradlew test".to_string(),
                    description: Some("Run tests".to_string()),
                    is_default: false,
                },
                BuildScript {
                    name: "run".to_string(),
                    command: "./gradlew bootRun".to_string(),
                    description: Some("Run Spring Boot application".to_string()),
                    is_default: true,
                },
            ]);
            break;
        }
    }
    
    // Look for application properties
    let app_props_locations = [
        "src/main/resources/application.properties",
        "src/main/resources/application.yml",
        "src/main/resources/application.yaml",
    ];
    
    for props_path in &app_props_locations {
        let full_path = root.join(props_path);
        if is_readable_file(&full_path) {
            analyze_application_properties(&full_path, ports, env_vars, config)?;
        }
    }
    
    Ok(())
}

/// Analyzes application properties files
fn analyze_application_properties(
    path: &Path,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    config: &AnalysisConfig,
) -> Result<()> {
    let content = read_file_safe(path, config.max_file_size)?;
    
    // Look for server.port
    let port_regex = create_regex(r"server\.port\s*[=:]\s*(\d{1,5})")?;
    for cap in port_regex.captures_iter(&content) {
        if let Some(port_str) = cap.get(1) {
            if let Ok(port) = port_str.as_str().parse::<u16>() {
                ports.insert(Port {
                    number: port,
                    protocol: Protocol::Http,
                    description: Some("Spring Boot server".to_string()),
                });
            }
        }
    }
    
    // Look for ${ENV_VAR} placeholders
    let env_regex = create_regex(r"\$\{([A-Z_][A-Z0-9_]*)\}")?;
    for cap in env_regex.captures_iter(&content) {
        if let Some(var_name) = cap.get(1) {
            let name = var_name.as_str().to_string();
            env_vars.entry(name.clone()).or_insert((None, false, None));
        }
    }
    
    Ok(())
}

/// Analyzes Docker files for ports and environment variables
fn analyze_docker_files(
    root: &Path,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
) -> Result<()> {
    let dockerfile = root.join("Dockerfile");
    
    if is_readable_file(&dockerfile) {
        let content = std::fs::read_to_string(&dockerfile)?;
        
        // Look for EXPOSE directives
        let expose_regex = create_regex(r"EXPOSE\s+(\d{1,5})(?:/(\w+))?")?;
        for cap in expose_regex.captures_iter(&content) {
            if let Some(port_str) = cap.get(1) {
                if let Ok(port) = port_str.as_str().parse::<u16>() {
                    let protocol = cap.get(2)
                        .and_then(|p| match p.as_str().to_lowercase().as_str() {
                            "tcp" => Some(Protocol::Tcp),
                            "udp" => Some(Protocol::Udp),
                            _ => None,
                        })
                        .unwrap_or(Protocol::Tcp);
                    
                    ports.insert(Port {
                        number: port,
                        protocol,
                        description: Some("Exposed in Dockerfile".to_string()),
                    });
                }
            }
        }
        
        // Look for ENV directives
        let env_regex = create_regex(r"ENV\s+([A-Z_][A-Z0-9_]*)\s+(.+)")?;
        for cap in env_regex.captures_iter(&content) {
            if let (Some(name), Some(value)) = (cap.get(1), cap.get(2)) {
                let var_name = name.as_str().to_string();
                let var_value = value.as_str().trim().to_string();
                env_vars.entry(var_name).or_insert((Some(var_value), false, None));
            }
        }
    }
    
    // Check docker-compose files
    let compose_files = ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"];
    for compose_file in &compose_files {
        let path = root.join(compose_file);
        if is_readable_file(&path) {
            analyze_docker_compose(&path, ports, env_vars)?;
            break;
        }
    }
    
    Ok(())
}

/// Analyzes docker-compose files
fn analyze_docker_compose(
    path: &Path,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| AnalysisError::InvalidStructure(format!("Invalid YAML: {}", e)))?;
    
    if let Some(services) = value.get("services").and_then(|s| s.as_mapping()) {
        for (service_name, service) in services {
            let service_name_str = service_name.as_str().unwrap_or("unknown");
            
            // Determine service type based on image, name, and other indicators
            let service_type = determine_service_type(service_name_str, service);
            
            // Extract ports
            if let Some(service_ports) = service.get("ports").and_then(|p| p.as_sequence()) {
                for port_entry in service_ports {
                    if let Some(port_str) = port_entry.as_str() {
                        // Parse port mappings like "8080:80" or just "80"
                        let parts: Vec<&str> = port_str.split(':').collect();
                        
                        let (external_port, internal_port, protocol_suffix) = if parts.len() >= 2 {
                            // Format: "external:internal" or "external:internal/protocol"
                            let external = parts[0].trim();
                            let internal_parts: Vec<&str> = parts[1].split('/').collect();
                            let internal = internal_parts[0].trim();
                            let protocol = internal_parts.get(1).map(|p| p.trim());
                            (external, internal, protocol)
                        } else {
                            // Format: just "port" or "port/protocol"
                            let port_parts: Vec<&str> = parts[0].split('/').collect();
                            let port = port_parts[0].trim();
                            let protocol = port_parts.get(1).map(|p| p.trim());
                            (port, port, protocol)
                        };
                        
                        // Determine protocol
                        let protocol = match protocol_suffix {
                            Some("udp") => Protocol::Udp,
                            _ => Protocol::Tcp,
                        };
                        
                        // Create descriptive port entry
                        if let Ok(port) = external_port.parse::<u16>() {
                            let description = create_port_description(&service_type, service_name_str, external_port, internal_port);
                            
                            ports.insert(Port {
                                number: port,
                                protocol,
                                description: Some(description),
                            });
                        }
                    }
                }
            }
            
            // Extract environment variables with context
            if let Some(env) = service.get("environment") {
                let env_context = format!(" ({})", service_type.as_str());
                
                if let Some(env_map) = env.as_mapping() {
                    for (key, value) in env_map {
                        if let Some(key_str) = key.as_str() {
                            let val_str = value.as_str().map(|s| s.to_string());
                            let description = get_env_var_description(key_str, &service_type);
                            env_vars.entry(key_str.to_string())
                                .or_insert((val_str, false, description.or_else(|| Some(env_context.clone()))));
                        }
                    }
                } else if let Some(env_list) = env.as_sequence() {
                    for item in env_list {
                        if let Some(env_str) = item.as_str() {
                            if let Some(eq_pos) = env_str.find('=') {
                                let (key, value) = env_str.split_at(eq_pos);
                                let value = &value[1..]; // Skip the '='
                                let description = get_env_var_description(key, &service_type);
                                env_vars.entry(key.to_string())
                                    .or_insert((Some(value.to_string()), false, description.or_else(|| Some(env_context.clone()))));
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Service types found in Docker Compose
#[derive(Debug, Clone)]
enum ServiceType {
    PostgreSQL,
    MySQL,
    MongoDB,
    Redis,
    RabbitMQ,
    Kafka,
    Elasticsearch,
    Application,
    Nginx,
    Unknown,
}

impl ServiceType {
    fn as_str(&self) -> &'static str {
        match self {
            ServiceType::PostgreSQL => "PostgreSQL database",
            ServiceType::MySQL => "MySQL database",
            ServiceType::MongoDB => "MongoDB database",
            ServiceType::Redis => "Redis cache",
            ServiceType::RabbitMQ => "RabbitMQ message broker",
            ServiceType::Kafka => "Kafka message broker",
            ServiceType::Elasticsearch => "Elasticsearch search engine",
            ServiceType::Application => "Application service",
            ServiceType::Nginx => "Nginx web server",
            ServiceType::Unknown => "Service",
        }
    }
}

/// Determines the type of service based on various indicators
fn determine_service_type(name: &str, service: &serde_yaml::Value) -> ServiceType {
    let name_lower = name.to_lowercase();
    
    // Check service name
    if name_lower.contains("postgres") || name_lower.contains("pg") || name_lower.contains("psql") {
        return ServiceType::PostgreSQL;
    } else if name_lower.contains("mysql") || name_lower.contains("mariadb") {
        return ServiceType::MySQL;
    } else if name_lower.contains("mongo") {
        return ServiceType::MongoDB;
    } else if name_lower.contains("redis") {
        return ServiceType::Redis;
    } else if name_lower.contains("rabbit") || name_lower.contains("amqp") {
        return ServiceType::RabbitMQ;
    } else if name_lower.contains("kafka") {
        return ServiceType::Kafka;
    } else if name_lower.contains("elastic") || name_lower.contains("es") {
        return ServiceType::Elasticsearch;
    } else if name_lower.contains("nginx") || name_lower.contains("proxy") {
        return ServiceType::Nginx;
    }
    
    // Check image name
    if let Some(image) = service.get("image").and_then(|i| i.as_str()) {
        let image_lower = image.to_lowercase();
        if image_lower.contains("postgres") {
            return ServiceType::PostgreSQL;
        } else if image_lower.contains("mysql") || image_lower.contains("mariadb") {
            return ServiceType::MySQL;
        } else if image_lower.contains("mongo") {
            return ServiceType::MongoDB;
        } else if image_lower.contains("redis") {
            return ServiceType::Redis;
        } else if image_lower.contains("rabbitmq") {
            return ServiceType::RabbitMQ;
        } else if image_lower.contains("kafka") {
            return ServiceType::Kafka;
        } else if image_lower.contains("elastic") {
            return ServiceType::Elasticsearch;
        } else if image_lower.contains("nginx") {
            return ServiceType::Nginx;
        }
    }
    
    // Check environment variables for clues
    if let Some(env) = service.get("environment") {
        if let Some(env_map) = env.as_mapping() {
            for (key, _) in env_map {
                if let Some(key_str) = key.as_str() {
                    if key_str.contains("POSTGRES") || key_str.contains("PGPASSWORD") {
                        return ServiceType::PostgreSQL;
                    } else if key_str.contains("MYSQL") {
                        return ServiceType::MySQL;
                    } else if key_str.contains("MONGO") {
                        return ServiceType::MongoDB;
                    }
                }
            }
        }
    }
    
    // Check if it has a build context (likely application)
    if service.get("build").is_some() {
        return ServiceType::Application;
    }
    
    ServiceType::Unknown
}

/// Creates a descriptive port description based on service type
fn create_port_description(service_type: &ServiceType, service_name: &str, external: &str, internal: &str) -> String {
    let base_desc = match service_type {
        ServiceType::PostgreSQL => format!("PostgreSQL database ({})", service_name),
        ServiceType::MySQL => format!("MySQL database ({})", service_name),
        ServiceType::MongoDB => format!("MongoDB database ({})", service_name),
        ServiceType::Redis => format!("Redis cache ({})", service_name),
        ServiceType::RabbitMQ => format!("RabbitMQ message broker ({})", service_name),
        ServiceType::Kafka => format!("Kafka message broker ({})", service_name),
        ServiceType::Elasticsearch => format!("Elasticsearch ({})", service_name),
        ServiceType::Nginx => format!("Nginx proxy ({})", service_name),
        ServiceType::Application => format!("Application service ({})", service_name),
        ServiceType::Unknown => format!("Docker service ({})", service_name),
    };
    
    if external != internal {
        format!("{} - external:{}, internal:{}", base_desc, external, internal)
    } else {
        format!("{} - port {}", base_desc, external)
    }
}

/// Gets a descriptive context for environment variables based on service type
fn get_env_var_description(var_name: &str, service_type: &ServiceType) -> Option<String> {
    match var_name {
        "POSTGRES_PASSWORD" | "POSTGRES_USER" | "POSTGRES_DB" => 
            Some("PostgreSQL configuration".to_string()),
        "MYSQL_ROOT_PASSWORD" | "MYSQL_PASSWORD" | "MYSQL_USER" | "MYSQL_DATABASE" => 
            Some("MySQL configuration".to_string()),
        "MONGO_INITDB_ROOT_USERNAME" | "MONGO_INITDB_ROOT_PASSWORD" => 
            Some("MongoDB configuration".to_string()),
        "REDIS_PASSWORD" => Some("Redis configuration".to_string()),
        "RABBITMQ_DEFAULT_USER" | "RABBITMQ_DEFAULT_PASS" => 
            Some("RabbitMQ configuration".to_string()),
        "DATABASE_URL" | "DB_CONNECTION_STRING" => 
            Some("Database connection string".to_string()),
        "GOOGLE_APPLICATION_CREDENTIALS" => 
            Some("Google Cloud service account credentials".to_string()),
        _ => None,
    }
}

/// Analyzes .env files
fn analyze_env_files(
    root: &Path,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
) -> Result<()> {
    let env_files = [".env", ".env.example", ".env.local", ".env.development", ".env.production"];
    
    for env_file in &env_files {
        let path = root.join(env_file);
        if is_readable_file(&path) {
            let content = std::fs::read_to_string(&path)?;
            
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                
                if let Some(eq_pos) = line.find('=') {
                    let (key, value) = line.split_at(eq_pos);
                    let key = key.trim();
                    let value = value[1..].trim(); // Skip the '='
                    
                    // Check if it's marked as required (common convention)
                    let required = value.is_empty() || value == "required" || value == "REQUIRED";
                    let actual_value = if required { None } else { Some(value.to_string()) };
                    
                    env_vars.entry(key.to_string()).or_insert((actual_value, required, None));
                }
            }
        }
    }
    
    Ok(())
}

/// Analyzes Makefile for build scripts
fn analyze_makefile(
    root: &Path,
    build_scripts: &mut Vec<BuildScript>,
) -> Result<()> {
    let makefiles = ["Makefile", "makefile"];
    
    for makefile in &makefiles {
        let path = root.join(makefile);
        if is_readable_file(&path) {
            let content = std::fs::read_to_string(&path)?;
            
            // Simple Makefile target extraction
            let target_regex = create_regex(r"^([a-zA-Z0-9_-]+):\s*(?:[^\n]*)?$")?;
            let mut in_recipe = false;
            let mut current_target = String::new();
            let mut current_command = String::new();
            
            for line in content.lines() {
                if let Some(cap) = target_regex.captures(line) {
                    // Save previous target if any
                    if !current_target.is_empty() && !current_command.is_empty() {
                        build_scripts.push(BuildScript {
                            name: current_target.clone(),
                            command: format!("make {}", current_target),
                            description: None,
                            is_default: current_target == "run" || current_target == "start",
                        });
                    }
                    
                    if let Some(target) = cap.get(1) {
                        current_target = target.as_str().to_string();
                        current_command.clear();
                        in_recipe = true;
                    }
                } else if in_recipe && line.starts_with('\t') {
                    if current_command.is_empty() {
                        current_command = line.trim().to_string();
                    }
                } else if !line.trim().is_empty() {
                    in_recipe = false;
                }
            }
            
            // Save last target
            if !current_target.is_empty() && !current_command.is_empty() {
                build_scripts.push(BuildScript {
                    name: current_target.clone(),
                    command: format!("make {}", current_target),
                    description: None,
                    is_default: current_target == "run" || current_target == "start",
                });
            }
            
            break;
        }
    }
    
    Ok(())
}

/// Analyzes technology-specific configurations
fn analyze_technology_specifics(
    technology: &DetectedTechnology,
    root: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
) -> Result<()> {
    match technology.name.as_str() {
        "Next.js" => {
            // Next.js typically runs on port 3000
            ports.insert(Port {
                number: 3000,
                protocol: Protocol::Http,
                description: Some("Next.js development server".to_string()),
            });
            
            // Look for pages directory
            let pages_dir = root.join("pages");
            if pages_dir.is_dir() {
                entry_points.push(EntryPoint {
                    file: pages_dir,
                    function: None,
                    command: Some("npm run dev".to_string()),
                });
            }
        }
        "Express" | "Fastify" | "Koa" | "Hono" | "Elysia" => {
            // Common Node.js web framework ports
            ports.insert(Port {
                number: 3000,
                protocol: Protocol::Http,
                description: Some(format!("{} server", technology.name)),
            });
        }
        "Encore" => {
            // Encore development server typically runs on port 4000
            ports.insert(Port {
                number: 4000,
                protocol: Protocol::Http,
                description: Some("Encore development server".to_string()),
            });
        }
        "Astro" => {
            // Astro development server typically runs on port 3000 or 4321
            ports.insert(Port {
                number: 4321,
                protocol: Protocol::Http,
                description: Some("Astro development server".to_string()),
            });
        }
        "SvelteKit" => {
            // SvelteKit development server typically runs on port 5173
            ports.insert(Port {
                number: 5173,
                protocol: Protocol::Http,
                description: Some("SvelteKit development server".to_string()),
            });
        }
        "Nuxt.js" => {
            // Nuxt.js development server typically runs on port 3000
            ports.insert(Port {
                number: 3000,
                protocol: Protocol::Http,
                description: Some("Nuxt.js development server".to_string()),
            });
        }
        "Tanstack Start" => {
            // Modern React framework typically runs on port 3000
            ports.insert(Port {
                number: 3000,
                protocol: Protocol::Http,
                description: Some(format!("{} development server", technology.name)),
            });
        }
        "React Router v7" => {
            // React Router v7 development server typically runs on port 5173
            ports.insert(Port {
                number: 5173,
                protocol: Protocol::Http,
                description: Some("React Router v7 development server".to_string()),
            });
        }
        "Django" => {
            ports.insert(Port {
                number: 8000,
                protocol: Protocol::Http,
                description: Some("Django development server".to_string()),
            });
        }
        "Flask" | "FastAPI" => {
            ports.insert(Port {
                number: 5000,
                protocol: Protocol::Http,
                description: Some(format!("{} server", technology.name)),
            });
        }
        "Spring Boot" => {
            ports.insert(Port {
                number: 8080,
                protocol: Protocol::Http,
                description: Some("Spring Boot server".to_string()),
            });
        }
        "Actix Web" | "Rocket" => {
            ports.insert(Port {
                number: 8080,
                protocol: Protocol::Http,
                description: Some(format!("{} server", technology.name)),
            });
        }
        _ => {}
    }
    
    Ok(())
}

/// Extracts ports from command strings
fn extract_ports_from_command(command: &str, ports: &mut HashSet<Port>) {
    // Look for common port patterns in commands
    let patterns = [
        r"-p\s+(\d{1,5})",
        r"--port\s+(\d{1,5})",
        r"--port=(\d{1,5})",
        r"PORT=(\d{1,5})",
    ];
    
    for pattern in &patterns {
        if let Ok(regex) = Regex::new(pattern) {
            for cap in regex.captures_iter(command) {
                if let Some(port_str) = cap.get(1) {
                    if let Ok(port) = port_str.as_str().parse::<u16>() {
                        ports.insert(Port {
                            number: port,
                            protocol: Protocol::Http,
                            description: Some("Port from command".to_string()),
                        });
                    }
                }
            }
        }
    }
}

/// Helper function to get script description
fn get_script_description(name: &str) -> Option<String> {
    match name {
        "start" => Some("Start the application".to_string()),
        "dev" => Some("Start development server".to_string()),
        "build" => Some("Build the application".to_string()),
        "test" => Some("Run tests".to_string()),
        "lint" => Some("Run linter".to_string()),
        "format" => Some("Format code".to_string()),
        _ => None,
    }
}

/// Determines the project type based on analysis
fn determine_project_type(
    languages: &[DetectedLanguage],
    technologies: &[DetectedTechnology],
    entry_points: &[EntryPoint],
    ports: &[Port],
) -> ProjectType {
    // Check for microservice architecture indicators
    let has_database_ports = ports.iter().any(|p| {
        if let Some(desc) = &p.description {
            let desc_lower = desc.to_lowercase();
            desc_lower.contains("postgres") || desc_lower.contains("mysql") || 
            desc_lower.contains("mongodb") || desc_lower.contains("database")
        } else {
            false
        }
    });
    
    let has_multiple_services = ports.iter()
        .filter_map(|p| p.description.as_ref())
        .filter(|desc| {
            let desc_lower = desc.to_lowercase();
            desc_lower.contains("service") || desc_lower.contains("application")
        })
        .count() > 1;
    
    let has_orchestration_framework = technologies.iter()
        .any(|t| t.name == "Encore" || t.name == "Dapr" || t.name == "Temporal");
    
    // Check for web frameworks
    let web_frameworks = ["Express", "Fastify", "Koa", "Next.js", "React", "Vue", "Angular",
                         "Django", "Flask", "FastAPI", "Spring Boot", "Actix Web", "Rocket",
                         "Gin", "Echo", "Fiber", "Svelte", "SvelteKit", "SolidJS", "Astro",
                         "Encore", "Hono", "Elysia", "React Router v7", "Tanstack Start",
                         "SolidStart", "Qwik", "Nuxt.js", "Gatsby"];
    
    let has_web_framework = technologies.iter()
        .any(|t| web_frameworks.contains(&t.name.as_str()));
    
    // Check for CLI indicators
    let cli_indicators = ["cobra", "clap", "argparse", "commander"];
    let has_cli_framework = technologies.iter()
        .any(|t| cli_indicators.contains(&t.name.to_lowercase().as_str()));
    
    // Check for API indicators
    let api_frameworks = ["FastAPI", "Express", "Gin", "Echo", "Actix Web", "Spring Boot",
                          "Fastify", "Koa", "Nest.js", "Encore", "Hono", "Elysia"];
    let has_api_framework = technologies.iter()
        .any(|t| api_frameworks.contains(&t.name.as_str()));
    
    // Check for static site generators
    let static_generators = ["Gatsby", "Hugo", "Jekyll", "Eleventy", "Astro"];
    let has_static_generator = technologies.iter()
        .any(|t| static_generators.contains(&t.name.as_str()));
    
    // Determine type based on indicators
    if (has_database_ports || has_multiple_services) && (has_orchestration_framework || has_api_framework) {
        ProjectType::Microservice
    } else if has_static_generator {
        ProjectType::StaticSite
    } else if has_api_framework && !has_web_framework {
        ProjectType::ApiService
    } else if has_web_framework {
        ProjectType::WebApplication
    } else if has_cli_framework || (entry_points.len() == 1 && ports.is_empty()) {
        ProjectType::CliTool
    } else if entry_points.is_empty() && ports.is_empty() {
        // Check if it's a library
        let has_lib_indicators = languages.iter().any(|l| {
            match l.name.as_str() {
                "Rust" => l.files.iter().any(|f| f.to_string_lossy().contains("lib.rs")),
                "Python" => l.files.iter().any(|f| f.to_string_lossy().contains("__init__.py")),
                "JavaScript" | "TypeScript" => l.main_dependencies.is_empty(),
                _ => false,
            }
        });
        
        if has_lib_indicators {
            ProjectType::Library
        } else {
            ProjectType::Unknown
        }
    } else {
        ProjectType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{TechnologyCategory, LibraryType};
    use std::fs;
    use tempfile::TempDir;
    
    fn create_test_language(name: &str) -> DetectedLanguage {
        DetectedLanguage {
            name: name.to_string(),
            version: None,
            confidence: 0.9,
            files: vec![],
            main_dependencies: vec![],
            dev_dependencies: vec![],
            package_manager: None,
        }
    }
    
    fn create_test_technology(name: &str, category: TechnologyCategory) -> DetectedTechnology {
        DetectedTechnology {
            name: name.to_string(),
            version: None,
            category,
            confidence: 0.8,
            requires: vec![],
            conflicts_with: vec![],
            is_primary: false,
        }
    }
    
    #[test]
    fn test_node_project_context() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create package.json with scripts
        let package_json = r#"{
            "name": "test-app",
            "main": "index.js",
            "scripts": {
                "start": "node index.js",
                "dev": "nodemon index.js",
                "test": "jest",
                "build": "webpack"
            }
        }"#;
        fs::write(root.join("package.json"), package_json).unwrap();
        
        // Create index.js with port and env vars
        let index_js = r#"
const express = require('express');
const app = express();

const PORT = process.env.PORT || 3000;
const API_KEY = process.env.API_KEY;
const DATABASE_URL = process.env.DATABASE_URL;

app.listen(PORT, () => {
    console.log(`Server running on port ${PORT}`);
});
        "#;
        fs::write(root.join("index.js"), index_js).unwrap();
        
        let languages = vec![create_test_language("JavaScript")];
        let technologies = vec![create_test_technology("Express", TechnologyCategory::BackendFramework)];
        let config = AnalysisConfig::default();
        
        let context = analyze_context(root, &languages, &technologies, &config).unwrap();
        
        // Verify entry points
        assert!(!context.entry_points.is_empty());
        assert!(context.entry_points.iter().any(|ep| ep.file.ends_with("index.js")));
        
        // Verify ports
        assert!(!context.ports.is_empty());
        assert!(context.ports.iter().any(|p| p.number == 3000));
        
        // Verify environment variables
        assert!(context.environment_variables.iter().any(|ev| ev.name == "PORT"));
        assert!(context.environment_variables.iter().any(|ev| ev.name == "API_KEY"));
        assert!(context.environment_variables.iter().any(|ev| ev.name == "DATABASE_URL"));
        
        // Verify build scripts
        assert_eq!(context.build_scripts.len(), 4);
        assert!(context.build_scripts.iter().any(|bs| bs.name == "start" && bs.is_default));
        assert!(context.build_scripts.iter().any(|bs| bs.name == "dev" && bs.is_default));
        assert!(context.build_scripts.iter().any(|bs| bs.name == "test"));
        assert!(context.build_scripts.iter().any(|bs| bs.name == "build"));
        
        // Verify project type
        assert_eq!(context.project_type, ProjectType::WebApplication);
    }
    
    #[test]
    fn test_python_project_context() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create app.py with Flask
        let app_py = r#"
import os
from flask import Flask

app = Flask(__name__)

PORT = 5000
SECRET_KEY = os.environ.get('SECRET_KEY')
DEBUG = os.getenv('DEBUG', 'False')

if __name__ == '__main__':
    app.run(port=PORT)
        "#;
        fs::write(root.join("app.py"), app_py).unwrap();
        
        let languages = vec![create_test_language("Python")];
        let technologies = vec![create_test_technology("Flask", TechnologyCategory::BackendFramework)];
        let config = AnalysisConfig::default();
        
        let context = analyze_context(root, &languages, &technologies, &config).unwrap();
        
        // Verify entry points
        assert!(context.entry_points.iter().any(|ep| ep.file.ends_with("app.py")));
        
        // Verify ports
        assert!(context.ports.iter().any(|p| p.number == 5000));
        
        // Verify environment variables
        assert!(context.environment_variables.iter().any(|ev| ev.name == "SECRET_KEY"));
        assert!(context.environment_variables.iter().any(|ev| ev.name == "DEBUG"));
        
        // Verify project type
        assert_eq!(context.project_type, ProjectType::WebApplication);
    }
    
    #[test]
    fn test_rust_project_context() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create Cargo.toml
        let cargo_toml = r#"
[package]
name = "test-server"
version = "0.1.0"

[[bin]]
name = "server"
path = "src/main.rs"
        "#;
        fs::write(root.join("Cargo.toml"), cargo_toml).unwrap();
        
        // Create src directory
        fs::create_dir_all(root.join("src")).unwrap();
        
        // Create main.rs
        let main_rs = r#"
use std::env;

fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    println!("Starting server on port {}", port);
}
        "#;
        fs::write(root.join("src/main.rs"), main_rs).unwrap();
        
        let languages = vec![create_test_language("Rust")];
        let frameworks = vec![];
        let config = AnalysisConfig::default();
        
        let context = analyze_context(root, &languages, &frameworks, &config).unwrap();
        
        // Verify entry points
        assert!(context.entry_points.iter().any(|ep| ep.file.ends_with("main.rs")));
        assert!(context.entry_points.iter().any(|ep| ep.command == Some("cargo run".to_string())));
        
        // Verify build scripts
        assert!(context.build_scripts.iter().any(|bs| bs.name == "build"));
        assert!(context.build_scripts.iter().any(|bs| bs.name == "test"));
        assert!(context.build_scripts.iter().any(|bs| bs.name == "run" && bs.is_default));
        
        // Verify environment variables
        assert!(context.environment_variables.iter().any(|ev| ev.name == "PORT"));
        assert!(context.environment_variables.iter().any(|ev| ev.name == "DATABASE_URL"));
    }
    
    #[test]
    fn test_dockerfile_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create Dockerfile
        let dockerfile = r#"
FROM node:14
WORKDIR /app

ENV NODE_ENV=production
ENV PORT=3000

EXPOSE 3000
EXPOSE 9229/tcp

CMD ["node", "server.js"]
        "#;
        fs::write(root.join("Dockerfile"), dockerfile).unwrap();
        
        let languages = vec![];
        let frameworks = vec![];
        let config = AnalysisConfig::default();
        
        let context = analyze_context(root, &languages, &frameworks, &config).unwrap();
        
        // Verify ports from EXPOSE
        assert!(context.ports.iter().any(|p| p.number == 3000));
        assert!(context.ports.iter().any(|p| p.number == 9229 && p.protocol == Protocol::Tcp));
        
        // Verify environment variables from ENV
        assert!(context.environment_variables.iter().any(|ev| 
            ev.name == "NODE_ENV" && ev.default_value == Some("production".to_string())
        ));
        assert!(context.environment_variables.iter().any(|ev| 
            ev.name == "PORT" && ev.default_value == Some("3000".to_string())
        ));
    }
    
    #[test]
    fn test_docker_compose_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create docker-compose.yml
        let compose = r#"
version: '3.8'
services:
  web:
    build: .
    ports:
      - "8080:80"
      - "443"
    environment:
      - DATABASE_URL=postgres://user:pass@db:5432/mydb
      - REDIS_URL=redis://cache:6379
  db:
    image: postgres
    ports:
      - "5432"
    environment:
      POSTGRES_PASSWORD: secret
        "#;
        fs::write(root.join("docker-compose.yml"), compose).unwrap();
        
        let languages = vec![];
        let frameworks = vec![];
        let config = AnalysisConfig::default();
        
        let context = analyze_context(root, &languages, &frameworks, &config).unwrap();
        
        // Verify ports
        assert!(context.ports.iter().any(|p| p.number == 80));
        assert!(context.ports.iter().any(|p| p.number == 443));
        assert!(context.ports.iter().any(|p| p.number == 5432));
        
        // Verify environment variables
        assert!(context.environment_variables.iter().any(|ev| ev.name == "DATABASE_URL"));
        assert!(context.environment_variables.iter().any(|ev| ev.name == "REDIS_URL"));
        assert!(context.environment_variables.iter().any(|ev| ev.name == "POSTGRES_PASSWORD"));
    }
    
    #[test]
    fn test_env_file_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create .env file
        let env_file = r#"
# Database configuration
DATABASE_URL=postgresql://localhost:5432/myapp
REDIS_URL=redis://localhost:6379

# API Keys
API_KEY=
SECRET_KEY=required

# Feature flags
ENABLE_FEATURE_X=true
DEBUG=false
        "#;
        fs::write(root.join(".env"), env_file).unwrap();
        
        let languages = vec![];
        let frameworks = vec![];
        let config = AnalysisConfig::default();
        
        let context = analyze_context(root, &languages, &frameworks, &config).unwrap();
        
        // Verify environment variables
        assert!(context.environment_variables.iter().any(|ev| 
            ev.name == "DATABASE_URL" && ev.default_value.is_some()
        ));
        assert!(context.environment_variables.iter().any(|ev| 
            ev.name == "API_KEY" && ev.required
        ));
        assert!(context.environment_variables.iter().any(|ev| 
            ev.name == "SECRET_KEY" && ev.required
        ));
        assert!(context.environment_variables.iter().any(|ev| 
            ev.name == "ENABLE_FEATURE_X" && ev.default_value == Some("true".to_string())
        ));
    }
    
    #[test]
    fn test_makefile_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create Makefile
        let makefile = r#"
build:
	go build -o app main.go

test:
	go test ./...

run: build
	./app

docker-build:
	docker build -t myapp .

clean:
	rm -f app
        "#;
        fs::write(root.join("Makefile"), makefile).unwrap();
        
        let languages = vec![];
        let frameworks = vec![];
        let config = AnalysisConfig::default();
        
        let context = analyze_context(root, &languages, &frameworks, &config).unwrap();
        
        // Verify build scripts
        assert!(context.build_scripts.iter().any(|bs| bs.name == "build"));
        assert!(context.build_scripts.iter().any(|bs| bs.name == "test"));
        assert!(context.build_scripts.iter().any(|bs| bs.name == "run" && bs.is_default));
        assert!(context.build_scripts.iter().any(|bs| bs.name == "docker-build"));
        assert!(context.build_scripts.iter().any(|bs| bs.name == "clean"));
    }
    
    #[test]
    fn test_project_type_detection() {
        // Test CLI tool detection
        let languages = vec![create_test_language("Rust")];
        let technologies = vec![create_test_technology("clap", TechnologyCategory::Library(LibraryType::Other("CLI".to_string())))];
        let entry_points = vec![EntryPoint {
            file: PathBuf::from("src/main.rs"),
            function: Some("main".to_string()),
            command: Some("cargo run".to_string()),
        }];
        let ports = vec![];
        
        let project_type = determine_project_type(&languages, &technologies, &entry_points, &ports);
        assert_eq!(project_type, ProjectType::CliTool);
        
        // Test API service detection
        let technologies = vec![create_test_technology("FastAPI", TechnologyCategory::BackendFramework)];
        let ports = vec![Port {
            number: 8000,
            protocol: Protocol::Http,
            description: None,
        }];
        
        let project_type = determine_project_type(&languages, &technologies, &vec![], &ports);
        assert_eq!(project_type, ProjectType::ApiService);
        
        // Test library detection
        let languages = vec![create_test_language("Python")];
        let mut lang = languages[0].clone();
        lang.files = vec![PathBuf::from("__init__.py")];
        let languages = vec![lang];
        
        let project_type = determine_project_type(&languages, &vec![], &vec![], &vec![]);
        assert_eq!(project_type, ProjectType::Library);
    }
} 