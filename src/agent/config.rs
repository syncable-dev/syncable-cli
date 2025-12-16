//! Agent configuration and credentials management
//!
//! Handles storing and retrieving LLM provider credentials securely.
//! Credentials are stored in ~/.syncable/credentials.toml

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use super::{AgentError, AgentResult, ProviderType};

/// Credentials for LLM providers
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentCredentials {
    /// Default provider to use
    #[serde(default)]
    pub default_provider: Option<String>,
    
    /// Default model to use
    #[serde(default)]
    pub default_model: Option<String>,
    
    /// OpenAI API key
    #[serde(default)]
    pub openai_api_key: Option<String>,
    
    /// Anthropic API key
    #[serde(default)]
    pub anthropic_api_key: Option<String>,
}

impl AgentCredentials {
    /// Get the syncable config directory (~/.syncable)
    pub fn config_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".syncable"))
    }
    
    /// Get the credentials file path
    pub fn credentials_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("credentials.toml"))
    }
    
    /// Load credentials from file
    pub fn load() -> AgentResult<Self> {
        let path = Self::credentials_path()
            .ok_or_else(|| AgentError::ClientError("Could not determine home directory".into()))?;
        
        if !path.exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(&path)
            .map_err(|e| AgentError::ClientError(format!("Failed to read credentials: {}", e)))?;
        
        toml::from_str(&content)
            .map_err(|e| AgentError::ClientError(format!("Failed to parse credentials: {}", e)))
    }
    
    /// Save credentials to file
    pub fn save(&self) -> AgentResult<()> {
        let dir = Self::config_dir()
            .ok_or_else(|| AgentError::ClientError("Could not determine home directory".into()))?;
        
        // Create directory if it doesn't exist
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .map_err(|e| AgentError::ClientError(format!("Failed to create config dir: {}", e)))?;
        }
        
        let path = dir.join("credentials.toml");
        let content = toml::to_string_pretty(self)
            .map_err(|e| AgentError::ClientError(format!("Failed to serialize credentials: {}", e)))?;
        
        fs::write(&path, content)
            .map_err(|e| AgentError::ClientError(format!("Failed to write credentials: {}", e)))?;
        
        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&path, perms).ok();
        }
        
        Ok(())
    }
    
    /// Check if credentials exist for a provider
    pub fn has_credentials(&self, provider: ProviderType) -> bool {
        match provider {
            ProviderType::OpenAI => self.openai_api_key.is_some(),
            ProviderType::Anthropic => self.anthropic_api_key.is_some(),
        }
    }
    
    /// Get the API key for a provider
    pub fn get_api_key(&self, provider: ProviderType) -> Option<&str> {
        match provider {
            ProviderType::OpenAI => self.openai_api_key.as_deref(),
            ProviderType::Anthropic => self.anthropic_api_key.as_deref(),
        }
    }
    
    /// Set the API key for a provider
    pub fn set_api_key(&mut self, provider: ProviderType, key: String) {
        match provider {
            ProviderType::OpenAI => self.openai_api_key = Some(key),
            ProviderType::Anthropic => self.anthropic_api_key = Some(key),
        }
    }
    
    /// Get the default provider
    pub fn get_default_provider(&self) -> Option<ProviderType> {
        self.default_provider.as_ref().and_then(|p| p.parse().ok())
    }
    
    /// Set the default provider
    pub fn set_default_provider(&mut self, provider: ProviderType) {
        self.default_provider = Some(provider.to_string());
    }
}

