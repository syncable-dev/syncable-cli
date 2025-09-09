use crate::analyzer::tool_management::ToolDetector;
use crate::error::{AnalysisError, IaCGeneratorError, Result};
use super::common::InstallationUtils;
use std::collections::HashMap;
use log::{info, warn};

/// Install cargo-audit for Rust vulnerability scanning
pub fn install_cargo_audit(
    tool_detector: &mut ToolDetector,
    installed_tools: &mut HashMap<String, bool>,
) -> Result<()> {
    if tool_detector.detect_tool("cargo-audit").available {
        return Ok(());
    }
    
    info!("🔧 Installing cargo-audit for Rust vulnerability scanning...");
    
    let success = InstallationUtils::execute_command("cargo", &["install", "cargo-audit"])?;
    
    if success {
        info!("✅ cargo-audit installed successfully");
        installed_tools.insert("cargo-audit".to_string(), true);
        tool_detector.clear_cache(); // Refresh cache
    } else {
        return Err(IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
            file: "cargo-audit installation".to_string(),
            reason: "Installation failed".to_string(),
        }));
    }
    
    Ok(())
}