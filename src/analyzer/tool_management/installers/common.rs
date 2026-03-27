use crate::error::Result;
use log::{debug, info, warn};
use std::process::Command;

#[derive(Debug, Clone)]
pub enum InstallationStrategy {
    PackageManager(String), // e.g., "brew", "apt", "cargo"
    DirectDownload { url: String, extract_to: String },
    Script { command: String, args: Vec<String> },
    Manual { instructions: String },
}

/// Common utilities for tool installation
pub struct InstallationUtils;

impl InstallationUtils {
    /// Execute a command and return success status
    pub fn execute_command(command: &str, args: &[&str]) -> Result<bool> {
        debug!("Executing command: {} {}", command, args.join(" "));

        let output = Command::new(command).args(args).output()?;

        if output.status.success() {
            info!(
                "✅ Command executed successfully: {} {}",
                command,
                args.join(" ")
            );
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                "❌ Command failed: {} {} - {}",
                command,
                args.join(" "),
                stderr
            );
            Ok(false)
        }
    }

    /// Check if a command is available in PATH
    pub fn is_command_available(command: &str) -> bool {
        Command::new(command)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get platform-specific installation directory
    pub fn get_user_bin_dir() -> std::path::PathBuf {
        if cfg!(windows) {
            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                std::path::PathBuf::from(userprofile)
                    .join(".local")
                    .join("bin")
            } else {
                std::path::PathBuf::from(".").join("bin")
            }
        } else if let Ok(home) = std::env::var("HOME") {
            std::path::PathBuf::from(home).join(".local").join("bin")
        } else {
            std::path::PathBuf::from(".").join("bin")
        }
    }

    /// Create directory if it doesn't exist
    pub fn ensure_dir_exists(path: &std::path::Path) -> Result<()> {
        if !path.exists() {
            std::fs::create_dir_all(path)?;
            info!("Created directory: {}", path.display());
        }
        Ok(())
    }
}
