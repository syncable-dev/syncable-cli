use crate::analyzer::dependency_parser::Language;
use crate::analyzer::tool_detector::{ToolDetector, InstallationSource};
use crate::error::{AnalysisError, IaCGeneratorError, Result};
use log::{info, warn, debug};
use std::process::Command;
use std::collections::HashMap;
use std::path::PathBuf;

/// Tool installer for vulnerability scanning dependencies
pub struct ToolInstaller {
    installed_tools: HashMap<String, bool>,
    tool_detector: ToolDetector,
}

impl ToolInstaller {
    pub fn new() -> Self {
        Self {
            installed_tools: HashMap::new(),
            tool_detector: ToolDetector::new(),
        }
    }
    
    /// Ensure all required tools for vulnerability scanning are available
    pub fn ensure_tools_for_languages(&mut self, languages: &[Language]) -> Result<()> {
        for language in languages {
            match language {
                Language::Rust => self.ensure_cargo_audit()?,
                Language::JavaScript | Language::TypeScript => {
                    // For JS/TS, we try to ensure bun first, then npm as fallback
                    if self.ensure_bun().is_err() {
                        self.ensure_npm()?;
                    }
                },
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
    
    /// Check if bun is available, install if needed
    fn ensure_bun(&mut self) -> Result<()> {
        if self.is_tool_installed("bun") {
            return Ok(());
        }
        
        info!("🔧 Installing bun runtime and package manager...");
        
        // Check if already installed
        if self.tool_detector.detect_tool("bun").available {
            info!("✅ Bun is already installed");
            return Ok(());
        }
        
        // Install bun using their official installer
        let install_result = if cfg!(target_os = "windows") {
            self.install_bun_windows()
        } else {
            self.install_bun_unix()
        };
        
        match install_result {
            Ok(()) => {
                info!("✅ Bun installed successfully");
                // Refresh cache
                self.tool_detector.clear_cache();
                self.installed_tools.insert("bun".to_string(), true);
                Ok(())
            }
            Err(e) => {
                warn!("❌ Failed to install bun: {}", e);
                warn!("📦 Please install bun manually from https://bun.sh/");
                warn!("   Falling back to npm for JavaScript/TypeScript vulnerability scanning");
                self.ensure_npm() // Fallback to npm
            }
        }
    }
    
    /// Install bun on Windows using PowerShell
    fn install_bun_windows(&self) -> Result<()> {
        info!("💻 Installing bun on Windows using PowerShell...");
        
        let output = Command::new("powershell")
            .args(&[
                "-Command",
                "irm bun.sh/install.ps1 | iex"
            ])
            .output()
            .map_err(|e| IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "bun installation".to_string(),
                reason: format!("Failed to execute PowerShell installer: {}", e),
            }))?;
            
        if output.status.success() {
            info!("✅ Bun installed successfully via PowerShell");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "bun installation".to_string(),
                reason: format!("PowerShell installation failed: {}", stderr),
            }))
        }
    }
    
    /// Install bun on Unix systems using curl
    fn install_bun_unix(&self) -> Result<()> {
        info!("🐧 Installing bun on Unix using curl...");
        
        let output = Command::new("curl")
            .args(&["-fsSL", "https://bun.sh/install"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|curl_process| {
                Command::new("bash")
                    .stdin(curl_process.stdout.unwrap())
                    .output()
            })
            .map_err(|e| IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "bun installation".to_string(),
                reason: format!("Failed to execute curl | bash installer: {}", e),
            }))?;
            
        if output.status.success() {
            info!("✅ Bun installed successfully via curl");
            info!("💡 Note: You may need to restart your terminal or run 'source ~/.bashrc' to use bun");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "bun installation".to_string(),
                reason: format!("curl installation failed: {}", stderr),
            }))
        }
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
    fn is_tool_installed(&mut self, tool: &str) -> bool {
        let status = self.tool_detector.detect_tool(tool);
        
        // Update cache with the detected status
        self.installed_tools.insert(tool.to_string(), status.available);
        
        status.available
    }
    

    
    /// Test if a tool is available by running version command (public method for external use)
    pub fn test_tool_availability(&mut self, tool: &str) -> bool {
        self.is_tool_installed(tool)
    }
    
    /// Get installation status summary
    pub fn get_tool_status(&self) -> HashMap<String, bool> {
        self.installed_tools.clone()
    }
    
    /// Ensure JavaScript audit tools for detected package managers
    pub fn ensure_js_audit_tools(&mut self, detected_managers: &[crate::analyzer::runtime_detector::PackageManager]) -> Result<()> {
        for manager in detected_managers {
            match manager {
                crate::analyzer::runtime_detector::PackageManager::Bun => self.ensure_bun()?,
                crate::analyzer::runtime_detector::PackageManager::Npm => self.ensure_npm()?,
                crate::analyzer::runtime_detector::PackageManager::Yarn => self.ensure_yarn()?,
                crate::analyzer::runtime_detector::PackageManager::Pnpm => self.ensure_pnpm()?,
                crate::analyzer::runtime_detector::PackageManager::Unknown => {
                    // Install npm as default
                    self.ensure_npm()?
                }
            }
        }
        Ok(())
    }
    
    /// Ensure yarn is available
    fn ensure_yarn(&mut self) -> Result<()> {
        if self.is_tool_installed("yarn") {
            return Ok(());
        }
        
        info!("🔧 Installing yarn package manager...");
        
        let output = Command::new("npm")
            .args(&["install", "-g", "yarn"])
            .output()
            .map_err(|e| IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "yarn installation".to_string(),
                reason: format!("Failed to install yarn: {}", e),
            }))?;
            
        if output.status.success() {
            info!("✅ yarn installed successfully");
            self.installed_tools.insert("yarn".to_string(), true);
        } else {
            warn!("❌ Failed to install yarn via npm");
            warn!("📦 Please install yarn manually: https://yarnpkg.com/");
        }
        
        Ok(())
    }
    
    /// Ensure pnpm is available
    fn ensure_pnpm(&mut self) -> Result<()> {
        if self.is_tool_installed("pnpm") {
            return Ok(());
        }
        
        info!("🔧 Installing pnpm package manager...");
        
        let output = Command::new("npm")
            .args(&["install", "-g", "pnpm"])
            .output()
            .map_err(|e| IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "pnpm installation".to_string(),
                reason: format!("Failed to install pnpm: {}", e),
            }))?;
            
        if output.status.success() {
            info!("✅ pnpm installed successfully");
            self.installed_tools.insert("pnpm".to_string(), true);
        } else {
            warn!("❌ Failed to install pnpm via npm");
            warn!("📦 Please install pnpm manually: https://pnpm.io/");
        }
        
        Ok(())
    }
    
    /// Print tool installation status with detailed information
    pub fn print_tool_status(&mut self, languages: &[Language]) {
        println!("\n🔧 Vulnerability Scanning Tools Status:");
        println!("{}", "=".repeat(50));
        
        let tool_statuses = self.tool_detector.detect_all_vulnerability_tools(languages);
        
        for language in languages {
            match language {
                Language::Rust => {
                    self.print_single_tool_status("cargo-audit", &tool_statuses, language);
                }
                Language::JavaScript | Language::TypeScript => {
                    // Show all JavaScript package managers
                    let js_tools = ["bun", "npm", "yarn", "pnpm"];
                    for tool in &js_tools {
                        if let Some(status) = tool_statuses.get(*tool) {
                            self.print_js_tool_status(tool, status, language);
                        }
                    }
                }
                Language::Python => {
                    self.print_single_tool_status("pip-audit", &tool_statuses, language);
                }
                Language::Go => {
                    self.print_single_tool_status("govulncheck", &tool_statuses, language);
                }
                Language::Java | Language::Kotlin => {
                    self.print_single_tool_status("grype", &tool_statuses, language);
                }
                _ => continue,
            }
        }
        println!();
    }
    
    /// Print status for a single tool
    fn print_single_tool_status(
        &self,
        tool_name: &str,
        tool_statuses: &HashMap<String, crate::analyzer::tool_detector::ToolStatus>,
        language: &Language,
    ) {
        if let Some(status) = tool_statuses.get(tool_name) {
            let status_icon = if status.available { "✅" } else { "❌" };
            print!("  {} {:?}: {} {}", status_icon, language, tool_name, 
                   if status.available { "installed" } else { "missing" });
            
            if status.available {
                if let Some(ref version) = status.version {
                    print!(" (v{})", version);
                }
                if let Some(ref path) = status.path {
                    print!(" at {}", path.display());
                }
                match &status.installation_source {
                    crate::analyzer::tool_detector::InstallationSource::SystemPath => print!(" [system]"),
                    crate::analyzer::tool_detector::InstallationSource::UserLocal => print!(" [user]"),
                    crate::analyzer::tool_detector::InstallationSource::CargoHome => print!(" [cargo]"),
                    crate::analyzer::tool_detector::InstallationSource::GoHome => print!(" [go]"),
                    crate::analyzer::tool_detector::InstallationSource::PackageManager(pm) => print!(" [{}]", pm),
                    crate::analyzer::tool_detector::InstallationSource::Manual => print!(" [manual]"),
                    crate::analyzer::tool_detector::InstallationSource::NotFound => {},
                }
            } else {
                // Provide installation guidance for missing tools
                print!(" - Install with: ");
                match tool_name {
                    "cargo-audit" => print!("cargo install cargo-audit"),
                    "pip-audit" => print!("pip install pip-audit or pipx install pip-audit"),
                    "govulncheck" => print!("go install golang.org/x/vuln/cmd/govulncheck@latest"),
                    "grype" => print!("brew install grype or download from GitHub"),
                    _ => print!("check documentation"),
                }
            }
            println!();
        }
    }
    
    /// Print status for JavaScript package manager tools
    fn print_js_tool_status(
        &self,
        tool_name: &str,
        status: &crate::analyzer::tool_detector::ToolStatus,
        language: &Language,
    ) {
        let status_icon = if status.available { "✅" } else { "❌" };
        print!("  {} {:?}: {} {}", status_icon, language, tool_name, 
               if status.available { "installed" } else { "missing" });
        
        if status.available {
            if let Some(ref version) = status.version {
                print!(" (v{})", version);
            }
            if let Some(ref path) = status.path {
                print!(" at {}", path.display());
            }
            match &status.installation_source {
                crate::analyzer::tool_detector::InstallationSource::SystemPath => print!(" [system]"),
                crate::analyzer::tool_detector::InstallationSource::UserLocal => print!(" [user]"),
                crate::analyzer::tool_detector::InstallationSource::CargoHome => print!(" [cargo]"),
                crate::analyzer::tool_detector::InstallationSource::GoHome => print!(" [go]"),
                crate::analyzer::tool_detector::InstallationSource::PackageManager(pm) => print!(" [{}]", pm),
                crate::analyzer::tool_detector::InstallationSource::Manual => print!(" [manual]"),
                crate::analyzer::tool_detector::InstallationSource::NotFound => {},
            }
        } else {
            // Provide installation guidance for missing JS tools
            print!(" - Install with: ");
            match tool_name {
                "bun" => print!("curl -fsSL https://bun.sh/install | bash"),
                "npm" => print!("Install Node.js from https://nodejs.org/"),
                "yarn" => print!("npm install -g yarn"),
                "pnpm" => print!("npm install -g pnpm"),
                _ => print!("check documentation"),
            }
        }
        println!();
    }
} 