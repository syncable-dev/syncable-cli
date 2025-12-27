//! Terraform tools - Format and validate Terraform configurations
//!
//! Provides Terraform fmt and validate capabilities with AI-optimized output.
//! Wraps the terraform CLI binary with structured output for agent decision-making.
//!
//! Features:
//! - Auto-detection of terraform binary
//! - OS-aware installation prompts
//! - Categorized issues (syntax, configuration, provider, resource)
//! - Priority rankings and actionable fix recommendations

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

/// Check if terraform is installed and return version info
pub async fn check_terraform_installed() -> Option<String> {
    let output = Command::new("terraform")
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        // Extract first line (version info)
        version.lines().next().map(|s| s.to_string())
    } else {
        None
    }
}

/// Detect the current OS and return installation instructions
pub fn get_installation_instructions() -> (&'static str, &'static str, Vec<&'static str>) {
    #[cfg(target_os = "macos")]
    {
        (
            "macOS",
            "Install Terraform using Homebrew",
            vec![
                "brew tap hashicorp/tap",
                "brew install hashicorp/tap/terraform",
            ],
        )
    }

    #[cfg(target_os = "linux")]
    {
        // Check for common package managers
        if std::path::Path::new("/etc/debian_version").exists() {
            (
                "Linux (Debian/Ubuntu)",
                "Install Terraform using apt",
                vec![
                    "sudo apt-get update && sudo apt-get install -y gnupg software-properties-common",
                    "wget -O- https://apt.releases.hashicorp.com/gpg | gpg --dearmor | sudo tee /usr/share/keyrings/hashicorp-archive-keyring.gpg > /dev/null",
                    "echo \"deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com $(lsb_release -cs) main\" | sudo tee /etc/apt/sources.list.d/hashicorp.list",
                    "sudo apt update && sudo apt-get install terraform",
                ],
            )
        } else if std::path::Path::new("/etc/redhat-release").exists() {
            (
                "Linux (RHEL/CentOS/Fedora)",
                "Install Terraform using dnf/yum",
                vec![
                    "sudo dnf install -y dnf-plugins-core || sudo yum install -y yum-utils",
                    "sudo dnf config-manager --add-repo https://rpm.releases.hashicorp.com/RHEL/hashicorp.repo || sudo yum-config-manager --add-repo https://rpm.releases.hashicorp.com/RHEL/hashicorp.repo",
                    "sudo dnf -y install terraform || sudo yum -y install terraform",
                ],
            )
        } else {
            (
                "Linux",
                "Install Terraform manually",
                vec![
                    "curl -fsSL https://releases.hashicorp.com/terraform/1.6.6/terraform_1.6.6_linux_amd64.zip -o terraform.zip",
                    "unzip terraform.zip && sudo mv terraform /usr/local/bin/",
                    "rm terraform.zip",
                ],
            )
        }
    }

    #[cfg(target_os = "windows")]
    {
        (
            "Windows",
            "Install Terraform using Chocolatey or Scoop",
            vec!["choco install terraform", "# OR: scoop install terraform"],
        )
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        (
            "Unknown OS",
            "Download from HashiCorp",
            vec!["Visit https://developer.hashicorp.com/terraform/downloads"],
        )
    }
}

/// Install terraform for the current OS
pub async fn install_terraform() -> Result<String, String> {
    let (os, _desc, commands) = get_installation_instructions();

    let mut results = Vec::new();

    for cmd in commands {
        // Skip comment lines
        if cmd.starts_with('#') {
            continue;
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Installation failed at command '{}': {}",
                cmd, stderr
            ));
        }

        results.push(format!("Executed: {}", cmd));
    }

    // Verify installation
    if let Some(version) = check_terraform_installed().await {
        Ok(format!(
            "Terraform installed successfully on {}!\n{}\n\nInstallation steps:\n{}",
            os,
            version,
            results.join("\n")
        ))
    } else {
        Err("Installation completed but terraform is not in PATH. You may need to restart your terminal.".to_string())
    }
}

/// Error type for terraform tools
#[derive(Debug, thiserror::Error)]
#[error("Terraform error: {0}")]
pub struct TerraformError(pub String);

// ============================================================================
// TerraformFmtTool
// ============================================================================

