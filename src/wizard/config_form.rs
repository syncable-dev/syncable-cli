//! Deployment configuration form for the wizard

use crate::analyzer::DiscoveredDockerfile;
use crate::platform::api::types::{
    CloudProvider, DeploymentSecretInput, DeploymentTarget, WizardDeploymentConfig,
};
use crate::wizard::render::display_step_header;
use colored::Colorize;
use inquire::{Confirm, InquireError, Select, Text};
use std::path::{Path, PathBuf};

const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "vendor",
    "dist",
    ".next",
    ".nuxt",
    "__pycache__",
    ".venv",
    "venv",
];
const MAX_DEPTH: usize = 3;

/// Discover `.env` files in the project directory (max depth 3, skipping common build dirs).
///
/// Returns paths relative to `root`, sorted.
pub fn discover_env_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk_for_env_files(root, root, 0, &mut found);
    found.sort();
    found
}

fn walk_for_env_files(root: &Path, dir: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    if depth > MAX_DEPTH {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if path.is_file() && name_str.starts_with(".env") && !name_str.starts_with(".envrc") {
            if let Ok(rel) = path.strip_prefix(root) {
                found.push(rel.to_path_buf());
            }
        } else if path.is_dir() && !IGNORED_DIRS.contains(&name_str.as_ref()) {
            walk_for_env_files(root, &path, depth + 1, found);
        }
    }
}

/// Parsed entry from a `.env` file.
#[derive(Debug, Clone)]
pub struct EnvFileEntry {
    pub key: String,
    pub value: String,
    pub is_secret: bool,
}

/// Parse a `.env` file into key/value entries.
///
/// Skips empty lines and comments (`#`). Strips surrounding quotes from values.
/// Each entry is tagged with `is_secret` based on key patterns.
pub fn parse_env_file(path: &Path) -> Result<Vec<EnvFileEntry>, std::io::Error> {
    let content = std::fs::read_to_string(path)?;
    let entries = content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .map(|v| v.to_string())
                .or_else(|| {
                    value
                        .strip_prefix('\'')
                        .and_then(|v| v.strip_suffix('\''))
                        .map(|v| v.to_string())
                })
                .unwrap_or(value);
            if key.is_empty() {
                return None;
            }
            Some(EnvFileEntry {
                is_secret: is_likely_secret(&key),
                key,
                value,
            })
        })
        .collect();
    Ok(entries)
}

/// Count non-empty, non-comment KEY=VALUE lines in a file.
fn count_env_vars_in_file(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|c| {
            c.lines()
                .filter(|l| {
                    let l = l.trim();
                    !l.is_empty() && !l.starts_with('#') && l.contains('=')
                })
                .count()
        })
        .unwrap_or(0)
}

/// Result of config form step
#[derive(Debug, Clone)]
pub enum ConfigFormResult {
    /// User completed the form
    Completed(WizardDeploymentConfig),
    /// User wants to go back
    Back,
    /// User cancelled the wizard
    Cancelled,
}

