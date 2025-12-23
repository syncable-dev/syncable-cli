use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub analysis: AnalysisConfig,
    pub generation: GenerationConfig,
    pub output: OutputConfig,
    pub telemetry: TelemetryConfig,
    #[serde(default)]
    pub agent: AgentConfig,
}

/// Analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub include_dev_dependencies: bool,
    pub deep_analysis: bool,
    pub ignore_patterns: Vec<String>,
    pub max_file_size: usize,
}

/// Generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub dockerfile: DockerfileConfig,
    pub compose: ComposeConfig,
    pub terraform: TerraformConfig,
}

/// Dockerfile generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerfileConfig {
    pub base_image_override: Option<String>,
    pub use_multi_stage: bool,
    pub optimize_for_size: bool,
    pub include_health_check: bool,
}

/// Docker Compose generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeConfig {
    pub version: String,
    pub include_database: bool,
    pub include_redis: bool,
}

/// Terraform generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraformConfig {
    pub provider: String,
    pub include_networking: bool,
    pub include_monitoring: bool,
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub format: OutputFormat,
    pub overwrite_existing: bool,
    pub create_backup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Files,
    Stdout,
    Json,
}

// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
}

/// Agent/Chat configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentConfig {
    /// OpenAI API key (legacy, use profiles instead)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
    /// Anthropic API key (legacy, use profiles instead)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic_api_key: Option<String>,
    /// AWS Bedrock configuration (legacy, use profiles instead)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bedrock: Option<BedrockConfig>,
    /// AWS Bedrock configured flag (legacy)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bedrock_configured: Option<bool>,
    /// Default provider (openai, anthropic, or bedrock)
    #[serde(default = "default_provider")]
    pub default_provider: String,
    /// Default model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,

    // --- Global Profile support ---
    /// Named profiles containing all provider settings
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub profiles: HashMap<String, Profile>,
    /// Currently active profile name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_profile: Option<String>,

    // --- Legacy per-provider profiles (deprecated, kept for migration) ---
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub openai_profiles: HashMap<String, OpenAIProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_active_profile: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub anthropic_profiles: HashMap<String, AnthropicProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic_active_profile: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub bedrock_profiles: HashMap<String, BedrockConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bedrock_active_profile: Option<String>,
}

/// A global profile containing settings for all providers
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    /// Description of this profile (e.g., "Work", "Personal")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Default provider for this profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<String>,
    /// Default model for this profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    /// OpenAI settings for this profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai: Option<OpenAIProfile>,
    /// Anthropic settings for this profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic: Option<AnthropicProfile>,
    /// Bedrock settings for this profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bedrock: Option<BedrockConfig>,
}

/// OpenAI profile configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAIProfile {
    /// API key for this profile
    pub api_key: String,
    /// Optional description/label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Preferred model for this profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
}

/// Anthropic profile configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnthropicProfile {
    /// API key for this profile
    pub api_key: String,
    /// Optional description/label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Preferred model for this profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
}

/// AWS Bedrock configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BedrockConfig {
    /// AWS region (e.g., us-east-1, us-west-2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// AWS profile name from ~/.aws/credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// AWS Access Key ID (alternative to profile)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    /// AWS Secret Access Key (alternative to profile)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    /// AWS Bearer Token for Bedrock (used by Bedrock API Gateway)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_token: Option<String>,
    /// Preferred model ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
}

fn default_provider() -> String {
    "openai".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            analysis: AnalysisConfig {
                include_dev_dependencies: false,
                deep_analysis: true,
                ignore_patterns: vec![
                    "node_modules".to_string(),
                    ".git".to_string(),
                    "target".to_string(),
                    "build".to_string(),
                ],
                max_file_size: 1024 * 1024, // 1MB
            },
            generation: GenerationConfig {
                dockerfile: DockerfileConfig {
                    base_image_override: None,
                    use_multi_stage: true,
                    optimize_for_size: true,
                    include_health_check: true,
                },
                compose: ComposeConfig {
                    version: "3.8".to_string(),
                    include_database: false,
                    include_redis: false,
                },
                terraform: TerraformConfig {
                    provider: "docker".to_string(),
                    include_networking: true,
                    include_monitoring: false,
                },
            },
            output: OutputConfig {
                format: OutputFormat::Files,
                overwrite_existing: false,
                create_backup: true,
            },
            telemetry: TelemetryConfig {
                enabled: true,
            },
            agent: AgentConfig::default(),
        }
    }
}