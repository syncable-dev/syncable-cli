//! Agent module for interactive AI-powered CLI assistance
//!
//! This module provides an agent layer using the Rig library that allows users
//! to interact with the CLI through natural language conversations.
//!
//! # Features
//!
//! - **Conversation History**: Maintains context across multiple turns
//! - **Automatic Compaction**: Compresses old history when token count exceeds threshold
//! - **Tool Tracking**: Records tool calls for better context preservation
//!
//! # Usage
//!
//! ```bash
//! # Interactive mode
//! sync-ctl chat
//!
//! # With specific provider
//! sync-ctl chat --provider openai --model gpt-5.2
//!
//! # Single query
//! sync-ctl chat --query "What security issues does this project have?"
//! ```
//!
//! # Interactive Commands
//!
//! - `/model` - Switch to a different AI model
//! - `/provider` - Switch provider (prompts for API key if needed)
//! - `/help` - Show available commands
//! - `/clear` - Clear conversation history
//! - `/exit` - Exit the chat

pub mod commands;
pub mod history;
pub mod ide;
pub mod prompts;
pub mod session;
pub mod tools;
pub mod ui;

use colored::Colorize;
use history::{ConversationHistory, ToolCallRecord};
use ide::IdeClient;
use rig::{
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::{anthropic, openai},
};
use session::ChatSession;
use commands::TokenUsage;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use ui::{ResponseFormatter, ToolDisplayHook};

/// Provider type for the agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProviderType {
    #[default]
    OpenAI,
    Anthropic,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
        }
    }
}

impl std::str::FromStr for ProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(ProviderType::OpenAI),
            "anthropic" => Ok(ProviderType::Anthropic),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

/// Error types for the agent
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Missing API key. Set {0} environment variable.")]
    MissingApiKey(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Tool error: {0}")]
    ToolError(String),
}

pub type AgentResult<T> = Result<T, AgentError>;

/// Get the system prompt for the agent based on query type
fn get_system_prompt(project_path: &Path, query: Option<&str>) -> String {
    if let Some(q) = query {
        // First check if it's a code development task (highest priority)
        if prompts::is_code_development_query(q) {
            return prompts::get_code_development_prompt(project_path);
        }
        // Then check if it's DevOps generation (Docker, Terraform, Helm)
        if prompts::is_generation_query(q) {
            return prompts::get_devops_prompt(project_path);
        }
    }
    // Default to analysis prompt
    prompts::get_analysis_prompt(project_path)
}

