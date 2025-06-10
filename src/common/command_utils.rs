use crate::error::Result;
use std::process::{Command, Output};

/// Execute a command safely and return the output
pub fn execute_command(cmd: &str, args: &[&str]) -> Result<Output> {
    let output = Command::new(cmd)
        .args(args)
        .output()?;
    
    Ok(output)
}

/// Check if a command is available in PATH
pub fn is_command_available(cmd: &str) -> bool {
    // Try the command directly first
    if Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false) {
        return true;
    }
    
    // On Windows, also try with .exe extension
    if cfg!(windows) && !cmd.ends_with(".exe") {
        let cmd_with_exe = format!("{}.exe", cmd);
        return Command::new(&cmd_with_exe)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    }
    
    false
} 