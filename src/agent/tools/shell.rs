//! Shell tool for executing validation commands
//!
//! Provides a restricted shell tool for DevOps validation commands:
//! - Docker build validation
//! - Terraform validate/plan
//! - Helm lint
//! - Kubernetes dry-run
//!
//! Includes interactive confirmation before execution and streaming output display.

use crate::agent::ui::confirmation::{confirm_shell_command, AllowedCommands, ConfirmationResult};
use crate::agent::ui::shell_output::StreamingShellOutput;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

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

#[derive(Debug, Clone)]
pub struct ShellTool {
    project_path: PathBuf,
    /// Session-level allowed command prefixes (shared across tool instances)
    allowed_commands: Arc<AllowedCommands>,
    /// Whether to require confirmation before executing commands
    require_confirmation: bool,
}

impl ShellTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            allowed_commands: Arc::new(AllowedCommands::new()),
            require_confirmation: true,
        }
    }

    /// Create with shared allowed commands state (for session persistence)
    pub fn with_allowed_commands(project_path: PathBuf, allowed_commands: Arc<AllowedCommands>) -> Self {
        Self {
            project_path,
            allowed_commands,
            require_confirmation: true,
        }
    }

    /// Disable confirmation prompts (useful for scripted/batch mode)
    pub fn without_confirmation(mut self) -> Self {
        self.require_confirmation = false;
        self
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
        let working_dir_str = working_dir.to_string_lossy().to_string();

        // Set timeout (max 5 minutes)
        let timeout_secs = args.timeout_secs.unwrap_or(60).min(300);

        // Check if confirmation is needed
        let needs_confirmation = self.require_confirmation
            && !self.allowed_commands.is_allowed(&args.command);

        if needs_confirmation {
            // Show confirmation prompt
            let confirmation = confirm_shell_command(&args.command, &working_dir_str);

            match confirmation {
                ConfirmationResult::Proceed => {
                    // Continue with execution
                }
                ConfirmationResult::ProceedAlways(prefix) => {
                    // Remember this command prefix for the session
                    self.allowed_commands.allow(prefix);
                }
                ConfirmationResult::Modify(feedback) => {
                    // Return feedback to the agent so it can try a different approach
                    let result = json!({
                        "cancelled": true,
                        "reason": "User requested modification",
                        "user_feedback": feedback,
                        "original_command": args.command
                    });
                    return serde_json::to_string_pretty(&result)
                        .map_err(|e| ShellError(format!("Failed to serialize: {}", e)));
                }
                ConfirmationResult::Cancel => {
                    // User cancelled the operation
                    let result = json!({
                        "cancelled": true,
                        "reason": "User cancelled the operation",
                        "original_command": args.command
                    });
                    return serde_json::to_string_pretty(&result)
                        .map_err(|e| ShellError(format!("Failed to serialize: {}", e)));
                }
            }
        }

        // Create streaming output display
        let mut stream_display = StreamingShellOutput::new(&args.command, timeout_secs);
        stream_display.render();

        // Execute command with streaming output
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&args.command)
            .current_dir(&working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ShellError(format!("Failed to spawn command: {}", e)))?;

        // Read stdout and stderr in parallel, streaming output
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let mut stdout_content = String::new();
        let mut stderr_content = String::new();

        // Read stdout
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    stdout_content.push_str(&line);
                    stdout_content.push('\n');
                    stream_display.push_line(&line);
                }
            }
        }

        // Read stderr
        if let Some(stderr) = stderr {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    stderr_content.push_str(&line);
                    stderr_content.push('\n');
                    stream_display.push_line(&line);
                }
            }
        }

        // Wait for command to complete
        let status = child
            .wait()
            .map_err(|e| ShellError(format!("Command execution failed: {}", e)))?;

        // Finalize display
        stream_display.finish(status.success(), status.code());

        // Truncate output if too long
        const MAX_OUTPUT: usize = 10000;
        let stdout_truncated = if stdout_content.len() > MAX_OUTPUT {
            format!(
                "{}...\n[Output truncated, {} total bytes]",
                &stdout_content[..MAX_OUTPUT],
                stdout_content.len()
            )
        } else {
            stdout_content
        };

        let stderr_truncated = if stderr_content.len() > MAX_OUTPUT {
            format!(
                "{}...\n[Output truncated, {} total bytes]",
                &stderr_content[..MAX_OUTPUT],
                stderr_content.len()
            )
        } else {
            stderr_content
        };

        let result = json!({
            "command": args.command,
            "working_dir": working_dir_str,
            "exit_code": status.code(),
            "success": status.success(),
            "stdout": stdout_truncated,
            "stderr": stderr_truncated
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| ShellError(format!("Failed to serialize: {}", e)))
    }
}
