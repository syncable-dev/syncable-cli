//! Dockerfile selection step for the deployment wizard
//!
//! Provides smart Dockerfile discovery and selection with build context options.

use crate::analyzer::DiscoveredDockerfile;
use crate::wizard::render::{display_step_header, wizard_render_config};
use colored::Colorize;
use inquire::{Confirm, InquireError, Select, Text};
use std::fmt;
use std::path::Path;

/// Result of Dockerfile selection step
#[derive(Debug, Clone)]
pub enum DockerfileSelectionResult {
    /// User selected a Dockerfile with build context
    Selected {
        dockerfile: DiscoveredDockerfile,
        build_context: String,
    },
    /// User wants the agent to create a Dockerfile
    StartAgent(String),
    /// User wants to go back
    Back,
    /// User cancelled the wizard
    Cancelled,
}

/// Build context options for the user to choose from
#[derive(Debug, Clone)]
enum BuildContextOption {
    /// Directory containing the Dockerfile
    DockerfileDirectory(String),
    /// Repository root
    RepositoryRoot,
    /// Custom user-specified path
    Custom,
}

impl fmt::Display for BuildContextOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildContextOption::DockerfileDirectory(path) => {
                write!(f, "Dockerfile's directory    {}", path.dimmed())
            }
            BuildContextOption::RepositoryRoot => {
                write!(f, "Repository root           {}", ".".dimmed())
            }
            BuildContextOption::Custom => {
                write!(f, "Custom path...")
            }
        }
    }
}

/// Wrapper for displaying Dockerfile options in the selection menu
struct DockerfileOption<'a> {
    dockerfile: &'a DiscoveredDockerfile,
    project_root: &'a Path,
}

impl<'a> fmt::Display for DockerfileOption<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Get relative path from project root
        let relative_path = self
            .dockerfile
            .path
            .strip_prefix(self.project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| self.dockerfile.path.to_string_lossy().to_string());

        // Show: path → build_context
        let build_context = if self.dockerfile.build_context == "." {
            ". (root)".to_string()
        } else {
            self.dockerfile.build_context.clone()
        };

        write!(
            f,
            "{}  {}  {}",
            relative_path,
            "→".dimmed(),
            build_context.dimmed()
        )
    }
}

/// Select a Dockerfile from discovered Dockerfiles
///
/// Handles three cases:
/// - Multiple Dockerfiles: Show selection menu
/// - Single Dockerfile: Auto-select with confirmation
/// - No Dockerfiles: Offer to start agent for creation
pub fn select_dockerfile(
    dockerfiles: &[DiscoveredDockerfile],
    project_root: &Path,
) -> DockerfileSelectionResult {
    display_step_header(
        5,
        "Select Dockerfile",
        "Choose the Dockerfile to use for deployment.",
    );

    match dockerfiles.len() {
        0 => handle_no_dockerfiles(),
        1 => handle_single_dockerfile(&dockerfiles[0], project_root),
        _ => handle_multiple_dockerfiles(dockerfiles, project_root),
    }
}

/// Handle case when no Dockerfiles are found
fn handle_no_dockerfiles() -> DockerfileSelectionResult {
    println!(
        "\n{} {}",
        "⚠".yellow(),
        "No Dockerfiles found in this project.".yellow()
    );

    match Confirm::new("Would you like the agent to help create one?")
        .with_default(true)
        .with_help_message("Start an AI-assisted session to generate a Dockerfile")
        .prompt()
    {
        Ok(true) => {
            let prompt = "Help me create a Dockerfile for this project. Analyze the codebase and suggest an appropriate Dockerfile with best practices for production deployment.".to_string();
            DockerfileSelectionResult::StartAgent(prompt)
        }
        Ok(false) => DockerfileSelectionResult::Cancelled,
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            DockerfileSelectionResult::Cancelled
        }
        Err(_) => DockerfileSelectionResult::Cancelled,
    }
}

/// Handle case when only one Dockerfile is found
fn handle_single_dockerfile(
    dockerfile: &DiscoveredDockerfile,
    project_root: &Path,
) -> DockerfileSelectionResult {
    let relative_path = dockerfile
        .path
        .strip_prefix(project_root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| dockerfile.path.to_string_lossy().to_string());

    println!(
        "\n{} Found: {}",
        "✓".green(),
        relative_path.cyan()
    );

    // Show additional info if available
    if let Some(ref base) = dockerfile.base_image {
        println!("  {} Base image: {}", "│".dimmed(), base.dimmed());
    }
    if let Some(port) = dockerfile.suggested_port {
        println!("  {} Suggested port: {}", "│".dimmed(), port.to_string().dimmed());
    }

    // Proceed to build context selection
    select_build_context(dockerfile)
}

/// Handle case when multiple Dockerfiles are found
fn handle_multiple_dockerfiles(
    dockerfiles: &[DiscoveredDockerfile],
    project_root: &Path,
) -> DockerfileSelectionResult {
    println!(
        "\n{} Found {} Dockerfiles:",
        "ℹ".blue(),
        dockerfiles.len().to_string().cyan()
    );

    // Create display options
    let options: Vec<DockerfileOption> = dockerfiles
        .iter()
        .map(|df| DockerfileOption {
            dockerfile: df,
            project_root,
        })
        .collect();

    // Build the selection menu
    let selection = Select::new("Select Dockerfile:", options)
        .with_render_config(wizard_render_config())
        .with_help_message("Use ↑/↓ to navigate, Enter to select")
        .prompt();

    match selection {
        Ok(selected) => {
            // Find the selected dockerfile by matching path
            let selected_df = dockerfiles
                .iter()
                .find(|df| std::ptr::eq(*df, selected.dockerfile))
                .unwrap();
            select_build_context(selected_df)
        }
        Err(InquireError::OperationCanceled) => DockerfileSelectionResult::Back,
        Err(InquireError::OperationInterrupted) => DockerfileSelectionResult::Cancelled,
        Err(_) => DockerfileSelectionResult::Cancelled,
    }
}

