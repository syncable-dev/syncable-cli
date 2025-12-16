pub mod types;

use crate::error::Result;
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = ".syncable.toml";

/// Get the global config file path (~/.syncable.toml)
pub fn global_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(CONFIG_FILE_NAME))
}

/// Get the local config file path (project/.syncable.toml)
pub fn local_config_path(project_path: &Path) -> PathBuf {
    project_path.join(CONFIG_FILE_NAME)
}

/// Load configuration from file or use defaults
/// Checks local config first, then global config
pub fn load_config(project_path: Option<&Path>) -> Result<types::Config> {
    // Try local config first
    if let Some(path) = project_path {
        let local = local_config_path(path);
        if local.exists() {
            if let Ok(content) = fs::read_to_string(&local) {
                if let Ok(config) = toml::from_str(&content) {
                    return Ok(config);
                }
            }
        }
    }
    
    // Try global config
    if let Some(global) = global_config_path() {
        if global.exists() {
            if let Ok(content) = fs::read_to_string(&global) {
                if let Ok(config) = toml::from_str(&content) {
                    return Ok(config);
                }
            }
        }
    }
    
    Ok(types::Config::default())
}

/// Save configuration to global config file
pub fn save_global_config(config: &types::Config) -> Result<()> {
    if let Some(path) = global_config_path() {
        let content = toml::to_string_pretty(config)
            .map_err(|e| crate::error::ConfigError::ParsingFailed(e.to_string()))?;
        fs::write(&path, content)?;
    }
    Ok(())
}

/// Load only the agent config section (for API keys)
pub fn load_agent_config() -> types::AgentConfig {
    if let Some(global) = global_config_path() {
        if global.exists() {
            if let Ok(content) = fs::read_to_string(&global) {
                if let Ok(config) = toml::from_str::<types::Config>(&content) {
                    return config.agent;
                }
            }
        }
    }
    types::AgentConfig::default()
}

/// Save agent config, preserving other config sections
pub fn save_agent_config(agent: &types::AgentConfig) -> Result<()> {
    let mut config = load_config(None)?;
    config.agent = agent.clone();
    save_global_config(&config)
} 