/// Run the agent in interactive mode with custom REPL supporting /model and /provider commands
pub async fn run_interactive(
    project_path: &Path,
    provider: ProviderType,
    model: Option<String>,
) -> AgentResult<()> {
    use tools::*;

    let mut session = ChatSession::new(project_path, provider, model);

    // Initialize conversation history with compaction support
    let mut conversation_history = ConversationHistory::new();

    // Initialize IDE client for native diff viewing
    let ide_client: Option<Arc<TokioMutex<IdeClient>>> = {
        let mut client = IdeClient::new().await;
        if client.is_ide_available() {
            match client.connect().await {
                Ok(()) => {
                    println!(
                        "{} Connected to {} IDE companion",
                        "✓".green(),
                        client.ide_name().unwrap_or("VS Code")
                    );
                    Some(Arc::new(TokioMutex::new(client)))
                }
                Err(e) => {
                    // IDE detected but companion not running or connection failed
                    println!(
                        "{} IDE companion not connected: {}",
                        "!".yellow(),
                        e
                    );
                    None
                }
            }
        } else {
            println!("{} No IDE detected (TERM_PROGRAM={})", "·".dimmed(), std::env::var("TERM_PROGRAM").unwrap_or_default());
            None
        }
    };

    // Load API key from config file to env if not already set
    ChatSession::load_api_key_to_env(session.provider);

    // Check if API key is configured, prompt if not
    if !ChatSession::has_api_key(session.provider) {
        ChatSession::prompt_api_key(session.provider)?;
    }

    session.print_banner();

    loop {
        // Show conversation status if we have history
        if !conversation_history.is_empty() {
            println!("{}", format!("  💬 Context: {}", conversation_history.status()).dimmed());
        }

        // Read user input
        let input = match session.read_input() {
            Ok(input) => input,
            Err(_) => break,
        };

        if input.is_empty() {
            continue;
        }

        // Check for commands
        if ChatSession::is_command(&input) {
            // Special handling for /clear to also clear conversation history
            if input.trim().to_lowercase() == "/clear" || input.trim().to_lowercase() == "/c" {
                conversation_history.clear();
            }
            match session.process_command(&input) {
                Ok(true) => continue,
                Ok(false) => break, // /exit
                Err(e) => {
                    eprintln!("{}", format!("Error: {}", e).red());
                    continue;
                }
            }
        }

        // Check API key before making request (in case provider changed)
        if !ChatSession::has_api_key(session.provider) {
            eprintln!("{}", "No API key configured. Use /provider to set one.".yellow());
            continue;
        }

        // Check if compaction is needed before making the request
        if conversation_history.needs_compaction() {
            println!("{}", "  📦 Compacting conversation history...".dimmed());
            if let Some(summary) = conversation_history.compact() {
                println!("{}", format!("  ✓ Compressed {} turns", summary.matches("Turn").count()).dimmed());
            }
        }

        // Retry loop for automatic error recovery
        // MAX_RETRIES is for failures without progress
        // MAX_CONTINUATIONS is for truncations WITH progress (more generous)
        // TOOL_CALL_CHECKPOINT is the interval at which we ask user to confirm
        // MAX_TOOL_CALLS is the absolute maximum (300 = 6 checkpoints x 50)
        const MAX_RETRIES: u32 = 3;
        const MAX_CONTINUATIONS: u32 = 10;
        const TOOL_CALL_CHECKPOINT: usize = 50;
        const MAX_TOOL_CALLS: usize = 300;
        let mut retry_attempt = 0;
        let mut continuation_count = 0;
        let mut total_tool_calls: usize = 0;
        let mut auto_continue_tools = false; // User can select "always" to skip future prompts
        let mut current_input = input.clone();
        let mut succeeded = false;

        while retry_attempt < MAX_RETRIES && continuation_count < MAX_CONTINUATIONS && !succeeded {

            // Log if this is a continuation attempt
            if continuation_count > 0 {
                eprintln!("{}", format!("  📡 Sending continuation request...").dimmed());
            }

            // Create hook for Claude Code style tool display
            let hook = ToolDisplayHook::new();

            let project_path_buf = session.project_path.clone();
            // Select prompt based on query type (analysis vs generation)
            let preamble = get_system_prompt(&session.project_path, Some(&current_input));
            let is_generation = prompts::is_generation_query(&current_input);

            // Convert conversation history to Rig Message format
            let mut chat_history = conversation_history.to_messages();

            let response = match session.provider {
                ProviderType::OpenAI => {
                    let client = openai::Client::from_env();
                    // For GPT-5.x reasoning models, enable reasoning with summary output
                    // so we can see the model's thinking process
                    let reasoning_params = if session.model.starts_with("gpt-5") || session.model.starts_with("o1") {
                        Some(serde_json::json!({
                            "reasoning": {
                                "effort": "medium",
                                "summary": "detailed"
                            }
                        }))
                    } else {
                        None
                    };

                    let mut builder = client
                        .agent(&session.model)
                        .preamble(&preamble)
                        .max_tokens(4096)
                        .tool(AnalyzeTool::new(project_path_buf.clone()))
                        .tool(SecurityScanTool::new(project_path_buf.clone()))
                        .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                        .tool(HadolintTool::new(project_path_buf.clone()))
                        .tool(ReadFileTool::new(project_path_buf.clone()))
                        .tool(ListDirectoryTool::new(project_path_buf.clone()));

                    // Add generation tools if this is a generation query
                    if is_generation {
                        // Create file tools with IDE client if connected
                        let (write_file_tool, write_files_tool) = if let Some(ref client) = ide_client {
                            (
                                WriteFileTool::new(project_path_buf.clone())
                                    .with_ide_client(client.clone()),
                                WriteFilesTool::new(project_path_buf.clone())
                                    .with_ide_client(client.clone()),
                            )
                        } else {
                            (
                                WriteFileTool::new(project_path_buf.clone()),
                                WriteFilesTool::new(project_path_buf.clone()),
                            )
                        };
                        builder = builder
                            .tool(write_file_tool)
                            .tool(write_files_tool)
                            .tool(ShellTool::new(project_path_buf.clone()));
                    }

                    if let Some(params) = reasoning_params {
                        builder = builder.additional_params(params);
                    }

                    let agent = builder.build();
                    // Allow up to 50 tool call turns for complex generation tasks
                    // Use hook to display tool calls as they happen
                    // Pass conversation history for context continuity
                    agent.prompt(&current_input)
                        .with_history(&mut chat_history)
                        .with_hook(hook.clone())
                        .multi_turn(50)
                        .await
                }
                ProviderType::Anthropic => {
                    let client = anthropic::Client::from_env();
                    let mut builder = client
                        .agent(&session.model)
                        .preamble(&preamble)
                        .max_tokens(4096)
                        .tool(AnalyzeTool::new(project_path_buf.clone()))
                        .tool(SecurityScanTool::new(project_path_buf.clone()))
                        .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                        .tool(HadolintTool::new(project_path_buf.clone()))
                        .tool(ReadFileTool::new(project_path_buf.clone()))
                        .tool(ListDirectoryTool::new(project_path_buf.clone()));

                    // Add generation tools if this is a generation query
                    if is_generation {
                        // Create file tools with IDE client if connected
                        let (write_file_tool, write_files_tool) = if let Some(ref client) = ide_client {
                            (
                                WriteFileTool::new(project_path_buf.clone())
                                    .with_ide_client(client.clone()),
                                WriteFilesTool::new(project_path_buf.clone())
                                    .with_ide_client(client.clone()),
                            )
                        } else {
                            (
                                WriteFileTool::new(project_path_buf.clone()),
                                WriteFilesTool::new(project_path_buf.clone()),
                            )
                        };
                        builder = builder
                            .tool(write_file_tool)
                            .tool(write_files_tool)
                            .tool(ShellTool::new(project_path_buf.clone()));
                    }

                    let agent = builder.build();

                    // Allow up to 50 tool call turns for complex generation tasks
                    // Use hook to display tool calls as they happen
                    // Pass conversation history for context continuity
                    agent.prompt(&current_input)
                        .with_history(&mut chat_history)
                        .with_hook(hook.clone())
                        .multi_turn(50)
                        .await
                }
            };

            match response {
                Ok(text) => {
                    // Show final response
                    println!();
                    ResponseFormatter::print_response(&text);

                    // Track token usage (estimate since Rig doesn't expose exact counts)
                    let prompt_tokens = TokenUsage::estimate_tokens(&input);
                    let completion_tokens = TokenUsage::estimate_tokens(&text);
                    session.token_usage.add_request(prompt_tokens, completion_tokens);

                    // Extract tool calls from the hook state for history tracking
                    let tool_calls = extract_tool_calls_from_hook(&hook).await;
                    let batch_tool_count = tool_calls.len();
                    total_tool_calls += batch_tool_count;

                    // Show tool call summary if significant
                    if batch_tool_count > 10 {
                        println!("{}", format!("  ✓ Completed with {} tool calls ({} total this session)", batch_tool_count, total_tool_calls).dimmed());
                    }

                    // Add to conversation history with tool call records
                    conversation_history.add_turn(input.clone(), text.clone(), tool_calls);

                    // Check if this heavy turn requires immediate compaction
                    // This helps prevent context overflow in subsequent requests
                    if conversation_history.needs_compaction() {
                        println!("{}", "  📦 Compacting conversation history...".dimmed());
                        if let Some(summary) = conversation_history.compact() {
                            println!("{}", format!("  ✓ Compressed {} turns", summary.matches("Turn").count()).dimmed());
                        }
                    }

                    // Also update legacy session history for compatibility
                    session.history.push(("user".to_string(), input.clone()));
                    session.history.push(("assistant".to_string(), text));
                    succeeded = true;
                }
                Err(e) => {
                    let err_str = e.to_string();

                    println!();

                    // Check if this is a max depth error - handle as checkpoint
                    if err_str.contains("MaxDepth") || err_str.contains("max_depth") || err_str.contains("reached limit") {
                        // Extract what was done before hitting the limit
                        let completed_tools = extract_tool_calls_from_hook(&hook).await;
                        let agent_thinking = extract_agent_messages_from_hook(&hook).await;
                        let batch_tool_count = completed_tools.len();
                        total_tool_calls += batch_tool_count;

                        eprintln!("{}", format!(
                            "⚠ Reached {} tool calls this batch ({} total). Maximum allowed: {}",
                            batch_tool_count, total_tool_calls, MAX_TOOL_CALLS
                        ).yellow());

                        // Check if we've hit the absolute maximum
                        if total_tool_calls >= MAX_TOOL_CALLS {
                            eprintln!("{}", format!("Maximum tool call limit ({}) reached.", MAX_TOOL_CALLS).red());
                            eprintln!("{}", "The task is too complex. Try breaking it into smaller parts.".dimmed());
                            break;
                        }

                        // Ask user if they want to continue (unless auto-continue is enabled)
                        let should_continue = if auto_continue_tools {
                            eprintln!("{}", "  Auto-continuing (you selected 'always')...".dimmed());
                            true
                        } else {
                            eprintln!("{}", "Excessive tool calls used. Want to continue?".yellow());
                            eprintln!("{}", "  [y] Yes, continue  [n] No, stop  [a] Always continue".dimmed());
                            print!("  > ");
                            let _ = std::io::Write::flush(&mut std::io::stdout());

                            // Read user input
                            let mut response = String::new();
                            match std::io::stdin().read_line(&mut response) {
                                Ok(_) => {
                                    let resp = response.trim().to_lowercase();
                                    if resp == "a" || resp == "always" {
                                        auto_continue_tools = true;
                                        true
                                    } else {
                                        resp == "y" || resp == "yes" || resp.is_empty()
                                    }
                                }
                                Err(_) => false,
                            }
                        };

                        if !should_continue {
                            eprintln!("{}", "Stopped by user. Type 'continue' to resume later.".dimmed());
                            // Add partial progress to history
                            if !completed_tools.is_empty() {
                                conversation_history.add_turn(
                                    current_input.clone(),
                                    format!("[Stopped at checkpoint - {} tools completed]", batch_tool_count),
                                    vec![]
                                );
                            }
                            break;
                        }

                        // Continue from checkpoint
                        eprintln!("{}", format!(
                            "  → Continuing... {} remaining tool calls available",
                            MAX_TOOL_CALLS - total_tool_calls
                        ).dimmed());

                        // Add partial progress to history (without duplicating tool calls)
                        conversation_history.add_turn(
                            current_input.clone(),
                            format!("[Checkpoint - {} tools completed, continuing...]", batch_tool_count),
                            vec![]
                        );

                        // Build continuation prompt
                        current_input = build_continuation_prompt(&input, &completed_tools, &agent_thinking);

                        // Brief delay before continuation
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        continue; // Continue the loop without incrementing retry_attempt
                    } else if err_str.contains("rate") || err_str.contains("Rate") || err_str.contains("429") {
                        eprintln!("{}", "⚠ Rate limited by API provider.".yellow());
                        // Wait before retry for rate limits
                        retry_attempt += 1;
                        eprintln!("{}", format!("  Waiting 5 seconds before retry ({}/{})...", retry_attempt, MAX_RETRIES).dimmed());
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    } else if is_truncation_error(&err_str) {
                        // Truncation error - try intelligent continuation
                        let completed_tools = extract_tool_calls_from_hook(&hook).await;
                        let agent_thinking = extract_agent_messages_from_hook(&hook).await;

                        // Count actually completed tools (not in-progress)
                        let completed_count = completed_tools.iter()
                            .filter(|t| !t.result_summary.contains("IN PROGRESS"))
                            .count();
                        let in_progress_count = completed_tools.len() - completed_count;

                        if !completed_tools.is_empty() && continuation_count < MAX_CONTINUATIONS {
                            // We have partial progress - continue from where we left off
                            continuation_count += 1;
                            let status_msg = if in_progress_count > 0 {
                                format!(
                                    "⚠ Response truncated. {} completed, {} in-progress. Auto-continuing ({}/{})...",
                                    completed_count, in_progress_count, continuation_count, MAX_CONTINUATIONS
                                )
                            } else {
                                format!(
                                    "⚠ Response truncated. {} tool calls completed. Auto-continuing ({}/{})...",
                                    completed_count, continuation_count, MAX_CONTINUATIONS
                                )
                            };
                            eprintln!("{}", status_msg.yellow());

                            // Add partial progress to conversation history
                            // NOTE: We intentionally pass empty tool_calls here because the
                            // continuation prompt already contains the detailed file list.
                            // Including them in history would duplicate the context and waste tokens.
                            conversation_history.add_turn(
                                current_input.clone(),
                                format!("[Partial response - {} tools completed, {} in-progress before truncation. See continuation prompt for details.]",
                                    completed_count, in_progress_count),
                                vec![]  // Don't duplicate - continuation prompt has the details
                            );

                            // Check if we need compaction after adding this heavy turn
                            // This is important for long multi-turn sessions with many tool calls
                            if conversation_history.needs_compaction() {
                                eprintln!("{}", "  📦 Compacting history before continuation...".dimmed());
                                if let Some(summary) = conversation_history.compact() {
                                    eprintln!("{}", format!("  ✓ Compressed {} turns", summary.matches("Turn").count()).dimmed());
                                }
                            }

                            // Build continuation prompt with context
                            current_input = build_continuation_prompt(&input, &completed_tools, &agent_thinking);

                            // Log continuation details for debugging
                            eprintln!("{}", format!(
                                "  → Continuing with {} files read, {} written, {} other actions tracked",
                                completed_tools.iter().filter(|t| t.tool_name == "read_file").count(),
                                completed_tools.iter().filter(|t| t.tool_name == "write_file" || t.tool_name == "write_files").count(),
                                completed_tools.iter().filter(|t| t.tool_name != "read_file" && t.tool_name != "write_file" && t.tool_name != "write_files" && t.tool_name != "list_directory").count()
                            ).dimmed());

                            // Brief delay before continuation
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            // Don't increment retry_attempt - this is progress via continuation
                        } else if retry_attempt < MAX_RETRIES {
                            // No tool calls completed - simple retry
                            retry_attempt += 1;
                            eprintln!("{}", format!("⚠ Response error (attempt {}/{}). Retrying...", retry_attempt, MAX_RETRIES).yellow());
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        } else {
                            // Max retries/continuations reached
                            eprintln!("{}", format!("Error: {}", e).red());
                            if continuation_count >= MAX_CONTINUATIONS {
                                eprintln!("{}", format!("Max continuations ({}) reached. The task is too complex for one request.", MAX_CONTINUATIONS).dimmed());
                            } else {
                                eprintln!("{}", "Max retries reached. The response may be too complex.".dimmed());
                            }
                            eprintln!("{}", "Try breaking your request into smaller parts.".dimmed());
                            break;
                        }
                    } else if err_str.contains("timeout") || err_str.contains("Timeout") {
                        // Timeout - simple retry
                        retry_attempt += 1;
                        if retry_attempt < MAX_RETRIES {
                            eprintln!("{}", format!("⚠ Request timed out (attempt {}/{}). Retrying...", retry_attempt, MAX_RETRIES).yellow());
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        } else {
                            eprintln!("{}", "Request timed out. Please try again.".red());
                            break;
                        }
                    } else {
                        // Unknown error - show details and break
                        eprintln!("{}", format!("Error: {}", e).red());
                        if continuation_count > 0 {
                            eprintln!("{}", format!("  (occurred during continuation attempt {})", continuation_count).dimmed());
                        }
                        eprintln!("{}", "Error details for debugging:".dimmed());
                        eprintln!("{}", format!("  - retry_attempt: {}/{}", retry_attempt, MAX_RETRIES).dimmed());
                        eprintln!("{}", format!("  - continuation_count: {}/{}", continuation_count, MAX_CONTINUATIONS).dimmed());
                        break;
                    }
                }
            }
        }
        println!();
    }

    Ok(())
}

