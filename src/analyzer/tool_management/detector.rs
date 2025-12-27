use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub available: bool,
    pub path: Option<PathBuf>,
    pub execution_path: Option<PathBuf>, // Path to use for execution
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
    PackageManager(String),
    Manual,
    NotFound,
}

#[derive(Debug, Clone)]
pub struct ToolDetectionConfig {
    pub cache_ttl: Duration,
    pub enable_cache: bool,
    pub search_user_paths: bool,
    pub search_system_paths: bool,
}

impl Default for ToolDetectionConfig {
    fn default() -> Self {
        Self {
            cache_ttl: Duration::from_secs(300), // 5 minutes
            enable_cache: true,
            search_user_paths: true,
            search_system_paths: true,
        }
    }
}

pub struct ToolDetector {
    cache: HashMap<String, ToolStatus>,
    config: ToolDetectionConfig,
}

impl ToolDetector {
    pub fn new() -> Self {
        Self::with_config(ToolDetectionConfig::default())
    }

    pub fn with_config(config: ToolDetectionConfig) -> Self {
        Self {
            cache: HashMap::new(),
            config,
        }
    }

    /// Detect tool availability with caching
    pub fn detect_tool(&mut self, tool_name: &str) -> ToolStatus {
        if !self.config.enable_cache {
            return self.detect_tool_real_time(tool_name);
        }

        // Check cache first
        if let Some(cached) = self.cache.get(tool_name)
            && cached.last_checked.elapsed().unwrap_or(Duration::MAX) < self.config.cache_ttl
        {
            debug!(
                "Using cached status for {}: available={}",
                tool_name, cached.available
            );
            return cached.clone();
        }

        // Perform real detection
        let status = self.detect_tool_real_time(tool_name);
        debug!(
            "Real-time detection for {}: available={}, path={:?}",
            tool_name, status.available, status.path
        );
        self.cache.insert(tool_name.to_string(), status.clone());
        status
    }

    /// Detect all vulnerability scanning tools for given languages
    pub fn detect_all_vulnerability_tools(
        &mut self,
        languages: &[crate::analyzer::dependency_parser::Language],
    ) -> HashMap<String, ToolStatus> {
        let mut results = HashMap::new();

        for language in languages {
            let tool_names = self.get_tools_for_language(language);

            for tool_name in tool_names {
                if !results.contains_key(tool_name) {
                    results.insert(tool_name.to_string(), self.detect_tool(tool_name));
                }
            }
        }

        results
    }

