//! Autocomplete support for slash commands and file references using inquire
//!
//! Provides a custom Autocomplete implementation that shows:
//! - Slash command suggestions when user types "/"
//! - File path suggestions when user types "@"

use crate::agent::commands::SLASH_COMMANDS;
use inquire::autocompletion::{Autocomplete, Replacement};
use std::path::PathBuf;

/// Autocomplete provider for slash commands and file references
/// Shows suggestions when user types "/" or "@" followed by characters
#[derive(Clone)]
pub struct SlashCommandAutocomplete {
    /// Cache of filtered commands for current input
    filtered_commands: Vec<&'static str>,
    /// Project root for file searches
    project_path: PathBuf,
    /// Cache of file paths found
    cached_files: Vec<String>,
    /// Current autocomplete mode
    mode: AutocompleteMode,
}

#[derive(Clone, Debug, PartialEq)]
enum AutocompleteMode {
    None,
    Command,
    File,
}

impl Default for SlashCommandAutocomplete {
    fn default() -> Self {
        Self::new()
    }
}

impl SlashCommandAutocomplete {
    pub fn new() -> Self {
        Self {
            filtered_commands: Vec::new(),
            project_path: std::env::current_dir().unwrap_or_default(),
            cached_files: Vec::new(),
            mode: AutocompleteMode::None,
        }
    }

    /// Set the project path for file searches
    pub fn with_project_path(mut self, path: PathBuf) -> Self {
        self.project_path = path;
        self
    }

    /// Find the @ trigger position in the input
    fn find_at_trigger(&self, input: &str) -> Option<usize> {
        // Find the last @ that starts a file reference
        // It should be either at the start or after a space
        for (i, c) in input.char_indices().rev() {
            if c == '@' {
                // Check if it's at the start or after a space
                if i == 0
                    || input
                        .chars()
                        .nth(i - 1)
                        .map(|c| c.is_whitespace())
                        .unwrap_or(false)
                {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Extract the file filter from input after @
    fn extract_file_filter(&self, input: &str) -> Option<String> {
        if let Some(at_pos) = self.find_at_trigger(input) {
            let after_at = &input[at_pos + 1..];
            // Get everything until next space or end
            let filter: String = after_at
                .chars()
                .take_while(|c| !c.is_whitespace())
                .collect();
            return Some(filter);
        }
        None
    }

    /// Search for files matching a pattern
    fn search_files(&mut self, filter: &str) -> Vec<String> {
        let mut results = Vec::new();
        let filter_lower = filter.to_lowercase();

        // Walk directory tree (limited depth)
        self.walk_dir(
            &self.project_path.clone(),
            &filter_lower,
            &mut results,
            0,
            4,
        );

        // Sort by relevance (exact matches first, then by length)
        results.sort_by(|a, b| {
            let a_exact = a.to_lowercase().contains(&filter_lower);
            let b_exact = b.to_lowercase().contains(&filter_lower);
            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.len().cmp(&b.len()),
            }
        });

        results.truncate(8);
        results
    }

    /// Recursively walk directory for matching files
    fn walk_dir(
        &self,
        dir: &PathBuf,
        filter: &str,
        results: &mut Vec<String>,
        depth: usize,
        max_depth: usize,
    ) {
        if depth > max_depth || results.len() >= 20 {
            return;
        }

        // Skip common non-relevant directories
        let skip_dirs = [
            "node_modules",
            ".git",
            "target",
            "__pycache__",
            ".venv",
            "venv",
            "dist",
            "build",
            ".next",
        ];

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files/dirs (except .env, .gitignore, etc.)
            if file_name.starts_with('.')
                && !file_name.starts_with(".env")
                && !file_name.starts_with(".git")
            {
                continue;
            }

            if path.is_dir() {
                if !skip_dirs.contains(&file_name.as_str()) {
                    self.walk_dir(&path, filter, results, depth + 1, max_depth);
                }
            } else {
                // Get relative path from project root
                let rel_path = path
                    .strip_prefix(&self.project_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| file_name.clone());

                // Match against filter
                if filter.is_empty()
                    || rel_path.to_lowercase().contains(filter)
                    || file_name.to_lowercase().contains(filter)
                {
                    results.push(rel_path);
                }
            }
        }
    }
}

impl Autocomplete for SlashCommandAutocomplete {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, inquire::CustomUserError> {
        // Check for @ file reference trigger
        if let Some(filter) = self.extract_file_filter(input) {
            self.mode = AutocompleteMode::File;
            self.cached_files = self.search_files(&filter);

            let suggestions: Vec<String> = self
                .cached_files
                .iter()
                .map(|f| format!("@{}", f))
                .collect();

            return Ok(suggestions);
        }

        // Check for / command trigger (only at start of input)
        if input.starts_with('/') {
            self.mode = AutocompleteMode::Command;
            let filter = input.trim_start_matches('/').to_lowercase();

            // Store the command names for use in get_completion
            self.filtered_commands = SLASH_COMMANDS
                .iter()
                .filter(|cmd| {
                    cmd.name.to_lowercase().starts_with(&filter)
                        || cmd
                            .alias
                            .map(|a| a.to_lowercase().starts_with(&filter))
                            .unwrap_or(false)
                })
                .take(6)
                .map(|cmd| cmd.name)
                .collect();

            // Return formatted suggestions for display
            let suggestions: Vec<String> = SLASH_COMMANDS
                .iter()
                .filter(|cmd| {
                    cmd.name.to_lowercase().starts_with(&filter)
                        || cmd
                            .alias
                            .map(|a| a.to_lowercase().starts_with(&filter))
                            .unwrap_or(false)
                })
                .take(6)
                .map(|cmd| format!("/{:<12} {}", cmd.name, cmd.description))
                .collect();

            return Ok(suggestions);
        }

        // No trigger found
        self.mode = AutocompleteMode::None;
        self.filtered_commands.clear();
        self.cached_files.clear();
        Ok(vec![])
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<Replacement, inquire::CustomUserError> {
        if let Some(suggestion) = highlighted_suggestion {
            match self.mode {
                AutocompleteMode::File => {
                    // For file suggestions, replace the @filter part with the selected file
                    if let Some(at_pos) = self.find_at_trigger(input) {
                        let before_at = &input[..at_pos];
                        // The suggestion is "@path/to/file", we want to insert it
                        let new_input = format!("{}{} ", before_at, suggestion);
                        return Ok(Replacement::Some(new_input));
                    }
                }
                AutocompleteMode::Command => {
                    // Extract just the command name - first word after the /
                    // Format is: "/model        Select a different AI model"
                    if let Some(cmd_with_slash) = suggestion.split_whitespace().next() {
                        return Ok(Replacement::Some(cmd_with_slash.to_string()));
                    }
                }
                AutocompleteMode::None => {}
            }
        }
        Ok(Replacement::None)
    }
}
