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
use rig::completion::{CompletionModel, CompletionResponse, Message, Usage};
use rig::message::{AssistantContent, Reasoning};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Maximum lines to show in preview before collapsing
const PREVIEW_LINES: usize = 4;

/// Safely truncate a string to a maximum character count, handling UTF-8 properly.
/// Adds "..." suffix when truncation occurs.
fn truncate_safe(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncate_to = max_chars.saturating_sub(3);
        let truncated: String = s.chars().take(truncate_to).collect();
        format!("{}...", truncated)
    }
}

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
    /// AG-UI tool call ID for event correlation
    pub ag_ui_tool_call_id: Option<ag_ui_core::ToolCallId>,
}

/// Accumulated usage from API responses
#[derive(Debug, Default, Clone)]
pub struct AccumulatedUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

impl AccumulatedUsage {
    /// Add usage from a completion response
    pub fn add(&mut self, usage: &Usage) {
        self.input_tokens += usage.input_tokens;
        self.output_tokens += usage.output_tokens;
        self.total_tokens += usage.total_tokens;
    }

    /// Check if we have any actual usage data
    pub fn has_data(&self) -> bool {
        self.input_tokens > 0 || self.output_tokens > 0 || self.total_tokens > 0
    }
}

/// Shared state for the display
pub struct DisplayState {
    pub tool_calls: Vec<ToolCallState>,
    pub agent_messages: Vec<String>,
    pub current_tool_index: Option<usize>,
    pub last_expandable_index: Option<usize>,
    /// Accumulated token usage from API responses
    pub usage: AccumulatedUsage,
    /// Optional progress indicator state for real-time token display
    pub progress_state: Option<std::sync::Arc<crate::agent::ui::progress::ProgressState>>,
    /// Cancel signal from rig - stored for external cancellation trigger
    pub cancel_signal: Option<CancelSignal>,
    /// Optional AG-UI EventBridge for streaming tool events to frontends
    pub event_bridge: Option<crate::server::EventBridge>,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            tool_calls: Vec::new(),
            agent_messages: Vec::new(),
            current_tool_index: None,
            last_expandable_index: None,
            usage: AccumulatedUsage::default(),
            progress_state: None,
            cancel_signal: None,
            event_bridge: None,
        }
    }
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

    /// Get accumulated usage (blocks on lock)
    pub async fn get_usage(&self) -> AccumulatedUsage {
        let state = self.state.lock().await;
        state.usage.clone()
    }

    /// Reset usage counter (e.g., at start of a new request batch)
    pub async fn reset_usage(&self) {
        let mut state = self.state.lock().await;
        state.usage = AccumulatedUsage::default();
    }

    /// Set the progress indicator state for real-time token display
    pub async fn set_progress_state(
        &self,
        progress: std::sync::Arc<crate::agent::ui::progress::ProgressState>,
    ) {
        let mut state = self.state.lock().await;
        state.progress_state = Some(progress);
    }

    /// Clear the progress state
    pub async fn clear_progress_state(&self) {
        let mut state = self.state.lock().await;
        state.progress_state = None;
    }

    /// Trigger cancellation of the current request.
    /// This will cause rig to stop after the current tool/response completes.
    pub async fn cancel(&self) {
        let state = self.state.lock().await;
        if let Some(ref cancel_sig) = state.cancel_signal {
            cancel_sig.cancel();
        }
    }

    /// Check if cancellation is possible (a cancel signal is stored)
    pub async fn can_cancel(&self) -> bool {
        let state = self.state.lock().await;
        state.cancel_signal.is_some()
    }

    /// Set the AG-UI EventBridge for streaming tool events to frontends
    pub async fn set_event_bridge(&self, bridge: crate::server::EventBridge) {
        let mut state = self.state.lock().await;
        state.event_bridge = Some(bridge);
    }

    /// Clear the AG-UI EventBridge
    pub async fn clear_event_bridge(&self) {
        let mut state = self.state.lock().await;
        state.event_bridge = None;
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
        cancel: CancelSignal,
    ) -> impl std::future::Future<Output = ()> + Send {
        let state = self.state.clone();
        let name = tool_name.to_string();
        let args_str = args.to_string();

        async move {
            // Store the cancel signal for external cancellation
            {
                let mut s = state.lock().await;
                s.cancel_signal = Some(cancel);
            }
            // Pause progress indicator before printing
            {
                let s = state.lock().await;
                if let Some(ref progress) = s.progress_state {
                    progress.pause();
                }
            }

            // Clear any progress line that might still be visible (timing issue)
            // Progress loop will also clear, but we do it here to avoid race
            print!("\r{}", ansi::CLEAR_LINE);
            let _ = io::stdout().flush();

            // Print tool header with spacing
            println!(); // Add blank line before tool output
            print_tool_header(&name, &args_str);

            // Update progress indicator with current action (for when it resumes)
            {
                let s = state.lock().await;
                if let Some(ref progress) = s.progress_state {
                    // Set action based on tool type
                    let action = tool_to_action(&name);
                    progress.set_action(&action);

                    // Set focus to tool details
                    let focus = tool_to_focus(&name, &args_str);
                    if let Some(f) = focus {
                        progress.set_focus(&f);
                    }
                }
            }

            // Emit AG-UI ToolCallStart event if bridge is connected
            let ag_ui_tool_call_id = {
                let s = state.lock().await;
                if let Some(ref bridge) = s.event_bridge {
                    // Parse args as JSON for the event
                    let args_json: serde_json::Value = serde_json::from_str(&args_str)
                        .unwrap_or_else(|_| serde_json::json!({"raw": args_str}));
                    Some(bridge.start_tool_call(&name, &args_json).await)
                } else {
                    None
                }
            };

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
                ag_ui_tool_call_id,
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
            let (status_ok, output_lines, is_collapsible) =
                print_tool_result(&name, &args_str, &result_str);

            // Update state and emit AG-UI ToolCallEnd event
            let mut s = state.lock().await;
            if let Some(idx) = s.current_tool_index {
                // Get tool call ID before mutating
                let ag_ui_tool_call_id = s.tool_calls.get(idx)
                    .and_then(|t| t.ag_ui_tool_call_id.clone());

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

                // Emit AG-UI ToolCallEnd event if bridge is connected
                if let (Some(bridge), Some(tool_call_id)) = (&s.event_bridge, &ag_ui_tool_call_id) {
                    bridge.end_tool_call(tool_call_id).await;
                }
            }
            s.current_tool_index = None;

            // Resume progress indicator after tool completes
            if let Some(ref progress) = s.progress_state {
                progress.set_action("Thinking");
                progress.clear_focus();
                progress.resume();
            }
        }
    }

    fn on_completion_response(
        &self,
        _prompt: &Message,
        response: &CompletionResponse<M::Response>,
        cancel: CancelSignal,
    ) -> impl std::future::Future<Output = ()> + Send {
        let state = self.state.clone();

        // Capture usage from response for token tracking
        let usage = response.usage;

        // Store the cancel signal immediately - this is called before tool calls
        // so we can support Ctrl+C during initial "Thinking" phase
        let cancel_for_store = cancel.clone();

        // Check if response contains tool calls - if so, any text is "thinking"
        // If no tool calls, this is the final response - don't show as thinking
        let has_tool_calls = response
            .choice
            .iter()
            .any(|content| matches!(content, AssistantContent::ToolCall(_)));

        // Extract reasoning content (GPT-5.2 thinking summaries)
        let reasoning_parts: Vec<String> = response
            .choice
            .iter()
            .filter_map(|content| {
                if let AssistantContent::Reasoning(Reasoning { reasoning, .. }) = content {
                    // Join all reasoning strings
                    let text = reasoning.to_vec().join("\n");
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
        let text_parts: Vec<String> = response
            .choice
            .iter()
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
            // Store the cancel signal first - enables Ctrl+C during initial "Thinking"
            {
                let mut s = state.lock().await;
                s.cancel_signal = Some(cancel_for_store);
            }

            // Accumulate usage tokens from this response
            {
                let mut s = state.lock().await;
                s.usage.add(&usage);

                // Update progress indicator if connected
                if let Some(ref progress) = s.progress_state {
                    progress.update_tokens(usage.input_tokens, usage.output_tokens);
                }
            }

            // First, show reasoning content if available (GPT-5.2 thinking)
            if !reasoning_parts.is_empty() {
                let thinking_text = reasoning_parts.join("\n");

                // Store in state for history tracking and pause progress
                let mut s = state.lock().await;
                s.agent_messages.push(thinking_text.clone());
                if let Some(ref progress) = s.progress_state {
                    progress.pause();
                }
                drop(s);

                // Clear any progress line (race condition prevention)
                print!("\r{}", ansi::CLEAR_LINE);
                let _ = io::stdout().flush();

                // Display reasoning as thinking (minimal style - no redundant header)
                print_agent_thinking(&thinking_text);

                // Resume progress after
                let s = state.lock().await;
                if let Some(ref progress) = s.progress_state {
                    progress.resume();
                }
            }

            // Also show text content if it's intermediate (has tool calls)
            // but NOT if it's the final response
            if !text_parts.is_empty() && has_tool_calls {
                let thinking_text = text_parts.join("\n");

                // Store in state for history tracking and pause progress
                let mut s = state.lock().await;
                s.agent_messages.push(thinking_text.clone());
                if let Some(ref progress) = s.progress_state {
                    progress.pause();
                }
                drop(s);

                // Clear any progress line (race condition prevention)
                print!("\r{}", ansi::CLEAR_LINE);
                let _ = io::stdout().flush();

                // Display as thinking (minimal style)
                print_agent_thinking(&thinking_text);

                // Resume progress after
                let s = state.lock().await;
                if let Some(ref progress) = s.progress_state {
                    progress.resume();
                }
            }
        }
    }
}

/// Print agent thinking/reasoning text with nice formatting
/// Note: No header needed - progress indicator shows "Thinking" action
fn print_agent_thinking(text: &str) {
    use crate::agent::ui::response::brand;

    println!();

    // Format the content with markdown support (subtle style)
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim();

        // Handle code blocks
        if trimmed.starts_with("```") {
            if in_code_block {
                println!(
                    "{}  └────────────────────────────────────────────────────────┘{}",
                    brand::LIGHT_PEACH,
                    brand::RESET
                );
                in_code_block = false;
            } else {
                let lang = trimmed.strip_prefix("```").unwrap_or("");
                let lang_display = if lang.is_empty() { "code" } else { lang };
                println!(
                    "{}  ┌─ {}{}{} ────────────────────────────────────────────────────┐{}",
                    brand::LIGHT_PEACH,
                    brand::CYAN,
                    lang_display,
                    brand::LIGHT_PEACH,
                    brand::RESET
                );
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            println!(
                "{}  │ {}{}{}  │",
                brand::LIGHT_PEACH,
                brand::CYAN,
                line,
                brand::RESET
            );
            continue;
        }

        // Handle bullet points
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let content = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .unwrap_or(trimmed);
            println!(
                "{}  • {}{}",
                brand::PEACH,
                format_thinking_inline(content),
                brand::RESET
            );
            continue;
        }

        // Handle numbered lists
        if trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && trimmed.chars().nth(1) == Some('.')
        {
            println!(
                "{}  {}{}",
                brand::PEACH,
                format_thinking_inline(trimmed),
                brand::RESET
            );
            continue;
        }

        // Regular text with inline formatting
        if trimmed.is_empty() {
            println!();
        } else {
            // Word wrap long lines
            let wrapped = wrap_text(trimmed, 76);
            for wrapped_line in wrapped {
                println!(
                    "{}  {}{}",
                    brand::PEACH,
                    format_thinking_inline(&wrapped_line),
                    brand::RESET
                );
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
        if chars[i] == '`'
            && (i + 1 >= chars.len() || chars[i + 1] != '`')
            && let Some(end) = chars[i + 1..].iter().position(|&c| c == '`')
        {
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

        // Handle **bold**
        if i + 1 < chars.len()
            && chars[i] == '*'
            && chars[i + 1] == '*'
            && let Some(end_offset) = find_double_star(&chars, i + 2)
        {
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
        println!(
            "\n{} {}({})",
            "●".yellow(),
            name.cyan().bold(),
            args_display.dimmed()
        );
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
    let parsed: Result<serde_json::Value, _> =
        serde_json::from_str(result).map(|v: serde_json::Value| {
            // If the parsed value is a string, it might be double-encoded JSON
            // Try to parse the inner string, but fall back to original if it fails
            if let Some(inner_str) = v.as_str() {
                serde_json::from_str(inner_str).unwrap_or(v)
            } else {
                v
            }
        });

    // If parsing failed, check if it's a tool error message
    // Tool errors come through as plain strings like "Shell error: ..."
    let parsed = if parsed.is_err() && !result.is_empty() {
        // Check for common error patterns
        let is_tool_error = result.contains("error:")
            || result.contains("Error:")
            || result.starts_with("Shell error")
            || result.starts_with("Toolset error")
            || result.starts_with("ToolCallError");

        if is_tool_error {
            // Wrap the error message in a JSON structure so formatters can handle it
            let clean_msg = result
                .replace("Toolset error: ", "")
                .replace("ToolCallError: ", "")
                .replace("Shell error: ", "");
            Ok(serde_json::json!({
                "error": true,
                "message": clean_msg,
                "success": false
            }))
        } else {
            parsed
        }
    } else {
        parsed
    };

    // Format output based on tool type
    let (status_ok, output_lines) = match name {
        "shell" => format_shell_result(&parsed),
        "write_file" | "write_files" => format_write_result(&parsed),
        "read_file" => format_read_result(&parsed),
        "list_directory" => format_list_result(&parsed),
        "analyze_project" => format_analyze_result(&parsed),
        "security_scan" | "check_vulnerabilities" => format_security_result(&parsed),
        "hadolint" => format_hadolint_result(&parsed),
        "kubelint" => format_kubelint_result(&parsed),
        "helmlint" => format_helmlint_result(&parsed),
        "retrieve_output" => format_retrieve_result(&parsed),
        _ => (true, vec!["done".to_string()]),
    };

    // Clear the header line to update dot color
    print!("{}{}", ansi::CURSOR_UP, ansi::CLEAR_LINE);

    // Reprint header with green/red dot and args
    let dot = if status_ok {
        "●".green()
    } else {
        "●".red()
    };

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
fn format_args_display(
    name: &str,
    parsed: &Result<serde_json::Value, serde_json::Error>,
) -> String {
    match name {
        "shell" => {
            if let Ok(v) = parsed {
                v.get("command")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                String::new()
            }
        }
        "write_file" => {
            if let Ok(v) = parsed {
                v.get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .to_string()
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
                v.get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                String::new()
            }
        }
        "list_directory" => {
            if let Ok(v) = parsed {
                v.get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or(".")
                    .to_string()
            } else {
                ".".to_string()
            }
        }
        "kubelint" | "helmlint" | "hadolint" | "dclint" => {
            if let Ok(v) = parsed {
                // Show path if provided
                if let Some(path) = v.get("path").and_then(|p| p.as_str()) {
                    return path.to_string();
                }
                // Show content indicator if provided
                if v.get("content").and_then(|c| c.as_str()).is_some() {
                    return "<inline>".to_string();
                }
                // No path - will use auto-discovery
                "<auto>".to_string()
            } else {
                String::new()
            }
        }
        "retrieve_output" => {
            if let Ok(v) = parsed {
                let ref_id = v.get("ref_id").and_then(|r| r.as_str()).unwrap_or("?");
                let query = v.get("query").and_then(|q| q.as_str());

                if let Some(q) = query {
                    format!("{}, \"{}\"", ref_id, q)
                } else {
                    ref_id.to_string()
                }
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

/// Format shell command result
fn format_shell_result(
    parsed: &Result<serde_json::Value, serde_json::Error>,
) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        // Check if this is an error message (from tool error or blocked command)
        if let Some(error_msg) = v.get("message").and_then(|m| m.as_str())
            && v.get("error").and_then(|e| e.as_bool()).unwrap_or(false)
        {
            return (false, vec![error_msg.to_string()]);
        }

        // Check for cancelled or blocked operations (plan mode, user cancel)
        if v.get("cancelled")
            .and_then(|c| c.as_bool())
            .unwrap_or(false)
        {
            let reason = v
                .get("reason")
                .and_then(|r| r.as_str())
                .unwrap_or("cancelled");
            return (false, vec![reason.to_string()]);
        }

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
            lines.push(if success {
                "completed".to_string()
            } else {
                "failed".to_string()
            });
        }

        (success, lines)
    } else {
        (false, vec!["parse error".to_string()])
    }
}

/// Format write file result
fn format_write_result(
    parsed: &Result<serde_json::Value, serde_json::Error>,
) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let success = v.get("success").and_then(|s| s.as_bool()).unwrap_or(false);
        let action = v.get("action").and_then(|a| a.as_str()).unwrap_or("wrote");
        let lines_written = v
            .get("lines_written")
            .or_else(|| v.get("total_lines"))
            .and_then(|n| n.as_u64())
            .unwrap_or(0);
        let files_written = v.get("files_written").and_then(|n| n.as_u64()).unwrap_or(1);

        let msg = if files_written > 1 {
            format!(
                "{} {} files ({} lines)",
                action, files_written, lines_written
            )
        } else {
            format!("{} ({} lines)", action, lines_written)
        };

        (success, vec![msg])
    } else {
        (false, vec!["write failed".to_string()])
    }
}

/// Format read file result
fn format_read_result(
    parsed: &Result<serde_json::Value, serde_json::Error>,
) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        // Handle error field
        if v.get("error").is_some() {
            let error_msg = v
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("file not found");
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
fn format_list_result(
    parsed: &Result<serde_json::Value, serde_json::Error>,
) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let entries = v.get("entries").and_then(|e| e.as_array());

        let mut lines = Vec::new();

        if let Some(entries) = entries {
            let total = entries.len();
            for entry in entries.iter().take(PREVIEW_LINES + 2) {
                let name = entry.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                let entry_type = entry.get("type").and_then(|t| t.as_str()).unwrap_or("file");
                let prefix = if entry_type == "directory" {
                    "📁"
                } else {
                    "📄"
                };
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

/// Format analyze result - handles both raw and compressed outputs
fn format_analyze_result(
    parsed: &Result<serde_json::Value, serde_json::Error>,
) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let mut lines = Vec::new();

        // Check if this is compressed output (has full_data_ref)
        let is_compressed = v.get("full_data_ref").is_some();

        if is_compressed {
            // Compressed output format
            let ref_id = v
                .get("full_data_ref")
                .and_then(|r| r.as_str())
                .unwrap_or("?");

            // Project count (monorepo)
            if let Some(count) = v.get("project_count").and_then(|c| c.as_u64()) {
                lines.push(format!(
                    "{}📁 {} projects detected{}",
                    ansi::SUCCESS,
                    count,
                    ansi::RESET
                ));
            }

            // Languages (compressed uses languages_detected as array of strings)
            if let Some(langs) = v.get("languages_detected").and_then(|l| l.as_array()) {
                let names: Vec<&str> = langs.iter().filter_map(|l| l.as_str()).take(5).collect();
                if !names.is_empty() {
                    lines.push(format!("  │ Languages: {}", names.join(", ")));
                }
            }

            // Frameworks/Technologies (compressed uses frameworks_detected)
            if let Some(fws) = v.get("frameworks_detected").and_then(|f| f.as_array()) {
                let names: Vec<&str> = fws.iter().filter_map(|f| f.as_str()).take(5).collect();
                if !names.is_empty() {
                    lines.push(format!("  │ Frameworks: {}", names.join(", ")));
                }
            }

            // Technologies (ProjectAnalysis format)
            if let Some(techs) = v.get("technologies_detected").and_then(|t| t.as_array()) {
                let names: Vec<&str> = techs.iter().filter_map(|t| t.as_str()).take(5).collect();
                if !names.is_empty() {
                    lines.push(format!("  │ Technologies: {}", names.join(", ")));
                }
            }

            // Services
            if let Some(services) = v.get("services_detected").and_then(|s| s.as_array()) {
                let names: Vec<&str> = services.iter().filter_map(|s| s.as_str()).take(4).collect();
                if !names.is_empty() {
                    lines.push(format!("  │ Services: {}", names.join(", ")));
                }
            } else if let Some(count) = v.get("services_count").and_then(|c| c.as_u64())
                && count > 0
            {
                lines.push(format!("  │ Services: {} detected", count));
            }

            // Retrieval hint
            lines.push(format!(
                "{}  └ Full data: retrieve_output('{}'){}",
                ansi::GRAY,
                ref_id,
                ansi::RESET
            ));

            return (true, lines);
        }

        // Raw (non-compressed) output format
        // Languages (raw format has objects with name field)
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

        // Frameworks (raw format has objects with name field)
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
fn format_security_result(
    parsed: &Result<serde_json::Value, serde_json::Error>,
) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let findings = v
            .get("findings")
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
fn format_hadolint_result(
    parsed: &Result<serde_json::Value, serde_json::Error>,
) -> (bool, Vec<String>) {
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
                ansi::SUCCESS,
                ansi::RESET
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
            priority_parts.push(format!(
                "{}🔴 {} critical{}",
                ansi::CRITICAL,
                critical,
                ansi::RESET
            ));
        }
        if high > 0 {
            priority_parts.push(format!("{}🟠 {} high{}", ansi::HIGH, high, ansi::RESET));
        }
        if medium > 0 {
            priority_parts.push(format!(
                "{}🟡 {} medium{}",
                ansi::MEDIUM,
                medium,
                ansi::RESET
            ));
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
        if shown < MAX_PREVIEW
            && let Some(high_issues) = action_plan
                .and_then(|a| a.get("high"))
                .and_then(|h| h.as_array())
        {
            for issue in high_issues.iter().take(MAX_PREVIEW - shown) {
                lines.push(format_hadolint_issue(issue, "🟠", ansi::HIGH));
                shown += 1;
            }
        }

        // Show quick fix hint for most important issue
        if let Some(quick_fixes) = v.get("quick_fixes").and_then(|q| q.as_array())
            && let Some(first_fix) = quick_fixes.first().and_then(|f| f.as_str())
        {
            let truncated = truncate_safe(first_fix, 70);
            lines.push(format!(
                "{}  → Fix: {}{}",
                ansi::INFO_BLUE,
                truncated,
                ansi::RESET
            ));
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
    let msg_display = truncate_safe(message, 50);

    format!(
        "{}{} L{}:{} {}{}[{}]{} {} {}",
        color,
        icon,
        line_num,
        ansi::RESET,
        ansi::DOCKER_BLUE,
        ansi::BOLD,
        code,
        ansi::RESET,
        badge,
        msg_display
    )
}

/// Format kubelint result - inline preview format like hadolint
fn format_kubelint_result(
    parsed: &Result<serde_json::Value, serde_json::Error>,
) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let success = v.get("success").and_then(|s| s.as_bool()).unwrap_or(true);
        let summary = v.get("summary");
        let action_plan = v.get("action_plan");
        let parse_errors = v.get("parse_errors").and_then(|p| p.as_array());

        let total = summary
            .and_then(|s| s.get("total_issues"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);

        let mut lines = Vec::new();

        // Check for parse errors first
        if let Some(errors) = parse_errors
            && !errors.is_empty()
        {
            lines.push(format!(
                "{}☸ {} parse error{} (files could not be fully analyzed){}",
                ansi::HIGH,
                errors.len(),
                if errors.len() == 1 { "" } else { "s" },
                ansi::RESET
            ));
            for (i, err) in errors.iter().take(3).enumerate() {
                if let Some(err_str) = err.as_str() {
                    let truncated = truncate_safe(err_str, 70);
                    lines.push(format!(
                        "{}  {} {}{}",
                        ansi::HIGH,
                        if i == errors.len().min(3) - 1 {
                            "└"
                        } else {
                            "│"
                        },
                        truncated,
                        ansi::RESET
                    ));
                }
            }
            if errors.len() > 3 {
                lines.push(format!(
                    "{}  +{} more errors{}",
                    ansi::GRAY,
                    errors.len() - 3,
                    ansi::RESET
                ));
            }
            // If we only have parse errors and no lint issues, return early
            if total == 0 {
                return (false, lines);
            }
        }

        if total == 0 && parse_errors.map(|e| e.is_empty()).unwrap_or(true) {
            lines.push(format!(
                "{}☸ K8s manifests OK - no issues found{}",
                ansi::SUCCESS,
                ansi::RESET
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
            priority_parts.push(format!(
                "{}🔴 {} critical{}",
                ansi::CRITICAL,
                critical,
                ansi::RESET
            ));
        }
        if high > 0 {
            priority_parts.push(format!("{}🟠 {} high{}", ansi::HIGH, high, ansi::RESET));
        }
        if medium > 0 {
            priority_parts.push(format!(
                "{}🟡 {} medium{}",
                ansi::MEDIUM,
                medium,
                ansi::RESET
            ));
        }
        if low > 0 {
            priority_parts.push(format!("{}🟢 {} low{}", ansi::LOW, low, ansi::RESET));
        }

        let header_color = if critical > 0 {
            ansi::CRITICAL
        } else if high > 0 {
            ansi::HIGH
        } else {
            ansi::CYAN
        };

        lines.push(format!(
            "{}☸ {} issue{} found: {}{}",
            header_color,
            total,
            if total == 1 { "" } else { "s" },
            priority_parts.join(" "),
            ansi::RESET
        ));

        // Show critical and high priority issues
        let mut shown = 0;
        const MAX_PREVIEW: usize = 6;

        // Critical issues first
        if let Some(critical_issues) = action_plan
            .and_then(|a| a.get("critical"))
            .and_then(|c| c.as_array())
        {
            for issue in critical_issues.iter().take(MAX_PREVIEW - shown) {
                lines.push(format_kubelint_issue(issue, "🔴", ansi::CRITICAL));
                shown += 1;
            }
        }

        // Then high priority
        if shown < MAX_PREVIEW
            && let Some(high_issues) = action_plan
                .and_then(|a| a.get("high"))
                .and_then(|h| h.as_array())
        {
            for issue in high_issues.iter().take(MAX_PREVIEW - shown) {
                lines.push(format_kubelint_issue(issue, "🟠", ansi::HIGH));
                shown += 1;
            }
        }

        // Show quick fix hint
        if let Some(quick_fixes) = v.get("quick_fixes").and_then(|q| q.as_array())
            && let Some(first_fix) = quick_fixes.first().and_then(|f| f.as_str())
        {
            let truncated = truncate_safe(first_fix, 70);
            lines.push(format!(
                "{}  → Fix: {}{}",
                ansi::INFO_BLUE,
                truncated,
                ansi::RESET
            ));
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

        (success && total == 0, lines)
    } else {
        (false, vec!["kubelint analysis complete".to_string()])
    }
}
fn format_kubelint_issue(issue: &serde_json::Value, icon: &str, color: &str) -> String {
    let check = issue.get("check").and_then(|c| c.as_str()).unwrap_or("?");
    let message = issue.get("message").and_then(|m| m.as_str()).unwrap_or("?");
    let line_num = issue.get("line").and_then(|l| l.as_u64()).unwrap_or(0);
    let category = issue.get("category").and_then(|c| c.as_str()).unwrap_or("");

    // Category badge
    let badge = match category {
        "security" => format!("{}[SEC]{}", ansi::CRITICAL, ansi::RESET),
        "rbac" => format!("{}[RBAC]{}", ansi::CRITICAL, ansi::RESET),
        "best-practice" => format!("{}[BP]{}", ansi::INFO_BLUE, ansi::RESET),
        "validation" => format!("{}[VAL]{}", ansi::MEDIUM, ansi::RESET),
        _ => String::new(),
    };

    // Truncate message
    let msg_display = truncate_safe(message, 50);

    format!(
        "{}{} L{}:{} {}{}[{}]{} {} {}",
        color,
        icon,
        line_num,
        ansi::RESET,
        ansi::CYAN,
        ansi::BOLD,
        check,
        ansi::RESET,
        badge,
        msg_display
    )
}

/// Format helmlint result - inline preview format like hadolint
fn format_helmlint_result(
    parsed: &Result<serde_json::Value, serde_json::Error>,
) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let success = v.get("success").and_then(|s| s.as_bool()).unwrap_or(true);
        let summary = v.get("summary");
        let action_plan = v.get("action_plan");
        let parse_errors = v.get("parse_errors").and_then(|p| p.as_array());

        let total = summary
            .and_then(|s| s.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);

        let mut lines = Vec::new();

        // Check for parse errors first
        if let Some(errors) = parse_errors
            && !errors.is_empty()
        {
            lines.push(format!(
                "{}⎈ {} parse error{} (chart could not be fully analyzed){}",
                ansi::HIGH,
                errors.len(),
                if errors.len() == 1 { "" } else { "s" },
                ansi::RESET
            ));
            for (i, err) in errors.iter().take(3).enumerate() {
                if let Some(err_str) = err.as_str() {
                    let truncated = truncate_safe(err_str, 70);
                    lines.push(format!(
                        "{}  {} {}{}",
                        ansi::HIGH,
                        if i == errors.len().min(3) - 1 {
                            "└"
                        } else {
                            "│"
                        },
                        truncated,
                        ansi::RESET
                    ));
                }
            }
            if errors.len() > 3 {
                lines.push(format!(
                    "{}  +{} more errors{}",
                    ansi::GRAY,
                    errors.len() - 3,
                    ansi::RESET
                ));
            }
            // If we only have parse errors and no lint issues, return early
            if total == 0 {
                return (false, lines);
            }
        }

        if total == 0 && parse_errors.map(|e| e.is_empty()).unwrap_or(true) {
            lines.push(format!(
                "{}⎈ Helm chart OK - no issues found{}",
                ansi::SUCCESS,
                ansi::RESET
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
            priority_parts.push(format!(
                "{}🔴 {} critical{}",
                ansi::CRITICAL,
                critical,
                ansi::RESET
            ));
        }
        if high > 0 {
            priority_parts.push(format!("{}🟠 {} high{}", ansi::HIGH, high, ansi::RESET));
        }
        if medium > 0 {
            priority_parts.push(format!(
                "{}🟡 {} medium{}",
                ansi::MEDIUM,
                medium,
                ansi::RESET
            ));
        }
        if low > 0 {
            priority_parts.push(format!("{}🟢 {} low{}", ansi::LOW, low, ansi::RESET));
        }

        let header_color = if critical > 0 {
            ansi::CRITICAL
        } else if high > 0 {
            ansi::HIGH
        } else {
            ansi::CYAN
        };

        lines.push(format!(
            "{}⎈ {} issue{} found: {}{}",
            header_color,
            total,
            if total == 1 { "" } else { "s" },
            priority_parts.join(" "),
            ansi::RESET
        ));

        // Show critical and high priority issues
        let mut shown = 0;
        const MAX_PREVIEW: usize = 6;

        // Critical issues first
        if let Some(critical_issues) = action_plan
            .and_then(|a| a.get("critical"))
            .and_then(|c| c.as_array())
        {
            for issue in critical_issues.iter().take(MAX_PREVIEW - shown) {
                lines.push(format_helmlint_issue(issue, "🔴", ansi::CRITICAL));
                shown += 1;
            }
        }

        // Then high priority
        if shown < MAX_PREVIEW
            && let Some(high_issues) = action_plan
                .and_then(|a| a.get("high"))
                .and_then(|h| h.as_array())
        {
            for issue in high_issues.iter().take(MAX_PREVIEW - shown) {
                lines.push(format_helmlint_issue(issue, "🟠", ansi::HIGH));
                shown += 1;
            }
        }

        // Show quick fix hint
        if let Some(quick_fixes) = v.get("quick_fixes").and_then(|q| q.as_array())
            && let Some(first_fix) = quick_fixes.first().and_then(|f| f.as_str())
        {
            let truncated = truncate_safe(first_fix, 70);
            lines.push(format!(
                "{}  → Fix: {}{}",
                ansi::INFO_BLUE,
                truncated,
                ansi::RESET
            ));
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

        (success && total == 0, lines)
    } else {
        (false, vec!["helmlint analysis complete".to_string()])
    }
}

/// Format a single helmlint issue for display
fn format_helmlint_issue(issue: &serde_json::Value, icon: &str, color: &str) -> String {
    let code = issue.get("code").and_then(|c| c.as_str()).unwrap_or("?");
    let message = issue.get("message").and_then(|m| m.as_str()).unwrap_or("?");
    let file = issue.get("file").and_then(|f| f.as_str()).unwrap_or("");
    let line_num = issue.get("line").and_then(|l| l.as_u64()).unwrap_or(0);
    let category = issue.get("category").and_then(|c| c.as_str()).unwrap_or("");

    // Category badge
    let badge = match category {
        "Security" | "security" => format!("{}[SEC]{}", ansi::CRITICAL, ansi::RESET),
        "Structure" | "structure" => format!("{}[STRUCT]{}", ansi::GRAY, ansi::RESET),
        "Template" | "template" => format!("{}[TPL]{}", ansi::MEDIUM, ansi::RESET),
        "Values" | "values" => format!("{}[VAL]{}", ansi::MEDIUM, ansi::RESET),
        _ => String::new(),
    };

    // Short file name
    let file_short = if file.chars().count() > 20 {
        let skip = file.chars().count().saturating_sub(17);
        format!("...{}", file.chars().skip(skip).collect::<String>())
    } else {
        file.to_string()
    };

    // Truncate message
    let msg_display = truncate_safe(message, 40);

    format!(
        "{}{} {}:{}:{} {}{}[{}]{} {} {}",
        color,
        icon,
        file_short,
        line_num,
        ansi::RESET,
        ansi::CYAN,
        ansi::BOLD,
        code,
        ansi::RESET,
        badge,
        msg_display
    )
}

/// Format retrieve_output result - shows what data was retrieved
fn format_retrieve_result(
    parsed: &Result<serde_json::Value, serde_json::Error>,
) -> (bool, Vec<String>) {
    if let Ok(v) = parsed {
        let mut lines = Vec::new();

        // Check for error field first
        if let Some(error) = v.get("error").and_then(|e| e.as_str()) {
            lines.push(format!("{}❌ {}{}", ansi::CRITICAL, error, ansi::RESET));
            return (false, lines);
        }

        // Check if this is a query result with total_matches
        if let Some(total) = v.get("total_matches").and_then(|t| t.as_u64()) {
            let query = v
                .get("query")
                .and_then(|q| q.as_str())
                .unwrap_or("unfiltered");

            lines.push(format!(
                "{}📦 Retrieved {} match{} for '{}'{}",
                ansi::SUCCESS,
                total,
                if total == 1 { "" } else { "es" },
                query,
                ansi::RESET
            ));

            // Show preview of results
            if let Some(results) = v.get("results").and_then(|r| r.as_array()) {
                for (i, result) in results.iter().take(3).enumerate() {
                    let preview = format_result_preview(result);
                    let prefix = if i == results.len().min(3) - 1 && results.len() <= 3 {
                        "└"
                    } else {
                        "│"
                    };
                    lines.push(format!("  {} {}", prefix, preview));
                }
                if results.len() > 3 {
                    lines.push(format!(
                        "{}  └ +{} more results{}",
                        ansi::GRAY,
                        results.len() - 3,
                        ansi::RESET
                    ));
                }
            }

            return (true, lines);
        }

        // Check for analyze_project section results
        if v.get("project_count").is_some() || v.get("total_projects").is_some() {
            let count = v
                .get("project_count")
                .or_else(|| v.get("total_projects"))
                .and_then(|c| c.as_u64())
                .unwrap_or(0);

            lines.push(format!(
                "{}📦 Retrieved project summary ({} projects){}",
                ansi::SUCCESS,
                count,
                ansi::RESET
            ));

            // Show project names if available
            if let Some(names) = v.get("project_names").and_then(|n| n.as_array()) {
                let name_list: Vec<&str> =
                    names.iter().filter_map(|n| n.as_str()).take(5).collect();
                if !name_list.is_empty() {
                    lines.push(format!("  │ Projects: {}", name_list.join(", ")));
                }
                if names.len() > 5 {
                    lines.push(format!(
                        "{}  └ +{} more{}",
                        ansi::GRAY,
                        names.len() - 5,
                        ansi::RESET
                    ));
                }
            }

            return (true, lines);
        }

        // Check for services list
        if let Some(total) = v.get("total_services").and_then(|t| t.as_u64()) {
            lines.push(format!(
                "{}📦 Retrieved {} service{}{}",
                ansi::SUCCESS,
                total,
                if total == 1 { "" } else { "s" },
                ansi::RESET
            ));

            if let Some(services) = v.get("services").and_then(|s| s.as_array()) {
                for (i, svc) in services.iter().take(4).enumerate() {
                    let name = svc.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                    let svc_type = svc
                        .get("service_type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    let prefix = if i == services.len().min(4) - 1 && services.len() <= 4 {
                        "└"
                    } else {
                        "│"
                    };
                    lines.push(format!("  {} 🔧 {} {}", prefix, name, svc_type));
                }
                if services.len() > 4 {
                    lines.push(format!(
                        "{}  └ +{} more{}",
                        ansi::GRAY,
                        services.len() - 4,
                        ansi::RESET
                    ));
                }
            }

            return (true, lines);
        }

        // Check for languages/frameworks result
        if v.get("languages").is_some() || v.get("technologies").is_some() {
            lines.push(format!(
                "{}📦 Retrieved analysis data{}",
                ansi::SUCCESS,
                ansi::RESET
            ));

            if let Some(langs) = v.get("languages").and_then(|l| l.as_array()) {
                let names: Vec<&str> = langs
                    .iter()
                    .filter_map(|l| l.get("name").and_then(|n| n.as_str()))
                    .take(5)
                    .collect();
                if !names.is_empty() {
                    lines.push(format!("  │ Languages: {}", names.join(", ")));
                }
            }

            if let Some(techs) = v.get("technologies").and_then(|t| t.as_array()) {
                let names: Vec<&str> = techs
                    .iter()
                    .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
                    .take(5)
                    .collect();
                if !names.is_empty() {
                    lines.push(format!("  └ Technologies: {}", names.join(", ")));
                }
            }

            return (true, lines);
        }

        // Generic fallback - estimate data size
        let json_str = serde_json::to_string(v).unwrap_or_default();
        let size_kb = json_str.len() as f64 / 1024.0;

        lines.push(format!(
            "{}📦 Retrieved {:.1} KB of data{}",
            ansi::SUCCESS,
            size_kb,
            ansi::RESET
        ));

        // Try to show some structure info
        if let Some(obj) = v.as_object() {
            let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).take(5).collect();
            if !keys.is_empty() {
                lines.push(format!("  └ Fields: {}", keys.join(", ")));
            }
        }

        (true, lines)
    } else {
        (false, vec!["retrieve failed".to_string()])
    }
}

/// Format a single result item for preview
fn format_result_preview(result: &serde_json::Value) -> String {
    // Try to get meaningful identifiers
    let name = result
        .get("name")
        .or_else(|| result.get("code"))
        .or_else(|| result.get("check"))
        .and_then(|v| v.as_str())
        .unwrap_or("item");

    let detail = result
        .get("message")
        .or_else(|| result.get("description"))
        .or_else(|| result.get("path"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let detail_short = truncate_safe(detail, 40);

    if detail_short.is_empty() {
        name.to_string()
    } else {
        format!("{}: {}", name, detail_short)
    }
}

/// Convert tool name to a friendly action description for progress indicator
fn tool_to_action(tool_name: &str) -> String {
    match tool_name {
        "read_file" => "Reading file".to_string(),
        "write_file" | "write_files" => "Writing file".to_string(),
        "list_directory" => "Listing directory".to_string(),
        "shell" => "Running command".to_string(),
        "analyze_project" => "Analyzing project".to_string(),
        "security_scan" | "check_vulnerabilities" => "Scanning security".to_string(),
        "hadolint" => "Linting Dockerfile".to_string(),
        "dclint" => "Linting docker-compose".to_string(),
        "kubelint" => "Linting Kubernetes".to_string(),
        "helmlint" => "Linting Helm chart".to_string(),
        "terraform_fmt" => "Formatting Terraform".to_string(),
        "terraform_validate" => "Validating Terraform".to_string(),
        "plan_create" => "Creating plan".to_string(),
        "plan_list" => "Listing plans".to_string(),
        "plan_next" | "plan_update" => "Updating plan".to_string(),
        "retrieve_output" => "Retrieving data".to_string(),
        "list_stored_outputs" => "Listing outputs".to_string(),
        _ => "Processing".to_string(),
    }
}

/// Extract focus/detail from tool arguments for progress indicator
fn tool_to_focus(tool_name: &str, args: &str) -> Option<String> {
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(args);
    let parsed = parsed.ok()?;

    match tool_name {
        "read_file" | "write_file" => {
            parsed.get("path").and_then(|p| p.as_str()).map(|p| {
                // Shorten long paths
                let char_count = p.chars().count();
                if char_count > 50 {
                    let skip = char_count.saturating_sub(47);
                    format!("...{}", p.chars().skip(skip).collect::<String>())
                } else {
                    p.to_string()
                }
            })
        }
        "list_directory" => parsed
            .get("path")
            .and_then(|p| p.as_str())
            .map(|p| p.to_string()),
        "shell" => parsed.get("command").and_then(|c| c.as_str()).map(|cmd| {
            // Truncate long commands
            truncate_safe(cmd, 60)
        }),
        "hadolint" | "dclint" | "kubelint" | "helmlint" => parsed
            .get("path")
            .and_then(|p| p.as_str())
            .map(|p| p.to_string())
            .or_else(|| {
                if parsed.get("content").is_some() {
                    Some("<inline content>".to_string())
                } else {
                    Some("<auto-detect>".to_string())
                }
            }),
        "plan_create" => parsed
            .get("name")
            .and_then(|n| n.as_str())
            .map(|n| n.to_string()),
        "retrieve_output" => {
            let ref_id = parsed.get("ref_id").and_then(|r| r.as_str())?;
            let query = parsed.get("query").and_then(|q| q.as_str());
            Some(if let Some(q) = query {
                format!("{} ({})", ref_id, q)
            } else {
                ref_id.to_string()
            })
        }
        _ => None,
    }
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
