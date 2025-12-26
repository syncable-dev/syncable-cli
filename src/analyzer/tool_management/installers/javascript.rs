use super::common::InstallationUtils;
use crate::analyzer::tool_management::ToolDetector;
use crate::error::{AnalysisError, IaCGeneratorError, Result};
use log::{info, warn};
use std::collections::HashMap;
use std::process::Command;

/// Ensure npm is available (comes with Node.js)
pub fn ensure_npm(
    tool_detector: &mut ToolDetector,
    _installed_tools: &mut HashMap<String, bool>,
) -> Result<()> {
    if tool_detector.detect_tool("npm").available {
        return Ok(());
    }

    warn!("📦 npm not found. Please install Node.js from https://nodejs.org/");
    warn!("   npm audit is required for JavaScript/TypeScript vulnerability scanning");

    Ok(()) // Don't fail, just warn
}

/// Install bun runtime and package manager
pub fn install_bun(
    tool_detector: &mut ToolDetector,
    installed_tools: &mut HashMap<String, bool>,
) -> Result<()> {
    if tool_detector.detect_tool("bun").available {
        return Ok(());
    }

    info!("🔧 Installing bun runtime and package manager...");

    let install_result = if cfg!(target_os = "windows") {
        install_bun_windows()
    } else {
        install_bun_unix()
    };

    match install_result {
        Ok(()) => {
            info!("✅ Bun installed successfully");
            tool_detector.clear_cache();
            installed_tools.insert("bun".to_string(), true);
            Ok(())
        }
        Err(e) => {
            warn!("❌ Failed to install bun: {}", e);
            warn!("📦 Please install bun manually from https://bun.sh/");
            warn!("   Falling back to npm for JavaScript/TypeScript vulnerability scanning");
            ensure_npm(tool_detector, installed_tools)
        }
    }
}

/// Install bun on Windows using PowerShell
fn install_bun_windows() -> Result<()> {
    info!("💻 Installing bun on Windows using PowerShell...");

    let success = InstallationUtils::execute_command(
        "powershell",
        &["-Command", "irm bun.sh/install.ps1 | iex"],
    )?;

    if success {
        info!("✅ Bun installed successfully via PowerShell");
        Ok(())
    } else {
        Err(IaCGeneratorError::Analysis(
            AnalysisError::DependencyParsing {
                file: "bun installation".to_string(),
                reason: "PowerShell installation failed".to_string(),
            },
        ))
    }
}

/// Install bun on Unix systems using curl
fn install_bun_unix() -> Result<()> {
    info!("🐧 Installing bun on Unix using curl...");

    let output = Command::new("curl")
        .args(&["-fsSL", "https://bun.sh/install"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|curl_process| {
            Command::new("bash")
                .stdin(curl_process.stdout.unwrap())
                .output()
        })
        .map_err(|e| {
            IaCGeneratorError::Analysis(AnalysisError::DependencyParsing {
                file: "bun installation".to_string(),
                reason: format!("Failed to execute curl | bash installer: {}", e),
            })
        })?;

    if output.status.success() {
        info!("✅ Bun installed successfully via curl");
        info!(
            "💡 Note: You may need to restart your terminal or run 'source ~/.bashrc' to use bun"
        );
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(IaCGeneratorError::Analysis(
            AnalysisError::DependencyParsing {
                file: "bun installation".to_string(),
                reason: format!("curl installation failed: {}", stderr),
            },
        ))
    }
}

/// Ensure yarn is available
pub fn ensure_yarn(
    tool_detector: &mut ToolDetector,
    installed_tools: &mut HashMap<String, bool>,
) -> Result<()> {
    if tool_detector.detect_tool("yarn").available {
        return Ok(());
    }

    info!("🔧 Installing yarn package manager...");

    let success = InstallationUtils::execute_command("npm", &["install", "-g", "yarn"])?;

    if success {
        info!("✅ yarn installed successfully");
        installed_tools.insert("yarn".to_string(), true);
        tool_detector.clear_cache();
    } else {
        warn!("❌ Failed to install yarn via npm");
        warn!("📦 Please install yarn manually: https://yarnpkg.com/");
    }

    Ok(())
}

/// Ensure pnpm is available
pub fn ensure_pnpm(
    tool_detector: &mut ToolDetector,
    installed_tools: &mut HashMap<String, bool>,
) -> Result<()> {
    if tool_detector.detect_tool("pnpm").available {
        return Ok(());
    }

    info!("🔧 Installing pnpm package manager...");

    let success = InstallationUtils::execute_command("npm", &["install", "-g", "pnpm"])?;

    if success {
        info!("✅ pnpm installed successfully");
        installed_tools.insert("pnpm".to_string(), true);
        tool_detector.clear_cache();
    } else {
        warn!("❌ Failed to install pnpm via npm");
        warn!("📦 Please install pnpm manually: https://pnpm.io/");
    }

    Ok(())
}
