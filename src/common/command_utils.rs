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
    Command::new(cmd)
        .arg("--version")
        .output()
        .is_ok()
} 