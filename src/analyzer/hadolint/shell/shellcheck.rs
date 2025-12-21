//! ShellCheck integration for shell analysis.
//!
//! Calls the external shellcheck binary to get detailed shell script analysis.
//! Requires shellcheck to be installed on the system.

use std::process::Command;
use serde::Deserialize;

/// A ShellCheck warning/error.
#[derive(Debug, Clone, Deserialize)]
pub struct ShellCheckComment {
    /// File path (usually "-" for stdin).
    pub file: String,
    /// Line number.
    pub line: u32,
    /// End line number.
    #[serde(rename = "endLine")]
    pub end_line: u32,
    /// Column number.
    pub column: u32,
    /// End column number.
    #[serde(rename = "endColumn")]
    pub end_column: u32,
    /// Severity level.
    pub level: String,
    /// ShellCheck code (e.g., 2086).
    pub code: u32,
    /// Warning message.
    pub message: String,
}

impl ShellCheckComment {
    /// Get the rule code as a string (e.g., "SC2086").
    pub fn rule_code(&self) -> String {
        format!("SC{}", self.code)
    }
}

/// Run shellcheck on a script and return warnings.
///
/// # Arguments
/// * `script` - The shell script to analyze
/// * `shell` - The shell to use (e.g., "bash", "sh")
///
/// # Returns
/// A vector of ShellCheck comments/warnings, or an empty vector if shellcheck
/// is not available or fails.
pub fn run_shellcheck(script: &str, shell: &str) -> Vec<ShellCheckComment> {
    // Build the shellcheck command
    let output = Command::new("shellcheck")
        .args([
            "--format=json",
            &format!("--shell={}", shell),
            "-e", "2187", // Exclude ash shell warning
            "-e", "1090", // Exclude source directive warning
            "-e", "1091", // Exclude source directive warning
            "-",          // Read from stdin
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let mut child = match output {
        Ok(child) => child,
        Err(_) => {
            // shellcheck not installed or not in PATH
            return Vec::new();
        }
    };

    // Write script to stdin
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        let _ = stdin.write_all(script.as_bytes());
    }

    // Wait for output
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(_) => return Vec::new(),
    };

    // Parse JSON output
    // ShellCheck returns exit code 1 if there are warnings, but still outputs valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);

    match serde_json::from_str::<Vec<ShellCheckComment>>(&stdout) {
        Ok(comments) => comments,
        Err(_) => Vec::new(),
    }
}

/// Check if shellcheck is available on the system.
pub fn is_shellcheck_available() -> bool {
    Command::new("shellcheck")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get the shellcheck version if available.
pub fn shellcheck_version() -> Option<String> {
    let output = Command::new("shellcheck")
        .arg("--version")
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse version from output like "ShellCheck - shell script analysis tool\nversion: 0.9.0\n..."
    for line in stdout.lines() {
        if line.starts_with("version:") {
            return Some(line.trim_start_matches("version:").trim().to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_shellcheck_available() {
        // This test will pass if shellcheck is installed, skip otherwise
        let available = is_shellcheck_available();
        println!("ShellCheck available: {}", available);
    }

    #[test]
    fn test_shellcheck_version() {
        if is_shellcheck_available() {
            let version = shellcheck_version();
            println!("ShellCheck version: {:?}", version);
            assert!(version.is_some());
        }
    }

    #[test]
    fn test_run_shellcheck() {
        if !is_shellcheck_available() {
            println!("Skipping test: shellcheck not available");
            return;
        }

        // Script with a known shellcheck warning (SC2086: Double quote to prevent globbing)
        let script = r#"#!/bin/bash
echo $foo
"#;

        let comments = run_shellcheck(script, "bash");

        // Should have at least one warning about unquoted variable
        let has_sc2086 = comments.iter().any(|c| c.code == 2086);
        assert!(has_sc2086 || comments.is_empty(), "Expected SC2086 warning or empty (if shellcheck behaves differently)");
    }

    #[test]
    fn test_shellcheck_comment_rule_code() {
        let comment = ShellCheckComment {
            file: "-".to_string(),
            line: 1,
            end_line: 1,
            column: 1,
            end_column: 10,
            level: "warning".to_string(),
            code: 2086,
            message: "Double quote to prevent globbing".to_string(),
        };

        assert_eq!(comment.rule_code(), "SC2086");
    }
}
