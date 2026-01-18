//! Repository selection step for the deployment wizard
//!
//! Detects the repository from local git remote or asks user to select.

use crate::platform::api::types::ProjectRepository;
use crate::platform::api::PlatformApiClient;
use crate::wizard::render::{display_step_header, wizard_render_config};
use colored::Colorize;
use inquire::{InquireError, Select};
use std::fmt;
use std::path::Path;
use std::process::Command;

/// Result of repository selection step
#[derive(Debug, Clone)]
pub enum RepositorySelectionResult {
    /// User selected a repository
    Selected(ProjectRepository),
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
        if let Some(path) = url.split('/').skip(3).collect::<Vec<_>>().join("/").strip_suffix(".git") {
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

/// Select repository for deployment
///
/// Attempts to auto-detect from git remote, falls back to user selection.
pub async fn select_repository(
    client: &PlatformApiClient,
    project_id: &str,
    project_path: &Path,
) -> RepositorySelectionResult {
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

    let repositories = repos_response.repositories;

    if repositories.is_empty() {
        println!(
            "\n{} No repositories connected to this project.",
            "⚠".yellow()
        );
        println!(
            "{}",
            "Connect a repository in the platform UI first.".dimmed()
        );
        return RepositorySelectionResult::NoRepositories;
    }

    // Try to auto-detect from git remote
    let detected_repo_name = detect_git_remote(project_path)
        .and_then(|url| parse_repo_from_url(&url));

    // Find matching repository ID (save the ID to avoid borrow issues)
    let detected_repo_id: Option<String> = detected_repo_name.as_ref().and_then(|name| {
        repositories
            .iter()
            .find(|r| r.repository_full_name.eq_ignore_ascii_case(name))
            .map(|r| r.id.clone())
    });

    // If exactly one repo and it matches detected, use it automatically
    if repositories.len() == 1 {
        let repo = &repositories[0];
        if detected_repo_id.as_ref().map(|id| id == &repo.id).unwrap_or(false) {
            println!(
                "\n{} Using detected repository: {}",
                "✓".green(),
                repo.repository_full_name.cyan()
            );
            return RepositorySelectionResult::Selected(repo.clone());
        }
    }

    // Show selection UI
    display_step_header(
        0,
        "Select Repository",
        "Choose which repository to deploy from.",
    );

    // Build options, marking detected one
    let options: Vec<RepositoryOption> = repositories
        .into_iter()
        .map(|repo| {
            let is_detected = detected_repo_id
                .as_ref()
                .map(|id| id == &repo.id)
                .unwrap_or(false);
            RepositoryOption {
                repository: repo,
                is_detected,
            }
        })
        .collect();

    // Put detected repo first if found
    let mut sorted_options = options;
    sorted_options.sort_by(|a, b| b.is_detected.cmp(&a.is_detected));

    let selection = Select::new("Select repository:", sorted_options)
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
        let _ = RepositorySelectionResult::Selected(repo);
        let _ = RepositorySelectionResult::NoRepositories;
        let _ = RepositorySelectionResult::Cancelled;
        let _ = RepositorySelectionResult::Error("test".to_string());
    }
}