/// Select build context for the chosen Dockerfile
fn select_build_context(dockerfile: &DiscoveredDockerfile) -> DockerfileSelectionResult {
    println!();
    println!(
        "{}",
        "─── Build Context ───────────────────────────".dimmed()
    );
    println!(
        "  {}",
        "The build context is the directory sent to Docker during build.".dimmed()
    );

    // Compute dockerfile directory (default build context)
    let dockerfile_dir = dockerfile
        .path
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                ".".to_string()
            } else {
                p.to_string_lossy().to_string()
            }
        })
        .unwrap_or_else(|| ".".to_string());

    // Use the computed build_context from discovery as dockerfile directory display
    let display_dir = if dockerfile.build_context.is_empty() || dockerfile.build_context == "." {
        ".".to_string()
    } else {
        dockerfile.build_context.clone()
    };

    // Build options
    let options = vec![
        BuildContextOption::DockerfileDirectory(display_dir.clone()),
        BuildContextOption::RepositoryRoot,
        BuildContextOption::Custom,
    ];

    let selection = Select::new("Build context:", options)
        .with_render_config(wizard_render_config())
        .with_help_message("Select the directory to use as Docker build context")
        .prompt();

    match selection {
        Ok(BuildContextOption::DockerfileDirectory(_)) => DockerfileSelectionResult::Selected {
            dockerfile: dockerfile.clone(),
            build_context: display_dir,
        },
        Ok(BuildContextOption::RepositoryRoot) => DockerfileSelectionResult::Selected {
            dockerfile: dockerfile.clone(),
            build_context: ".".to_string(),
        },
        Ok(BuildContextOption::Custom) => {
            // Prompt for custom path
            match Text::new("Custom build context path:")
                .with_default(&dockerfile_dir)
                .with_help_message("Relative path from repository root")
                .prompt()
            {
                Ok(path) => DockerfileSelectionResult::Selected {
                    dockerfile: dockerfile.clone(),
                    build_context: path,
                },
                Err(InquireError::OperationCanceled) => DockerfileSelectionResult::Back,
                Err(InquireError::OperationInterrupted) => DockerfileSelectionResult::Cancelled,
                Err(_) => DockerfileSelectionResult::Cancelled,
            }
        }
        Err(InquireError::OperationCanceled) => DockerfileSelectionResult::Back,
        Err(InquireError::OperationInterrupted) => DockerfileSelectionResult::Cancelled,
        Err(_) => DockerfileSelectionResult::Cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_dockerfile(path: &str, build_context: &str) -> DiscoveredDockerfile {
        DiscoveredDockerfile {
            path: PathBuf::from(path),
            build_context: build_context.to_string(),
            suggested_service_name: "test-service".to_string(),
            suggested_port: Some(8080),
            base_image: Some("node:18".to_string()),
            is_multistage: false,
            environment: None,
        }
    }

    #[test]
    fn test_dockerfile_option_display() {
        let df = create_test_dockerfile("/project/services/api/Dockerfile", "services/api");
        let project_root = PathBuf::from("/project");
        let option = DockerfileOption {
            dockerfile: &df,
            project_root: &project_root,
        };
        let display = format!("{}", option);
        assert!(display.contains("services/api/Dockerfile"));
        assert!(display.contains("→"));
    }

    #[test]
    fn test_dockerfile_option_display_root() {
        let df = create_test_dockerfile("/project/Dockerfile", ".");
        let project_root = PathBuf::from("/project");
        let option = DockerfileOption {
            dockerfile: &df,
            project_root: &project_root,
        };
        let display = format!("{}", option);
        assert!(display.contains("Dockerfile"));
        assert!(display.contains("(root)"));
    }

    #[test]
    fn test_build_context_option_display() {
        let dir_option = BuildContextOption::DockerfileDirectory("services/api".to_string());
        assert!(format!("{}", dir_option).contains("services/api"));

        let root_option = BuildContextOption::RepositoryRoot;
        assert!(format!("{}", root_option).contains("."));

        let custom_option = BuildContextOption::Custom;
        assert!(format!("{}", custom_option).contains("Custom"));
    }

    #[test]
    fn test_dockerfile_selection_result_variants() {
        let df = create_test_dockerfile("/project/Dockerfile", ".");

        // Test Selected variant
        let selected = DockerfileSelectionResult::Selected {
            dockerfile: df.clone(),
            build_context: ".".to_string(),
        };
        matches!(selected, DockerfileSelectionResult::Selected { .. });

        // Test StartAgent variant
        let agent = DockerfileSelectionResult::StartAgent("prompt".to_string());
        matches!(agent, DockerfileSelectionResult::StartAgent(_));

        // Test Back and Cancelled variants
        let _ = DockerfileSelectionResult::Back;
        let _ = DockerfileSelectionResult::Cancelled;
    }
}
