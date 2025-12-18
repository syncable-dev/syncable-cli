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
    // If query suggests generation (Docker, Terraform, Helm), use DevOps prompt
    if let Some(q) = query {
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

        // Create hook for Claude Code style tool display
        let hook = ToolDisplayHook::new();

        let project_path_buf = session.project_path.clone();
        // Select prompt based on query type (analysis vs generation)
        let preamble = get_system_prompt(&session.project_path, Some(&input));
        let is_generation = prompts::is_generation_query(&input);

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
                agent.prompt(&input)
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
                agent.prompt(&input)
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

                // Add to conversation history with tool call records
                conversation_history.add_turn(input.clone(), text.clone(), tool_calls);

                // Also update legacy session history for compatibility
                session.history.push(("user".to_string(), input));
                session.history.push(("assistant".to_string(), text));
            }
            Err(e) => {
                let err_str = e.to_string();
                println!();
                // Check if this is a max depth error
                if err_str.contains("MaxDepth") || err_str.contains("max_depth") || err_str.contains("reached limit") {
                    eprintln!("{}", "Reached tool call limit (50 turns).".yellow());
                    eprintln!("{}", "Type 'continue' to resume, or ask a new question.".dimmed());
                } else {
                    eprintln!("{}", format!("Error: {}", e).red());
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

    guard.tool_calls.iter().map(|tc| {
        ToolCallRecord {
            tool_name: tc.name.clone(),
            args_summary: truncate_string(&tc.args, 100),
            result_summary: tc.output.as_ref()
                .map(|o| truncate_string(o, 200))
                .unwrap_or_else(|| "completed".to_string()),
        }
    }).collect()
}

/// Helper to truncate strings for summaries
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
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
