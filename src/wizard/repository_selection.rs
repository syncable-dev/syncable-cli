//! Repository selection step for the deployment wizard
//!
//! Detects the repository from local git remote or asks user to select.

use crate::platform::api::PlatformApiClient;
use crate::platform::api::types::{AvailableRepository, ProjectRepository};
use crate::wizard::render::{display_step_header, wizard_render_config};
use colored::Colorize;
use inquire::{Confirm, InquireError, Select};
use std::fmt;
use std::path::Path;
use std::process::Command;

/// Result of repository selection step
#[derive(Debug, Clone)]
pub enum RepositorySelectionResult {
    /// User selected a repository (already connected)
    Selected(ProjectRepository),
    /// User chose to connect a new repository
    ConnectNew(AvailableRepository),
    /// Need GitHub App installation for this org
    NeedsGitHubApp {
        installation_url: String,
        org_name: String,
    },
    /// No GitHub App installations found
    NoInstallations { installation_url: String },
    /// No repositories connected to project
    NoRepositories,
    /// User cancelled the wizard
    Cancelled,
    /// An error occurred
    Error(String),
}

/// Wrapper for displaying repository options in the selection menu
struct RepositoryOption {
    repository: ProjectRepository,
    is_detected: bool,
}

impl fmt::Display for RepositoryOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let marker = if self.is_detected { " (detected)" } else { "" };
        write!(
            f,
            "{}{}  {}",
            self.repository.repository_full_name.cyan(),
            marker.green(),
            self.repository
                .default_branch
                .as_deref()
                .unwrap_or("main")
                .dimmed()
        )
    }
}

/// Detect the git remote URL from the current directory
fn detect_git_remote(project_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(project_path)
        .output()
        .ok()?;

    if output.status.success() {
        let url = String::from_utf8(output.stdout).ok()?;
        Some(url.trim().to_string())
    } else {
        None
    }
}

/// Parse repository full name from git remote URL
/// Handles both SSH (git@github.com:owner/repo.git) and HTTPS (https://github.com/owner/repo.git)
fn parse_repo_from_url(url: &str) -> Option<String> {
    let url = url.trim();

    // SSH format: git@github.com:owner/repo.git
    if url.starts_with("git@") {
        let parts: Vec<&str> = url.split(':').collect();
        if parts.len() == 2 {
            let path = parts[1].trim_end_matches(".git");
            return Some(path.to_string());
        }
    }

    // HTTPS format: https://github.com/owner/repo.git
    if url.starts_with("https://") || url.starts_with("http://") {
        if let Some(path) = url
            .split('/')
            .skip(3)
            .collect::<Vec<_>>()
            .join("/")
            .strip_suffix(".git")
        {
            return Some(path.to_string());
        }
        // Without .git suffix
        let path: String = url.split('/').skip(3).collect::<Vec<_>>().join("/");
        if !path.is_empty() {
            return Some(path);
        }
    }

    None
}

/// Find a repository in the available repositories list by full name
fn find_in_available<'a>(
    repo_full_name: &str,
    available: &'a [AvailableRepository],
) -> Option<&'a AvailableRepository> {
    available
        .iter()
        .find(|r| r.full_name.eq_ignore_ascii_case(repo_full_name))
}

/// Check if a repository ID is in the connected list
fn is_repo_connected(repo_id: i64, connected_ids: &[i64]) -> bool {
    connected_ids.contains(&repo_id)
}

/// Extract organization/owner name from a repo full name
fn extract_org_name(repo_full_name: &str) -> String {
    repo_full_name
        .split('/')
        .next()
        .unwrap_or(repo_full_name)
        .to_string()
}