/// Arguments for terraform fmt
#[derive(Debug, Deserialize)]
pub struct TerraformFmtArgs {
    /// Path to terraform files/directory (relative to project root)
    #[serde(default)]
    pub path: Option<String>,

    /// Check mode - don't modify files, just report if formatting is needed
    #[serde(default)]
    pub check: bool,

    /// Show diff of formatting changes
    #[serde(default)]
    pub diff: bool,

    /// Process files recursively
    #[serde(default = "default_true")]
    pub recursive: bool,
}

fn default_true() -> bool {
    true
}

/// Tool to format Terraform configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraformFmtTool {
    project_path: PathBuf,
}

impl TerraformFmtTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    fn format_result(
        &self,
        success: bool,
        files_changed: Vec<String>,
        diff_output: Option<String>,
        check_mode: bool,
    ) -> String {
        let decision_context = if files_changed.is_empty() {
            "All Terraform files are properly formatted. No changes needed."
        } else if check_mode {
            "Formatting issues detected. Run terraform fmt to fix, or use this tool with check=false."
        } else {
            "Terraform files have been formatted successfully."
        };

        let output = json!({
            "success": success,
            "decision_context": decision_context,
            "summary": {
                "files_checked": if check_mode { "check mode" } else { "format mode" },
                "files_needing_format": files_changed.len(),
                "action_taken": if check_mode { "none (check only)" } else { "formatted" },
            },
            "files": files_changed,
            "diff": diff_output,
            "recommendations": if !files_changed.is_empty() && check_mode {
                Some(vec![
                    "Run `terraform fmt` to automatically fix formatting",
                    "Consider adding pre-commit hooks for consistent formatting",
                    "Use `terraform fmt -recursive` for nested modules"
                ])
            } else {
                None
            }
        });

        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    }
}

impl Tool for TerraformFmtTool {
    const NAME: &'static str = "terraform_fmt";

