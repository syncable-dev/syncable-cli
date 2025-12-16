//! Rig PromptHook implementations for UI updates
//!
//! Provides hooks that update the UI when tools are called during agent execution.

use crate::agent::ui::Spinner;
use rig::agent::CancelSignal;
use rig::completion::CompletionModel;
use std::sync::Arc;
use tokio::sync::mpsc;

/// A hook that updates the spinner when tools are executed
#[derive(Clone)]
pub struct ToolDisplayHook {
    sender: mpsc::Sender<ToolEvent>,
}

/// Events sent from the hook to the UI
#[derive(Debug, Clone)]
pub enum ToolEvent {
    ToolStart { name: String, args: String },
    ToolComplete { name: String, result: String },
}

impl ToolDisplayHook {
    /// Create a new hook with a channel to send tool events
    pub fn new() -> (Self, mpsc::Receiver<ToolEvent>) {
        let (sender, receiver) = mpsc::channel(32);
        (Self { sender }, receiver)
    }

    /// Create a hook from an existing sender
    pub fn from_sender(sender: mpsc::Sender<ToolEvent>) -> Self {
        Self { sender }
    }
}

impl Default for ToolDisplayHook {
    fn default() -> Self {
        let (hook, _) = Self::new();
        hook
    }
}

impl<M> rig::agent::PromptHook<M> for ToolDisplayHook
where
    M: CompletionModel,
{
    fn on_tool_call(
        &self,
        tool_name: &str,
        args: &str,
        _cancel: CancelSignal,
    ) -> impl std::future::Future<Output = ()> + Send {
        let sender = self.sender.clone();
        let name = tool_name.to_string();
        let args_str = args.to_string();

        async move {
            let _ = sender
                .send(ToolEvent::ToolStart {
                    name,
                    args: args_str,
                })
                .await;
        }
    }

    fn on_tool_result(
        &self,
        tool_name: &str,
        _args: &str,
        result: &str,
        _cancel: CancelSignal,
    ) -> impl std::future::Future<Output = ()> + Send {
        let sender = self.sender.clone();
        let name = tool_name.to_string();
        let result_str = result.to_string();

        async move {
            let _ = sender
                .send(ToolEvent::ToolComplete {
                    name,
                    result: result_str,
                })
                .await;
        }
    }
}

/// Spawns a task that listens for tool events and updates the spinner
pub fn spawn_tool_display_handler(
    mut receiver: mpsc::Receiver<ToolEvent>,
    spinner: Arc<Spinner>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(event) = receiver.recv().await {
            match event {
                ToolEvent::ToolStart { name, args } => {
                    // Format a nice description from the tool name
                    let description = format_tool_description(&name, &args);
                    spinner.tool_executing(&name, &description).await;
                }
                ToolEvent::ToolComplete { name, .. } => {
                    spinner.tool_complete(&name).await;
                }
            }
        }
    })
}

/// Format a user-friendly description for a tool based on its name and args
fn format_tool_description(name: &str, args: &str) -> String {
    match name {
        "analyze_project" => "Analyzing project structure...".to_string(),
        "security_scan" => "Running security scan...".to_string(),
        "check_vulnerabilities" => "Checking for vulnerabilities...".to_string(),
        "read_file" => {
            // Try to extract the file path from args
            if let Ok(args_value) = serde_json::from_str::<serde_json::Value>(args) {
                if let Some(path) = args_value.get("path").and_then(|p| p.as_str()) {
                    return format!("Reading {}", truncate_path(path));
                }
            }
            "Reading file...".to_string()
        }
        "list_directory" => {
            if let Ok(args_value) = serde_json::from_str::<serde_json::Value>(args) {
                if let Some(path) = args_value.get("path").and_then(|p| p.as_str()) {
                    return format!("Listing {}", truncate_path(path));
                }
            }
            "Listing directory...".to_string()
        }
        "search_code" => {
            if let Ok(args_value) = serde_json::from_str::<serde_json::Value>(args) {
                if let Some(pattern) = args_value.get("pattern").and_then(|p| p.as_str()) {
                    return format!("Searching for '{}'...", truncate_text(pattern, 30));
                }
            }
            "Searching code...".to_string()
        }
        "find_files" => "Finding files...".to_string(),
        "generate_iac" => "Generating infrastructure config...".to_string(),
        "discover_services" => "Discovering services...".to_string(),
        _ => format!("Executing {}...", name),
    }
}

/// Truncate a path for display
fn truncate_path(path: &str) -> String {
    if path.len() <= 40 {
        path.to_string()
    } else {
        // Show last 40 chars with ...
        format!("...{}", &path[path.len() - 37..])
    }
}

/// Truncate text for display
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_path() {
        assert_eq!(truncate_path("short.txt"), "short.txt");
        let long_path = "/very/long/path/that/exceeds/forty/characters/file.rs";
        assert!(truncate_path(long_path).len() <= 40);
        assert!(truncate_path(long_path).starts_with("..."));
    }
}
