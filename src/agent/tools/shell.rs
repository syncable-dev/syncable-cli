//! Shell tool for executing validation commands
//!
//! Provides a restricted shell tool for DevOps validation commands:
//! - Docker build validation
//! - Terraform validate/plan
//! - Helm lint
//! - Kubernetes dry-run
//!
//! Includes interactive confirmation before execution and streaming output display.
//!
//! ## Output Truncation
//!
//! Shell outputs are truncated using prefix/suffix strategy:
//! - First 200 lines + last 200 lines are kept
//! - Middle content is summarized with line count
//! - Long lines (>2000 chars) are truncated

use super::truncation::{TruncationLimits, truncate_shell_output};
use crate::agent::ui::confirmation::{AllowedCommands, ConfirmationResult, confirm_shell_command};
use crate::agent::ui::shell_output::StreamingShellOutput;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

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
    // Kubernetes commands
    "kubectl apply --dry-run",
    "kubectl diff",
    "kubectl get svc",
    "kubectl get services",
    "kubectl get pods",
    "kubectl get namespaces",
    "kubectl port-forward",
    "kubectl config current-context",
    "kubectl config get-contexts",
    "kubectl describe",
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

/// Read-only commands allowed in plan mode
/// These commands only read/analyze and don't modify the filesystem
const READ_ONLY_COMMANDS: &[&str] = &[
    // File listing/reading
    "ls",
    "cat",
    "head",
    "tail",
    "less",
    "more",
    "wc",
    "file",
    // Search/find
    "grep",
    "find",
    "locate",
    "which",
    "whereis",
    // Git read-only
    "git status",
    "git log",
    "git diff",
    "git show",
    "git branch",
    "git remote",
    "git tag",
    // Directory navigation
    "pwd",
    "tree",
    // System info
    "uname",
    "env",
    "printenv",
    "echo",
    // Code analysis
    "hadolint",
    "tflint",
    "yamllint",
    "shellcheck",
    // Kubernetes read-only
    "kubectl get",
    "kubectl describe",
    "kubectl config",
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
    /// Whether in read-only mode (plan mode) - only allows read-only commands
    read_only: bool,
}