/// Collect deployment configuration details from user
///
/// Region, machine type, Dockerfile path, and build context are already selected
/// in previous steps. This form collects service name, port, branch, public access,
/// health check, and auto-deploy settings.
#[allow(clippy::too_many_arguments)]
pub fn collect_config(
    provider: CloudProvider,
    target: DeploymentTarget,
    cluster_id: Option<String>,
    registry_id: Option<String>,
    environment_id: &str,
    dockerfile_path: &str,
    build_context: &str,
    discovered_dockerfile: &DiscoveredDockerfile,
    region: Option<String>,
    machine_type: Option<String>,
    cpu: Option<String>,
    memory: Option<String>,
    step_number: u8,
) -> ConfigFormResult {
    display_step_header(
        step_number,
        "Configure Service",
        "Provide details for your service deployment.",
    );

    // Show previously selected options
    println!("  {} Dockerfile: {}", "│".dimmed(), dockerfile_path.cyan());
    println!("  {} Build context: {}", "│".dimmed(), build_context.cyan());
    if let Some(ref r) = region {
        println!("  {} Region: {}", "│".dimmed(), r.cyan());
    }
    if let Some(ref c) = cpu {
        if let Some(ref m) = memory {
            println!(
                "  {} Resources: {} vCPU / {}",
                "│".dimmed(),
                c.cyan(),
                m.cyan()
            );
        }
    } else if let Some(ref m) = machine_type {
        println!("  {} Machine: {}", "│".dimmed(), m.cyan());
    }
    println!();

    // Pre-populate from discovery
    let default_name = discovered_dockerfile.suggested_service_name.clone();
    let default_port = discovered_dockerfile.suggested_port.unwrap_or(8080);

    // Get current git branch for default
    let default_branch = get_current_branch().unwrap_or_else(|| "main".to_string());

    // Service name
    let service_name = match Text::new("Service name:")
        .with_default(&default_name)
        .with_help_message("K8s-compatible name (lowercase, hyphens)")
        .prompt()
    {
        Ok(name) => sanitize_service_name(&name),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return ConfigFormResult::Cancelled;
        }
        Err(_) => return ConfigFormResult::Cancelled,
    };

    // Port
    let port_str = default_port.to_string();
    let port = match Text::new("Service port:")
        .with_default(&port_str)
        .with_help_message("Port your service listens on")
        .prompt()
    {
        Ok(p) => p.parse::<u16>().unwrap_or(default_port),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return ConfigFormResult::Cancelled;
        }
        Err(_) => return ConfigFormResult::Cancelled,
    };

    // Branch
    let branch = match Text::new("Git branch:")
        .with_default(&default_branch)
        .with_help_message("Branch to deploy from")
        .prompt()
    {
        Ok(b) => b,
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return ConfigFormResult::Cancelled;
        }
        Err(_) => return ConfigFormResult::Cancelled,
    };

    // Public access toggle (for Cloud Runner)
    let is_public = if target == DeploymentTarget::CloudRunner {
        println!();
        println!(
            "{}",
            "─── Access Configuration ────────────────────".dimmed()
        );
        match Confirm::new("Enable public access?")
            .with_default(true)
            .with_help_message("Make service accessible via public IP/URL")
            .prompt()
        {
            Ok(v) => v,
            Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
                return ConfigFormResult::Cancelled;
            }
            Err(_) => return ConfigFormResult::Cancelled,
        }
    } else {
        true // Default to public for K8s
    };

    // Health check (optional)
    let health_check_path = if target == DeploymentTarget::CloudRunner {
        match Confirm::new("Configure health check endpoint?")
            .with_default(false)
            .with_help_message("Optional HTTP health probe for your service")
            .prompt()
        {
            Ok(true) => {
                match Text::new("Health check path:")
                    .with_default("/health")
                    .with_help_message("e.g., /health, /healthz, /api/health")
                    .prompt()
                {
                    Ok(path) => Some(path),
                    Err(InquireError::OperationCanceled)
                    | Err(InquireError::OperationInterrupted) => {
                        return ConfigFormResult::Cancelled;
                    }
                    Err(_) => return ConfigFormResult::Cancelled,
                }
            }
            Ok(false) => None,
            Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
                return ConfigFormResult::Cancelled;
            }
            Err(_) => return ConfigFormResult::Cancelled,
        }
    } else {
        None
    };

    // Auto-deploy disabled by default (CI/CD not ready yet)
    let auto_deploy = false;

    // Build the config
    let config = WizardDeploymentConfig {
        service_name: Some(service_name.clone()),
        dockerfile_path: Some(dockerfile_path.to_string()),
        build_context: Some(build_context.to_string()),
        port: Some(port),
        branch: Some(branch),
        target: Some(target),
        provider: Some(provider),
        cluster_id,
        registry_id,
        environment_id: Some(environment_id.to_string()),
        auto_deploy,
        region,
        machine_type,
        cpu,
        memory,
        is_public,
        health_check_path,
        secrets: Vec::new(), // Populated by collect_env_vars() in orchestrator
    };

    println!("\n{} Configuration complete: {}", "✓".green(), service_name);

    ConfigFormResult::Completed(config)
}