/// Prompt user to connect a detected repository
fn prompt_connect_repository(
    available: &AvailableRepository,
    connected: &[ProjectRepository],
) -> RepositorySelectionResult {
    println!(
        "\n{} Detected repository: {}",
        "→".cyan(),
        available.full_name.cyan()
    );
    println!(
        "{}",
        "This repository is not connected to the project.".dimmed()
    );

    // Build options
    let connect_option = format!("Connect {} (detected)", available.full_name);
    let mut options = vec![connect_option];

    // Add connected repos as alternatives
    for repo in connected {
        options.push(format!(
            "Use {} (already connected)",
            repo.repository_full_name
        ));
    }

    let selection = Select::new("What would you like to do?", options)
        .with_render_config(wizard_render_config())
        .with_help_message("Use ↑/↓ to navigate, Enter to select")
        .prompt();

    match selection {
        Ok(choice) if choice.starts_with("Connect") => {
            RepositorySelectionResult::ConnectNew(available.clone())
        }
        Ok(choice) => {
            // Find which connected repo was selected
            let repo_name = choice
                .split(" (already connected)")
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches("Use ");
            if let Some(repo) = connected
                .iter()
                .find(|r| r.repository_full_name == repo_name)
            {
                RepositorySelectionResult::Selected(repo.clone())
            } else {
                RepositorySelectionResult::Cancelled
            }
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            RepositorySelectionResult::Cancelled
        }
        Err(_) => RepositorySelectionResult::Cancelled,
    }
}

/// Prompt user to install GitHub App
async fn prompt_github_app_install(
    client: &PlatformApiClient,
    org_name: &str,
) -> RepositorySelectionResult {
    println!(
        "\n{} GitHub App not installed for: {}",
        "⚠".yellow(),
        org_name.cyan()
    );
    println!(
        "{}",
        "The Syncable GitHub App needs to be installed to connect this repository.".dimmed()
    );

    match client.get_github_installation_url().await {
        Ok(response) => {
            let install = Confirm::new("Open browser to install GitHub App?")
                .with_default(true)
                .prompt();

            if let Ok(true) = install {
                if webbrowser::open(&response.installation_url).is_ok() {
                    println!(
                        "{} Opened browser. Complete the installation, then run this command again.",
                        "→".cyan()
                    );
                } else {
                    println!("Visit: {}", response.installation_url);
                }
            }
            RepositorySelectionResult::NeedsGitHubApp {
                installation_url: response.installation_url,
                org_name: org_name.to_string(),
            }
        }
        Err(e) => {
            RepositorySelectionResult::Error(format!("Failed to get installation URL: {}", e))
        }
    }
}