impl ShellTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            allowed_commands: Arc::new(AllowedCommands::new()),
            require_confirmation: true,
            read_only: false,
        }
    }

    /// Create with shared allowed commands state (for session persistence)
    pub fn with_allowed_commands(
        project_path: PathBuf,
        allowed_commands: Arc<AllowedCommands>,
    ) -> Self {
        Self {
            project_path,
            allowed_commands,
            require_confirmation: true,
            read_only: false,
        }
    }

    /// Disable confirmation prompts (useful for scripted/batch mode)
    pub fn without_confirmation(mut self) -> Self {
        self.require_confirmation = false;
        self
    }

    /// Enable read-only mode (for plan mode) - only allows read-only commands
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    fn is_command_allowed(&self, command: &str) -> bool {
        let trimmed = command.trim();
        ALLOWED_COMMANDS
            .iter()
            .any(|allowed| trimmed.starts_with(allowed) || trimmed == *allowed)
    }

    /// Check if a command is read-only (safe for plan mode)
    fn is_read_only_command(&self, command: &str) -> bool {
        let trimmed = command.trim();

        // Block output redirection (writes to files)
        if trimmed.contains(" > ") || trimmed.contains(" >> ") {
            return false;
        }

        // Block dangerous commands explicitly
        let dangerous = [
            "rm ",
            "rm\t",
            "rmdir",
            "mv ",
            "cp ",
            "mkdir ",
            "touch ",
            "chmod ",
            "chown ",
            "npm install",
            "yarn install",
            "pnpm install",
        ];
        for d in dangerous {
            if trimmed.contains(d) {
                return false;
            }
        }

        // Split on && and || to check each command in chain
        // Also split on | for pipes
        let separators = ["&&", "||", "|", ";"];
        let mut parts: Vec<&str> = vec![trimmed];
        for sep in separators {
            parts = parts.iter().flat_map(|p| p.split(sep)).collect();
        }

        // Each part must be a read-only command
        for part in parts {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            // Skip "cd" commands - they don't modify anything
            if part.starts_with("cd ") || part == "cd" {
                continue;
            }

            // Check if this part starts with a read-only command
            let is_allowed = READ_ONLY_COMMANDS
                .iter()
                .any(|allowed| part.starts_with(allowed) || part == *allowed);

            if !is_allowed {
                return false;
            }
        }

        true
    }

    fn validate_working_dir(&self, dir: &Option<String>) -> Result<PathBuf, ShellError> {
        let canonical_project = self
            .project_path
            .canonicalize()
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

        let canonical_target = target
            .canonicalize()
            .map_err(|e| ShellError(format!("Invalid working directory: {}", e)))?;

        if !canonical_target.starts_with(&canonical_project) {
            return Err(ShellError(
                "Working directory must be within project".to_string(),
            ));
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
            description: r#"Execute shell commands for building and validation. RESTRICTED to commands that CANNOT be done with native tools.

**DO NOT use shell for linting - use NATIVE tools instead:**
- Dockerfile linting → use `hadolint` tool (NOT shell hadolint)
- docker-compose linting → use `dclint` tool (NOT shell docker-compose config)
- Helm chart linting → use `helmlint` tool (NOT shell helm lint)
- Kubernetes YAML linting → use `kubelint` tool (NOT shell kubectl/kubeval)

**Use shell ONLY for:**
- `docker build` - Actually building Docker images
- `terraform init/validate/plan` - Terraform workflows
- `make`, `npm run`, `cargo build` - Build commands
- `git` commands - Version control operations

The native linting tools return AI-optimized JSON with priorities and fix recommendations.
Shell linting produces plain text that's harder to parse and act on."#.to_string(),
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
        // In read-only mode (plan mode), only allow read-only commands
        if self.read_only {
            if !self.is_read_only_command(&args.command) {
                let result = json!({
                    "error": true,
                    "reason": "Plan mode is active - only read-only commands allowed",
                    "blocked_command": args.command,
                    "allowed_commands": READ_ONLY_COMMANDS,
                    "hint": "Exit plan mode (Shift+Tab) to run write commands"
                });
                return serde_json::to_string_pretty(&result)
                    .map_err(|e| ShellError(format!("Failed to serialize: {}", e)));
            }
        } else {
            // Validate command is allowed (standard mode)
            if !self.is_command_allowed(&args.command) {
                return Err(ShellError(format!(
                    "Command not allowed. Allowed commands are: {}",
                    ALLOWED_COMMANDS.join(", ")
                )));
            }
        }

        // Validate and get working directory
        let working_dir = self.validate_working_dir(&args.working_dir)?;
        let working_dir_str = working_dir.to_string_lossy().to_string();

        // Set timeout (max 5 minutes)
        let timeout_secs = args.timeout_secs.unwrap_or(60).min(300);

        // Check if confirmation is needed
        let needs_confirmation =
            self.require_confirmation && !self.allowed_commands.is_allowed(&args.command);

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

        // Execute command with async streaming output
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&args.command)
            .current_dir(&working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ShellError(format!("Failed to spawn command: {}", e)))?;

        // Take ownership of stdout/stderr for async reading
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Channel for streaming output lines from both stdout and stderr
        let (tx, mut rx) = mpsc::channel::<(String, bool)>(100); // (line, is_stderr)

        // Spawn task to read stdout
        let tx_stdout = tx.clone();
        let stdout_handle = stdout.map(|stdout| {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                let mut content = String::new();
                while let Ok(Some(line)) = reader.next_line().await {
                    content.push_str(&line);
                    content.push('\n');
                    let _ = tx_stdout.send((line, false)).await;
                }
                content
            })
        });

        // Spawn task to read stderr
        let tx_stderr = tx;
        let stderr_handle = stderr.map(|stderr| {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                let mut content = String::new();
                while let Ok(Some(line)) = reader.next_line().await {
                    content.push_str(&line);
                    content.push('\n');
                    let _ = tx_stderr.send((line, true)).await;
                }
                content
            })
        });

        // Process incoming lines and update display in real-time on the main task
        // Use tokio::select! to handle both the receiver and the reader completion
        let mut stdout_content = String::new();
        let mut stderr_content = String::new();

        // Wait for readers while processing display updates
        loop {
            tokio::select! {
                // Receive lines from either stdout or stderr
                line_result = rx.recv() => {
                    match line_result {
                        Some((line, _is_stderr)) => {
                            stream_display.push_line(&line);
                        }
                        None => {
                            // Channel closed, all readers done
                            break;
                        }
                    }
                }
            }
        }

        // Collect final content from reader handles
        if let Some(handle) = stdout_handle {
            stdout_content = handle.await.unwrap_or_default();
        }
        if let Some(handle) = stderr_handle {
            stderr_content = handle.await.unwrap_or_default();
        }

        // Wait for command to complete
        let status = child
            .wait()
            .await
            .map_err(|e| ShellError(format!("Command execution failed: {}", e)))?;

        // Finalize display
        stream_display.finish(status.success(), status.code());

        // Apply smart truncation: prefix + suffix strategy
        // This keeps the first N and last M lines, hiding the middle
        let limits = TruncationLimits::default();
        let truncated = truncate_shell_output(&stdout_content, &stderr_content, &limits);

        let result = json!({
            "command": args.command,
            "working_dir": working_dir_str,
            "exit_code": status.code(),
            "success": status.success(),
            "stdout": truncated.stdout,
            "stderr": truncated.stderr,
            "stdout_total_lines": truncated.stdout_total_lines,
            "stderr_total_lines": truncated.stderr_total_lines,
            "stdout_truncated": truncated.stdout_truncated,
            "stderr_truncated": truncated.stderr_truncated
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| ShellError(format!("Failed to serialize: {}", e)))
    }
}
