//! IDE Detection
//!
//! Detects which IDE the CLI is running in by examining environment variables
//! and traversing the process tree to find the IDE process.

use std::env;
use std::process::Command;

/// Information about a detected IDE
#[derive(Debug, Clone)]
pub struct IdeInfo {
    pub name: String,
    pub display_name: String,
}

/// Known IDE definitions
pub mod ide_definitions {
    use super::IdeInfo;

    pub fn vscode() -> IdeInfo {
        IdeInfo {
            name: "vscode".to_string(),
            display_name: "VS Code".to_string(),
        }
    }

    pub fn cursor() -> IdeInfo {
        IdeInfo {
            name: "cursor".to_string(),
            display_name: "Cursor".to_string(),
        }
    }

    pub fn codespaces() -> IdeInfo {
        IdeInfo {
            name: "codespaces".to_string(),
            display_name: "GitHub Codespaces".to_string(),
        }
    }

    pub fn vscodefork() -> IdeInfo {
        IdeInfo {
            name: "vscodefork".to_string(),
            display_name: "IDE".to_string(),
        }
    }

    pub fn windsurf() -> IdeInfo {
        IdeInfo {
            name: "windsurf".to_string(),
            display_name: "Windsurf".to_string(),
        }
    }

    pub fn zed() -> IdeInfo {
        IdeInfo {
            name: "zed".to_string(),
            display_name: "Zed".to_string(),
        }
    }
}

/// Detect IDE from environment variables
pub fn detect_ide_from_env() -> Option<IdeInfo> {
    // Check for Cursor
    if env::var("CURSOR_TRACE_ID").is_ok() {
        return Some(ide_definitions::cursor());
    }

    // Check for GitHub Codespaces
    if env::var("CODESPACES").is_ok() {
        return Some(ide_definitions::codespaces());
    }

    // Check for Windsurf
    if env::var("WINDSURF_TRACE_ID").is_ok() {
        return Some(ide_definitions::windsurf());
    }

    // Check for Zed
    if env::var("ZED_TERM").is_ok() {
        return Some(ide_definitions::zed());
    }

    // Default to VS Code if TERM_PROGRAM is vscode
    if env::var("TERM_PROGRAM").ok().as_deref() == Some("vscode") {
        return Some(ide_definitions::vscode());
    }

    None
}

/// Verify if the detected IDE is actually VS Code or a fork
fn verify_vscode(ide: IdeInfo, command: &str) -> IdeInfo {
    if ide.name != "vscode" {
        return ide;
    }

    // Check if the command indicates VS Code or a fork
    let cmd_lower = command.to_lowercase();
    if cmd_lower.contains("code") || cmd_lower.is_empty() {
        ide_definitions::vscode()
    } else {
        ide_definitions::vscodefork()
    }
}

/// Detect the IDE, using both environment and process information
pub fn detect_ide(process_info: Option<&IdeProcessInfo>) -> Option<IdeInfo> {
    // Only VSCode-based integrations are currently supported
    if env::var("TERM_PROGRAM").ok().as_deref() != Some("vscode") {
        return None;
    }

    let ide = detect_ide_from_env()?;

    if let Some(info) = process_info {
        Some(verify_vscode(ide, &info.command))
    } else {
        Some(ide)
    }
}

/// Information about the IDE process
#[derive(Debug, Clone)]
pub struct IdeProcessInfo {
    pub pid: u32,
    pub command: String,
}

/// Get process info by traversing the process tree
/// This finds the IDE process by walking up the parent chain
#[cfg(unix)]
pub async fn get_ide_process_info() -> Option<IdeProcessInfo> {
    const MAX_TRAVERSAL_DEPTH: usize = 32;
    let shells = ["zsh", "bash", "sh", "tcsh", "csh", "ksh", "fish", "dash"];

    let mut current_pid = std::process::id();

    for _ in 0..MAX_TRAVERSAL_DEPTH {
        if let Some((parent_pid, name, _command)) = get_process_info(current_pid) {
            let is_shell = shells.iter().any(|&s| name == s);

            if is_shell {
                // Found a shell, the IDE is the grandparent
                // First get the parent of the shell (often ptyhost or similar)
                let mut ide_pid = parent_pid;

                // Try to get the grandparent (the actual IDE)
                if let Some((grandparent_pid, _, _)) = get_process_info(parent_pid)
                    && grandparent_pid > 1
                {
                    ide_pid = grandparent_pid;
                }

                // Get the command of the IDE process
                if let Some((_, _, ide_command)) = get_process_info(ide_pid) {
                    return Some(IdeProcessInfo {
                        pid: ide_pid,
                        command: ide_command,
                    });
                }

                return Some(IdeProcessInfo {
                    pid: ide_pid,
                    command: String::new(),
                });
            }

            if parent_pid <= 1 {
                break;
            }
            current_pid = parent_pid;
        } else {
            break;
        }
    }

    // Return current process info as fallback
    get_process_info(current_pid).map(|(_, _, command)| IdeProcessInfo {
        pid: current_pid,
        command,
    })
}

/// Get process info for a given PID (Unix)
#[cfg(unix)]
fn get_process_info(pid: u32) -> Option<(u32, String, String)> {
    let output = Command::new("ps")
        .args(["-o", "ppid=,command=", "-p", &pid.to_string()])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();

    if trimmed.is_empty() {
        return None;
    }

    let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
    if parts.is_empty() {
        return None;
    }

    let parent_pid: u32 = parts[0].trim().parse().unwrap_or(1);
    let full_command = parts.get(1).map(|s| s.trim()).unwrap_or("");
    let process_name = full_command
        .split_whitespace()
        .next()
        .map(|s| {
            std::path::Path::new(s)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        })
        .unwrap_or_default();

    Some((parent_pid, process_name, full_command.to_string()))
}

/// Get IDE process info for Windows
#[cfg(windows)]
pub async fn get_ide_process_info() -> Option<IdeProcessInfo> {
    // Windows implementation using PowerShell
    let output = Command::new("powershell")
        .args([
            "-Command",
            "Get-CimInstance Win32_Process | Where-Object { $_.ProcessId -eq $PID } | Select-Object ParentProcessId | ConvertTo-Json"
        ])
        .output()
        .ok()?;

    // Simplified Windows implementation - just get the current process parent
    let stdout = String::from_utf8_lossy(&output.stdout);

    // For now, return a basic implementation
    // A full implementation would traverse the process tree like gemini-cli does
    Some(IdeProcessInfo {
        pid: std::process::id(),
        command: String::new(),
    })
}

#[cfg(windows)]
fn get_process_info(_pid: u32) -> Option<(u32, String, String)> {
    // Windows implementation would use PowerShell
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_ide_from_env_vscode() {
        // This test would need to mock environment variables
        // Just testing that the function doesn't panic
        let _ = detect_ide_from_env();
    }

    #[test]
    fn test_ide_definitions() {
        let vscode = ide_definitions::vscode();
        assert_eq!(vscode.name, "vscode");
        assert_eq!(vscode.display_name, "VS Code");

        let cursor = ide_definitions::cursor();
        assert_eq!(cursor.name, "cursor");
    }
}