    fn get_tools_for_language(
        &self,
        language: &crate::analyzer::dependency_parser::Language,
    ) -> Vec<&'static str> {
        match language {
            crate::analyzer::dependency_parser::Language::Rust => vec!["cargo-audit"],
            crate::analyzer::dependency_parser::Language::JavaScript
            | crate::analyzer::dependency_parser::Language::TypeScript => {
                vec!["bun", "npm", "yarn", "pnpm"]
            }
            crate::analyzer::dependency_parser::Language::Python => vec!["pip-audit"],
            crate::analyzer::dependency_parser::Language::Go => vec!["govulncheck"],
            crate::analyzer::dependency_parser::Language::Java
            | crate::analyzer::dependency_parser::Language::Kotlin => vec!["grype"],
            _ => vec![],
        }
    }

    /// Clear the cache to force fresh detection
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Detect bun specifically with multiple alternatives
    pub fn detect_bun(&mut self) -> ToolStatus {
        self.detect_tool_with_alternatives("bun", &["bun", "bunx"])
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
    pub fn detect_tool_with_alternatives(
        &mut self,
        primary_name: &str,
        alternatives: &[&str],
    ) -> ToolStatus {
        // Check cache first for primary name
        if self.config.enable_cache
            && let Some(cached) = self.cache.get(primary_name)
            && cached.last_checked.elapsed().unwrap_or(Duration::MAX) < self.config.cache_ttl
        {
            debug!(
                "Using cached status for {}: available={}",
                primary_name, cached.available
            );
            return cached.clone();
        }

        // Try each alternative
        for alternative in alternatives {
            debug!("Trying to detect tool: {}", alternative);
            let status = self.detect_tool_real_time(alternative);
            if status.available {
                debug!("Found {} via alternative: {}", primary_name, alternative);
                if self.config.enable_cache {
                    self.cache.insert(primary_name.to_string(), status.clone());
                }
                return status;
            }
        }

        // Not found
        let not_found = ToolStatus {
            available: false,
            path: None,
            execution_path: None,
            version: None,
            installation_source: InstallationSource::NotFound,
            last_checked: SystemTime::now(),
        };

        if self.config.enable_cache {
            self.cache
                .insert(primary_name.to_string(), not_found.clone());
        }
        not_found
    }

    /// Perform real-time tool detection without caching
    fn detect_tool_real_time(&self, tool_name: &str) -> ToolStatus {
        debug!("Starting real-time detection for {}", tool_name);

        // Try direct command first (in PATH)
        if let Some((path, version)) = self.try_command_in_path(tool_name) {
            info!(
                "Found {} in PATH at {:?} with version {:?}",
                tool_name, path, version
            );
            return ToolStatus {
                available: true,
                path: Some(path),
                execution_path: None, // Execute by name when in PATH
                version,
                installation_source: InstallationSource::SystemPath,
                last_checked: SystemTime::now(),
            };
        }

        // Try alternative paths if enabled
        if self.config.search_user_paths || self.config.search_system_paths {
            let search_paths = self.get_tool_search_paths(tool_name);
            debug!(
                "Searching alternative paths for {}: {:?}",
                tool_name, search_paths
            );

            for search_path in search_paths {
                let tool_path = search_path.join(tool_name);
                debug!("Checking path: {:?}", tool_path);

                if let Some(version) = self.verify_tool_at_path(&tool_path, tool_name) {
                    let source = self.determine_installation_source(&search_path);
                    info!(
                        "Found {} at {:?} with version {:?} (source: {:?})",
                        tool_name, tool_path, version, source
                    );
                    return ToolStatus {
                        available: true,
                        path: Some(tool_path.clone()),
                        execution_path: Some(tool_path), // Use full path for execution
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
                        info!(
                            "Found {} at {:?} with version {:?} (source: {:?})",
                            tool_name, tool_path_exe, version, source
                        );
                        return ToolStatus {
                            available: true,
                            path: Some(tool_path_exe.clone()),
                            execution_path: Some(tool_path_exe), // Use full path for execution
                            version: Some(version),
                            installation_source: source,
                            last_checked: SystemTime::now(),
                        };
                    }
                }
            }
        }

        // Tool not found
        debug!("Tool {} not found in any location", tool_name);
        ToolStatus {
            available: false,
            path: None,
            execution_path: None,
            version: None,
            installation_source: InstallationSource::NotFound,
            last_checked: SystemTime::now(),
        }
    }

    /// Get search paths for a specific tool
    fn get_tool_search_paths(&self, tool_name: &str) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        if !self.config.search_user_paths && !self.config.search_system_paths {
            return paths;
        }

        // User-specific paths
        if self.config.search_user_paths
            && let Ok(home) = std::env::var("HOME")
        {
            let home_path = PathBuf::from(home);

            // Common user install locations
            paths.push(home_path.join(".local").join("bin"));
            paths.push(home_path.join(".cargo").join("bin"));
            paths.push(home_path.join("go").join("bin"));

            // Tool-specific locations
            self.add_tool_specific_paths(tool_name, &home_path, &mut paths);

            // Windows-specific paths
            #[cfg(windows)]
            {
                if let Ok(userprofile) = std::env::var("USERPROFILE") {
                    let userprofile_path = PathBuf::from(userprofile);
                    paths.push(userprofile_path.join(".local").join("bin"));
                    paths.push(userprofile_path.join("scoop").join("shims"));
                    paths.push(userprofile_path.join(".cargo").join("bin"));
                    paths.push(userprofile_path.join("go").join("bin"));
                }
                if let Ok(appdata) = std::env::var("APPDATA") {
                    paths.push(PathBuf::from(&appdata).join("syncable-cli").join("bin"));
                    paths.push(PathBuf::from(&appdata).join("npm"));
                }
                // Program Files
                paths.push(PathBuf::from("C:\\Program Files"));
                paths.push(PathBuf::from("C:\\Program Files (x86)"));
            }
        }

        // System-wide paths
        if self.config.search_system_paths {
            paths.push(PathBuf::from("/usr/local/bin"));
            paths.push(PathBuf::from("/usr/bin"));
            paths.push(PathBuf::from("/bin"));
        }

        // Remove duplicates and non-existent paths
        paths.sort();
        paths.dedup();
        paths.into_iter().filter(|p| p.exists()).collect()
    }

    fn add_tool_specific_paths(&self, tool_name: &str, home_path: &Path, paths: &mut Vec<PathBuf>) {
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
                paths.push(PathBuf::from("/opt/homebrew/bin"));
                paths.push(PathBuf::from("/usr/local/bin"));
            }
            "pip-audit" => {
                paths.push(home_path.join(".local").join("bin"));
                if let Ok(output) = Command::new("python3")
                    .args(["--", "site", "--user-base"])
                    .output()
                    && let Ok(user_base) = String::from_utf8(output.stdout)
                {
                    paths.push(PathBuf::from(user_base.trim()).join("bin"));
                }
                if let Ok(output) = Command::new("python")
                    .args(["-m", "site", "--user-base"])
                    .output()
                    && let Ok(user_base) = String::from_utf8(output.stdout)
                {
                    paths.push(PathBuf::from(user_base.trim()).join("bin"));
                }
            }
            "bun" | "bunx" => {
                paths.push(home_path.join(".bun").join("bin"));
                paths.push(home_path.join(".npm-global").join("bin"));
                paths.push(PathBuf::from("/opt/homebrew/bin"));
                paths.push(PathBuf::from("/usr/local/bin"));
                paths.push(home_path.join(".local").join("bin"));
            }
            "yarn" => {
                paths.push(home_path.join(".yarn").join("bin"));
                paths.push(home_path.join(".npm-global").join("bin"));
            }
            "pnpm" => {
                paths.push(home_path.join(".local").join("share").join("pnpm"));
                paths.push(home_path.join(".npm-global").join("bin"));
            }
            "npm" => {
                if let Ok(node_path) = std::env::var("NODE_PATH") {
                    paths.push(PathBuf::from(node_path).join(".bin"));
                }
                paths.push(home_path.join(".npm-global").join("bin"));
                paths.push(PathBuf::from("/usr/local/lib/node_modules/.bin"));
            }
            _ => {}
        }
    }

    /// Try to run a command in PATH
    fn try_command_in_path(&self, tool_name: &str) -> Option<(PathBuf, Option<String>)> {
        let version_args = self.get_version_args(tool_name);
        debug!("Trying {} with args: {:?}", tool_name, version_args);

        let output = Command::new(tool_name).args(&version_args).output().ok()?;

        if output.status.success() {
            let version = self.parse_version_output(&output.stdout, tool_name);
            let path = self
                .find_tool_path(tool_name)
                .unwrap_or_else(|| PathBuf::from(tool_name));
            return Some((path, version));
        }

        // For some tools, stderr might contain version info even on non-zero exit
        if !output.stderr.is_empty()
            && let Some(version) = self.parse_version_output(&output.stderr, tool_name)
        {
            let path = self
                .find_tool_path(tool_name)
                .unwrap_or_else(|| PathBuf::from(tool_name));
            return Some((path, Some(version)));
        }

        None
    }

    /// Verify tool installation at a specific path
    fn verify_tool_at_path(&self, tool_path: &Path, tool_name: &str) -> Option<String> {
        if !tool_path.exists() {
            return None;
        }

        let version_args = self.get_version_args(tool_name);
        debug!(
            "Verifying {} at {:?} with args: {:?}",
            tool_name, tool_path, version_args
        );

        let output = Command::new(tool_path).args(&version_args).output().ok()?;

        if output.status.success() {
            self.parse_version_output(&output.stdout, tool_name)
        } else if !output.stderr.is_empty() {
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
        debug!(
            "Parsing version output for {}: {}",
            tool_name,
            output_str.trim()
        );

        match tool_name {
            "cargo-audit" => {
                for line in output_str.lines() {
                    if line.contains("cargo-audit")
                        && let Some(version) = line.split_whitespace().nth(1)
                    {
                        return Some(version.to_string());
                    }
                }
            }
            "grype" => {
                for line in output_str.lines() {
                    if line.trim_start().starts_with("grype")
                        && let Some(version) = line.split_whitespace().nth(1)
                    {
                        return Some(version.to_string());
                    }
                    if line.contains("\"version\"")
                        && let Ok(json) = serde_json::from_str::<serde_json::Value>(line)
                        && let Some(version) = json.get("version").and_then(|v| v.as_str())
                    {
                        return Some(version.to_string());
                    }
                }
            }
            "govulncheck" => {
                for line in output_str.lines() {
                    if let Some(at_pos) = line.find('@') {
                        let version_part = &line[at_pos + 1..];
                        if let Some(version) = version_part.split_whitespace().next() {
                            return Some(version.trim_start_matches('v').to_string());
                        }
                    }
                    if line.contains("govulncheck")
                        && let Some(version) = line.split_whitespace().nth(1)
                    {
                        return Some(version.trim_start_matches('v').to_string());
                    }
                }
            }
            "npm" | "yarn" | "pnpm" => {
                if let Some(first_line) = output_str.lines().next() {
                    let version = first_line.trim();
                    if !version.is_empty() {
                        return Some(version.to_string());
                    }
                }
            }
            "bun" | "bunx" => {
                for line in output_str.lines() {
                    let line = line.trim();
                    if line.starts_with("bun ")
                        && let Some(version) = line.split_whitespace().nth(1)
                    {
                        return Some(version.to_string());
                    }
                    if let Some(version) = extract_version_generic(line) {
                        return Some(version);
                    }
                }
            }
            "pip-audit" => {
                for line in output_str.lines() {
                    if line.contains("pip-audit")
                        && let Some(version) = line.split_whitespace().nth(1)
                    {
                        return Some(version.to_string());
                    }
                }
                if let Some(version) = extract_version_generic(&output_str) {
                    return Some(version);
                }
            }
            _ => {
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
        } else if path_str.contains("/usr/local")
            || path_str.contains("/usr/bin")
            || path_str.contains("/bin")
        {
            InstallationSource::SystemPath
        } else {
            InstallationSource::Manual
        }
    }

    /// Find the actual path of a tool using system commands
    fn find_tool_path(&self, tool_name: &str) -> Option<PathBuf> {
        #[cfg(unix)]
        {
            if let Ok(output) = Command::new("which").arg(tool_name).output()
                && output.status.success()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let path_str = output_str.trim();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }
        }

        #[cfg(windows)]
        {
            if let Ok(output) = Command::new("where").arg(tool_name).output()
                && output.status.success()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let path_str = output_str.trim();
                if let Some(first_path) = path_str.lines().next()
                    && !first_path.is_empty()
                {
                    return Some(PathBuf::from(first_path));
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
    use regex::Regex;

    let patterns = vec![
        r"\b(\d+\.\d+\.\d+(?:[+-][a-zA-Z0-9-.]+)?)\b",
        r"\bv?(\d+\.\d+\.\d+)\b",
        r"\b(\d+\.\d+)\b",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern)
            && let Some(captures) = re.captures(text)
            && let Some(version) = captures.get(1)
        {
            let version_str = version.as_str();
            if !version_str.starts_with("127.") && !version_str.starts_with("192.") {
                return Some(version_str.to_string());
            }
        }
    }

    None
}
