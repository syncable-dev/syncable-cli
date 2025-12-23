//! Rig PromptHook implementations for Claude Code style UI
//!
//! Shows tool calls in a collapsible format:
//! - `● tool_name(args...)` header with full command visible
//! - Preview of output (first few lines)
//! - `... +N lines` for long outputs
//! - `└ Running...` while executing
//! - Agent thinking shown between tool calls

use crate::agent::ui::colors::ansi;
use colored::Colorize;
use rig::agent::CancelSignal;
use rig::completion::{CompletionModel, CompletionResponse, Message};
use rig::message::{AssistantContent, Reasoning};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Maximum lines to show in preview before collapsing
const PREVIEW_LINES: usize = 4;

/// Tool call state with full output for expansion
#[derive(Debug, Clone)]
pub struct ToolCallState {
    pub name: String,
    pub args: String,
    pub output: Option<String>,
    pub output_lines: Vec<String>,
    pub is_running: bool,
    pub is_expanded: bool,
    pub is_collapsible: bool,
    pub status_ok: bool,
}

/// Shared state for the display
#[derive(Debug, Default)]
pub struct DisplayState {
    pub tool_calls: Vec<ToolCallState>,
    pub agent_messages: Vec<String>,
    pub current_tool_index: Option<usize>,
    pub last_expandable_index: Option<usize>,
}

/// A hook that shows Claude Code style tool execution
#[derive(Clone)]
pub struct ToolDisplayHook {
    state: Arc<Mutex<DisplayState>>,
}

impl ToolDisplayHook {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(DisplayState::default())),
        }
    }

    /// Get the shared state for external access
    pub fn state(&self) -> Arc<Mutex<DisplayState>> {
        self.state.clone()
    }
}