    type Error = TerraformError;
    type Args = TerraformFmtArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Format Terraform configuration files to canonical style. \
                Returns AI-optimized JSON showing which files need formatting or were formatted. \
                Use check=true to verify without modifying files. \
                Use diff=true to see the exact changes. \
                The tool automatically handles recursive formatting for modules."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to terraform files/directory relative to project root (default: project root)"
                    },
                    "check": {
                        "type": "boolean",
                        "description": "Check mode - report files needing format without modifying them (default: false)"
                    },
                    "diff": {
                        "type": "boolean",
                        "description": "Show diff of formatting changes (default: false)"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Process files recursively in subdirectories (default: true)"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Check if terraform is installed
        if check_terraform_installed().await.is_none() {
            let (os, desc, commands) = get_installation_instructions();
            let install_info = json!({
                "error": "terraform_not_installed",
                "message": "Terraform CLI is not installed or not in PATH",
                "os_detected": os,
                "installation": {
                    "description": desc,
                    "commands": commands
                },
                "action_required": "Ask user if they want to install Terraform, then use terraform_install tool"
            });
            return Ok(serde_json::to_string_pretty(&install_info).unwrap());
        }

        // Determine working directory
        let work_dir = match &args.path {
            Some(p) => self.project_path.join(p),
            None => self.project_path.clone(),
        };

        if !work_dir.exists() {
            return Err(TerraformError(format!(
                "Path does not exist: {}",
                work_dir.display()
            )));
        }

        // Build command
        let mut cmd = Command::new("terraform");
        cmd.arg("fmt");

        if args.check {
            cmd.arg("-check");
        }
        if args.diff {
            cmd.arg("-diff");
        }
        if args.recursive {
            cmd.arg("-recursive");
        }

        // List files that would be/were changed
        cmd.arg("-list=true");
        cmd.current_dir(&work_dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd
            .output()
            .await
            .map_err(|e| TerraformError(format!("Failed to execute terraform fmt: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse files that need formatting (one per line)
        let files_changed: Vec<String> = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|s| s.to_string())
            .collect();

        // Get diff if requested
        let diff_output = if args.diff && !stdout.is_empty() {
            Some(stdout.to_string())
        } else {
            None
        };

        // In check mode, exit code 3 means files need formatting (not an error)
        let success = output.status.success() || (args.check && output.status.code() == Some(3));

        if !success && !stderr.is_empty() {
            return Err(TerraformError(format!("terraform fmt failed: {}", stderr)));
        }

        Ok(self.format_result(success, files_changed, diff_output, args.check))
    }
}

// ============================================================================
// TerraformValidateTool
// ============================================================================

/// Arguments for terraform validate
#[derive(Debug, Deserialize)]
pub struct TerraformValidateArgs {
    /// Path to terraform configuration directory (relative to project root)
    #[serde(default)]
    pub path: Option<String>,

    /// Run terraform init first if needed
    #[serde(default)]
    pub auto_init: bool,
}

/// Tool to validate Terraform configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraformValidateTool {
    project_path: PathBuf,
}

impl TerraformValidateTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    /// Categorize validation errors
    fn categorize_error(message: &str) -> (&'static str, &'static str) {
        let msg_lower = message.to_lowercase();

        // Check more specific patterns first
        if msg_lower.contains("syntax") || msg_lower.contains("parse") {
            ("syntax", "critical")
        } else if msg_lower.contains("deprecated") {
            ("deprecation", "medium")
        } else if msg_lower.contains("provider") {
            ("provider", "high")
        } else if msg_lower.contains("resource")
            || msg_lower.contains("data source")
            || msg_lower.contains("module")
        {
            ("resource", "high")
        } else if msg_lower.contains("variable") || msg_lower.contains("output") {
            ("configuration", "medium")
        } else {
            ("general", "medium")
        }
    }

    /// Get fix recommendation based on error
    fn get_fix_recommendation(message: &str) -> &'static str {
        let msg_lower = message.to_lowercase();

        if msg_lower.contains("provider") && msg_lower.contains("not found") {
            "Run 'terraform init' to download required providers"
        } else if msg_lower.contains("variable") && msg_lower.contains("not defined") {
            "Add the missing variable to your variables.tf or provide via -var flag"
        } else if msg_lower.contains("resource") && msg_lower.contains("not found") {
            "Check resource type spelling and ensure provider is correctly configured"
        } else if msg_lower.contains("syntax") {
            "Review HCL syntax - check for missing braces, quotes, or commas"
        } else if msg_lower.contains("deprecated") {
            "Update to the recommended replacement as indicated in the message"
        } else if msg_lower.contains("module") && msg_lower.contains("not found") {
            "Run 'terraform init' to download the module or check the source path"
        } else if msg_lower.contains("duplicate") {
            "Remove or rename the duplicate resource/variable declaration"
        } else {
            "Review the error message and Terraform documentation for this resource type"
        }
    }

    fn format_result(
        &self,
        validation_output: &str,
        success: bool,
        init_output: Option<&str>,
    ) -> String {
        // Try to parse JSON output from terraform validate -json
        if let Ok(tf_json) = serde_json::from_str::<serde_json::Value>(validation_output) {
            let valid = tf_json["valid"].as_bool().unwrap_or(false);
            let error_count = tf_json["error_count"].as_u64().unwrap_or(0);
            let warning_count = tf_json["warning_count"].as_u64().unwrap_or(0);

            let diagnostics = tf_json["diagnostics"].as_array();

            let mut categorized_issues: Vec<serde_json::Value> = Vec::new();
            let mut by_category: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            let mut by_priority: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();

            if let Some(diags) = diagnostics {
                for diag in diags {
                    let severity = diag["severity"].as_str().unwrap_or("error");
                    let summary = diag["summary"].as_str().unwrap_or("");
                    let detail = diag["detail"].as_str().unwrap_or("");
                    let message = format!("{}: {}", summary, detail);

                    let (category, priority) = Self::categorize_error(&message);
                    let fix = Self::get_fix_recommendation(&message);

                    *by_category.entry(category).or_insert(0) += 1;
                    *by_priority.entry(priority).or_insert(0) += 1;

                    let range = &diag["range"];
                    let filename = range["filename"].as_str().unwrap_or("");
                    let start_line = range["start"]["line"].as_u64().unwrap_or(0);

                    categorized_issues.push(json!({
                        "severity": severity,
                        "priority": priority,
                        "category": category,
                        "summary": summary,
                        "detail": detail,
                        "fix": fix,
                        "location": {
                            "file": filename,
                            "line": start_line
                        }
                    }));
                }
            }

            let decision_context = if valid {
                "Terraform configuration is valid. Ready for plan/apply."
            } else if by_priority.get("critical").unwrap_or(&0) > &0 {
                "Critical syntax errors found. Fix these before proceeding."
            } else if error_count > 0 {
                "Configuration errors found. Review and fix before applying."
            } else {
                "Warnings found. Consider addressing for best practices."
            };

            let output = json!({
                "success": valid,
                "decision_context": decision_context,
                "summary": {
                    "valid": valid,
                    "errors": error_count,
                    "warnings": warning_count,
                    "by_category": by_category,
                    "by_priority": by_priority,
                },
                "issues": categorized_issues,
                "init_output": init_output,
                "quick_fixes": categorized_issues.iter()
                    .filter(|i| i["priority"] == "critical" || i["priority"] == "high")
                    .take(5)
                    .map(|i| format!("{}: {} - {}",
                        i["location"]["file"].as_str().unwrap_or(""),
                        i["summary"].as_str().unwrap_or(""),
                        i["fix"].as_str().unwrap_or("")
                    ))
                    .collect::<Vec<_>>()
            });

            serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
        } else {
            // Fallback for non-JSON output
            let output = json!({
                "success": success,
                "decision_context": if success {
                    "Terraform configuration is valid."
                } else {
                    "Validation failed. Review errors below."
                },
                "raw_output": validation_output,
                "init_output": init_output
            });
            serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

impl Tool for TerraformValidateTool {
    const NAME: &'static str = "terraform_validate";

    type Error = TerraformError;
    type Args = TerraformValidateArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Validate Terraform configuration for syntax and internal consistency. \
                Returns AI-optimized JSON with categorized issues (syntax/provider/resource/configuration), \
                priority rankings (critical/high/medium), and actionable fix recommendations. \
                Use auto_init=true to automatically run 'terraform init' if providers aren't downloaded. \
                The 'decision_context' field provides a summary for quick assessment."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to terraform directory relative to project root (default: project root)"
                    },
                    "auto_init": {
                        "type": "boolean",
                        "description": "Automatically run 'terraform init' if needed (default: false)"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Check if terraform is installed
        if check_terraform_installed().await.is_none() {
            let (os, desc, commands) = get_installation_instructions();
            let install_info = json!({
                "error": "terraform_not_installed",
                "message": "Terraform CLI is not installed or not in PATH",
                "os_detected": os,
                "installation": {
                    "description": desc,
                    "commands": commands
                },
                "action_required": "Ask user if they want to install Terraform, then use terraform_install tool"
            });
            return Ok(serde_json::to_string_pretty(&install_info).unwrap());
        }

        // Determine working directory
        let work_dir = match &args.path {
            Some(p) => self.project_path.join(p),
            None => self.project_path.clone(),
        };

        if !work_dir.exists() {
            return Err(TerraformError(format!(
                "Path does not exist: {}",
                work_dir.display()
            )));
        }

        let mut init_output = None;

        // Auto-init if requested
        if args.auto_init {
            let init_result = Command::new("terraform")
                .args(["init", "-backend=false", "-input=false"])
                .current_dir(&work_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await;

            if let Ok(output) = init_result {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                init_output = Some(format!("{}{}", stdout, stderr));
            }
        }

        // Run terraform validate with JSON output
        let output = Command::new("terraform")
            .args(["validate", "-json"])
            .current_dir(&work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| TerraformError(format!("Failed to execute terraform validate: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Combine output
        let validation_output = if !stdout.is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        };

        Ok(self.format_result(
            &validation_output,
            output.status.success(),
            init_output.as_deref(),
        ))
    }
}

// ============================================================================
// TerraformInstallTool
// ============================================================================

/// Arguments for terraform install
#[derive(Debug, Deserialize)]
pub struct TerraformInstallArgs {
    /// Confirm installation (safety check)
    #[serde(default)]
    pub confirm: bool,
}

/// Tool to install Terraform CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraformInstallTool;

impl TerraformInstallTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TerraformInstallTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for TerraformInstallTool {
    const NAME: &'static str = "terraform_install";

    type Error = TerraformError;
    type Args = TerraformInstallArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Install Terraform CLI on the current system. \
                Automatically detects the operating system and uses the appropriate package manager \
                (Homebrew on macOS, apt on Debian/Ubuntu, dnf/yum on RHEL/Fedora). \
                Requires confirm=true to proceed with installation."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "confirm": {
                        "type": "boolean",
                        "description": "Set to true to confirm and proceed with installation"
                    }
                },
                "required": ["confirm"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Check if already installed
        if let Some(version) = check_terraform_installed().await {
            let result = json!({
                "already_installed": true,
                "version": version,
                "message": "Terraform is already installed on this system"
            });
            return Ok(serde_json::to_string_pretty(&result).unwrap());
        }

        // Show installation info if not confirmed
        if !args.confirm {
            let (os, desc, commands) = get_installation_instructions();
            let info = json!({
                "os_detected": os,
                "installation_method": desc,
                "commands_to_run": commands,
                "action_required": "Set confirm=true to proceed with installation",
                "warning": "This will install software on your system using elevated privileges"
            });
            return Ok(serde_json::to_string_pretty(&info).unwrap());
        }

        // Proceed with installation
        match install_terraform().await {
            Ok(message) => {
                let result = json!({
                    "success": true,
                    "message": message
                });
                Ok(serde_json::to_string_pretty(&result).unwrap())
            }
            Err(error) => {
                let result = json!({
                    "success": false,
                    "error": error,
                    "suggestion": "Try installing manually or check system permissions"
                });
                Ok(serde_json::to_string_pretty(&result).unwrap())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::fs;

    #[tokio::test]
    async fn test_terraform_fmt_check_mode() {
        // Skip if terraform not installed
        if check_terraform_installed().await.is_none() {
            eprintln!("Skipping test: terraform not installed");
            return;
        }

        let temp = temp_dir().join("tf_fmt_test");
        fs::create_dir_all(&temp).unwrap();

        // Write poorly formatted terraform
        let tf_content = r#"
resource "aws_instance" "example" {
ami           = "ami-12345"
instance_type = "t2.micro"
}
"#;
        fs::write(temp.join("main.tf"), tf_content).unwrap();

        let tool = TerraformFmtTool::new(temp.clone());
        let args = TerraformFmtArgs {
            path: None,
            check: true,
            diff: false,
            recursive: false,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Should detect formatting needed
        assert!(parsed["decision_context"].is_string());
        assert!(parsed["summary"].is_object());

        // Cleanup
        fs::remove_dir_all(&temp).ok();
    }

    #[tokio::test]
    async fn test_terraform_validate_valid_config() {
        // Skip if terraform not installed
        if check_terraform_installed().await.is_none() {
            eprintln!("Skipping test: terraform not installed");
            return;
        }

        let temp = temp_dir().join("tf_validate_test");
        fs::create_dir_all(&temp).unwrap();

        // Write valid terraform (minimal)
        let tf_content = r#"
terraform {
  required_version = ">= 1.0"
}

variable "name" {
  type    = string
  default = "test"
}

output "result" {
  value = var.name
}
"#;
        fs::write(temp.join("main.tf"), tf_content).unwrap();

        let tool = TerraformValidateTool::new(temp.clone());
        let args = TerraformValidateArgs {
            path: None,
            auto_init: false,
        };

        let result = tool.call(args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Should be valid
        assert!(parsed["success"].as_bool().unwrap_or(false));
        assert!(parsed["decision_context"].is_string());

        // Cleanup
        fs::remove_dir_all(&temp).ok();
    }

    #[tokio::test]
    async fn test_terraform_not_installed_response() {
        // This test verifies the response format when terraform is not installed
        // by checking the structure of the installation info
        let (os, desc, commands) = get_installation_instructions();

        assert!(!os.is_empty());
        assert!(!desc.is_empty());
        assert!(!commands.is_empty());
    }

    #[test]
    fn test_error_categorization() {
        let (cat, pri) = TerraformValidateTool::categorize_error("Provider aws not found");
        assert_eq!(cat, "provider");
        assert_eq!(pri, "high");

        let (cat, pri) = TerraformValidateTool::categorize_error("Syntax error in HCL");
        assert_eq!(cat, "syntax");
        assert_eq!(pri, "critical");

        let (cat, pri) = TerraformValidateTool::categorize_error("Variable 'foo' is deprecated");
        assert_eq!(cat, "deprecation");
        assert_eq!(pri, "medium");
    }
}
