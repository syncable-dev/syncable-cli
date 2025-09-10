use crate::analyzer::dependency_parser::Language;
use crate::analyzer::tool_management::{ToolDetector, InstallationSource};
use crate::error::Result;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolInstallationError {
    #[error("Installation failed: {0}")]
    InstallationFailed(String),
    
    #[error("Tool not supported on this platform: {0}")]
    UnsupportedPlatform(String),
    
    #[error("Command execution failed: {0}")]
    CommandFailed(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

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
                    if self.ensure_bun().is_err() {
                        self.ensure_npm()?;
                    }
                },
                Language::Python => self.ensure_pip_audit()?,
                Language::Go => self.ensure_govulncheck()?,
                Language::Java | Language::Kotlin => self.ensure_grype()?,
                _ => {}
            }
        }
        Ok(())
    }
    
    /// Check if a tool is installed and available
    fn is_tool_installed(&mut self, tool: &str) -> bool {
        let status = self.tool_detector.detect_tool(tool);
        self.installed_tools.insert(tool.to_string(), status.available);
        status.available
    }
    
    /// Test if a tool is available by running version command
    pub fn test_tool_availability(&mut self, tool: &str) -> bool {
        self.is_tool_installed(tool)
    }
    
    /// Get installation status summary
    pub fn get_tool_status(&self) -> HashMap<String, bool> {
        self.installed_tools.clone()
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
    
    fn print_single_tool_status(
        &self,
        tool_name: &str,
        tool_statuses: &HashMap<String, crate::analyzer::tool_management::ToolStatus>,
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
                    InstallationSource::SystemPath => print!(" [system]"),
                    InstallationSource::UserLocal => print!(" [user]"),
                    InstallationSource::CargoHome => print!(" [cargo]"),
                    InstallationSource::GoHome => print!(" [go]"),
                    InstallationSource::PackageManager(pm) => print!(" [{}]", pm),
                    InstallationSource::Manual => print!(" [manual]"),
                    InstallationSource::NotFound => {},
                }
            } else {
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
    
    fn print_js_tool_status(
        &self,
        tool_name: &str,
        status: &crate::analyzer::tool_management::ToolStatus,
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
                InstallationSource::SystemPath => print!(" [system]"),
                InstallationSource::UserLocal => print!(" [user]"),
                InstallationSource::CargoHome => print!(" [cargo]"),
                InstallationSource::GoHome => print!(" [go]"),
                InstallationSource::PackageManager(pm) => print!(" [{}]", pm),
                InstallationSource::Manual => print!(" [manual]"),
                InstallationSource::NotFound => {},
            }
        } else {
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
    
    // Installation methods - these will be moved to installers/ modules
    fn ensure_cargo_audit(&mut self) -> Result<()> {
        use crate::analyzer::tool_management::installers::rust::install_cargo_audit;
        install_cargo_audit(&mut self.tool_detector, &mut self.installed_tools)
    }
    
    fn ensure_npm(&mut self) -> Result<()> {
        use crate::analyzer::tool_management::installers::javascript::ensure_npm;
        ensure_npm(&mut self.tool_detector, &mut self.installed_tools)
    }
    
    fn ensure_bun(&mut self) -> Result<()> {
        use crate::analyzer::tool_management::installers::javascript::install_bun;
        install_bun(&mut self.tool_detector, &mut self.installed_tools)
    }
    
    fn ensure_pip_audit(&mut self) -> Result<()> {
        use crate::analyzer::tool_management::installers::python::install_pip_audit;
        install_pip_audit(&mut self.tool_detector, &mut self.installed_tools)
    }
    
    fn ensure_govulncheck(&mut self) -> Result<()> {
        use crate::analyzer::tool_management::installers::go::install_govulncheck;
        install_govulncheck(&mut self.tool_detector, &mut self.installed_tools)
    }
    
    fn ensure_grype(&mut self) -> Result<()> {
        use crate::analyzer::tool_management::installers::java::install_grype;
        install_grype(&mut self.tool_detector, &mut self.installed_tools)
    }
}