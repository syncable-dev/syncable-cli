use crate::analyzer::dependency_parser::Language;
use crate::error::{AnalysisError, IaCGeneratorError, Result};
use log::{info, warn, debug};
use std::process::Command;
use std::collections::HashMap;

/// Tool installer for vulnerability scanning dependencies
pub struct ToolInstaller {
    installed_tools: HashMap<String, bool>,
}

impl ToolInstaller {
    pub fn new() -> Self {
        Self {
            installed_tools: HashMap::new(),
        }
    }
    
    /// Ensure all required tools for vulnerability scanning are available
    pub fn ensure_tools_for_languages(&mut self, languages: &[Language]) -> Result<()> {
        for language in languages {
            match language {
                Language::Rust => self.ensure_cargo_audit()?,
                Language::JavaScript | Language::TypeScript => self.ensure_npm()?,
                Language::Python => self.ensure_pip_audit()?,
                Language::Go => self.ensure_govulncheck()?,
                Language::Java | Language::Kotlin => self.ensure_grype()?,
                _ => {} // Unknown languages don't need tools
            }
        }
        Ok(())
    }
    
    /// Check if cargo-audit is installed, install if needed
    fn ensure_cargo_audit(&mut self) -> Result<()> {
        if self.is_tool_installed("cargo-audit") {
            return Ok(());
        }
        
        info!("🔧 Installing cargo-audit for Rust vulnerability scanning...");
        
        let output = Command::new("cargo")
            .args(&["install", "cargo-audit"])
            .output()
            .map_err(|e| IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "cargo-audit installation".to_string(),
                reason: format!("Failed to install cargo-audit: {}", e),
            }))?;
        
        if output.status.success() {
            info!("✅ cargo-audit installed successfully");
            self.installed_tools.insert("cargo-audit".to_string(), true);
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("❌ Failed to install cargo-audit: {}", stderr);
            return Err(IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "cargo-audit installation".to_string(),
                reason: format!("Installation failed: {}", stderr),
            }));
        }
        
        Ok(())
    }
    
    /// Check if npm is available (comes with Node.js)
    fn ensure_npm(&mut self) -> Result<()> {
        if self.is_tool_installed("npm") {
            return Ok(());
        }
        
        warn!("📦 npm not found. Please install Node.js from https://nodejs.org/");
        warn!("   npm audit is required for JavaScript/TypeScript vulnerability scanning");
        
        Ok(()) // Don't fail, just warn
    }
    
    /// Check if pip-audit is installed, install if needed
    fn ensure_pip_audit(&mut self) -> Result<()> {
        if self.is_tool_installed("pip-audit") {
            return Ok(());
        }
        
        info!("🔧 Installing pip-audit for Python vulnerability scanning...");
        
        // Try different installation methods
        let install_commands = vec![
            vec!["pipx", "install", "pip-audit"],
            vec!["pip3", "install", "--user", "pip-audit"],
            vec!["pip", "install", "--user", "pip-audit"],
        ];
        
        for cmd in install_commands {
            debug!("Trying installation command: {:?}", cmd);
            
            let output = Command::new(&cmd[0])
                .args(&cmd[1..])
                .output();
                
            if let Ok(result) = output {
                if result.status.success() {
                    info!("✅ pip-audit installed successfully using {}", cmd[0]);
                    self.installed_tools.insert("pip-audit".to_string(), true);
                    return Ok(());
                }
            }
        }
        
        warn!("📦 Failed to auto-install pip-audit. Please install manually:");
        warn!("   Option 1: pipx install pip-audit");
        warn!("   Option 2: pip3 install --user pip-audit");
        
        Ok(()) // Don't fail, just warn
    }
    
    /// Check if govulncheck is installed, install if needed
    fn ensure_govulncheck(&mut self) -> Result<()> {
        if self.is_tool_installed("govulncheck") {
            return Ok(());
        }
        
        info!("🔧 Installing govulncheck for Go vulnerability scanning...");
        
        let output = Command::new("go")
            .args(&["install", "golang.org/x/vuln/cmd/govulncheck@latest"])
            .output()
            .map_err(|e| IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "govulncheck installation".to_string(),
                reason: format!("Failed to install govulncheck (is Go installed?): {}", e),
            }))?;
        
        if output.status.success() {
            info!("✅ govulncheck installed successfully");
            self.installed_tools.insert("govulncheck".to_string(), true);
            
            // Also add Go bin directory to PATH hint
            info!("💡 Note: Make sure ~/go/bin is in your PATH to use govulncheck");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("❌ Failed to install govulncheck: {}", stderr);
            warn!("📦 Please install Go from https://golang.org/ first");
        }
        
        Ok(())
    }
    
    /// Check if Grype is available, install if possible
    fn ensure_grype(&mut self) -> Result<()> {
        if self.is_tool_installed("grype") {
            return Ok(());
        }
        
        info!("🔧 Installing grype for vulnerability scanning...");
        
        // Detect platform and architecture
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        
        // Try platform-specific installation methods
        match os {
            "macos" => {
                // Try to install with Homebrew
                let output = Command::new("brew")
                    .args(&["install", "grype"])
                    .output();
                    
                match output {
                    Ok(result) if result.status.success() => {
                        info!("✅ grype installed successfully via Homebrew");
                        self.installed_tools.insert("grype".to_string(), true);
                        return Ok(());
                    }
                    _ => {
                        warn!("❌ Failed to install via Homebrew. Trying manual installation...");
                    }
                }
            }
            _ => {}
        }
        
        // Try manual installation via curl
        self.install_grype_manually(os, arch)
    }
    
    /// Install grype manually by downloading from GitHub releases
    fn install_grype_manually(&mut self, os: &str, arch: &str) -> Result<()> {
        use std::fs;
        use std::path::PathBuf;
        
        info!("📥 Downloading grype from GitHub releases...");
        
        let version = "v0.92.2"; // Latest stable version
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let bin_dir = PathBuf::from(&home_dir).join(".local").join("bin");
        
        // Create bin directory
        fs::create_dir_all(&bin_dir).map_err(|e| {
            IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "grype installation".to_string(),
                reason: format!("Failed to create directory: {}", e),
            })
        })?;
        
        // Determine the correct binary name based on OS and architecture
        let (os_name, arch_name) = match (os, arch) {
            ("macos", "x86_64") => ("darwin", "amd64"),
            ("macos", "aarch64") => ("darwin", "arm64"),
            ("linux", "x86_64") => ("linux", "amd64"),
            ("linux", "aarch64") => ("linux", "arm64"),
            _ => {
                warn!("❌ Unsupported platform: {} {}", os, arch);
                return Ok(());
            }
        };
        
        let archive_name = format!("grype_{}_{}.tar.gz", os_name, arch_name);
        let download_url = format!(
            "https://github.com/anchore/grype/releases/download/{}/grype_{}_{}_{}.tar.gz",
            version, version.trim_start_matches('v'), os_name, arch_name
        );
        
        let archive_path = bin_dir.join(&archive_name);
        
        info!("📦 Downloading from: {}", download_url);
        let output = Command::new("curl")
            .args(&["-L", "-o", archive_path.to_str().unwrap(), &download_url])
            .output();
            
        match output {
            Ok(result) if result.status.success() => {
                info!("✅ Download complete. Extracting...");
                
                // Extract the archive
                let extract_output = Command::new("tar")
                    .args(&["-xzf", archive_path.to_str().unwrap(), "-C", bin_dir.to_str().unwrap()])
                    .output();
                    
                if extract_output.map(|o| o.status.success()).unwrap_or(false) {
                    // Make it executable
                    let grype_path = bin_dir.join("grype");
                    Command::new("chmod")
                        .args(&["+x", grype_path.to_str().unwrap()])
                        .output()
                        .ok();
                    
                    info!("✅ grype installed successfully to {}", bin_dir.display());
                    info!("💡 Make sure ~/.local/bin is in your PATH");
                    self.installed_tools.insert("grype".to_string(), true);
                    
                    // Clean up archive
                    fs::remove_file(&archive_path).ok();
                    
                    return Ok(());
                }
            }
            _ => {}
        }
        
        warn!("❌ Automatic installation failed. Please install manually:");
        warn!("   • macOS: brew install grype");
        warn!("   • Download: https://github.com/anchore/grype/releases");
        
        Ok(())
    }
    
    /// Check if OWASP dependency-check is available, install if possible
    fn ensure_dependency_check(&mut self) -> Result<()> {
        if self.is_tool_installed("dependency-check") {
            return Ok(());
        }
        
        info!("🔧 Installing dependency-check for Java/Kotlin vulnerability scanning...");
        
        // Detect platform and try to install
        let os = std::env::consts::OS;
        
        match os {
            "macos" => {
                // Try to install with Homebrew
                let output = Command::new("brew")
                    .args(&["install", "dependency-check"])
                    .output();
                    
                match output {
                    Ok(result) if result.status.success() => {
                        info!("✅ dependency-check installed successfully via Homebrew");
                        self.installed_tools.insert("dependency-check".to_string(), true);
                        return Ok(());
                    }
                    _ => {
                        warn!("❌ Failed to install via Homebrew. Trying manual installation...");
                    }
                }
            }
            "linux" => {
                // Try to install via snap
                let output = Command::new("snap")
                    .args(&["install", "dependency-check"])
                    .output();
                    
                if output.map(|o| o.status.success()).unwrap_or(false) {
                    info!("✅ dependency-check installed successfully via snap");
                    self.installed_tools.insert("dependency-check".to_string(), true);
                    return Ok(());
                }
            }
            _ => {}
        }
        
        // Try manual installation
        self.install_dependency_check_manually()
    }
    
    /// Install dependency-check manually by downloading from GitHub
    fn install_dependency_check_manually(&mut self) -> Result<()> {
        use std::fs;
        use std::path::PathBuf;
        
        info!("📥 Downloading dependency-check from GitHub releases...");
        
        let version = "10.0.4"; // Latest stable version
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let install_dir = PathBuf::from(&home_dir).join(".local").join("dependency-check");
        
        // Create installation directory
        fs::create_dir_all(&install_dir).map_err(|e| {
            IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "dependency-check installation".to_string(),
                reason: format!("Failed to create directory: {}", e),
            })
        })?;
        
        let archive_name = format!("dependency-check-{}-release.zip", version);
        let download_url = format!(
            "https://github.com/jeremylong/DependencyCheck/releases/download/v{}/{}",
            version, archive_name
        );
        
        // Download the archive
        let archive_path = install_dir.join(&archive_name);
        
        info!("📦 Downloading from: {}", download_url);
        let output = Command::new("curl")
            .args(&["-L", "-o", archive_path.to_str().unwrap(), &download_url])
            .output();
            
        match output {
            Ok(result) if result.status.success() => {
                info!("✅ Download complete. Extracting...");
                
                // Extract the archive
                let extract_output = Command::new("unzip")
                    .args(&["-o", archive_path.to_str().unwrap(), "-d", install_dir.to_str().unwrap()])
                    .output();
                    
                if extract_output.map(|o| o.status.success()).unwrap_or(false) {
                    // Create symlink to make it available in PATH
                    let bin_dir = PathBuf::from(&home_dir).join(".local").join("bin");
                    fs::create_dir_all(&bin_dir).ok();
                    
                    let dc_script = install_dir.join("dependency-check").join("bin").join("dependency-check.sh");
                    let symlink = bin_dir.join("dependency-check");
                    
                    // Remove old symlink if exists
                    fs::remove_file(&symlink).ok();
                    
                    // Create new symlink
                    if std::os::unix::fs::symlink(&dc_script, &symlink).is_ok() {
                        info!("✅ dependency-check installed successfully to {}", install_dir.display());
                        info!("💡 Added to ~/.local/bin/dependency-check");
                        info!("💡 Make sure ~/.local/bin is in your PATH");
                        self.installed_tools.insert("dependency-check".to_string(), true);
                        return Ok(());
                    }
                }
            }
            _ => {}
        }
        
        warn!("❌ Automatic installation failed. Please install manually:");
        warn!("   • macOS: brew install dependency-check");
        warn!("   • Download: https://github.com/jeremylong/DependencyCheck/releases");
        warn!("   • Documentation: https://owasp.org/www-project-dependency-check/");
        
        Ok(())
    }
    
    /// Check if a command-line tool is available
    fn is_tool_installed(&mut self, tool: &str) -> bool {
        // Check cache first
        if let Some(&cached) = self.installed_tools.get(tool) {
            return cached;
        }
        
        // Test if tool is available
        let available = self.test_tool_availability(tool);
        self.installed_tools.insert(tool.to_string(), available);
        available
    }
    
    /// Test if a tool is available by running --version
    fn test_tool_availability(&self, tool: &str) -> bool {
        let test_commands = match tool {
            "cargo-audit" => vec!["cargo", "audit", "--version"],
            "npm" => vec!["npm", "--version"],
            "pip-audit" => vec!["pip-audit", "--version"],
            "govulncheck" => vec!["govulncheck", "-version"],
            "dependency-check" => vec!["dependency-check", "--version"],
            "grype" => vec!["grype", "version"],
            _ => return false,
        };
        
        let result = Command::new(&test_commands[0])
            .args(&test_commands[1..])
            .output();
            
        match result {
            Ok(output) => output.status.success(),
            Err(_) => {
                // Try with ~/go/bin prefix for Go tools
                if tool == "govulncheck" {
                    let go_bin_path = std::env::var("HOME")
                        .map(|home| format!("{}/go/bin/govulncheck", home))
                        .unwrap_or_else(|_| "govulncheck".to_string());
                        
                    return Command::new(&go_bin_path)
                        .arg("-version")
                        .output()
                        .map(|out| out.status.success())
                        .unwrap_or(false);
                }
                
                // Try with ~/.local/bin prefix for dependency-check
                if tool == "dependency-check" {
                    let dc_path = std::env::var("HOME")
                        .map(|home| format!("{}/.local/bin/dependency-check", home))
                        .unwrap_or_else(|_| "dependency-check".to_string());
                        
                    return Command::new(&dc_path)
                        .arg("--version")
                        .output()
                        .map(|out| out.status.success())
                        .unwrap_or(false);
                }
                
                // Try with ~/.local/bin prefix for grype
                if tool == "grype" {
                    let grype_path = std::env::var("HOME")
                        .map(|home| format!("{}/.local/bin/grype", home))
                        .unwrap_or_else(|_| "grype".to_string());
                        
                    return Command::new(&grype_path)
                        .arg("version")
                        .output()
                        .map(|out| out.status.success())
                        .unwrap_or(false);
                }
                
                false
            }
        }
    }
    
    /// Get installation status summary
    pub fn get_tool_status(&self) -> HashMap<String, bool> {
        self.installed_tools.clone()
    }
    
    /// Print tool installation status
    pub fn print_tool_status(&self, languages: &[Language]) {
        println!("\n🔧 Vulnerability Scanning Tools Status:");
        println!("{}", "=".repeat(50));
        
        for language in languages {
            let (tool, status) = match language {
                Language::Rust => ("cargo-audit", self.installed_tools.get("cargo-audit").unwrap_or(&false)),
                Language::JavaScript | Language::TypeScript => ("npm", self.installed_tools.get("npm").unwrap_or(&false)),
                Language::Python => ("pip-audit", self.installed_tools.get("pip-audit").unwrap_or(&false)),
                Language::Go => ("govulncheck", self.installed_tools.get("govulncheck").unwrap_or(&false)),
                Language::Java | Language::Kotlin => ("grype", self.installed_tools.get("grype").unwrap_or(&false)),
                _ => continue,
            };
            
            let status_icon = if *status { "✅" } else { "❌" };
            println!("  {} {:?}: {} {}", status_icon, language, tool, if *status { "installed" } else { "missing" });
        }
        println!();
    }
} 