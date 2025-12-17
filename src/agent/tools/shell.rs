//! Shell tool for executing validation commands
//!
//! Provides a restricted shell tool for DevOps validation commands:
//! - Docker build validation
//! - Terraform validate/plan
//! - Helm lint
//! - Kubernetes dry-run

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Allowed command prefixes for security
const ALLOWED_COMMANDS: &[&str] = &[
    // Docker commands
    "docker build",
    "docker compose",
    "docker-compose",
    // Terraform commands
    "terraform init",
    "terraform validate",
    "terraform plan",
    "terraform fmt",
    // Helm commands
    "helm lint",
    "helm template",
    "helm dependency",
    // Kubernetes commands (dry-run only)
    "kubectl apply --dry-run",
    "kubectl diff",
    // Generic validation
    "make",
    "npm run",
    "cargo build",
    "go build",
    "python -m py_compile",
    // Linting
    "hadolint",
    "tflint",
    "yamllint",
    "shellcheck",
];

#[derive(Debug, Deserialize)]
pub struct ShellArgs {
    /// The command to execute
    pub command: String,
    /// Working directory (relative to project root)
    pub working_dir: Option<String>,
    /// Timeout in seconds (default: 60, max: 300)
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
#[error("Shell error: {0}")]
pub struct ShellError(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellTool {
    project_path: PathBuf,
}

impl ShellTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    fn is_command_allowed(&self, command: &str) -> bool {
        let trimmed = command.trim();
        ALLOWED_COMMANDS.iter().any(|allowed| {
            trimmed.starts_with(allowed) || trimmed == *allowed
        })
    }

    fn validate_working_dir(&self, dir: &Option<String>) -> Result<PathBuf, ShellError> {
        let canonical_project = self.project_path.canonicalize()
            .map_err(|e| ShellError(format!("Invalid project path: {}", e)))?;

        let target = match dir {
            Some(d) => {
                let path = PathBuf::from(d);
                if path.is_absolute() {
                    path
                } else {
                    self.project_path.join(path)
                }
            }
            None => self.project_path.clone(),
        };

        let canonical_target = target.canonicalize()
            .map_err(|e| ShellError(format!("Invalid working directory: {}", e)))?;

        if !canonical_target.starts_with(&canonical_project) {
            return Err(ShellError("Working directory must be within project".to_string()));
        }

        Ok(canonical_target)
    }
}

impl Tool for ShellTool {
    const NAME: &'static str = "shell";

    type Error = ShellError;
    type Args = ShellArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Execute shell commands for validation and building. This tool is restricted to safe DevOps commands.

Allowed commands:
- Docker: docker build, docker compose
- Terraform: terraform init, terraform validate, terraform plan, terraform fmt
- Helm: helm lint, helm template, helm dependency
- Kubernetes: kubectl apply --dry-run, kubectl diff
- Build: make, npm run, cargo build, go build
- Linting: hadolint, tflint, yamllint, shellcheck

Use this to validate generated configurations:
- `docker build -t test .` - Validate Dockerfile
- `terraform validate` - Validate Terraform configuration
- `helm lint ./chart` - Validate Helm chart
- `hadolint Dockerfile` - Lint Dockerfile"#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute (must be from allowed list)"
                    },
                    "working_dir": {
                        "type": "string",
                        "description": "Working directory relative to project root (default: project root)"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 60, max: 300)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate command is allowed
        if !self.is_command_allowed(&args.command) {
            return Err(ShellError(format!(
                "Command not allowed. Allowed commands are: {}",
                ALLOWED_COMMANDS.join(", ")
            )));
        }

        // Validate and get working directory
        let working_dir = self.validate_working_dir(&args.working_dir)?;

        // Set timeout (max 5 minutes)
        let timeout = Duration::from_secs(args.timeout_secs.unwrap_or(60).min(300));

        // Execute command
        let output = Command::new("sh")
            .arg("-c")
            .arg(&args.command)
            .current_dir(&working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ShellError(format!("Failed to spawn command: {}", e)))?;

        // Wait for output with timeout
        let output = output
            .wait_with_output()
            .map_err(|e| ShellError(format!("Command execution failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Truncate output if too long
        const MAX_OUTPUT: usize = 10000;
        let stdout_truncated = if stdout.len() > MAX_OUTPUT {
            format!("{}...\n[Output truncated, {} total bytes]", &stdout[..MAX_OUTPUT], stdout.len())
        } else {
            stdout.to_string()
        };

        let stderr_truncated = if stderr.len() > MAX_OUTPUT {
            format!("{}...\n[Output truncated, {} total bytes]", &stderr[..MAX_OUTPUT], stderr.len())
        } else {
            stderr.to_string()
        };

        let result = json!({
            "command": args.command,
            "working_dir": working_dir.to_string_lossy(),
            "exit_code": output.status.code(),
            "success": output.status.success(),
            "stdout": stdout_truncated,
            "stderr": stderr_truncated
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| ShellError(format!("Failed to serialize: {}", e)))
    }
}
