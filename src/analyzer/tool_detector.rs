use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use log::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub available: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub installation_source: InstallationSource,
    pub last_checked: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallationSource {
    SystemPath,
    UserLocal,
    CargoHome,
    GoHome,
    PackageManager(String), // brew, apt, etc.
    Manual,
    NotFound,
}

pub struct ToolDetector {
    cache: HashMap<String, ToolStatus>,
    cache_ttl: Duration,
}

impl ToolDetector {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            cache_ttl: Duration::from_secs(300), // 5 minutes
        }
    }
    
    /// Detect tool availability with caching
    pub fn detect_tool(&mut self, tool_name: &str) -> ToolStatus {
        // Check cache first
        if let Some(cached) = self.cache.get(tool_name) {
            if cached.last_checked.elapsed().unwrap_or(Duration::MAX) < self.cache_ttl {
                debug!("Using cached status for {}: available={}", tool_name, cached.available);
                return cached.clone();
            }
        }
        
        // Perform real detection
        let status = self.detect_tool_real_time(tool_name);
        debug!("Real-time detection for {}: available={}, path={:?}", 
               tool_name, status.available, status.path);
        self.cache.insert(tool_name.to_string(), status.clone());
        status
    }
    
    /// Detect all vulnerability scanning tools for given languages
    pub fn detect_all_vulnerability_tools(&mut self, languages: &[crate::analyzer::dependency_parser::Language]) -> HashMap<String, ToolStatus> {
        let mut results = HashMap::new();
        
        for language in languages {
            let tool_names = match language {
                crate::analyzer::dependency_parser::Language::Rust => vec!["cargo-audit"],
                crate::analyzer::dependency_parser::Language::JavaScript | 
                crate::analyzer::dependency_parser::Language::TypeScript => vec!["bun", "npm", "yarn", "pnpm"],
                crate::analyzer::dependency_parser::Language::Python => vec!["pip-audit"],
                crate::analyzer::dependency_parser::Language::Go => vec!["govulncheck"],
                crate::analyzer::dependency_parser::Language::Java | 
                crate::analyzer::dependency_parser::Language::Kotlin => vec!["grype"],
                _ => continue,
            };
            
            for tool_name in tool_names {
                if !results.contains_key(tool_name) {
                    results.insert(tool_name.to_string(), self.detect_tool(tool_name));
                }
            }
        }
        
        results
    }
    
    /// Clear the cache to force fresh detection
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    /// Detect bun specifically with multiple alternatives
    pub fn detect_bun(&mut self) -> ToolStatus {
        self.detect_tool_with_alternatives("bun", &[
            "bun",
            "bunx", // Bun's npx equivalent
        ])
    }
    
    /// Detect all JavaScript package managers
    pub fn detect_js_package_managers(&mut self) -> HashMap<String, ToolStatus> {
        let mut managers = HashMap::new();
        managers.insert("bun".to_string(), self.detect_bun());
        managers.insert("npm".to_string(), self.detect_tool("npm"));
        managers.insert("yarn".to_string(), self.detect_tool("yarn"));
        managers.insert("pnpm".to_string(), self.detect_tool("pnpm"));
        managers
    }
    
    /// Detect tool with alternative command names
    pub fn detect_tool_with_alternatives(&mut self, primary_name: &str, alternatives: &[&str]) -> ToolStatus {
        // Check cache first for primary name
        if let Some(cached) = self.cache.get(primary_name) {
            if cached.last_checked.elapsed().unwrap_or(Duration::MAX) < self.cache_ttl {
                debug!("Using cached status for {}: available={}", primary_name, cached.available);
                return cached.clone();
            }
        }
        
        // Try each alternative
        for alternative in alternatives {
            debug!("Trying to detect tool: {}", alternative);
            let status = self.detect_tool_real_time(alternative);
            if status.available {
                debug!("Found {} via alternative: {}", primary_name, alternative);
                // Cache under primary name
                self.cache.insert(primary_name.to_string(), status.clone());
                return status;
            }
        }
        
        // Not found
        let not_found = ToolStatus {
            available: false,
            path: None,
            version: None,
            installation_source: InstallationSource::NotFound,
            last_checked: SystemTime::now(),
        };
        
        self.cache.insert(primary_name.to_string(), not_found.clone());
        not_found
    }
    
    /// Perform real-time tool detection without caching
    fn detect_tool_real_time(&self, tool_name: &str) -> ToolStatus {
        debug!("Starting real-time detection for {}", tool_name);
        
        // Try direct command first (in PATH)
        if let Some((path, version)) = self.try_command_in_path(tool_name) {
            info!("Found {} in PATH at {:?} with version {:?}", tool_name, path, version);
            return ToolStatus {
                available: true,
                path: Some(path),
                version,
                installation_source: InstallationSource::SystemPath,
                last_checked: SystemTime::now(),
            };
        }
        
        // Try alternative paths
        let search_paths = self.get_tool_search_paths(tool_name);
        debug!("Searching alternative paths for {}: {:?}", tool_name, search_paths);
        
        for search_path in search_paths {
            let tool_path = search_path.join(tool_name);
            debug!("Checking path: {:?}", tool_path);
            
            if let Some(version) = self.verify_tool_at_path(&tool_path, tool_name) {
                let source = self.determine_installation_source(&search_path);
                info!("Found {} at {:?} with version {:?} (source: {:?})", 
                      tool_name, tool_path, version, source);
                return ToolStatus {
                    available: true,
                    path: Some(tool_path),
                    version: Some(version),
                    installation_source: source,
                    last_checked: SystemTime::now(),
                };
            }
            
            // Also try with .exe extension on Windows
            #[cfg(windows)]
            {
                let tool_path_exe = search_path.join(format!("{}.exe", tool_name));
                if let Some(version) = self.verify_tool_at_path(&tool_path_exe, tool_name) {
                    let source = self.determine_installation_source(&search_path);
                    info!("Found {} at {:?} with version {:?} (source: {:?})", 
                          tool_name, tool_path_exe, version, source);
                    return ToolStatus {
                        available: true,
                        path: Some(tool_path_exe),
                        version,
                        installation_source: source,
                        last_checked: SystemTime::now(),
                    };
                }
            }
        }
        
        // Tool not found
        debug!("Tool {} not found in any location", tool_name);
        ToolStatus {
            available: false,
            path: None,
            version: None,
            installation_source: InstallationSource::NotFound,
            last_checked: SystemTime::now(),
        }
    }
    
    /// Get search paths for a specific tool
    fn get_tool_search_paths(&self, tool_name: &str) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        
        // User-specific paths
        if let Ok(home) = std::env::var("HOME") {
            let home_path = PathBuf::from(home);
            
            // Common user install locations
            paths.push(home_path.join(".local").join("bin"));
            paths.push(home_path.join(".cargo").join("bin"));
            paths.push(home_path.join("go").join("bin"));
            
            // Tool-specific locations
            match tool_name {
                "cargo-audit" => {
                    paths.push(home_path.join(".cargo").join("bin"));
                }
                "govulncheck" => {
                    paths.push(home_path.join("go").join("bin"));
                    if let Ok(gopath) = std::env::var("GOPATH") {
                        paths.push(PathBuf::from(gopath).join("bin"));
                    }
                    if let Ok(goroot) = std::env::var("GOROOT") {
                        paths.push(PathBuf::from(goroot).join("bin"));
                    }
                }
                "grype" => {
                    paths.push(home_path.join(".local").join("bin"));
                    // Homebrew paths
                    paths.push(PathBuf::from("/opt/homebrew/bin"));
                    paths.push(PathBuf::from("/usr/local/bin"));
                }
                "pip-audit" => {
                    paths.push(home_path.join(".local").join("bin"));
                    // Python user site packages
                    if let Ok(output) = Command::new("python3")
                        .args(&["-m", "site", "--user-base"])
                        .output() {
                        if let Ok(user_base) = String::from_utf8(output.stdout) {
                            paths.push(PathBuf::from(user_base.trim()).join("bin"));
                        }
                    }
                    // Also try python (without 3)
                    if let Ok(output) = Command::new("python")
                        .args(&["-m", "site", "--user-base"])
                        .output() {
                        if let Ok(user_base) = String::from_utf8(output.stdout) {
                            paths.push(PathBuf::from(user_base.trim()).join("bin"));
                        }
                    }
                }
                "npm" => {
                    // npm is usually in standard locations, but check Node.js specific paths
                    if let Ok(node_path) = std::env::var("NODE_PATH") {
                        paths.push(PathBuf::from(node_path).join(".bin"));
                    }
                    // Common npm global locations
                    paths.push(home_path.join(".npm-global").join("bin"));
                    paths.push(PathBuf::from("/usr/local/lib/node_modules/.bin"));
                }
                "bun" => {
                    // Bun-specific installation paths
                    paths.push(home_path.join(".bun").join("bin"));
                    // Bun can also be installed globally via npm
                    paths.push(home_path.join(".npm-global").join("bin"));
                    // Homebrew path for bun
                    paths.push(PathBuf::from("/opt/homebrew/bin"));
                    paths.push(PathBuf::from("/usr/local/bin"));
                    // Manual installation path
                    paths.push(home_path.join(".local").join("bin"));
                }
                "bunx" => {
                    // Same as bun since bunx comes with bun
                    paths.push(home_path.join(".bun").join("bin"));
                    paths.push(home_path.join(".npm-global").join("bin"));
                    paths.push(PathBuf::from("/opt/homebrew/bin"));
                    paths.push(PathBuf::from("/usr/local/bin"));
                }
                "yarn" => {
                    // Yarn-specific paths
                    paths.push(home_path.join(".yarn").join("bin"));
                    paths.push(home_path.join(".npm-global").join("bin"));
                }
                "pnpm" => {
                    // pnpm-specific paths
                    paths.push(home_path.join(".local").join("share").join("pnpm"));
                    paths.push(home_path.join(".npm-global").join("bin"));
                }
                _ => {}
            }
        }
        
        // Windows-specific paths
        #[cfg(windows)]
        {
            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                let userprofile_path = PathBuf::from(userprofile);
                paths.push(userprofile_path.join(".local").join("bin"));
                paths.push(userprofile_path.join("scoop").join("shims"));
                
                // Cargo and Go paths on Windows
                paths.push(userprofile_path.join(".cargo").join("bin"));
                paths.push(userprofile_path.join("go").join("bin"));
            }
            if let Ok(appdata) = std::env::var("APPDATA") {
                paths.push(PathBuf::from(appdata).join("syncable-cli").join("bin"));
                // npm global on Windows
                paths.push(PathBuf::from(appdata).join("npm"));
            }
            // Program Files
            paths.push(PathBuf::from("C:\\Program Files"));
            paths.push(PathBuf::from("C:\\Program Files (x86)"));
        }
        
        // System-wide paths (usually already in PATH, but worth checking)
        paths.push(PathBuf::from("/usr/local/bin"));
        paths.push(PathBuf::from("/usr/bin"));
        paths.push(PathBuf::from("/bin"));
        
        // Remove duplicates and non-existent paths
        paths.sort();
        paths.dedup();
        paths.into_iter().filter(|p| p.exists()).collect()
    }
    
    /// Try to run a command in PATH
    fn try_command_in_path(&self, tool_name: &str) -> Option<(PathBuf, Option<String>)> {
        let version_args = self.get_version_args(tool_name);
        debug!("Trying {} with args: {:?}", tool_name, version_args);
        
        let output = Command::new(tool_name)
            .args(&version_args)
            .output()
            .ok()?;
            
        if output.status.success() {
            let version = self.parse_version_output(&output.stdout, tool_name);
            // Try to determine the actual path
            let path = self.find_tool_path(tool_name).unwrap_or_else(|| {
                PathBuf::from(tool_name) // Fallback to command name
            });
            return Some((path, version));
        }
        
        // For some tools, stderr might contain version info even on non-zero exit
        if !output.stderr.is_empty() {
            if let Some(version) = self.parse_version_output(&output.stderr, tool_name) {
                let path = self.find_tool_path(tool_name).unwrap_or_else(|| {
                    PathBuf::from(tool_name)
                });
                return Some((path, Some(version)));
            }
        }
        
        None
    }
    
    /// Verify tool installation at a specific path
    fn verify_tool_at_path(&self, tool_path: &Path, tool_name: &str) -> Option<String> {
        if !tool_path.exists() {
            return None;
        }
        
        let version_args = self.get_version_args(tool_name);
        debug!("Verifying {} at {:?} with args: {:?}", tool_name, tool_path, version_args);
        
        let output = Command::new(tool_path)
            .args(&version_args)
            .output()
            .ok()?;
            
        if output.status.success() {
            self.parse_version_output(&output.stdout, tool_name)
        } else if !output.stderr.is_empty() {
            // Some tools output version to stderr
            self.parse_version_output(&output.stderr, tool_name)
        } else {
            None
        }
    }
    
    /// Get appropriate version check arguments for each tool
    fn get_version_args(&self, tool_name: &str) -> Vec<&str> {
        match tool_name {
            "cargo-audit" => vec!["audit", "--version"],
            "npm" => vec!["--version"],
            "pip-audit" => vec!["--version"],
            "govulncheck" => vec!["-version"],
            "grype" => vec!["version"],
            "dependency-check" => vec!["--version"],
            "bun" => vec!["--version"],
            "bunx" => vec!["--version"],
            "yarn" => vec!["--version"],
            "pnpm" => vec!["--version"],
            _ => vec!["--version"],
        }
    }
    
    /// Parse version information from command output
    fn parse_version_output(&self, output: &[u8], tool_name: &str) -> Option<String> {
        let output_str = String::from_utf8_lossy(output);
        debug!("Parsing version output for {}: {}", tool_name, output_str.trim());
        
        // Tool-specific version parsing
        match tool_name {
            "cargo-audit" => {
                // Extract from "cargo-audit 0.18.3" or "cargo-audit-audit 0.18.3"
                for line in output_str.lines() {
                    if line.contains("cargo-audit") {
                        if let Some(version) = line.split_whitespace().nth(1) {
                            return Some(version.to_string());
                        }
                    }
                }
            }
            "grype" => {
                // Extract from "grype 0.92.2" or JSON format
                for line in output_str.lines() {
                    if line.trim_start().starts_with("grype") {
                        if let Some(version) = line.split_whitespace().nth(1) {
                            return Some(version.to_string());
                        }
                    }
                    // Also handle JSON format output
                    if line.contains("\"version\"") {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                            if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
                                return Some(version.to_string());
                            }
                        }
                    }
                }
            }
            "govulncheck" => {
                // Extract from "govulncheck@v1.0.4" or "go version devel +abc123"
                for line in output_str.lines() {
                    if let Some(at_pos) = line.find('@') {
                        let version_part = &line[at_pos + 1..];
                        if let Some(version) = version_part.split_whitespace().next() {
                            return Some(version.trim_start_matches('v').to_string());
                        }
                    }
                    // Also handle "govulncheck v1.0.4"
                    if line.contains("govulncheck") {
                        if let Some(version) = line.split_whitespace().nth(1) {
                            return Some(version.trim_start_matches('v').to_string());
                        }
                    }
                }
            }
            "npm" => {
                // Simple version number like "8.19.2"
                if let Some(first_line) = output_str.lines().next() {
                    let version = first_line.trim();
                    if !version.is_empty() {
                        return Some(version.to_string());
                    }
                }
            }
            "bun" | "bunx" => {
                // Bun version format: "1.0.3" or "bun 1.0.3"
                for line in output_str.lines() {
                    let line = line.trim();
                    // Handle "bun 1.0.3" format
                    if line.starts_with("bun ") {
                        if let Some(version) = line.split_whitespace().nth(1) {
                            return Some(version.to_string());
                        }
                    }
                    // Handle plain version number
                    if let Some(version) = extract_version_generic(line) {
                        return Some(version);
                    }
                }
            }
            "yarn" => {
                // Yarn version format: "1.22.19" or "4.0.2"
                if let Some(first_line) = output_str.lines().next() {
                    let version = first_line.trim();
                    if !version.is_empty() {
                        return Some(version.to_string());
                    }
                }
            }
            "pnpm" => {
                // pnpm version format: "8.10.0"
                if let Some(first_line) = output_str.lines().next() {
                    let version = first_line.trim();
                    if !version.is_empty() {
                        return Some(version.to_string());
                    }
                }
            }
            "pip-audit" => {
                // Extract from "pip-audit 2.6.1"
                for line in output_str.lines() {
                    if line.contains("pip-audit") {
                        if let Some(version) = line.split_whitespace().nth(1) {
                            return Some(version.to_string());
                        }
                    }
                }
                // Fallback to generic version extraction
                if let Some(version) = extract_version_generic(&output_str) {
                    return Some(version);
                }
            }
            _ => {
                // Generic version extraction
                if let Some(version) = extract_version_generic(&output_str) {
                    return Some(version);
                }
            }
        }
        
        None
    }
    
    /// Determine installation source based on path
    fn determine_installation_source(&self, path: &Path) -> InstallationSource {
        let path_str = path.to_string_lossy().to_lowercase();
        
        if path_str.contains(".cargo") {
            InstallationSource::CargoHome
        } else if path_str.contains("go/bin") || path_str.contains("gopath") {
            InstallationSource::GoHome
        } else if path_str.contains(".local") {
            InstallationSource::UserLocal
        } else if path_str.contains("homebrew") || path_str.contains("brew") {
            InstallationSource::PackageManager("brew".to_string())
        } else if path_str.contains("scoop") {
            InstallationSource::PackageManager("scoop".to_string())
        } else if path_str.contains("apt") || path_str.contains("/usr/bin") {
            InstallationSource::PackageManager("apt".to_string())
        } else if path_str.contains("/usr/local") || path_str.contains("/usr/bin") || path_str.contains("/bin") {
            InstallationSource::SystemPath
        } else {
            InstallationSource::Manual
        }
    }
    
    /// Find the actual path of a tool using system commands
    fn find_tool_path(&self, tool_name: &str) -> Option<PathBuf> {
        // Try 'which' on Unix systems
        #[cfg(unix)]
        {
            if let Ok(output) = Command::new("which").arg(tool_name).output() {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    let path_str = output_str.trim();
                    if !path_str.is_empty() {
                        return Some(PathBuf::from(path_str));
                    }
                }
            }
        }
        
        // Try 'where' on Windows
        #[cfg(windows)]
        {
            if let Ok(output) = Command::new("where").arg(tool_name).output() {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    let path_str = output_str.trim();
                    if let Some(first_path) = path_str.lines().next() {
                        if !first_path.is_empty() {
                            return Some(PathBuf::from(first_path));
                        }
                    }
                }
            }
        }
        
        None
    }
}

