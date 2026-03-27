use crate::analyzer::dependency_parser::Language;
use crate::analyzer::tool_management::{InstallationSource, ToolDetector};

/// Handles reporting and display of tool status information
#[derive(Default)]
pub struct ToolStatusReporter {
    tool_detector: ToolDetector,
}

impl ToolStatusReporter {
    pub fn new() -> Self {
        Self {
            tool_detector: ToolDetector::new(),
        }
    }

    /// Generate a comprehensive tool status report
    pub fn generate_report(&mut self, languages: &[Language]) -> ToolStatusReport {
        let tool_statuses = self.tool_detector.detect_all_vulnerability_tools(languages);

        let mut available_tools = Vec::new();
        let mut missing_tools = Vec::new();

        for (tool_name, status) in &tool_statuses {
            if status.available {
                available_tools.push(ToolInfo {
                    name: tool_name.clone(),
                    version: status.version.clone(),
                    path: status.path.clone(),
                    source: status.installation_source.clone(),
                });
            } else {
                missing_tools.push(MissingToolInfo {
                    name: tool_name.clone(),
                    language: self.get_language_for_tool(tool_name, languages),
                    install_command: self.get_install_command(tool_name),
                });
            }
        }

        let available_count = available_tools.len();

        ToolStatusReport {
            available_tools,
            missing_tools,
            total_tools: tool_statuses.len(),
            availability_percentage: (available_count as f32 / tool_statuses.len() as f32) * 100.0,
        }
    }

    fn get_language_for_tool(&self, tool_name: &str, languages: &[Language]) -> Option<Language> {
        for language in languages {
            let tools = match language {
                Language::Rust => vec!["cargo-audit"],
                Language::JavaScript | Language::TypeScript => vec!["bun", "npm", "yarn", "pnpm"],
                Language::Python => vec!["pip-audit"],
                Language::Go => vec!["govulncheck"],
                Language::Java | Language::Kotlin => vec!["grype"],
                _ => vec![],
            };

            if tools.contains(&tool_name) {
                return Some(language.clone());
            }
        }
        None
    }

    fn get_install_command(&self, tool_name: &str) -> String {
        match tool_name {
            "cargo-audit" => "cargo install cargo-audit".to_string(),
            "bun" => "curl -fsSL https://bun.sh/install | bash".to_string(),
            "npm" => "Install Node.js from https://nodejs.org/".to_string(),
            "yarn" => "npm install -g yarn".to_string(),
            "pnpm" => "npm install -g pnpm".to_string(),
            "pip-audit" => "pip install pip-audit or pipx install pip-audit".to_string(),
            "govulncheck" => "go install golang.org/x/vuln/cmd/govulncheck@latest".to_string(),
            "grype" => "brew install grype or download from GitHub".to_string(),
            _ => "check documentation".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolStatusReport {
    pub available_tools: Vec<ToolInfo>,
    pub missing_tools: Vec<MissingToolInfo>,
    pub total_tools: usize,
    pub availability_percentage: f32,
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<std::path::PathBuf>,
    pub source: InstallationSource,
}

#[derive(Debug, Clone)]
pub struct MissingToolInfo {
    pub name: String,
    pub language: Option<Language>,
    pub install_command: String,
}

impl ToolStatusReport {
    /// Print a formatted report to the console
    pub fn print_console_report(&self) {
        println!("\n🔧 Tool Status Report");
        println!("{}", "=".repeat(50));
        println!(
            "Overall availability: {:.1}% ({}/{})",
            self.availability_percentage,
            self.available_tools.len(),
            self.total_tools
        );

        if !self.available_tools.is_empty() {
            println!("\n✅ Available Tools:");
            for tool in &self.available_tools {
                print!("  • {}", tool.name);
                if let Some(ref version) = tool.version {
                    print!(" (v{})", version);
                }
                if let Some(ref path) = tool.path {
                    print!(" at {}", path.display());
                }
                match &tool.source {
                    InstallationSource::SystemPath => print!(" [system]"),
                    InstallationSource::UserLocal => print!(" [user]"),
                    InstallationSource::CargoHome => print!(" [cargo]"),
                    InstallationSource::GoHome => print!(" [go]"),
                    InstallationSource::PackageManager(pm) => print!(" [{}]", pm),
                    InstallationSource::Manual => print!(" [manual]"),
                    InstallationSource::NotFound => {}
                }
                println!();
            }
        }

        if !self.missing_tools.is_empty() {
            println!("\n❌ Missing Tools:");
            for tool in &self.missing_tools {
                print!("  • {}", tool.name);
                if let Some(ref lang) = tool.language {
                    print!(" ({:?})", lang);
                }
                println!(" - Install: {}", tool.install_command);
            }
        }

        println!();
    }
}
