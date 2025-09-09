use crate::analyzer::tool_management::ToolDetector;
use crate::error::Result;
use super::common::InstallationUtils;
use std::collections::HashMap;
use log::{info, warn};

/// Install grype for Java/Kotlin vulnerability scanning
pub fn install_grype(
    tool_detector: &mut ToolDetector,
    installed_tools: &mut HashMap<String, bool>,
) -> Result<()> {
    if tool_detector.detect_tool("grype").available {
        return Ok(());
    }
    
    info!("🔧 Installing grype for vulnerability scanning...");
    
    // Try platform-specific installation methods
    let os = std::env::consts::OS;
    
    match os {
        "macos" => {
            if InstallationUtils::is_command_available("brew") {
                let success = InstallationUtils::execute_command("brew", &["install", "grype"])?;
                if success {
                    info!("✅ grype installed successfully via Homebrew");
                    installed_tools.insert("grype".to_string(), true);
                    tool_detector.clear_cache();
                    return Ok(());
                }
            }
        }
        "linux" => {
            // Try snap first
            if InstallationUtils::is_command_available("snap") {
                let success = InstallationUtils::execute_command("snap", &["install", "grype"])?;
                if success {
                    info!("✅ grype installed successfully via snap");
                    installed_tools.insert("grype".to_string(), true);
                    tool_detector.clear_cache();
                    return Ok(());
                }
            }
        }
        _ => {}
    }
    
    warn!("❌ Automatic installation failed. Please install manually:");
    if cfg!(windows) {
        warn!("   • Download from: https://github.com/anchore/grype/releases");
        warn!("   • Or use: scoop install grype (if you have Scoop)");
    } else {
        warn!("   • macOS: brew install grype");
        warn!("   • Linux: snap install grype");
        warn!("   • Download: https://github.com/anchore/grype/releases");
    }
    
    Ok(())
}