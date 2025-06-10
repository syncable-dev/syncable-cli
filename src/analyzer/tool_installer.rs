use crate::analyzer::dependency_parser::Language;
use crate::error::{AnalysisError, IaCGeneratorError, Result};
use log::{info, warn, debug};
use std::process::Command;
use std::collections::HashMap;
use std::path::PathBuf;

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
        
        // Use platform-appropriate directories
        let bin_dir = if cfg!(windows) {
            // On Windows, use %USERPROFILE%\.local\bin or %APPDATA%\syncable-cli\bin
            let home_dir = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("APPDATA"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(&home_dir).join(".local").join("bin")
        } else {
            // On Unix systems, use $HOME/.local/bin
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(&home_dir).join(".local").join("bin")
        };
        
        // Create bin directory
        fs::create_dir_all(&bin_dir).map_err(|e| {
            IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "grype installation".to_string(),
                reason: format!("Failed to create directory: {}", e),
            })
        })?;
        
        // Determine the correct binary name based on OS and architecture
        let (os_name, arch_name, file_extension) = match (os, arch) {
            ("macos", "x86_64") => ("darwin", "amd64", ""),
            ("macos", "aarch64") => ("darwin", "arm64", ""),
            ("linux", "x86_64") => ("linux", "amd64", ""),
            ("linux", "aarch64") => ("linux", "arm64", ""),
            ("windows", "x86_64") => ("windows", "amd64", ".exe"),
            ("windows", "aarch64") => ("windows", "arm64", ".exe"),
            _ => {
                warn!("❌ Unsupported platform: {} {}", os, arch);
                return Ok(());
            }
        };
        
        // Windows uses zip files, Unix uses tar.gz
        let (archive_name, download_url) = if cfg!(windows) {
            let archive_name = format!("grype_{}_windows_{}.zip", version.trim_start_matches('v'), arch_name);
            let download_url = format!(
                "https://github.com/anchore/grype/releases/download/{}/{}",
                version, archive_name
            );
            (archive_name, download_url)
        } else {
            let archive_name = format!("grype_{}_{}.tar.gz", os_name, arch_name);
            let download_url = format!(
                "https://github.com/anchore/grype/releases/download/{}/grype_{}_{}_{}.tar.gz",
                version, version.trim_start_matches('v'), os_name, arch_name
            );
            (archive_name, download_url)
        };
        
        let archive_path = bin_dir.join(&archive_name);
        let grype_binary = bin_dir.join(format!("grype{}", file_extension));
        
        info!("📦 Downloading from: {}", download_url);
        
        // Use platform-appropriate download method
        let download_success = if cfg!(windows) {
            // On Windows, try PowerShell first, then curl if available
            self.download_file_windows(&download_url, &archive_path)
        } else {
            // On Unix, use curl
            self.download_file_unix(&download_url, &archive_path)
        };
        
        if download_success {
            info!("✅ Download complete. Extracting...");
            
            let extract_success = if cfg!(windows) {
                self.extract_zip_windows(&archive_path, &bin_dir)
            } else {
                self.extract_tar_unix(&archive_path, &bin_dir)
            };
            
            if extract_success {
                info!("✅ grype installed successfully to {}", bin_dir.display());
                if cfg!(windows) {
                    info!("💡 Make sure {} is in your PATH", bin_dir.display());
                } else {
                    info!("💡 Make sure ~/.local/bin is in your PATH");
                }
                self.installed_tools.insert("grype".to_string(), true);
                
                // Clean up archive
                fs::remove_file(&archive_path).ok();
                
                return Ok(());
            }
        }
        
        warn!("❌ Automatic installation failed. Please install manually:");
        if cfg!(windows) {
            warn!("   • Download from: https://github.com/anchore/grype/releases");
            warn!("   • Or use: scoop install grype (if you have Scoop)");
        } else {
            warn!("   • macOS: brew install grype");
            warn!("   • Download: https://github.com/anchore/grype/releases");
        }
        
        Ok(())
    }
    
    /// Download file on Windows using PowerShell or curl
    fn download_file_windows(&self, url: &str, output_path: &PathBuf) -> bool {
        use std::process::Command;
        
        // Try PowerShell first (available on all modern Windows)
        let powershell_result = Command::new("powershell")
            .args(&[
                "-Command",
                &format!(
                    "Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
                    url,
                    output_path.to_string_lossy()
                )
            ])
            .output();
            
        if let Ok(result) = powershell_result {
            if result.status.success() {
                return true;
            }
        }
        
        // Fallback to curl if available
        let curl_result = Command::new("curl")
            .args(&["-L", "-o", &output_path.to_string_lossy(), url])
            .output();
            
        curl_result.map(|o| o.status.success()).unwrap_or(false)
    }
    
    /// Download file on Unix using curl
    fn download_file_unix(&self, url: &str, output_path: &PathBuf) -> bool {
        use std::process::Command;
        
        let output = Command::new("curl")
            .args(&["-L", "-o", &output_path.to_string_lossy(), url])
            .output();
            
        output.map(|o| o.status.success()).unwrap_or(false)
    }
    
    /// Extract ZIP file on Windows
    fn extract_zip_windows(&self, archive_path: &PathBuf, extract_dir: &PathBuf) -> bool {
        use std::process::Command;
        
        // Try PowerShell Expand-Archive first
        let powershell_result = Command::new("powershell")
            .args(&[
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    archive_path.to_string_lossy(),
                    extract_dir.to_string_lossy()
                )
            ])
            .output();
            
        if let Ok(result) = powershell_result {
            if result.status.success() {
                return true;
            }
        }
        
        // Fallback: try tar (available in newer Windows versions)
        let tar_result = Command::new("tar")
            .args(&["-xf", &archive_path.to_string_lossy(), "-C", &extract_dir.to_string_lossy()])
            .output();
            
        tar_result.map(|o| o.status.success()).unwrap_or(false)
    }
    
    /// Extract TAR file on Unix
    fn extract_tar_unix(&self, archive_path: &PathBuf, extract_dir: &PathBuf) -> bool {
        use std::process::Command;
        
        let extract_output = Command::new("tar")
            .args(&["-xzf", &archive_path.to_string_lossy(), "-C", &extract_dir.to_string_lossy()])
            .output();
            
        if let Ok(result) = extract_output {
            if result.status.success() {
                // Make it executable on Unix
                #[cfg(unix)]
                {
                    let grype_path = extract_dir.join("grype");
                    Command::new("chmod")
                        .args(&["+x", &grype_path.to_string_lossy()])
                        .output()
                        .ok();
                }
                return true;
            }
        }
        
        false
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
        
        let version = "11.1.0"; // Latest stable version
        
        // Use platform-appropriate directories
        let (home_dir, install_dir) = if cfg!(windows) {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("APPDATA"))
                .unwrap_or_else(|_| ".".to_string());
            let install = PathBuf::from(&home).join("dependency-check");
            (home, install)
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let install = PathBuf::from(&home).join(".local").join("share").join("dependency-check");
            (home, install)
        };
        
        // Create installation directory
        fs::create_dir_all(&install_dir).map_err(|e| {
            IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "dependency-check installation".to_string(),
                reason: format!("Failed to create directory: {}", e),
            })
        })?;
        
        let archive_name = "dependency-check-11.1.0-release.zip";
        let download_url = format!(
            "https://github.com/jeremylong/DependencyCheck/releases/download/v{}/{}",
            version, archive_name
        );
        
        let archive_path = install_dir.join(archive_name);
        
        info!("📦 Downloading from: {}", download_url);
        
        // Use platform-appropriate download method
        let download_success = if cfg!(windows) {
            self.download_file_windows(&download_url, &archive_path)
        } else {
            self.download_file_unix(&download_url, &archive_path)
        };
        
        if download_success {
            info!("✅ Download complete. Extracting...");
            
            let extract_success = if cfg!(windows) {
                self.extract_zip_windows(&archive_path, &install_dir)
            } else {
                // Use unzip on Unix for .zip files
                let output = std::process::Command::new("unzip")
                    .args(&["-o", &archive_path.to_string_lossy(), "-d", &install_dir.to_string_lossy()])
                    .output();
                output.map(|o| o.status.success()).unwrap_or(false)
            };
                
            if extract_success {
                // Create appropriate launcher
                if cfg!(windows) {
                    self.create_windows_launcher(&install_dir, &home_dir)?;
                } else {
                    self.create_unix_launcher(&install_dir, &home_dir)?;
                }
                
                info!("✅ dependency-check installed successfully to {}", install_dir.display());
                self.installed_tools.insert("dependency-check".to_string(), true);
                
                // Clean up archive
                fs::remove_file(&archive_path).ok();
                return Ok(());
            }
        }
        
        warn!("❌ Automatic installation failed. Please install manually:");
        if cfg!(windows) {
            warn!("   • Download: https://github.com/jeremylong/DependencyCheck/releases");
            warn!("   • Or use: scoop install dependency-check (if you have Scoop)");
        } else {
            warn!("   • macOS: brew install dependency-check");
            warn!("   • Download: https://github.com/jeremylong/DependencyCheck/releases");
        }
        
        Ok(())
    }
    
    /// Create Windows launcher for dependency-check
    fn create_windows_launcher(&self, install_dir: &PathBuf, home_dir: &str) -> Result<()> {
        use std::fs;
        
        let bin_dir = PathBuf::from(home_dir).join(".local").join("bin");
        fs::create_dir_all(&bin_dir).ok();
        
        let dc_script = install_dir.join("dependency-check").join("bin").join("dependency-check.bat");
        let launcher_path = bin_dir.join("dependency-check.bat");
        
        // Create a batch file launcher
        let launcher_content = format!(
            "@echo off\n\"{}\" %*\n",
            dc_script.to_string_lossy()
        );
        
        fs::write(&launcher_path, launcher_content).map_err(|e| {
            IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "dependency-check launcher".to_string(),
                reason: format!("Failed to create launcher: {}", e),
            })
        })?;
        
        info!("💡 Added to {}", launcher_path.display());
        info!("💡 Make sure {} is in your PATH", bin_dir.display());
        
        Ok(())
    }
    
    /// Create Unix launcher for dependency-check
    fn create_unix_launcher(&self, install_dir: &PathBuf, home_dir: &str) -> Result<()> {
        use std::fs;
        
        let bin_dir = PathBuf::from(home_dir).join(".local").join("bin");
        fs::create_dir_all(&bin_dir).ok();
        
        let dc_script = install_dir.join("dependency-check").join("bin").join("dependency-check.sh");
        let symlink = bin_dir.join("dependency-check");
        
        // Remove old symlink if exists
        fs::remove_file(&symlink).ok();
        
        // Create new symlink (Unix only)
        #[cfg(unix)]
        {
            if std::os::unix::fs::symlink(&dc_script, &symlink).is_ok() {
                info!("💡 Added to ~/.local/bin/dependency-check");
                info!("💡 Make sure ~/.local/bin is in your PATH");
                return Ok(());
            }
        }
        
        // Fallback: create a shell script wrapper
        let wrapper_content = format!(
            "#!/bin/bash\nexec \"{}\" \"$@\"\n",
            dc_script.to_string_lossy()
        );
        
        fs::write(&symlink, wrapper_content).map_err(|e| {
            IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "dependency-check wrapper".to_string(),
                reason: format!("Failed to create wrapper: {}", e),
            })
        })?;
        
        // Make executable
        #[cfg(unix)]
        {
            use std::process::Command;
            Command::new("chmod")
                .args(&["+x", &symlink.to_string_lossy()])
                .output()
                .ok();
        }
        
        Ok(())
    }
    
    /// Check if a tool is installed and available
    fn is_tool_installed(&self, tool: &str) -> bool {
        use std::process::Command;
        
        // Check cache first
        if let Some(&cached) = self.installed_tools.get(tool) {
            return cached;
        }
        
        // Different version check commands for different tools
        let version_arg = match tool {
            "grype" => "version",
            "cargo-audit" => "--version",
            "pip-audit" => "--version", 
            "govulncheck" => "-version",
            "dependency-check" => "--version",
            _ => "--version",
        };
        
        let result = Command::new(tool)
            .arg(version_arg)
            .output();
            
        match result {
            Ok(output) => output.status.success(),
            Err(_) => {
                // Try platform-specific paths
                self.try_alternative_paths(tool, version_arg)
            }
        }
    }
    
    /// Try alternative paths for tools
    fn try_alternative_paths(&self, tool: &str, version_arg: &str) -> bool {
        use std::process::Command;
        
        let alternative_paths = if cfg!(windows) {
            // Windows-specific paths
            let userprofile = std::env::var("USERPROFILE").unwrap_or_default();
            let appdata = std::env::var("APPDATA").unwrap_or_default();
            vec![
                format!("{}/.local/bin/{}.exe", userprofile, tool),
                format!("{}/syncable-cli/bin/{}.exe", appdata, tool),
                format!("C:/Program Files/{}/{}.exe", tool, tool),
            ]
        } else {
            // Unix-specific paths
            let home = std::env::var("HOME").unwrap_or_default();
            vec![
                format!("{}/go/bin/{}", home, tool),
                format!("{}/.local/bin/{}", home, tool),
                format!("{}/.cargo/bin/{}", home, tool),
            ]
        };
        
        for path in alternative_paths {
            if let Ok(output) = Command::new(&path).arg(version_arg).output() {
                if output.status.success() {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Test if a tool is available by running version command (public method for external use)
    pub fn test_tool_availability(&self, tool: &str) -> bool {
        self.is_tool_installed(tool)
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