impl Default for ToolDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract version using common patterns
fn extract_version_generic(text: &str) -> Option<String> {
    // Look for semantic version patterns (x.y.z)
    use regex::Regex;
    
    let patterns = vec![
        // Standard semantic versioning
        r"\b(\d+\.\d+\.\d+(?:[+-][a-zA-Z0-9-.]+)?)\b",
        // Version with prefix
        r"\bv?(\d+\.\d+\.\d+)\b",
        // Simple x.y format
        r"\b(\d+\.\d+)\b",
    ];
    
    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(captures) = re.captures(text) {
                if let Some(version) = captures.get(1) {
                    let version_str = version.as_str();
                    // Avoid matching things like IP addresses or other numbers
                    if !version_str.starts_with("127.") && !version_str.starts_with("192.") {
                        return Some(version_str.to_string());
                    }
                }
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_extraction() {
        assert_eq!(extract_version_generic("cargo-audit 0.18.3"), Some("0.18.3".to_string()));
        assert_eq!(extract_version_generic("grype v0.92.2"), Some("0.92.2".to_string()));
        assert_eq!(extract_version_generic("8.19.2"), Some("8.19.2".to_string()));
        assert_eq!(extract_version_generic("pip-audit 2.6.1"), Some("2.6.1".to_string()));
        assert_eq!(extract_version_generic("version 1.0.4"), Some("1.0.4".to_string()));
        assert_eq!(extract_version_generic("bun 1.0.3"), Some("1.0.3".to_string()));
        assert_eq!(extract_version_generic("1.22.19"), Some("1.22.19".to_string()));
    }
    
    #[test]
    fn test_installation_source_detection() {
        let detector = ToolDetector::new();
        
        assert!(matches!(
            detector.determine_installation_source(&PathBuf::from("/home/user/.cargo/bin")),
            InstallationSource::CargoHome
        ));
        
        assert!(matches!(
            detector.determine_installation_source(&PathBuf::from("/home/user/go/bin")),
            InstallationSource::GoHome
        ));
        
        assert!(matches!(
            detector.determine_installation_source(&PathBuf::from("/opt/homebrew/bin")),
            InstallationSource::PackageManager(_)
        ));
    }
    
    #[test]
    fn test_version_args() {
        let detector = ToolDetector::new();
        
        assert_eq!(detector.get_version_args("cargo-audit"), vec!["audit", "--version"]);
        assert_eq!(detector.get_version_args("npm"), vec!["--version"]);
        assert_eq!(detector.get_version_args("govulncheck"), vec!["-version"]);
        assert_eq!(detector.get_version_args("grype"), vec!["version"]);
        assert_eq!(detector.get_version_args("bun"), vec!["--version"]);
        assert_eq!(detector.get_version_args("bunx"), vec!["--version"]);
        assert_eq!(detector.get_version_args("yarn"), vec!["--version"]);
        assert_eq!(detector.get_version_args("pnpm"), vec!["--version"]);
    }
    
    #[test]
    fn test_parse_version_output() {
        let detector = ToolDetector::new();
        
        // Test bun version parsing
        assert_eq!(
            detector.parse_version_output(b"1.0.3", "bun"),
            Some("1.0.3".to_string())
        );
        assert_eq!(
            detector.parse_version_output(b"bun 1.0.3", "bun"),
            Some("1.0.3".to_string())
        );
        
        // Test yarn version parsing
        assert_eq!(
            detector.parse_version_output(b"1.22.19", "yarn"),
            Some("1.22.19".to_string())
        );
        assert_eq!(
            detector.parse_version_output(b"4.0.2", "yarn"),
            Some("4.0.2".to_string())
        );
        
        // Test pnpm version parsing
        assert_eq!(
            detector.parse_version_output(b"8.10.0", "pnpm"),
            Some("8.10.0".to_string())
        );
        
        // Test npm version parsing
        assert_eq!(
            detector.parse_version_output(b"8.19.2", "npm"),
            Some("8.19.2".to_string())
        );
    }
    
    #[test]
    fn test_detect_all_vulnerability_tools_js() {
        let mut detector = ToolDetector::new();
        let languages = vec![
            crate::analyzer::dependency_parser::Language::JavaScript,
            crate::analyzer::dependency_parser::Language::TypeScript,
        ];
        
        let tools = detector.detect_all_vulnerability_tools(&languages);
        
        // Should include all JavaScript package managers
        assert!(tools.contains_key("bun"));
        assert!(tools.contains_key("npm"));
        assert!(tools.contains_key("yarn"));
        assert!(tools.contains_key("pnpm"));
        
        // Should not duplicate tools
        assert_eq!(tools.len(), 4); // bun, npm, yarn, pnpm
    }
    
    #[test]
    fn test_detect_js_package_managers() {
        let mut detector = ToolDetector::new();
        
        let managers = detector.detect_js_package_managers();
        
        assert_eq!(managers.len(), 4);
        assert!(managers.contains_key("bun"));
        assert!(managers.contains_key("npm"));
        assert!(managers.contains_key("yarn"));
        assert!(managers.contains_key("pnpm"));
        
        // All tools should have a status (available or not)
        for (name, status) in &managers {
            assert!(!name.is_empty());
            // last_checked should be recent
            assert!(status.last_checked.elapsed().unwrap().as_secs() < 10);
        }
    }
    
    #[test]
    fn test_detect_tool_with_alternatives() {
        let mut detector = ToolDetector::new();
        
        // Test bun detection with alternatives
        let status = detector.detect_tool_with_alternatives("bun", &["bun", "bunx"]);
        
        // Should have a status regardless of availability
        assert!(status.last_checked.elapsed().unwrap().as_secs() < 10);
        
        // Check that installation source is set
        match status.installation_source {
            InstallationSource::NotFound => {
                assert!(!status.available);
                assert!(status.path.is_none());
                assert!(status.version.is_none());
            }
            _ => {
                // If found, should have basic info
                assert!(status.available);
                // path and version may or may not be available depending on detection method
            }
        }
    }
    
    #[test]
    fn test_cache_functionality() {
        let mut detector = ToolDetector::new();
        
        // First detection
        let status1 = detector.detect_tool("bun");
        let time1 = status1.last_checked;
        
        // Should use cache for immediate second detection
        let status2 = detector.detect_tool("bun");
        let time2 = status2.last_checked;
        
        // Times should be the same (cache hit)
        assert_eq!(time1, time2);
        assert_eq!(status1.available, status2.available);
        
        // Clear cache
        detector.clear_cache();
        
        // Detection after cache clear should update timestamp
        let status3 = detector.detect_tool("bun");
        let time3 = status3.last_checked;
        
        // Time should be different (cache miss)
        assert!(time3 >= time1); // Should be same or later
    }
    
    #[test]
    fn test_get_tool_search_paths_bun() {
        let detector = ToolDetector::new();
        
        let paths = detector.get_tool_search_paths("bun");
        
        // Should include bun-specific paths
        let path_strings: Vec<String> = paths.iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        
        // At least one path should be bun-specific
        let has_bun_path = path_strings.iter().any(|p| p.contains(".bun"));
        
        // Should have common paths
        let has_local_bin = path_strings.iter().any(|p| p.contains(".local") && p.contains("bin"));
        
        // Note: We can't assert these are true since paths may not exist on test system
        // But we can verify the logic generates the expected paths
        assert!(!paths.is_empty()); // Should generate some paths
        
        // Verify no duplicate paths
        let mut sorted_paths = paths.clone();
        sorted_paths.sort();
        sorted_paths.dedup();
        assert_eq!(paths.len(), sorted_paths.len());
    }
    
    #[test]
    fn test_get_tool_search_paths_yarn_pnpm() {
        let detector = ToolDetector::new();
        
        // Test yarn paths
        let yarn_paths = detector.get_tool_search_paths("yarn");
        let yarn_strings: Vec<String> = yarn_paths.iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        
        // Test pnpm paths
        let pnpm_paths = detector.get_tool_search_paths("pnpm");
        let pnpm_strings: Vec<String> = pnpm_paths.iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        
        // Should generate some paths
        assert!(!yarn_paths.is_empty());
        assert!(!pnpm_paths.is_empty());
        
        // Should include specific directories for each package manager
        // Note: paths may not exist on test system, but should be generated
    }
}