/// Extract tool call records from the hook state for history tracking
async fn extract_tool_calls_from_hook(hook: &ToolDisplayHook) -> Vec<ToolCallRecord> {
    let state = hook.state();
    let guard = state.lock().await;

    guard.tool_calls.iter().enumerate().map(|(i, tc)| {
        let result = if tc.is_running {
            // Tool was in progress when error occurred
            "[IN PROGRESS - may need to be re-run]".to_string()
        } else if let Some(output) = &tc.output {
            truncate_string(output, 200)
        } else {
            "completed".to_string()
        };

        ToolCallRecord {
            tool_name: tc.name.clone(),
            args_summary: truncate_string(&tc.args, 100),
            result_summary: result,
            // Generate a unique tool ID for proper message pairing
            tool_id: Some(format!("tool_{}_{}", tc.name, i)),
        }
    }).collect()
}

/// Extract any agent thinking/messages from the hook for context
async fn extract_agent_messages_from_hook(hook: &ToolDisplayHook) -> Vec<String> {
    let state = hook.state();
    let guard = state.lock().await;
    guard.agent_messages.clone()
}

/// Helper to truncate strings for summaries
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Check if an error is a truncation/JSON parsing error that can be recovered via continuation
fn is_truncation_error(err_str: &str) -> bool {
    err_str.contains("JsonError")
        || err_str.contains("EOF while parsing")
        || err_str.contains("JSON")
        || err_str.contains("unexpected end")
}

