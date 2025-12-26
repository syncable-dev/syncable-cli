//! Summary frame generation for compacted context
//!
//! Creates structured summaries that preserve important information
//! while being optimized for model consumption.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A tool call summary for inclusion in context summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallSummary {
    pub tool_name: String,
    pub args_summary: String,
    pub result_summary: String,
    pub success: bool,
}

/// Summary of a conversation turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSummary {
    pub turn_number: usize,
    pub user_intent: String,
    pub assistant_action: String,
    pub tool_calls: Vec<ToolCallSummary>,
    pub key_decisions: Vec<String>,
}

/// Aggregated context from compacted messages
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextSummary {
    /// Number of turns that were compacted
    pub turns_compacted: usize,

    /// Summaries of individual turns
    pub turn_summaries: Vec<TurnSummary>,

    /// Files that were read during compacted turns
    pub files_read: HashSet<String>,

    /// Files that were written during compacted turns
    pub files_written: HashSet<String>,

    /// Directories that were listed
    pub directories_listed: HashSet<String>,

    /// Key decisions or constraints established
    pub key_decisions: Vec<String>,

    /// Errors or issues encountered
    pub errors_encountered: Vec<String>,

    /// Tools used with their counts
    pub tool_usage: HashMap<String, usize>,
}

impl ContextSummary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a turn summary
    pub fn add_turn(&mut self, turn: TurnSummary) {
        // Extract file operations
        for tc in &turn.tool_calls {
            *self.tool_usage.entry(tc.tool_name.clone()).or_insert(0) += 1;

            match tc.tool_name.as_str() {
                "read_file" => {
                    self.files_read.insert(tc.args_summary.clone());
                }
                "write_file" | "write_files" => {
                    self.files_written.insert(tc.args_summary.clone());
                }
                "list_directory" => {
                    self.directories_listed.insert(tc.args_summary.clone());
                }
                _ => {}
            }

            if !tc.success && !tc.result_summary.is_empty() {
                self.errors_encountered.push(format!(
                    "{}: {}",
                    tc.tool_name,
                    truncate(&tc.result_summary, 100)
                ));
            }
        }

        // Add key decisions
        self.key_decisions.extend(turn.key_decisions.clone());

        self.turn_summaries.push(turn);
        self.turns_compacted += 1;
    }

    /// Merge another summary into this one
    pub fn merge(&mut self, other: ContextSummary) {
        self.turns_compacted += other.turns_compacted;
        self.turn_summaries.extend(other.turn_summaries);
        self.files_read.extend(other.files_read);
        self.files_written.extend(other.files_written);
        self.directories_listed.extend(other.directories_listed);
        self.key_decisions.extend(other.key_decisions);
        self.errors_encountered.extend(other.errors_encountered);

        for (tool, count) in other.tool_usage {
            *self.tool_usage.entry(tool).or_insert(0) += count;
        }
    }
}

/// A summary frame ready to be inserted into context
#[derive(Debug, Clone)]
pub struct SummaryFrame {
    /// The rendered summary text
    pub content: String,
    /// Estimated token count
    pub token_count: usize,
}