impl Default for ToolDisplayHook {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> rig::agent::PromptHook<M> for ToolDisplayHook
where
    M: CompletionModel,
{
    fn on_tool_call(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        args: &str,
        _cancel: CancelSignal,
    ) -> impl std::future::Future<Output = ()> + Send {
        let state = self.state.clone();
        let name = tool_name.to_string();
        let args_str = args.to_string();

        async move {
            // Print tool header
            print_tool_header(&name, &args_str);

            // Store in state
            let mut s = state.lock().await;
            let idx = s.tool_calls.len();
            s.tool_calls.push(ToolCallState {
                name,
                args: args_str,
                output: None,
                output_lines: Vec::new(),
                is_running: true,
                is_expanded: false,
                is_collapsible: false,
                status_ok: true,
            });
            s.current_tool_index = Some(idx);
        }
    }

    fn on_tool_result(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        args: &str,
        result: &str,
        _cancel: CancelSignal,
    ) -> impl std::future::Future<Output = ()> + Send {
        let state = self.state.clone();
        let name = tool_name.to_string();
        let args_str = args.to_string();
        let result_str = result.to_string();

        async move {
            // Print tool result and get the output info
            let (status_ok, output_lines, is_collapsible) = print_tool_result(&name, &args_str, &result_str);

            // Update state
            let mut s = state.lock().await;
            if let Some(idx) = s.current_tool_index {
                if let Some(tool) = s.tool_calls.get_mut(idx) {
                    tool.output = Some(result_str);
                    tool.output_lines = output_lines;
                    tool.is_running = false;
                    tool.is_collapsible = is_collapsible;
                    tool.status_ok = status_ok;
                }
                // Track last expandable output
                if is_collapsible {
                    s.last_expandable_index = Some(idx);
                }
            }
            s.current_tool_index = None;
        }
    }

    fn on_completion_response(
        &self,
        _prompt: &Message,
        response: &CompletionResponse<M::Response>,
        _cancel: CancelSignal,
    ) -> impl std::future::Future<Output = ()> + Send {
        let state = self.state.clone();

        // Check if response contains tool calls - if so, any text is "thinking"
        // If no tool calls, this is the final response - don't show as thinking
        let has_tool_calls = response.choice.iter().any(|content| {
            matches!(content, AssistantContent::ToolCall(_))
        });

        // Extract reasoning content (GPT-5.2 thinking summaries)
        let reasoning_parts: Vec<String> = response.choice.iter()
            .filter_map(|content| {
                if let AssistantContent::Reasoning(Reasoning { reasoning, .. }) = content {
                    // Join all reasoning strings
                    let text = reasoning.iter().cloned().collect::<Vec<_>>().join("\n");
                    if !text.trim().is_empty() {
                        Some(text)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Extract text content from the response (for non-reasoning models)
        let text_parts: Vec<String> = response.choice.iter()
            .filter_map(|content| {
                if let AssistantContent::Text(text) = content {
                    // Filter out empty or whitespace-only text
                    let trimmed = text.text.trim();
                    if !trimmed.is_empty() {
                        Some(trimmed.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        async move {
            // First, show reasoning content if available (GPT-5.2 thinking)
            if !reasoning_parts.is_empty() {
                let thinking_text = reasoning_parts.join("\n");

                // Store in state for history tracking
                let mut s = state.lock().await;
                s.agent_messages.push(thinking_text.clone());
                drop(s);

                // Display reasoning as thinking
                print_agent_thinking(&thinking_text);
            }

            // Also show text content if it's intermediate (has tool calls)
            // but NOT if it's the final response
            if !text_parts.is_empty() && has_tool_calls {
                let thinking_text = text_parts.join("\n");

                // Store in state for history tracking
                let mut s = state.lock().await;
                s.agent_messages.push(thinking_text.clone());
                drop(s);

                // Display as thinking
                print_agent_thinking(&thinking_text);
            }
        }
    }
}

/// Print agent thinking/reasoning text with nice formatting
fn print_agent_thinking(text: &str) {
    use crate::agent::ui::response::brand;

    println!();

    // Print thinking header in peach/coral
    println!(
        "{}{}  💭 Thinking...{}",
        brand::CORAL,
        brand::ITALIC,
        brand::RESET
    );

    // Format the content with markdown support
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim();

        // Handle code blocks
        if trimmed.starts_with("```") {
            if in_code_block {
                println!("{}  └────────────────────────────────────────────────────────┘{}", brand::LIGHT_PEACH, brand::RESET);
                in_code_block = false;
            } else {
                let lang = trimmed.strip_prefix("```").unwrap_or("");
                let lang_display = if lang.is_empty() { "code" } else { lang };
                println!(
                    "{}  ┌─ {}{}{} ────────────────────────────────────────────────────┐{}",
                    brand::LIGHT_PEACH, brand::CYAN, lang_display, brand::LIGHT_PEACH, brand::RESET
                );
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            println!("{}  │ {}{}{}  │", brand::LIGHT_PEACH, brand::CYAN, line, brand::RESET);
            continue;
        }

        // Handle bullet points
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let content = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")).unwrap_or(trimmed);
            println!("{}  {} {}{}", brand::PEACH, "•", format_thinking_inline(content), brand::RESET);
            continue;
        }

        // Handle numbered lists
        if trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
            && trimmed.chars().nth(1) == Some('.')
        {
            println!("{}  {}{}", brand::PEACH, format_thinking_inline(trimmed), brand::RESET);
            continue;
        }

        // Regular text with inline formatting
        if trimmed.is_empty() {
            println!();
        } else {
            // Word wrap long lines
            let wrapped = wrap_text(trimmed, 76);
            for wrapped_line in wrapped {
                println!("{}  {}{}", brand::PEACH, format_thinking_inline(&wrapped_line), brand::RESET);
            }
        }
    }

    println!();
    let _ = io::stdout().flush();
}

/// Format inline elements in thinking text (code, bold)
fn format_thinking_inline(text: &str) -> String {
    use crate::agent::ui::response::brand;

    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Handle `code`
        if chars[i] == '`' && (i + 1 >= chars.len() || chars[i + 1] != '`') {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '`') {
                let code_text: String = chars[i + 1..i + 1 + end].iter().collect();
                result.push_str(brand::CYAN);
                result.push('`');
                result.push_str(&code_text);
                result.push('`');
                result.push_str(brand::RESET);
                result.push_str(brand::PEACH);
                i = i + 2 + end;
                continue;
            }
        }

        // Handle **bold**
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end_offset) = find_double_star(&chars, i + 2) {
                let bold_text: String = chars[i + 2..i + 2 + end_offset].iter().collect();
                result.push_str(brand::RESET);
                result.push_str(brand::CORAL);
                result.push_str(brand::BOLD);
                result.push_str(&bold_text);
                result.push_str(brand::RESET);
                result.push_str(brand::PEACH);
                i = i + 4 + end_offset;
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Find closing ** marker
fn find_double_star(chars: &[char], start: usize) -> Option<usize> {
    for i in start..chars.len().saturating_sub(1) {
        if chars[i] == '*' && chars[i + 1] == '*' {
            return Some(i - start);
        }
    }
    None
}

/// Simple word wrap helper
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.len() <= max_width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(text.to_string());
    }

    lines
}

/// Print tool call header in Claude Code style
fn print_tool_header(name: &str, args: &str) {
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(args);
    let args_display = format_args_display(name, &parsed);

    // Print header with yellow dot (running)
    if args_display.is_empty() {
        println!("\n{} {}", "●".yellow(), name.cyan().bold());
    } else {
        println!("\n{} {}({})", "●".yellow(), name.cyan().bold(), args_display.dimmed());
    }

    // Print running indicator
    println!("  {} {}", "└".dimmed(), "Running...".dimmed());

    let _ = io::stdout().flush();
}

/// Print tool result with preview and collapse
/// Returns (status_ok, output_lines, is_collapsible)
fn print_tool_result(name: &str, args: &str, result: &str) -> (bool, Vec<String>, bool) {
    // Clear the "Running..." line
    print!("{}{}", ansi::CURSOR_UP, ansi::CLEAR_LINE);
    let _ = io::stdout().flush();

    // Parse the result - handle potential double-encoding from Rig
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(result)
        .map(|v: serde_json::Value| {
            // If the parsed value is a string, it might be double-encoded JSON
            // Try to parse the inner string, but fall back to original if it fails
            if let Some(inner_str) = v.as_str() {
                serde_json::from_str(inner_str).unwrap_or(v)
            } else {
                v
            }
        });

    // Format output based on tool type
    let (status_ok, output_lines) = match name {
        "shell" => format_shell_result(&parsed),
        "write_file" | "write_files" => format_write_result(&parsed),
        "read_file" => format_read_result(&parsed),
        "list_directory" => format_list_result(&parsed),
        "analyze_project" => format_analyze_result(&parsed),
        "security_scan" | "check_vulnerabilities" => format_security_result(&parsed),
        "hadolint" => format_hadolint_result(&parsed),
        _ => (true, vec!["done".to_string()]),
    };

    // Clear the header line to update dot color
    print!("{}{}", ansi::CURSOR_UP, ansi::CLEAR_LINE);

    // Reprint header with green/red dot and args
    let dot = if status_ok { "●".green() } else { "●".red() };

    // Format args for display (same logic as print_tool_header)
    let args_parsed: Result<serde_json::Value, _> = serde_json::from_str(args);
    let args_display = format_args_display(name, &args_parsed);

    if args_display.is_empty() {
        println!("{} {}", dot, name.cyan().bold());
    } else {
        println!("{} {}({})", dot, name.cyan().bold(), args_display.dimmed());
    }

    // Print output preview
    let total_lines = output_lines.len();
    let is_collapsible = total_lines > PREVIEW_LINES;

    for (i, line) in output_lines.iter().take(PREVIEW_LINES).enumerate() {
        let prefix = if i == output_lines.len().min(PREVIEW_LINES) - 1 && !is_collapsible {
            "└"
        } else {
            "│"
        };
        println!("  {} {}", prefix.dimmed(), line);
    }

    // Show collapse indicator if needed
    if is_collapsible {
        println!(
            "  {} {}",
            "└".dimmed(),
            format!("+{} more lines", total_lines - PREVIEW_LINES).dimmed()
        );
    }

    let _ = io::stdout().flush();
    (status_ok, output_lines, is_collapsible)
}

/// Format args for display based on tool type
fn format_args_display(name: &str, parsed: &Result<serde_json::Value, serde_json::Error>) -> String {
    match name {
        "shell" => {
            if let Ok(v) = parsed {
                v.get("command").and_then(|c| c.as_str()).unwrap_or("").to_string()
            } else {
                String::new()
            }
        }
        "write_file" => {
            if let Ok(v) = parsed {
                v.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string()
            } else {
                String::new()
            }
        }
        "write_files" => {
            if let Ok(v) = parsed {
                if let Some(files) = v.get("files").and_then(|f| f.as_array()) {
                    let paths: Vec<&str> = files
                        .iter()
                        .filter_map(|f| f.get("path").and_then(|p| p.as_str()))
                        .take(3)
                        .collect();
                    let more = if files.len() > 3 {
                        format!(", +{} more", files.len() - 3)
                    } else {
                        String::new()
                    };
                    format!("{}{}", paths.join(", "), more)
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        }
        "read_file" => {
            if let Ok(v) = parsed {
                v.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string()
            } else {
                String::new()
            }
        }
        "list_directory" => {
            if let Ok(v) = parsed {
                v.get("path").and_then(|p| p.as_str()).unwrap_or(".").to_string()
            } else {
                ".".to_string()
            }
        }
        _ => String::new(),
    }
}

/// Format shell command result
fn format_shell_result(parsed: &Result<serde_json::Value, serde_json::Error>) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let success = v.get("success").and_then(|s| s.as_bool()).unwrap_or(false);
        let stdout = v.get("stdout").and_then(|s| s.as_str()).unwrap_or("");
        let stderr = v.get("stderr").and_then(|s| s.as_str()).unwrap_or("");
        let exit_code = v.get("exit_code").and_then(|c| c.as_i64());

        let mut lines = Vec::new();

        // Add stdout lines
        for line in stdout.lines() {
            if !line.trim().is_empty() {
                lines.push(line.to_string());
            }
        }

        // Add stderr lines if failed
        if !success {
            for line in stderr.lines() {
                if !line.trim().is_empty() {
                    lines.push(format!("{}", line.red()));
                }
            }
            if let Some(code) = exit_code {
                lines.push(format!("exit code: {}", code).red().to_string());
            }
        }

        if lines.is_empty() {
            lines.push(if success { "completed".to_string() } else { "failed".to_string() });
        }

        (success, lines)
    } else {
        (false, vec!["parse error".to_string()])
    }
}

/// Format write file result
fn format_write_result(parsed: &Result<serde_json::Value, serde_json::Error>) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let success = v.get("success").and_then(|s| s.as_bool()).unwrap_or(false);
        let action = v.get("action").and_then(|a| a.as_str()).unwrap_or("wrote");
        let lines_written = v.get("lines_written")
            .or_else(|| v.get("total_lines"))
            .and_then(|n| n.as_u64())
            .unwrap_or(0);
        let files_written = v.get("files_written").and_then(|n| n.as_u64()).unwrap_or(1);

        let msg = if files_written > 1 {
            format!("{} {} files ({} lines)", action, files_written, lines_written)
        } else {
            format!("{} ({} lines)", action, lines_written)
        };

        (success, vec![msg])
    } else {
        (false, vec!["write failed".to_string()])
    }
}

/// Format read file result
fn format_read_result(parsed: &Result<serde_json::Value, serde_json::Error>) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        // Handle error field
        if v.get("error").is_some() {
            let error_msg = v.get("error").and_then(|e| e.as_str()).unwrap_or("file not found");
            return (false, vec![error_msg.to_string()]);
        }

        // Try to get total_lines from object
        if let Some(total_lines) = v.get("total_lines").and_then(|n| n.as_u64()) {
            let msg = if total_lines == 1 {
                "read 1 line".to_string()
            } else {
                format!("read {} lines", total_lines)
            };
            return (true, vec![msg]);
        }

        // Fallback: if we have a string value (failed inner parse) or missing fields,
        // try to extract line count from content or just say "read"
        if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
            let lines = content.lines().count();
            return (true, vec![format!("read {} lines", lines)]);
        }

        // Last resort: check if it's a string (double-encoding fallback)
        if v.is_string() {
            // The inner JSON couldn't be parsed, but we got something
            return (true, vec!["read file".to_string()]);
        }

        (true, vec!["read file".to_string()])
    } else {
        (false, vec!["read failed".to_string()])
    }
}

/// Format list directory result
fn format_list_result(parsed: &Result<serde_json::Value, serde_json::Error>) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let entries = v.get("entries").and_then(|e| e.as_array());

        let mut lines = Vec::new();

        if let Some(entries) = entries {
            let total = entries.len();
            for entry in entries.iter().take(PREVIEW_LINES + 2) {
                let name = entry.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                let entry_type = entry.get("type").and_then(|t| t.as_str()).unwrap_or("file");
                let prefix = if entry_type == "directory" { "📁" } else { "📄" };
                lines.push(format!("{} {}", prefix, name));
            }
            // Add count if there are more entries than shown
            if total > PREVIEW_LINES + 2 {
                lines.push(format!("... and {} more", total - (PREVIEW_LINES + 2)));
            }
        }

        if lines.is_empty() {
            lines.push("empty directory".to_string());
        }

        (true, lines)
    } else {
        (false, vec!["parse error".to_string()])
    }
}

