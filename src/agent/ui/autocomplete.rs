//! Autocomplete support for slash commands using inquire
//!
//! Provides a custom Autocomplete implementation that shows
//! slash command suggestions as the user types.

use inquire::autocompletion::{Autocomplete, Replacement};
use crate::agent::commands::SLASH_COMMANDS;

/// Autocomplete provider for slash commands
/// Shows suggestions when user types "/" followed by characters
#[derive(Clone, Default)]
pub struct SlashCommandAutocomplete {
    /// Cache of filtered commands for current input
    filtered_commands: Vec<&'static str>,
}

impl SlashCommandAutocomplete {
    pub fn new() -> Self {
        Self {
            filtered_commands: Vec::new(),
        }
    }
}

impl Autocomplete for SlashCommandAutocomplete {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, inquire::CustomUserError> {
        // Only show suggestions when input starts with /
        if !input.starts_with('/') {
            self.filtered_commands.clear();
            return Ok(vec![]);
        }

        let filter = input.trim_start_matches('/').to_lowercase();

        // Store the command names for use in get_completion
        self.filtered_commands = SLASH_COMMANDS.iter()
            .filter(|cmd| {
                cmd.name.to_lowercase().starts_with(&filter) ||
                cmd.alias.map(|a| a.to_lowercase().starts_with(&filter)).unwrap_or(false)
            })
            .take(6)
            .map(|cmd| cmd.name)
            .collect();

        // Return formatted suggestions for display
        let suggestions: Vec<String> = SLASH_COMMANDS.iter()
            .filter(|cmd| {
                cmd.name.to_lowercase().starts_with(&filter) ||
                cmd.alias.map(|a| a.to_lowercase().starts_with(&filter)).unwrap_or(false)
            })
            .take(6)
            .map(|cmd| format!("/{:<12} {}", cmd.name, cmd.description))
            .collect();

        Ok(suggestions)
    }

    fn get_completion(
        &mut self,
        _input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<Replacement, inquire::CustomUserError> {
        if let Some(suggestion) = highlighted_suggestion {
            // Extract just the command name - first word after the /
            // Format is: "/model        Select a different AI model"
            if let Some(cmd_with_slash) = suggestion.split_whitespace().next() {
                return Ok(Replacement::Some(cmd_with_slash.to_string()));
            }
        }
        Ok(Replacement::None)
    }
}
