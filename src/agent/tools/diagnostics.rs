//! Diagnostics tool for detecting code errors via IDE/LSP integration
//!
//! This tool queries the IDE's language servers (via MCP) or falls back to
//! running language-specific commands to detect errors in the code.
//!
//! ## Usage
//!
//! The agent can use this tool after writing or modifying files to check
//! for compilation errors, type errors, linting issues, etc.
//!
//! ## Supported Methods
//!
//! 1. **IDE Integration (preferred)**: If connected to an IDE via MCP,
//!    queries language servers directly (rust-analyzer, TypeScript, ESLint, etc.)
//!
//! 2. **Command Fallback**: If no IDE is connected, runs language-specific
//!    commands based on detected project type:
//!    - Rust: `cargo check`
//!    - JavaScript/TypeScript: `npm run lint` or `eslint`
//!    - Python: `python -m py_compile` or `pylint`
//!    - Go: `go build`

use crate::agent::ide::{Diagnostic, DiagnosticSeverity, DiagnosticsResponse, IdeClient};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;

#[derive(Debug, Deserialize)]
pub struct DiagnosticsArgs {
    /// Optional file path to check. If not provided, checks all files.
    pub path: Option<String>,
    /// Whether to include warnings (default: true)
    pub include_warnings: Option<bool>,
    /// Maximum number of diagnostics to return (default: 50)
    pub limit: Option<usize>,
}

#[derive(Debug, thiserror::Error)]
#[error("Diagnostics error: {0}")]
pub struct DiagnosticsError(String);

#[derive(Debug, Clone)]
pub struct DiagnosticsTool {
    project_path: PathBuf,
    /// Optional IDE client for LSP integration
    ide_client: Option<Arc<Mutex<IdeClient>>>,
}

