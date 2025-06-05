use serde::{Deserialize, Serialize};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub analysis: AnalysisConfig,
    pub generation: GenerationConfig,
    pub output: OutputConfig,
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
        }
    }
} 