/// Format analyze result
fn format_analyze_result(parsed: &Result<serde_json::Value, serde_json::Error>) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let mut lines = Vec::new();

        // Languages
        if let Some(langs) = v.get("languages").and_then(|l| l.as_array()) {
            let lang_names: Vec<&str> = langs
                .iter()
                .filter_map(|l| l.get("name").and_then(|n| n.as_str()))
                .take(5)
                .collect();
            if !lang_names.is_empty() {
                lines.push(format!("Languages: {}", lang_names.join(", ")));
            }
        }

        // Frameworks
        if let Some(frameworks) = v.get("frameworks").and_then(|f| f.as_array()) {
            let fw_names: Vec<&str> = frameworks
                .iter()
                .filter_map(|f| f.get("name").and_then(|n| n.as_str()))
                .take(5)
                .collect();
            if !fw_names.is_empty() {
                lines.push(format!("Frameworks: {}", fw_names.join(", ")));
            }
        }

        if lines.is_empty() {
            lines.push("analysis complete".to_string());
        }

        (true, lines)
    } else {
        (false, vec!["parse error".to_string()])
    }
}

/// Format security scan result
fn format_security_result(parsed: &Result<serde_json::Value, serde_json::Error>) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let findings = v.get("findings")
            .or_else(|| v.get("vulnerabilities"))
            .and_then(|f| f.as_array())
            .map(|a| a.len())
            .unwrap_or(0);

        if findings == 0 {
            (true, vec!["no issues found".to_string()])
        } else {
            (false, vec![format!("{} issues found", findings)])
        }
    } else {
        (false, vec!["parse error".to_string()])
    }
}

