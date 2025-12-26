use super::common::InstallationUtils;
use crate::analyzer::tool_management::ToolDetector;
use crate::error::Result;
use log::{info, warn};
use std::collections::HashMap;

/// Install govulncheck for Go vulnerability scanning
pub fn install_govulncheck(
    tool_detector: &mut ToolDetector,
    installed_tools: &mut HashMap<String, bool>,
) -> Result<()> {
    if tool_detector.detect_tool("govulncheck").available {
        return Ok(());
    }

    info!("🔧 Installing govulncheck for Go vulnerability scanning...");

    let success = InstallationUtils::execute_command(
        "go",
        &["install", "golang.org/x/vuln/cmd/govulncheck@latest"],
    )?;

    if success {
        info!("✅ govulncheck installed successfully");
        installed_tools.insert("govulncheck".to_string(), true);
        tool_detector.clear_cache(); // Clear cache to force fresh detection
        info!("💡 Note: Make sure ~/go/bin is in your PATH to use govulncheck");
    } else {
        warn!("❌ Failed to install govulncheck");
        warn!("📦 Please install Go from https://golang.org/ first");
    }

    Ok(())
}