impl SummaryFrame {
    /// Generate a summary frame from a ContextSummary
    ///
    /// The format is designed for model consumption:
    /// - Structured XML-like sections
    /// - Hierarchical information
    /// - Key context preserved
    pub fn from_summary(summary: &ContextSummary) -> Self {
        let mut content = String::new();

        // Header
        content.push_str(&format!(
            "<conversation_summary turns=\"{}\">\n",
            summary.turns_compacted
        ));

        // High-level overview
        content.push_str("<overview>\n");
        content.push_str(&format!(
            "This summary covers {} conversation turn{}.\n",
            summary.turns_compacted,
            if summary.turns_compacted == 1 {
                ""
            } else {
                "s"
            }
        ));

        // Tool usage summary
        if !summary.tool_usage.is_empty() {
            content.push_str("Tools used: ");
            let tools: Vec<String> = summary
                .tool_usage
                .iter()
                .map(|(name, count)| format!("{}({}x)", name, count))
                .collect();
            content.push_str(&tools.join(", "));
            content.push('\n');
        }
        content.push_str("</overview>\n\n");

        // Turn summaries (condensed)
        content.push_str("<turns>\n");
        for turn in &summary.turn_summaries {
            content.push_str(&format!(
                "Turn {}: {} → {}\n",
                turn.turn_number,
                truncate(&turn.user_intent, 80),
                truncate(&turn.assistant_action, 100)
            ));

            // Include important tool calls
            let important_tools: Vec<_> = turn
                .tool_calls
                .iter()
                .filter(|tc| {
                    matches!(
                        tc.tool_name.as_str(),
                        "write_file" | "write_files" | "shell" | "analyze_project"
                    ) || !tc.success
                })
                .collect();

            for tc in important_tools.iter().take(3) {
                let status = if tc.success { "✓" } else { "✗" };
                content.push_str(&format!(
                    "  {} {}({})\n",
                    status,
                    tc.tool_name,
                    truncate(&tc.args_summary, 40)
                ));
            }

            if important_tools.len() > 3 {
                content.push_str(&format!(
                    "  ... +{} more tool calls\n",
                    important_tools.len() - 3
                ));
            }
        }
        content.push_str("</turns>\n\n");

        // Files context
        if !summary.files_read.is_empty() || !summary.files_written.is_empty() {
            content.push_str("<files_context>\n");

            if !summary.files_written.is_empty() {
                content.push_str("Files created/modified:\n");
                for file in summary.files_written.iter().take(20) {
                    content.push_str(&format!("  - {}\n", file));
                }
                if summary.files_written.len() > 20 {
                    content.push_str(&format!(
                        "  ... +{} more files\n",
                        summary.files_written.len() - 20
                    ));
                }
            }

            if !summary.files_read.is_empty() {
                content.push_str("Files read (content was available):\n");
                for file in summary.files_read.iter().take(15) {
                    content.push_str(&format!("  - {}\n", file));
                }
                if summary.files_read.len() > 15 {
                    content.push_str(&format!(
                        "  ... +{} more files\n",
                        summary.files_read.len() - 15
                    ));
                }
            }

            content.push_str("</files_context>\n\n");
        }

        // Key decisions
        if !summary.key_decisions.is_empty() {
            content.push_str("<key_decisions>\n");
            for decision in summary.key_decisions.iter().take(10) {
                content.push_str(&format!("- {}\n", decision));
            }
            content.push_str("</key_decisions>\n\n");
        }

        // Errors (important to preserve)
        if !summary.errors_encountered.is_empty() {
            content.push_str("<errors_encountered>\n");
            for error in summary.errors_encountered.iter().take(5) {
                content.push_str(&format!("- {}\n", error));
            }
            content.push_str("</errors_encountered>\n\n");
        }

        content.push_str("</conversation_summary>");

        // Estimate tokens
        let token_count = content.len() / 4;

        Self {
            content,
            token_count,
        }
    }

    /// Create a minimal summary frame (for very aggressive compaction)
    pub fn minimal(turns: usize, files_written: &[String]) -> Self {
        let mut content = format!(
            "<conversation_summary turns=\"{}\" minimal=\"true\">\n",
            turns
        );

        if !files_written.is_empty() {
            content.push_str("Files created: ");
            content.push_str(&files_written.join(", "));
            content.push('\n');
        }

        content.push_str("</conversation_summary>");

        let token_count = content.len() / 4;
        Self {
            content,
            token_count,
        }
    }
}

/// Helper to truncate text with ellipsis
fn truncate(text: &str, max_len: usize) -> String {
    let text = text.trim();
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}

/// Extract a brief intent from user message
pub fn extract_user_intent(message: &str, max_len: usize) -> String {
    let message = message.trim();

    // Remove common prefixes
    let cleaned = message
        .strip_prefix("please ")
        .or_else(|| message.strip_prefix("can you "))
        .or_else(|| message.strip_prefix("could you "))
        .unwrap_or(message);

    truncate(cleaned, max_len)
}

/// Extract action summary from assistant response
pub fn extract_assistant_action(response: &str, max_len: usize) -> String {
    let response = response.trim();

    // Take first sentence or line
    let first_part = response
        .split(|c| c == '.' || c == '\n')
        .next()
        .unwrap_or(response);

    truncate(first_part, max_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_summary() {
        let mut summary = ContextSummary::new();

        summary.add_turn(TurnSummary {
            turn_number: 1,
            user_intent: "Analyze the project".to_string(),
            assistant_action: "I analyzed the project structure".to_string(),
            tool_calls: vec![
                ToolCallSummary {
                    tool_name: "analyze_project".to_string(),
                    args_summary: ".".to_string(),
                    result_summary: "Found Rust project".to_string(),
                    success: true,
                },
                ToolCallSummary {
                    tool_name: "read_file".to_string(),
                    args_summary: "Cargo.toml".to_string(),
                    result_summary: "Read 50 lines".to_string(),
                    success: true,
                },
            ],
            key_decisions: vec!["This is a Rust CLI project".to_string()],
        });

        assert_eq!(summary.turns_compacted, 1);
        assert!(summary.files_read.contains("Cargo.toml"));
        assert_eq!(summary.tool_usage.get("read_file"), Some(&1));
    }

    #[test]
    fn test_summary_frame_generation() {
        let mut summary = ContextSummary::new();
        summary.files_written.insert("Dockerfile".to_string());
        summary.turns_compacted = 3;

        let frame = SummaryFrame::from_summary(&summary);

        assert!(frame.content.contains("conversation_summary"));
        assert!(frame.content.contains("Dockerfile"));
        assert!(frame.token_count > 0);
    }

    #[test]
    fn test_extract_user_intent() {
        assert_eq!(
            extract_user_intent("please analyze the codebase", 50),
            "analyze the codebase"
        );
        assert_eq!(
            extract_user_intent("can you create a Dockerfile", 50),
            "create a Dockerfile"
        );
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a longer text", 10), "this is...");
    }
}