/// Format hadolint result - uses new priority-based format with Docker styling
fn format_hadolint_result(parsed: &Result<serde_json::Value, serde_json::Error>) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let success = v.get("success").and_then(|s| s.as_bool()).unwrap_or(true);
        let summary = v.get("summary");
        let action_plan = v.get("action_plan");

        let mut lines = Vec::new();

        // Get total count
        let total = summary
            .and_then(|s| s.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);

        // Show docker-themed header
        if total == 0 {
            lines.push(format!(
                "{}🐳 Dockerfile OK - no issues found{}",
                ansi::SUCCESS, ansi::RESET
            ));
            return (true, lines);
        }

        // Get priority counts
        let critical = summary
            .and_then(|s| s.get("by_priority"))
            .and_then(|p| p.get("critical"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0);
        let high = summary
            .and_then(|s| s.get("by_priority"))
            .and_then(|p| p.get("high"))
            .and_then(|h| h.as_u64())
            .unwrap_or(0);
        let medium = summary
            .and_then(|s| s.get("by_priority"))
            .and_then(|p| p.get("medium"))
            .and_then(|m| m.as_u64())
            .unwrap_or(0);
        let low = summary
            .and_then(|s| s.get("by_priority"))
            .and_then(|p| p.get("low"))
            .and_then(|l| l.as_u64())
            .unwrap_or(0);

        // Summary with priority breakdown
        let mut priority_parts = Vec::new();
        if critical > 0 {
            priority_parts.push(format!("{}🔴 {} critical{}", ansi::CRITICAL, critical, ansi::RESET));
        }
        if high > 0 {
            priority_parts.push(format!("{}🟠 {} high{}", ansi::HIGH, high, ansi::RESET));
        }
        if medium > 0 {
            priority_parts.push(format!("{}🟡 {} medium{}", ansi::MEDIUM, medium, ansi::RESET));
        }
        if low > 0 {
            priority_parts.push(format!("{}🟢 {} low{}", ansi::LOW, low, ansi::RESET));
        }

        let header_color = if critical > 0 {
            ansi::CRITICAL
        } else if high > 0 {
            ansi::HIGH
        } else {
            ansi::DOCKER_BLUE
        };

        lines.push(format!(
            "{}🐳 {} issue{} found: {}{}",
            header_color,
            total,
            if total == 1 { "" } else { "s" },
            priority_parts.join(" "),
            ansi::RESET
        ));

        // Show critical and high priority issues (these are most important)
        let mut shown = 0;
        const MAX_PREVIEW: usize = 6;

        // Critical issues first
        if let Some(critical_issues) = action_plan
            .and_then(|a| a.get("critical"))
            .and_then(|c| c.as_array())
        {
            for issue in critical_issues.iter().take(MAX_PREVIEW - shown) {
                lines.push(format_hadolint_issue(issue, "🔴", ansi::CRITICAL));
                shown += 1;
            }
        }

        // Then high priority
        if shown < MAX_PREVIEW {
            if let Some(high_issues) = action_plan
                .and_then(|a| a.get("high"))
                .and_then(|h| h.as_array())
            {
                for issue in high_issues.iter().take(MAX_PREVIEW - shown) {
                    lines.push(format_hadolint_issue(issue, "🟠", ansi::HIGH));
                    shown += 1;
                }
            }
        }

        // Show quick fix hint for most important issue
        if let Some(quick_fixes) = v.get("quick_fixes").and_then(|q| q.as_array()) {
            if let Some(first_fix) = quick_fixes.first().and_then(|f| f.as_str()) {
                let truncated = if first_fix.len() > 70 {
                    format!("{}...", &first_fix[..67])
                } else {
                    first_fix.to_string()
                };
                lines.push(format!(
                    "{}  → Fix: {}{}",
                    ansi::INFO_BLUE, truncated, ansi::RESET
                ));
            }
        }

        // Note about remaining issues
        let remaining = total as usize - shown;
        if remaining > 0 {
            lines.push(format!(
                "{}  +{} more issue{}{}",
                ansi::GRAY,
                remaining,
                if remaining == 1 { "" } else { "s" },
                ansi::RESET
            ));
        }

        (success, lines)
    } else {
        (false, vec!["parse error".to_string()])
    }
}

