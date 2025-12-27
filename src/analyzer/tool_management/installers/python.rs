use super::common::InstallationUtils;
use crate::analyzer::tool_management::ToolDetector;
use crate::error::Result;
use log::{debug, info, warn};
use std::collections::HashMap;

/// Install pip-audit for Python vulnerability scanning
pub fn install_pip_audit(
    tool_detector: &mut ToolDetector,
    installed_tools: &mut HashMap<String, bool>,
) -> Result<()> {
    if tool_detector.detect_tool("pip-audit").available {
        return Ok(());
    }

    info!("🔧 Installing pip-audit for Python vulnerability scanning...");

    // Try different installation methods
    let install_commands = vec![
        ("pipx", vec!["install", "pip-audit"]),
        ("pip3", vec!["install", "--user", "pip-audit"]),
        ("pip", vec!["install", "--user", "pip-audit"]),
    ];

    for (cmd, args) in install_commands {
        debug!("Trying installation command: {} {}", cmd, args.join(" "));

        if InstallationUtils::is_command_available(cmd)
            && let Ok(success) = InstallationUtils::execute_command(cmd, &args.to_vec())
            && success
        {
            info!("✅ pip-audit installed successfully using {}", cmd);
            installed_tools.insert("pip-audit".to_string(), true);
            tool_detector.clear_cache();
            return Ok(());
        }
    }

    warn!("📦 Failed to auto-install pip-audit. Please install manually:");
    warn!("   Option 1: pipx install pip-audit");
    warn!("   Option 2: pip3 install --user pip-audit");

    Ok(()) // Don't fail, just warn
}