/// Run the first-time setup wizard for agent credentials
pub fn run_setup_wizard() -> AgentResult<(ProviderType, Option<String>)> {
    use dialoguer::{Select, Input, theme::ColorfulTheme};
    
    println!("\n Welcome to Syncable Agent Setup\n");
    println!("This wizard will help you configure your LLM provider.\n");
    
    // Provider selection
    let providers = &["OpenAI (GPT-4)", "Anthropic (Claude)"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select your LLM provider")
        .items(providers)
        .default(0)
        .interact()
        .map_err(|e| AgentError::ClientError(format!("Selection failed: {}", e)))?;
    
    let provider = match selection {
        0 => ProviderType::OpenAI,
        1 => ProviderType::Anthropic,
        _ => ProviderType::OpenAI,
    };
    
    // API key input
    let env_var = match provider {
        ProviderType::OpenAI => "OPENAI_API_KEY",
        ProviderType::Anthropic => "ANTHROPIC_API_KEY",
    };
    
    let key_hint = match provider {
        ProviderType::OpenAI => "sk-... (from platform.openai.com)",
        ProviderType::Anthropic => "sk-ant-... (from console.anthropic.com)",
    };
    
    println!("\nYou can get your API key from:");
    match provider {
        ProviderType::OpenAI => println!("  https://platform.openai.com/api-keys"),
        ProviderType::Anthropic => println!("  https://console.anthropic.com/settings/keys"),
    }
    println!();
    
    let api_key: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Enter your API key {}", key_hint))
        .interact_text()
        .map_err(|e| AgentError::ClientError(format!("Input failed: {}", e)))?;
    
    if api_key.is_empty() {
        return Err(AgentError::MissingApiKey(env_var.into()));
    }
    
    // Model selection (optional)
    let default_models = match provider {
        ProviderType::OpenAI => vec!["gpt-4o (recommended)", "gpt-4", "gpt-3.5-turbo"],
        ProviderType::Anthropic => vec!["claude-3-5-sonnet-latest (recommended)", "claude-3-opus-latest", "claude-3-haiku-20240307"],
    };
    
    let model_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select default model")
        .items(&default_models)
        .default(0)
        .interact()
        .map_err(|e| AgentError::ClientError(format!("Selection failed: {}", e)))?;
    
    let model = match provider {
        ProviderType::OpenAI => match model_selection {
            0 => "gpt-4o",
            1 => "gpt-4",
            2 => "gpt-3.5-turbo",
            _ => "gpt-4o",
        },
        ProviderType::Anthropic => match model_selection {
            0 => "claude-3-5-sonnet-latest",
            1 => "claude-3-opus-latest",
            2 => "claude-3-haiku-20240307",
            _ => "claude-3-5-sonnet-latest",
        },
    };
    
    // Save credentials
    let mut creds = AgentCredentials::load().unwrap_or_default();
    creds.set_api_key(provider, api_key.clone());
    creds.set_default_provider(provider);
    creds.default_model = Some(model.to_string());
    creds.save()?;
    
    // Also set the environment variable for this session
    // SAFETY: We're setting a well-known env var with a valid string value
    unsafe { std::env::set_var(env_var, &api_key) };
    
    println!("\n Credentials saved to ~/.syncable/credentials.toml");
    println!("You can update them anytime by running: sync-ctl chat --setup\n");
    
    Ok((provider, Some(model.to_string())))
}

/// Ensure credentials are available, prompting for setup if needed
pub fn ensure_credentials(provider: Option<ProviderType>) -> AgentResult<(ProviderType, Option<String>)> {
    let creds = AgentCredentials::load().unwrap_or_default();
    
    // Determine which provider to use
    let provider = provider
        .or_else(|| creds.get_default_provider())
        .unwrap_or(ProviderType::OpenAI);
    
    // Check if we have credentials for this provider
    let env_var = match provider {
        ProviderType::OpenAI => "OPENAI_API_KEY",
        ProviderType::Anthropic => "ANTHROPIC_API_KEY",
    };
    
    // First check environment variable
    if std::env::var(env_var).is_ok() {
        return Ok((provider, creds.default_model.clone()));
    }
    
    // Then check stored credentials
    if let Some(key) = creds.get_api_key(provider) {
        // Set environment variable for this session
        // SAFETY: We're setting a well-known env var with a valid string value
        unsafe { std::env::set_var(env_var, key) };
        return Ok((provider, creds.default_model.clone()));
    }
    
    // No credentials found, run setup
    println!("No API key found for {}.", provider);
    run_setup_wizard()
}
