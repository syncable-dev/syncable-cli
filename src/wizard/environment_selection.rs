//! Environment selection step for the deployment wizard
//!
//! Prompts user to select an environment or create a new one.

use crate::platform::api::types::Environment;
use crate::platform::api::PlatformApiClient;
use crate::wizard::render::{display_step_header, wizard_render_config};
use colored::Colorize;
use inquire::{InquireError, Select};
use std::fmt;

/// Result of environment selection step
#[derive(Debug, Clone)]
pub enum EnvironmentSelectionResult {
    /// User selected an environment
    Selected(Environment),
    /// User wants to create a new environment
    CreateNew,
    /// User cancelled the wizard
    Cancelled,
    /// An error occurred
    Error(String),
}

/// Wrapper for displaying environment options in the selection menu
struct EnvironmentOption {
    environment: Environment,
}

impl fmt::Display for EnvironmentOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}  {}",
            self.environment.name.cyan(),
            self.environment.environment_type.to_string().dimmed()
        )
    }
}

/// Option to create a new environment
struct CreateNewOption;

impl fmt::Display for CreateNewOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", "+ Create new environment".bright_green())
    }
}

/// Selection menu item that can be either an environment or create new
enum SelectionItem {
    Environment(EnvironmentOption),
    CreateNew(CreateNewOption),
}

impl fmt::Display for SelectionItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectionItem::Environment(env) => env.fmt(f),
            SelectionItem::CreateNew(create) => create.fmt(f),
        }
    }
}

/// Prompt user to select an environment for deployment
pub async fn select_environment(
    client: &PlatformApiClient,
    project_id: &str,
) -> EnvironmentSelectionResult {
    display_step_header(
        0,
        "Select Environment",
        "Choose the environment to deploy to.",
    );

    // Fetch environments
    let environments = match client.list_environments(project_id).await {
        Ok(envs) => envs,
        Err(e) => {
            return EnvironmentSelectionResult::Error(format!(
                "Failed to fetch environments: {}",
                e
            ));
        }
    };

    if environments.is_empty() {
        println!(
            "\n{} No environments found. Let's create one first.",
            "ℹ".cyan()
        );
        return EnvironmentSelectionResult::CreateNew;
    }

    // Build selection options
    let mut options: Vec<SelectionItem> = environments
        .into_iter()
        .map(|env| SelectionItem::Environment(EnvironmentOption { environment: env }))
        .collect();

    // Add create new option at the end
    options.push(SelectionItem::CreateNew(CreateNewOption));

    let selection = Select::new("Select environment:", options)
        .with_render_config(wizard_render_config())
        .with_help_message("Use ↑/↓ to navigate, Enter to select")
        .prompt();

    match selection {
        Ok(SelectionItem::Environment(env_opt)) => {
            println!(
                "\n{} Selected environment: {}",
                "✓".green(),
                env_opt.environment.name.cyan()
            );
            EnvironmentSelectionResult::Selected(env_opt.environment)
        }
        Ok(SelectionItem::CreateNew(_)) => EnvironmentSelectionResult::CreateNew,
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            EnvironmentSelectionResult::Cancelled
        }
        Err(_) => EnvironmentSelectionResult::Cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_selection_result_variants() {
        let env = Environment {
            id: "test-id".to_string(),
            name: "prod".to_string(),
            project_id: "proj-1".to_string(),
            environment_type: "cloud".to_string(),
            cluster_id: None,
            namespace: None,
            description: None,
            is_active: true,
            created_at: None,
            updated_at: None,
            provider_regions: None,
        };
        let _ = EnvironmentSelectionResult::Selected(env);
        let _ = EnvironmentSelectionResult::CreateNew;
        let _ = EnvironmentSelectionResult::Cancelled;
        let _ = EnvironmentSelectionResult::Error("test".to_string());
    }
}