/// Collect environment variables interactively
///
/// Auto-discovers `.env` files in the project directory and presents them
/// as selectable options alongside manual entry. Uses `is_likely_secret()`
/// per-key instead of marking all values as secret.
///
/// Returns collected env vars, or empty vec if user skips.
pub fn collect_env_vars(project_path: &Path) -> Vec<DeploymentSecretInput> {
    println!();
    println!(
        "{}",
        "─── Environment Variables ──────────────────────".dimmed()
    );

    let wants_env_vars = match Confirm::new("Add environment variables?")
        .with_default(false)
        .with_help_message("Configure env vars / secrets for the deployment")
        .prompt()
    {
        Ok(v) => v,
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return Vec::new();
        }
        Err(_) => return Vec::new(),
    };

    if !wants_env_vars {
        return Vec::new();
    }

    // Auto-discover .env files
    let discovered = discover_env_files(project_path);

    // Build select options
    let mut options: Vec<String> = Vec::new();

    if !discovered.is_empty() {
        println!(
            "\n  Found {} .env file(s):\n",
            discovered.len().to_string().cyan()
        );
        for f in &discovered {
            let abs = project_path.join(f);
            let count = count_env_vars_in_file(&abs);
            let label = format!("  {:<30} {} vars", f.display(), count.to_string().cyan());
            println!("    {}", label);
            options.push(format!("{:<30} {} vars", f.display(), count));
        }
        println!();
    }

    options.push("Enter path manually...".to_string());
    options.push("Manual entry (key/value)".to_string());

    let method = match Select::new("How would you like to add env vars?", options.clone()).prompt()
    {
        Ok(m) => m,
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return Vec::new();
        }
        Err(_) => return Vec::new(),
    };

    if method == "Manual entry (key/value)" {
        return collect_env_vars_manually();
    }

    if method == "Enter path manually..." {
        return collect_env_vars_from_file(project_path, None);
    }

    // User picked a discovered file — extract the path portion (before the var count)
    let idx = options.iter().position(|o| o == &method).unwrap_or(0);
    if idx < discovered.len() {
        let rel = &discovered[idx];
        let abs = project_path.join(rel);
        collect_env_vars_from_file(project_path, Some(&abs))
    } else {
        Vec::new()
    }
}

/// Collect env vars via manual key/value entry
fn collect_env_vars_manually() -> Vec<DeploymentSecretInput> {
    let mut secrets = Vec::new();

    loop {
        let key = match Text::new("Variable name:")
            .with_help_message("e.g., DATABASE_URL, API_KEY, NODE_ENV")
            .prompt()
        {
            Ok(k) if k.trim().is_empty() => break,
            Ok(k) => k.trim().to_uppercase(),
            Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
                break;
            }
            Err(_) => break,
        };

        let value = match Text::new("Value:")
            .with_help_message("The environment variable value")
            .prompt()
        {
            Ok(v) => v,
            Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
                break;
            }
            Err(_) => break,
        };

        let is_secret = match Confirm::new("Is this a secret?")
            .with_default(is_likely_secret(&key))
            .with_help_message("Secrets are masked in UI and API responses")
            .prompt()
        {
            Ok(v) => v,
            Err(_) => is_likely_secret(&key),
        };

        println!(
            "  {} {} {}",
            "✓".green(),
            key.cyan(),
            if is_secret {
                "(secret)".dimmed().to_string()
            } else {
                "".to_string()
            }
        );

        secrets.push(DeploymentSecretInput {
            key,
            value,
            is_secret,
        });

        let add_another = match Confirm::new("Add another?").with_default(false).prompt() {
            Ok(v) => v,
            Err(_) => false,
        };

        if !add_another {
            break;
        }
    }

    secrets
}