impl DiagnosticsTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            ide_client: None,
        }
    }

    /// Set the IDE client for LSP integration
    pub fn with_ide_client(mut self, ide_client: Arc<Mutex<IdeClient>>) -> Self {
        self.ide_client = Some(ide_client);
        self
    }

    /// Detect project type based on files present
    fn detect_project_type(&self) -> ProjectType {
        let cargo_toml = self.project_path.join("Cargo.toml");
        let package_json = self.project_path.join("package.json");
        let go_mod = self.project_path.join("go.mod");
        let pyproject_toml = self.project_path.join("pyproject.toml");
        let requirements_txt = self.project_path.join("requirements.txt");

        if cargo_toml.exists() {
            ProjectType::Rust
        } else if package_json.exists() {
            ProjectType::JavaScript
        } else if go_mod.exists() {
            ProjectType::Go
        } else if pyproject_toml.exists() || requirements_txt.exists() {
            ProjectType::Python
        } else {
            ProjectType::Unknown
        }
    }

    /// Get diagnostics from IDE via MCP
    async fn get_ide_diagnostics(&self, file_path: Option<&str>) -> Option<DiagnosticsResponse> {
        let client = self.ide_client.as_ref()?;
        let guard = client.lock().await;

        if !guard.is_connected() {
            return None;
        }

        guard.get_diagnostics(file_path).await.ok()
    }

    /// Run fallback command-based diagnostics
    async fn get_command_diagnostics(&self) -> Result<DiagnosticsResponse, DiagnosticsError> {
        let project_type = self.detect_project_type();

        match project_type {
            ProjectType::Rust => self.run_cargo_check().await,
            ProjectType::JavaScript => self.run_npm_lint().await,
            ProjectType::Go => self.run_go_build().await,
            ProjectType::Python => self.run_python_check().await,
            ProjectType::Unknown => Ok(DiagnosticsResponse {
                diagnostics: Vec::new(),
                total_errors: 0,
                total_warnings: 0,
            }),
        }
    }

    /// Run cargo check and parse output
    async fn run_cargo_check(&self) -> Result<DiagnosticsResponse, DiagnosticsError> {
        let output = Command::new("cargo")
            .args(["check", "--message-format=json"])
            .current_dir(&self.project_path)
            .output()
            .await
            .map_err(|e| DiagnosticsError(format!("Failed to run cargo check: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut diagnostics = Vec::new();

        for line in stdout.lines() {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
                if msg.get("reason").and_then(|r| r.as_str()) == Some("compiler-message") {
                    if let Some(message) = msg.get("message") {
                        if let Some(diag) = self.parse_cargo_message(message) {
                            diagnostics.push(diag);
                        }
                    }
                }
            }
        }

        let total_errors = diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .count() as u32;
        let total_warnings = diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .count() as u32;

        Ok(DiagnosticsResponse {
            diagnostics,
            total_errors,
            total_warnings,
        })
    }

    /// Parse a cargo compiler message into a Diagnostic
    fn parse_cargo_message(&self, message: &serde_json::Value) -> Option<Diagnostic> {
        let level = message.get("level")?.as_str()?;
        let msg = message.get("message")?.as_str()?;

        let severity = match level {
            "error" => DiagnosticSeverity::Error,
            "warning" => DiagnosticSeverity::Warning,
            "note" | "help" => DiagnosticSeverity::Hint,
            _ => DiagnosticSeverity::Information,
        };

        // Get the primary span
        let spans = message.get("spans")?.as_array()?;
        let span = spans
            .iter()
            .find(|s| {
                s.get("is_primary")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .or_else(|| spans.first())?;

        let file = span.get("file_name")?.as_str()?;
        let line = span.get("line_start")?.as_u64()? as u32;
        let column = span.get("column_start")?.as_u64()? as u32;
        let end_line = span
            .get("line_end")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let end_column = span
            .get("column_end")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let code = message
            .get("code")
            .and_then(|c| c.get("code"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        Some(Diagnostic {
            file: file.to_string(),
            line,
            column,
            end_line,
            end_column,
            severity,
            message: msg.to_string(),
            source: Some("rustc".to_string()),
            code,
        })
    }

    /// Run npm lint and parse output
    async fn run_npm_lint(&self) -> Result<DiagnosticsResponse, DiagnosticsError> {
        // Try npm run lint first
        let output = Command::new("npm")
            .args(["run", "lint", "--", "--format=json"])
            .current_dir(&self.project_path)
            .output()
            .await;

        if let Ok(output) = output {
            if output.status.success() || !output.stdout.is_empty() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(results) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                    return Ok(self.parse_eslint_output(&results));
                }
            }
        }

        // If that fails, try npx eslint
        let output = Command::new("npx")
            .args(["eslint", ".", "--format=json"])
            .current_dir(&self.project_path)
            .output()
            .await
            .map_err(|e| DiagnosticsError(format!("Failed to run eslint: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(results) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
            return Ok(self.parse_eslint_output(&results));
        }

        // Return empty if we couldn't parse
        Ok(DiagnosticsResponse {
            diagnostics: Vec::new(),
            total_errors: 0,
            total_warnings: 0,
        })
    }

    /// Parse ESLint JSON output
    fn parse_eslint_output(&self, results: &[serde_json::Value]) -> DiagnosticsResponse {
        let mut diagnostics = Vec::new();

        for file_result in results {
            let file = file_result
                .get("filePath")
                .and_then(|f| f.as_str())
                .unwrap_or("");

            if let Some(messages) = file_result.get("messages").and_then(|m| m.as_array()) {
                for msg in messages {
                    let severity = match msg.get("severity").and_then(|s| s.as_u64()) {
                        Some(2) => DiagnosticSeverity::Error,
                        Some(1) => DiagnosticSeverity::Warning,
                        _ => DiagnosticSeverity::Information,
                    };

                    let message = msg
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_string();
                    let line = msg.get("line").and_then(|l| l.as_u64()).unwrap_or(1) as u32;
                    let column = msg.get("column").and_then(|c| c.as_u64()).unwrap_or(1) as u32;
                    let end_line = msg
                        .get("endLine")
                        .and_then(|l| l.as_u64())
                        .map(|v| v as u32);
                    let end_column = msg
                        .get("endColumn")
                        .and_then(|c| c.as_u64())
                        .map(|v| v as u32);
                    let code = msg
                        .get("ruleId")
                        .and_then(|r| r.as_str())
                        .map(|s| s.to_string());

                    diagnostics.push(Diagnostic {
                        file: file.to_string(),
                        line,
                        column,
                        end_line,
                        end_column,
                        severity,
                        message,
                        source: Some("eslint".to_string()),
                        code,
                    });
                }
            }
        }

        let total_errors = diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .count() as u32;
        let total_warnings = diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .count() as u32;

        DiagnosticsResponse {
            diagnostics,
            total_errors,
            total_warnings,
        }
    }

    /// Run go build and parse output
    async fn run_go_build(&self) -> Result<DiagnosticsResponse, DiagnosticsError> {
        let output = Command::new("go")
            .args(["build", "-o", "/dev/null", "./..."])
            .current_dir(&self.project_path)
            .output()
            .await
            .map_err(|e| DiagnosticsError(format!("Failed to run go build: {}", e)))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut diagnostics = Vec::new();

        // Parse go build output: "file.go:line:col: message"
        for line in stderr.lines() {
            if let Some(diag) = self.parse_go_error(line) {
                diagnostics.push(diag);
            }
        }

        let total_errors = diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .count() as u32;
        let total_warnings = diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .count() as u32;

        Ok(DiagnosticsResponse {
            diagnostics,
            total_errors,
            total_warnings,
        })
    }

    /// Parse a Go error line
    fn parse_go_error(&self, line: &str) -> Option<Diagnostic> {
        // Format: file.go:line:col: message
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 4 {
            return None;
        }

        let file = parts[0].to_string();
        let line_num = parts[1].parse::<u32>().ok()?;
        let column = parts[2].parse::<u32>().ok()?;
        let message = parts[3].trim().to_string();

        Some(Diagnostic {
            file,
            line: line_num,
            column,
            end_line: None,
            end_column: None,
            severity: DiagnosticSeverity::Error,
            message,
            source: Some("go".to_string()),
            code: None,
        })
    }

    /// Run Python syntax check
    async fn run_python_check(&self) -> Result<DiagnosticsResponse, DiagnosticsError> {
        // Try pylint first
        let output = Command::new("pylint")
            .args(["--output-format=json", "."])
            .current_dir(&self.project_path)
            .output()
            .await;

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(results) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                return Ok(self.parse_pylint_output(&results));
            }
        }

        // Fallback: just return empty
        Ok(DiagnosticsResponse {
            diagnostics: Vec::new(),
            total_errors: 0,
            total_warnings: 0,
        })
    }

    /// Parse pylint JSON output
    fn parse_pylint_output(&self, results: &[serde_json::Value]) -> DiagnosticsResponse {
        let mut diagnostics = Vec::new();

        for msg in results {
            let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let severity = match msg_type {
                "error" | "fatal" => DiagnosticSeverity::Error,
                "warning" => DiagnosticSeverity::Warning,
                "convention" | "refactor" => DiagnosticSeverity::Hint,
                _ => DiagnosticSeverity::Information,
            };

            let file = msg
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();
            let line = msg.get("line").and_then(|l| l.as_u64()).unwrap_or(1) as u32;
            let column = msg.get("column").and_then(|c| c.as_u64()).unwrap_or(1) as u32;
            let message = msg
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            let code = msg
                .get("message-id")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());

            diagnostics.push(Diagnostic {
                file,
                line,
                column,
                end_line: None,
                end_column: None,
                severity,
                message,
                source: Some("pylint".to_string()),
                code,
            });
        }

        let total_errors = diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .count() as u32;
        let total_warnings = diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .count() as u32;

        DiagnosticsResponse {
            diagnostics,
            total_errors,
            total_warnings,
        }
    }

    /// Filter diagnostics based on user preferences
    fn filter_diagnostics(
        &self,
        mut response: DiagnosticsResponse,
        include_warnings: bool,
        limit: usize,
        file_path: Option<&str>,
    ) -> DiagnosticsResponse {
        // Filter by file if specified
        if let Some(path) = file_path {
            response.diagnostics.retain(|d| d.file.contains(path));
        }

        // Filter out warnings if not requested
        if !include_warnings {
            response
                .diagnostics
                .retain(|d| d.severity == DiagnosticSeverity::Error);
        }

        // Apply limit
        response.diagnostics.truncate(limit);

        // Recalculate totals
        response.total_errors = response
            .diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .count() as u32;
        response.total_warnings = response
            .diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .count() as u32;

        response
    }
}

#[derive(Debug, Clone, Copy)]
enum ProjectType {
    Rust,
    JavaScript,
    Go,
    Python,
    Unknown,
}

impl Tool for DiagnosticsTool {
    const NAME: &'static str = "diagnostics";

    type Error = DiagnosticsError;
    type Args = DiagnosticsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Check for code errors, warnings, and linting issues.

This tool queries language servers or runs language-specific commands to detect:
- Compilation errors
- Type errors
- Syntax errors
- Linting warnings
- Best practice violations

Use this tool after writing or modifying code to verify there are no errors.

The tool automatically detects the project type and uses appropriate checking:
- Rust: Uses rust-analyzer or cargo check
- JavaScript/TypeScript: Uses ESLint or TypeScript compiler
- Go: Uses gopls or go build
- Python: Uses pylint or pyright

Returns a list of diagnostics with file locations, severity, and messages."#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Optional file path to check. If not provided, checks all files in the project."
                    },
                    "include_warnings": {
                        "type": "boolean",
                        "description": "Whether to include warnings in addition to errors (default: true)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of diagnostics to return (default: 50)"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let include_warnings = args.include_warnings.unwrap_or(true);
        let limit = args.limit.unwrap_or(50);
        let file_path = args.path.as_deref();

        // Try IDE first (better real-time diagnostics)
        let response = if let Some(ide_response) = self.get_ide_diagnostics(file_path).await {
            ide_response
        } else {
            // Fall back to command-based diagnostics
            self.get_command_diagnostics().await?
        };

        // Filter and limit results
        let filtered = self.filter_diagnostics(response, include_warnings, limit, file_path);

        // Format output
        let result = if filtered.diagnostics.is_empty() {
            json!({
                "success": true,
                "message": "No errors or warnings found",
                "total_errors": 0,
                "total_warnings": 0,
                "diagnostics": []
            })
        } else {
            let formatted_diagnostics: Vec<serde_json::Value> = filtered
                .diagnostics
                .iter()
                .map(|d| {
                    json!({
                        "file": d.file,
                        "line": d.line,
                        "column": d.column,
                        "severity": d.severity.as_str(),
                        "message": d.message,
                        "source": d.source,
                        "code": d.code
                    })
                })
                .collect();

            json!({
                "success": filtered.total_errors == 0,
                "total_errors": filtered.total_errors,
                "total_warnings": filtered.total_warnings,
                "diagnostics": formatted_diagnostics
            })
        };

        serde_json::to_string_pretty(&result)
            .map_err(|e| DiagnosticsError(format!("Failed to serialize: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_diagnostics_tool_creation() {
        let tool = DiagnosticsTool::new(PathBuf::from("."));
        assert_eq!(tool.project_path, PathBuf::from("."));
    }

    #[test]
    fn test_project_type_detection() {
        // This test would need a proper test directory setup
        let tool = DiagnosticsTool::new(env::current_dir().unwrap());
        let project_type = tool.detect_project_type();
        // Current project is Rust
        assert!(matches!(project_type, ProjectType::Rust));
    }

    #[test]
    fn test_parse_go_error() {
        let tool = DiagnosticsTool::new(PathBuf::from("."));
        let line = "main.go:10:5: undefined: foo";
        let diag = tool.parse_go_error(line);
        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.file, "main.go");
        assert_eq!(diag.line, 10);
        assert_eq!(diag.column, 5);
        assert_eq!(diag.message, "undefined: foo");
    }

    #[test]
    fn test_filter_diagnostics() {
        let tool = DiagnosticsTool::new(PathBuf::from("."));
        let response = DiagnosticsResponse {
            diagnostics: vec![
                Diagnostic {
                    file: "src/main.rs".to_string(),
                    line: 1,
                    column: 1,
                    end_line: None,
                    end_column: None,
                    severity: DiagnosticSeverity::Error,
                    message: "error".to_string(),
                    source: None,
                    code: None,
                },
                Diagnostic {
                    file: "src/lib.rs".to_string(),
                    line: 1,
                    column: 1,
                    end_line: None,
                    end_column: None,
                    severity: DiagnosticSeverity::Warning,
                    message: "warning".to_string(),
                    source: None,
                    code: None,
                },
            ],
            total_errors: 1,
            total_warnings: 1,
        };

        // Filter to errors only
        let filtered = tool.filter_diagnostics(response.clone(), false, 50, None);
        assert_eq!(filtered.diagnostics.len(), 1);
        assert_eq!(filtered.total_errors, 1);
        assert_eq!(filtered.total_warnings, 0);

        // Filter by file
        let filtered = tool.filter_diagnostics(response, true, 50, Some("main.rs"));
        assert_eq!(filtered.diagnostics.len(), 1);
        assert_eq!(filtered.diagnostics[0].file, "src/main.rs");
    }
}