/// Build a continuation prompt that tells the AI what work was completed
/// and asks it to continue from where it left off
fn build_continuation_prompt(
    original_task: &str,
    completed_tools: &[ToolCallRecord],
    agent_thinking: &[String],
) -> String {
    use std::collections::HashSet;

    // Group tools by type and extract unique files read
    let mut files_read: HashSet<String> = HashSet::new();
    let mut files_written: HashSet<String> = HashSet::new();
    let mut dirs_listed: HashSet<String> = HashSet::new();
    let mut other_tools: Vec<String> = Vec::new();
    let mut in_progress: Vec<String> = Vec::new();

    for tool in completed_tools {
        let is_in_progress = tool.result_summary.contains("IN PROGRESS");

        if is_in_progress {
            in_progress.push(format!("{}({})", tool.tool_name, tool.args_summary));
            continue;
        }

        match tool.tool_name.as_str() {
            "read_file" => {
                // Extract path from args
                files_read.insert(tool.args_summary.clone());
            }
            "write_file" | "write_files" => {
                files_written.insert(tool.args_summary.clone());
            }
            "list_directory" => {
                dirs_listed.insert(tool.args_summary.clone());
            }
            _ => {
                other_tools.push(format!("{}({})", tool.tool_name, truncate_string(&tool.args_summary, 40)));
            }
        }
    }

    let mut prompt = format!(
        "[CONTINUE] Your previous response was interrupted. DO NOT repeat completed work.\n\n\
        Original task: {}\n",
        truncate_string(original_task, 500)
    );

    // Show files already read - CRITICAL for preventing re-reads
    if !files_read.is_empty() {
        prompt.push_str("\n== FILES ALREADY READ (do NOT read again) ==\n");
        for file in &files_read {
            prompt.push_str(&format!("  - {}\n", file));
        }
    }

    if !dirs_listed.is_empty() {
        prompt.push_str("\n== DIRECTORIES ALREADY LISTED ==\n");
        for dir in &dirs_listed {
            prompt.push_str(&format!("  - {}\n", dir));
        }
    }

    if !files_written.is_empty() {
        prompt.push_str("\n== FILES ALREADY WRITTEN ==\n");
        for file in &files_written {
            prompt.push_str(&format!("  - {}\n", file));
        }
    }

    if !other_tools.is_empty() {
        prompt.push_str("\n== OTHER COMPLETED ACTIONS ==\n");
        for tool in other_tools.iter().take(20) {
            prompt.push_str(&format!("  - {}\n", tool));
        }
        if other_tools.len() > 20 {
            prompt.push_str(&format!("  ... and {} more\n", other_tools.len() - 20));
        }
    }

    if !in_progress.is_empty() {
        prompt.push_str("\n== INTERRUPTED (may need re-run) ==\n");
        for tool in &in_progress {
            prompt.push_str(&format!("  ⚠ {}\n", tool));
        }
    }

    // Include last thinking context if available
    if !agent_thinking.is_empty() {
        if let Some(last_thought) = agent_thinking.last() {
            prompt.push_str(&format!(
                "\n== YOUR LAST THOUGHTS ==\n\"{}\"\n",
                truncate_string(last_thought, 300)
            ));
        }
    }

    prompt.push_str("\n== INSTRUCTIONS ==\n");
    prompt.push_str("IMPORTANT: Your previous response was too long and got cut off.\n");
    prompt.push_str("1. Do NOT re-read files listed above - they are already in context.\n");
    prompt.push_str("2. If writing a document, write it in SECTIONS - complete one section now, then continue.\n");
    prompt.push_str("3. Keep your response SHORT and focused. Better to complete small chunks than fail on large ones.\n");
    prompt.push_str("4. If the task involves writing a file, START WRITING NOW - don't explain what you'll do.\n");

    prompt
}

