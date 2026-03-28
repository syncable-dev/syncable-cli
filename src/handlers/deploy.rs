//! Non-interactive deployment handlers for `deploy preview` and `deploy run`.
//!
//! Wraps the existing `DeployServiceTool` (from the chat agent) as CLI commands
//! so AI coding agents (Claude Code, Cursor, etc.) can deploy via skills.

use crate::agent::tools::ExecutionContext;
use crate::agent::tools::platform::{DeployServiceArgs, DeployServiceTool, SecretKeyInput};
use rig::tool::Tool;
use std::path::PathBuf;

/// Handle `deploy preview` — returns JSON recommendation without deploying.
pub async fn handle_deploy_preview(
    project_path: PathBuf,
    service_path: PathBuf,
    service_name: Option<String>,
    provider: Option<String>,
    region: Option<String>,
    machine_type: Option<String>,
    port: Option<u16>,
    is_public: bool,
) -> crate::Result<String> {
    let rel_path = if service_path == PathBuf::from(".") {
        None
    } else {
        Some(service_path.display().to_string())
    };

    let args = DeployServiceArgs {
        path: rel_path,
        service_name,
        provider,
        machine_type,
        region,
        port,
        is_public,
        cpu: None,
        memory: None,
        min_instances: None,
        max_instances: None,
        preview_only: true,
        secret_keys: None,
    };

    let tool = DeployServiceTool::with_context(project_path, ExecutionContext::HeadlessServer);
    let result = tool.call(args).await.map_err(|e| {
        crate::error::IaCGeneratorError::Analysis(crate::error::AnalysisError::InvalidStructure(
            format!("Deploy preview failed: {}", e),
        ))
    })?;

    Ok(result)
}

/// Handle `deploy run` — triggers actual deployment.
pub async fn handle_deploy_run(
    project_path: PathBuf,
    service_path: PathBuf,
    service_name: Option<String>,
    provider: Option<String>,
    region: Option<String>,
    machine_type: Option<String>,
    port: Option<u16>,
    is_public: bool,
    cpu: Option<String>,
    memory: Option<String>,
    min_instances: Option<i32>,
    max_instances: Option<i32>,
    env_vars: Vec<String>,
    secrets: Vec<String>,
    env_file: Option<PathBuf>,
) -> crate::Result<String> {
    let rel_path = if service_path == PathBuf::from(".") {
        None
    } else {
        Some(service_path.display().to_string())
    };

    // Build secret_keys from --env and --secret flags
    let mut secret_keys: Vec<SecretKeyInput> = Vec::new();

    // Parse --env KEY=VALUE pairs (non-secret)
    for env_str in &env_vars {
        if let Some((key, value)) = env_str.split_once('=') {
            secret_keys.push(SecretKeyInput {
                key: key.to_string(),
                value: Some(value.to_string()),
                is_secret: false,
            });
        } else {
            eprintln!(
                "Warning: ignoring malformed --env '{}' (expected KEY=VALUE)",
                env_str
            );
        }
    }

    // Parse --secret keys (user will be prompted in terminal)
    for key in &secrets {
        secret_keys.push(SecretKeyInput {
            key: key.to_string(),
            value: None,
            is_secret: true,
        });
    }

    // Load --env-file if provided
    if let Some(ref env_file_path) = env_file {
        if env_file_path.exists() {
            if let Ok(content) = std::fs::read_to_string(env_file_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim().to_string();
                        let value = value
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string();
                        // Detect likely secrets by key name
                        let looks_secret = key.contains("SECRET")
                            || key.contains("KEY")
                            || key.contains("TOKEN")
                            || key.contains("PASSWORD")
                            || key.contains("PRIVATE");
                        if looks_secret {
                            // Don't include value — user will be prompted
                            secret_keys.push(SecretKeyInput {
                                key,
                                value: None,
                                is_secret: true,
                            });
                        } else {
                            secret_keys.push(SecretKeyInput {
                                key,
                                value: Some(value),
                                is_secret: false,
                            });
                        }
                    }
                }
            } else {
                eprintln!(
                    "Warning: could not read env file: {}",
                    env_file_path.display()
                );
            }
        } else {
            eprintln!("Warning: env file not found: {}", env_file_path.display());
        }
    }

    let args = DeployServiceArgs {
        path: rel_path,
        service_name,
        provider,
        machine_type,
        region,
        port,
        is_public,
        cpu,
        memory,
        min_instances,
        max_instances,
        preview_only: false,
        secret_keys: if secret_keys.is_empty() {
            None
        } else {
            Some(secret_keys)
        },
    };

    // Use InteractiveCli so secrets can be prompted in terminal
    let tool = DeployServiceTool::with_context(project_path, ExecutionContext::InteractiveCli);
    let result = tool.call(args).await.map_err(|e| {
        crate::error::IaCGeneratorError::Analysis(crate::error::AnalysisError::InvalidStructure(
            format!("Deploy failed: {}", e),
        ))
    })?;

    Ok(result)
}