/// Format a single hadolint issue for display
fn format_hadolint_issue(issue: &serde_json::Value, icon: &str, color: &str) -> String {
    let code = issue.get("code").and_then(|c| c.as_str()).unwrap_or("?");
    let message = issue.get("message").and_then(|m| m.as_str()).unwrap_or("?");
    let line_num = issue.get("line").and_then(|l| l.as_u64()).unwrap_or(0);
    let category = issue.get("category").and_then(|c| c.as_str()).unwrap_or("");

    // Category badge
    let badge = match category {
        "security" => format!("{}[SEC]{}", ansi::CRITICAL, ansi::RESET),
        "best-practice" => format!("{}[BP]{}", ansi::INFO_BLUE, ansi::RESET),
        "deprecated" => format!("{}[DEP]{}", ansi::MEDIUM, ansi::RESET),
        "performance" => format!("{}[PERF]{}", ansi::CYAN, ansi::RESET),
        _ => String::new(),
    };

    // Truncate message
    let msg_display = if message.len() > 50 {
        format!("{}...", &message[..47])
    } else {
        message.to_string()
    };

    format!(
        "{}{} L{}:{} {}{}[{}]{} {} {}",
        color, icon, line_num, ansi::RESET,
        ansi::DOCKER_BLUE, ansi::BOLD, code, ansi::RESET,
        badge,
        msg_display
    )
}

// Legacy exports for compatibility
pub use crate::agent::ui::Spinner;
use tokio::sync::mpsc;

/// Events for backward compatibility
#[derive(Debug, Clone)]
pub enum ToolEvent {
    ToolStart { name: String, args: String },
    ToolComplete { name: String, result: String },
}

/// Legacy spawn function - now a no-op since display is handled in hooks
pub fn spawn_tool_display_handler(
    _receiver: mpsc::Receiver<ToolEvent>,
    _spinner: Arc<crate::agent::ui::Spinner>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {})
}