/// Run a single query and return the response
pub async fn run_query(
    project_path: &Path,
    query: &str,
    provider: ProviderType,
    model: Option<String>,
) -> AgentResult<String> {
    use tools::*;

    let project_path_buf = project_path.to_path_buf();
    // Select prompt based on query type (analysis vs generation)
    let preamble = get_system_prompt(project_path, Some(query));
    let is_generation = prompts::is_generation_query(query);

    match provider {
        ProviderType::OpenAI => {
            let client = openai::Client::from_env();
            let model_name = model.as_deref().unwrap_or("gpt-5.2");

            // For GPT-5.x reasoning models, enable reasoning with summary output
            let reasoning_params = if model_name.starts_with("gpt-5") || model_name.starts_with("o1") {
                Some(serde_json::json!({
                    "reasoning": {
                        "effort": "medium",
                        "summary": "detailed"
                    }
                }))
            } else {
                None
            };

            let mut builder = client
                .agent(model_name)
                .preamble(&preamble)
                .max_tokens(4096)
                .tool(AnalyzeTool::new(project_path_buf.clone()))
                .tool(SecurityScanTool::new(project_path_buf.clone()))
                .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                .tool(HadolintTool::new(project_path_buf.clone()))
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf.clone()));

            // Add generation tools if this is a generation query
            if is_generation {
                builder = builder
                    .tool(WriteFileTool::new(project_path_buf.clone()))
                    .tool(WriteFilesTool::new(project_path_buf.clone()))
                    .tool(ShellTool::new(project_path_buf.clone()));
            }

            if let Some(params) = reasoning_params {
                builder = builder.additional_params(params);
            }

            let agent = builder.build();

            agent
                .prompt(query)
                .multi_turn(50)
                .await
                .map_err(|e| AgentError::ProviderError(e.to_string()))
        }
        ProviderType::Anthropic => {
            let client = anthropic::Client::from_env();
            let model_name = model.as_deref().unwrap_or("claude-sonnet-4-20250514");

            let mut builder = client
                .agent(model_name)
                .preamble(&preamble)
                .max_tokens(4096)
                .tool(AnalyzeTool::new(project_path_buf.clone()))
                .tool(SecurityScanTool::new(project_path_buf.clone()))
                .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                .tool(HadolintTool::new(project_path_buf.clone()))
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf.clone()));

            // Add generation tools if this is a generation query
            if is_generation {
                builder = builder
                    .tool(WriteFileTool::new(project_path_buf.clone()))
                    .tool(WriteFilesTool::new(project_path_buf.clone()))
                    .tool(ShellTool::new(project_path_buf.clone()));
            }

            let agent = builder.build();

            agent
                .prompt(query)
                .multi_turn(50)
                .await
                .map_err(|e| AgentError::ProviderError(e.to_string()))
        }
    }
}