/// Select repository for deployment
///
/// Smart repository selection with connection flow:
/// 1. Check for GitHub App installations
/// 2. Fetch connected and available repositories
/// 3. Detect local git remote and match against repos
/// 4. Offer to connect if local repo available but not connected
/// 5. Fall back to manual selection from available repos
pub async fn select_repository(
    client: &PlatformApiClient,
    project_id: &str,
    project_path: &Path,
) -> RepositorySelectionResult {
    // Check for GitHub App installations first
    let installations = match client.list_github_installations().await {
        Ok(response) => response.installations,
        Err(e) => {
            return RepositorySelectionResult::Error(format!(
                "Failed to fetch GitHub installations: {}",
                e
            ));
        }
    };

    // If no installations, prompt to install GitHub App
    if installations.is_empty() {
        println!("\n{} No GitHub App installations found.", "⚠".yellow());
        match client.get_github_installation_url().await {
            Ok(response) => {
                println!("Install the Syncable GitHub App to connect repositories.");
                let install = Confirm::new("Open browser to install GitHub App?")
                    .with_default(true)
                    .prompt();

                if let Ok(true) = install {
                    if webbrowser::open(&response.installation_url).is_ok() {
                        println!(
                            "{} Opened browser. Complete the installation, then run this command again.",
                            "→".cyan()
                        );
                    } else {
                        println!("Visit: {}", response.installation_url);
                    }
                }
                return RepositorySelectionResult::NoInstallations {
                    installation_url: response.installation_url,
                };
            }
            Err(e) => {
                return RepositorySelectionResult::Error(format!(
                    "Failed to get installation URL: {}",
                    e
                ));
            }
        }
    }

    // Fetch connected repositories
    let repos_response = match client.list_project_repositories(project_id).await {
        Ok(response) => response,
        Err(e) => {
            return RepositorySelectionResult::Error(format!(
                "Failed to fetch repositories: {}",
                e
            ));
        }
    };
    let connected_repos = repos_response.repositories;

    // Fetch available repositories (from all GitHub installations)
    let available_response = match client
        .list_available_repositories(Some(project_id), None, None)
        .await
    {
        Ok(response) => response,
        Err(e) => {
            return RepositorySelectionResult::Error(format!(
                "Failed to fetch available repositories: {}",
                e
            ));
        }
    };
    let available_repos = available_response.repositories;
    let connected_ids = available_response.connected_repositories;

    // Try to auto-detect from git remote
    let detected_repo_name =
        detect_git_remote(project_path).and_then(|url| parse_repo_from_url(&url));

    if let Some(ref local_repo_name) = detected_repo_name {
        // Check if already connected to this project
        if let Some(connected) = connected_repos
            .iter()
            .find(|r| r.repository_full_name.eq_ignore_ascii_case(local_repo_name))
        {
            // Auto-select connected repo
            println!(
                "\n{} Using detected repository: {}",
                "✓".green(),
                connected.repository_full_name.cyan()
            );
            return RepositorySelectionResult::Selected(connected.clone());
        }

        // Check if available but not connected
        if let Some(available) = find_in_available(local_repo_name, &available_repos) {
            if !is_repo_connected(available.id, &connected_ids) {
                // Offer to connect this repository
                return prompt_connect_repository(available, &connected_repos);
            }
        }

        // Local repo not in available list - might need GitHub App for this org
        let org_name = extract_org_name(local_repo_name);
        let org_has_installation = installations
            .iter()
            .any(|i| i.account_login.eq_ignore_ascii_case(&org_name));

        if !org_has_installation {
            // Need to install GitHub App for this organization
            return prompt_github_app_install(client, &org_name).await;
        }

        // Org has installation but repo not available - might be private or restricted
        println!(
            "\n{} Repository {} not accessible.",
            "⚠".yellow(),
            local_repo_name.cyan()
        );
        println!(
            "{}",
            "Check that the Syncable GitHub App has access to this repository.".dimmed()
        );
    }

    // No local repo detected or couldn't match - show selection UI
    if connected_repos.is_empty() && available_repos.is_empty() {
        println!("\n{} No repositories available.", "⚠".yellow());
        println!(
            "{}",
            "Connect a repository using the GitHub App installation.".dimmed()
        );
        return RepositorySelectionResult::NoRepositories;
    }

    display_step_header(
        0,
        "Select Repository",
        "Choose which repository to deploy from.",
    );

    // Build options: connected repos first, then available (unconnected) repos
    let mut options: Vec<RepositoryOption> = connected_repos
        .iter()
        .map(|repo| {
            let is_detected = detected_repo_name
                .as_ref()
                .map(|name| repo.repository_full_name.eq_ignore_ascii_case(name))
                .unwrap_or(false);
            RepositoryOption {
                repository: repo.clone(),
                is_detected,
            }
        })
        .collect();

    // Put detected repo first if found
    options.sort_by(|a, b| b.is_detected.cmp(&a.is_detected));

    if options.is_empty() {
        // No connected repos - offer available repos to connect
        println!(
            "{}",
            "No repositories connected yet. Select one to connect:".dimmed()
        );

        let available_options: Vec<String> = available_repos
            .iter()
            .filter(|r| !is_repo_connected(r.id, &connected_ids))
            .map(|r| r.full_name.clone())
            .collect();

        if available_options.is_empty() {
            return RepositorySelectionResult::NoRepositories;
        }

        let selection = Select::new("Select repository to connect:", available_options)
            .with_render_config(wizard_render_config())
            .with_help_message("Use ↑/↓ to navigate, Enter to select")
            .prompt();

        match selection {
            Ok(selected_name) => {
                if let Some(available) = available_repos
                    .iter()
                    .find(|r| r.full_name == selected_name)
                {
                    return RepositorySelectionResult::ConnectNew(available.clone());
                }
                RepositorySelectionResult::Cancelled
            }
            Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
                RepositorySelectionResult::Cancelled
            }
            Err(_) => RepositorySelectionResult::Cancelled,
        }
    } else {
        // Show connected repos for selection
        let selection = Select::new("Select repository:", options)
            .with_render_config(wizard_render_config())
            .with_help_message("Use ↑/↓ to navigate, Enter to select")
            .prompt();

        match selection {
            Ok(selected) => {
                println!(
                    "\n{} Selected repository: {}",
                    "✓".green(),
                    selected.repository.repository_full_name.cyan()
                );
                RepositorySelectionResult::Selected(selected.repository)
            }
            Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
                RepositorySelectionResult::Cancelled
            }
            Err(_) => RepositorySelectionResult::Cancelled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repo_from_ssh_url() {
        let url = "git@github.com:owner/my-repo.git";
        assert_eq!(parse_repo_from_url(url), Some("owner/my-repo".to_string()));
    }

    #[test]
    fn test_parse_repo_from_https_url() {
        let url = "https://github.com/owner/my-repo.git";
        assert_eq!(parse_repo_from_url(url), Some("owner/my-repo".to_string()));
    }

    #[test]
    fn test_parse_repo_from_https_url_no_git() {
        let url = "https://github.com/owner/my-repo";
        assert_eq!(parse_repo_from_url(url), Some("owner/my-repo".to_string()));
    }

    #[test]
    fn test_repository_selection_result_variants() {
        let repo = ProjectRepository {
            id: "test".to_string(),
            project_id: "proj".to_string(),
            repository_id: 123,
            repository_name: "test".to_string(),
            repository_full_name: "owner/test".to_string(),
            repository_owner: "owner".to_string(),
            repository_private: false,
            default_branch: Some("main".to_string()),
            is_active: true,
            connection_type: None,
            repository_type: None,
            is_primary_git_ops: None,
            github_installation_id: None,
            user_id: None,
            created_at: None,
            updated_at: None,
        };
        let available = AvailableRepository {
            id: 456,
            name: "test-repo".to_string(),
            full_name: "owner/test-repo".to_string(),
            owner: Some("owner".to_string()),
            private: false,
            default_branch: Some("main".to_string()),
            description: None,
            html_url: None,
            installation_id: Some(789),
        };
        let _ = RepositorySelectionResult::Selected(repo);
        let _ = RepositorySelectionResult::ConnectNew(available);
        let _ = RepositorySelectionResult::NeedsGitHubApp {
            installation_url: "https://github.com/apps/syncable".to_string(),
            org_name: "my-org".to_string(),
        };
        let _ = RepositorySelectionResult::NoInstallations {
            installation_url: "https://github.com/apps/syncable".to_string(),
        };
        let _ = RepositorySelectionResult::NoRepositories;
        let _ = RepositorySelectionResult::Cancelled;
        let _ = RepositorySelectionResult::Error("test".to_string());
    }

    #[test]
    fn test_extract_org_name() {
        assert_eq!(extract_org_name("owner/repo"), "owner");
        assert_eq!(extract_org_name("my-org/my-app"), "my-org");
        assert_eq!(extract_org_name("repo-only"), "repo-only");
    }

    #[test]
    fn test_is_repo_connected() {
        let connected = vec![1, 2, 3, 5];
        assert!(is_repo_connected(1, &connected));
        assert!(is_repo_connected(3, &connected));
        assert!(!is_repo_connected(4, &connected));
        assert!(!is_repo_connected(100, &connected));
    }

    #[test]
    fn test_find_in_available() {
        let available = vec![
            AvailableRepository {
                id: 1,
                name: "repo-a".to_string(),
                full_name: "owner/repo-a".to_string(),
                owner: Some("owner".to_string()),
                private: false,
                default_branch: Some("main".to_string()),
                description: None,
                html_url: None,
                installation_id: Some(100),
            },
            AvailableRepository {
                id: 2,
                name: "repo-b".to_string(),
                full_name: "other/repo-b".to_string(),
                owner: Some("other".to_string()),
                private: true,
                default_branch: Some("main".to_string()),
                description: None,
                html_url: None,
                installation_id: Some(200),
            },
        ];

        let found = find_in_available("owner/repo-a", &available);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, 1);

        // Case insensitive
        let found_case = find_in_available("OWNER/REPO-A", &available);
        assert!(found_case.is_some());

        let not_found = find_in_available("nonexistent/repo", &available);
        assert!(not_found.is_none());
    }
}