/// Collect env vars by loading and parsing a .env file.
///
/// If `resolved_path` is `Some`, the file is read directly (user picked a discovered file).
/// Otherwise the user is prompted for a path.
fn collect_env_vars_from_file(
    project_path: &Path,
    resolved_path: Option<&Path>,
) -> Vec<DeploymentSecretInput> {
    let (abs_path, display_path) = if let Some(p) = resolved_path {
        (p.to_path_buf(), p.display().to_string())
    } else {
        let file_path = match Text::new("Path to .env file:")
            .with_default(".env")
            .with_help_message("Relative or absolute path to your .env file")
            .prompt()
        {
            Ok(p) => p,
            Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
                return Vec::new();
            }
            Err(_) => return Vec::new(),
        };
        let p = Path::new(&file_path);
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            project_path.join(p)
        };
        (abs, file_path)
    };

    let content = match std::fs::read_to_string(&abs_path) {
        Ok(c) => c,
        Err(e) => {
            println!("{} Failed to read file: {}", "✗".red(), e);
            return Vec::new();
        }
    };

    let secrets: Vec<DeploymentSecretInput> = content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            // Parse KEY=VALUE (handle quoted values)
            let (key, value) = line.split_once('=')?;
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            // Strip surrounding quotes from value
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .map(|v| v.to_string())
                .or_else(|| {
                    value
                        .strip_prefix('\'')
                        .and_then(|v| v.strip_suffix('\''))
                        .map(|v| v.to_string())
                })
                .unwrap_or(value);

            if key.is_empty() {
                return None;
            }

            Some(DeploymentSecretInput {
                is_secret: is_likely_secret(&key),
                key,
                value,
            })
        })
        .collect();

    if secrets.is_empty() {
        println!("{} No variables found in file", "⚠".yellow());
        return Vec::new();
    }

    // Show loaded keys (NOT values) for confirmation
    println!();
    println!(
        "  Loaded {} variable(s) from {}:",
        secrets.len().to_string().cyan(),
        display_path.dimmed()
    );
    for s in &secrets {
        if s.is_secret {
            println!(
                "    {} {} {}",
                "•".dimmed(),
                s.key.cyan(),
                "(secret)".dimmed()
            );
        } else {
            println!("    {} {}", "•".dimmed(), s.key.cyan());
        }
    }
    println!();

    let secret_count = secrets.iter().filter(|s| s.is_secret).count();
    let plain_count = secrets.len() - secret_count;
    if secret_count > 0 {
        println!(
            "  {} {} secret(s), {} plain variable(s)",
            "ℹ".blue(),
            secret_count.to_string().yellow(),
            plain_count.to_string().cyan()
        );
    }

    let confirm = match Confirm::new("Use these variables?")
        .with_default(true)
        .prompt()
    {
        Ok(v) => v,
        Err(_) => false,
    };

    if confirm { secrets } else { Vec::new() }
}

/// Check if a key name looks like it should be a secret
fn is_likely_secret(key: &str) -> bool {
    let key_upper = key.to_uppercase();
    let secret_patterns = [
        "_KEY",
        "_SECRET",
        "_TOKEN",
        "_PASSWORD",
        "_PASSWD",
        "_PWD",
        "DATABASE_URL",
        "REDIS_URL",
        "MONGODB_URI",
        "CONNECTION_STRING",
        "_CREDENTIALS",
        "_AUTH",
        "_PRIVATE",
        "API_KEY",
        "APIKEY",
    ];
    secret_patterns.iter().any(|p| key_upper.contains(p))
}

/// Get current git branch name
fn get_current_branch() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Sanitize service name for K8s compatibility
fn sanitize_service_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_service_name() {
        assert_eq!(sanitize_service_name("My Service"), "my-service");
        assert_eq!(sanitize_service_name("foo_bar"), "foo-bar");
        assert_eq!(sanitize_service_name("--test--"), "test");
        assert_eq!(sanitize_service_name("API Server"), "api-server");
    }

    #[test]
    fn test_config_form_result_variants() {
        let config = WizardDeploymentConfig::default();
        let _ = ConfigFormResult::Completed(config);
        let _ = ConfigFormResult::Back;
        let _ = ConfigFormResult::Cancelled;
    }
}
