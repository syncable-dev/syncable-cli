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
pub mod compact;
pub mod history;
pub mod ide;
pub mod persistence;
pub mod prompts;
pub mod session;
pub mod tools;
pub mod ui;
use colored::Colorize;
use commands::TokenUsage;
use history::{ConversationHistory, ToolCallRecord};
use ide::IdeClient;
use rig::{
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::{anthropic, openai},
};
use session::{ChatSession, PlanMode};
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
    Bedrock,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::Bedrock => write!(f, "bedrock"),
        }
    }
}

impl std::str::FromStr for ProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(ProviderType::OpenAI),
            "anthropic" => Ok(ProviderType::Anthropic),
            "bedrock" | "aws" | "aws-bedrock" => Ok(ProviderType::Bedrock),
            _ => Err(format!(
                "Unknown provider: {}. Use: openai, anthropic, or bedrock",
                s
            )),
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

/// Get the system prompt for the agent based on query type and plan mode
fn get_system_prompt(project_path: &Path, query: Option<&str>, plan_mode: PlanMode) -> String {
    // In planning mode, use the read-only exploration prompt
    if plan_mode.is_planning() {
        return prompts::get_planning_prompt(project_path);
    }

    if let Some(q) = query {
        // First check if it's a code development task (highest priority)
        if prompts::is_code_development_query(q) {
            return prompts::get_code_development_prompt(project_path);
        }
        // Then check if it's DevOps generation (Docker, Terraform, Helm)
        if prompts::is_generation_query(q) {
            return prompts::get_devops_prompt(project_path, Some(q));
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

    // Terminal layout for split screen is disabled for now - see notes below
    // let terminal_layout = ui::TerminalLayout::new();
    // let layout_state = terminal_layout.state();

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
                    println!("{} IDE companion not connected: {}", "!".yellow(), e);
                    None
                }
            }
        } else {
            println!(
                "{} No IDE detected (TERM_PROGRAM={})",
                "·".dimmed(),
                std::env::var("TERM_PROGRAM").unwrap_or_default()
            );
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

    // NOTE: Terminal layout with ANSI scroll regions is disabled for now.
    // The scroll region approach conflicts with the existing input/output flow.
    // TODO: Implement proper scroll region support that integrates with the input handler.
    // For now, we rely on the pause/resume mechanism in progress indicator.
    //
    // if let Err(e) = terminal_layout.init() {
    //     eprintln!(
    //         "{}",
    //         format!("Note: Terminal layout initialization failed: {}. Using fallback mode.", e)
    //             .dimmed()
    //     );
    // }

    // Raw Rig messages for multi-turn - preserves Reasoning blocks for thinking
    // Our ConversationHistory only stores text summaries, but rig needs full Message structure
    let mut raw_chat_history: Vec<rig::completion::Message> = Vec::new();

    // Pending input for auto-continue after plan creation
    let mut pending_input: Option<String> = None;
    // Auto-accept mode for plan execution (skips write confirmations)
    let mut auto_accept_writes = false;

    // Initialize session recorder for conversation persistence
    let mut session_recorder = persistence::SessionRecorder::new(project_path);

    loop {
        // Show conversation status if we have history
        if !conversation_history.is_empty() {
            println!(
                "{}",
                format!("  💬 Context: {}", conversation_history.status()).dimmed()
            );
        }

        // Check for pending input (from plan menu selection)
        let input = if let Some(pending) = pending_input.take() {
            // Show what we're executing
            println!("{} {}", "→".cyan(), pending.dimmed());
            pending
        } else {
            // New user turn - reset auto-accept mode from previous plan execution
            auto_accept_writes = false;

            // Read user input (returns InputResult)
            let input_result = match session.read_input() {
                Ok(result) => result,
                Err(_) => break,
            };

            // Handle the input result
            match input_result {
                ui::InputResult::Submit(text) => ChatSession::process_submitted_text(&text),
                ui::InputResult::Cancel | ui::InputResult::Exit => break,
                ui::InputResult::TogglePlanMode => {
                    // Toggle planning mode - minimal feedback, no extra newlines
                    let new_mode = session.toggle_plan_mode();
                    if new_mode.is_planning() {
                        println!("{}", "★ plan mode".yellow());
                    } else {
                        println!("{}", "▶ standard mode".green());
                    }
                    continue;
                }
            }
        };

        if input.is_empty() {
            continue;
        }

        // Check for commands
        if ChatSession::is_command(&input) {
            // Special handling for /clear to also clear conversation history
            if input.trim().to_lowercase() == "/clear" || input.trim().to_lowercase() == "/c" {
                conversation_history.clear();
                raw_chat_history.clear();
            }
            match session.process_command(&input) {
                Ok(true) => {
                    // Check if /resume loaded a session
                    if let Some(record) = session.pending_resume.take() {
                        // Display previous messages
                        println!();
                        println!("{}", "─── Previous Conversation ───".dimmed());
                        for msg in &record.messages {
                            match msg.role {
                                persistence::MessageRole::User => {
                                    println!();
                                    println!(
                                        "{} {}",
                                        "You:".cyan().bold(),
                                        truncate_string(&msg.content, 500)
                                    );
                                }
                                persistence::MessageRole::Assistant => {
                                    println!();
                                    // Show tool calls if any (same format as live display)
                                    if let Some(ref tools) = msg.tool_calls {
                                        for tc in tools {
                                            // Match live tool display: green dot for completed, cyan bold name
                                            if tc.args_summary.is_empty() {
                                                println!(
                                                    "{} {}",
                                                    "●".green(),
                                                    tc.name.cyan().bold()
                                                );
                                            } else {
                                                println!(
                                                    "{} {}({})",
                                                    "●".green(),
                                                    tc.name.cyan().bold(),
                                                    truncate_string(&tc.args_summary, 50).dimmed()
                                                );
                                            }
                                        }
                                    }
                                    // Show response (same ResponseFormatter as live)
                                    if !msg.content.is_empty() {
                                        ResponseFormatter::print_response(&truncate_string(
                                            &msg.content,
                                            1000,
                                        ));
                                    }
                                }
                                persistence::MessageRole::System => {
                                    // Skip system messages in display
                                }
                            }
                        }
                        println!("{}", "─── End of History ───".dimmed());
                        println!();

                        // Load messages into raw_chat_history for AI context
                        for msg in &record.messages {
                            match msg.role {
                                persistence::MessageRole::User => {
                                    raw_chat_history.push(rig::completion::Message::User {
                                        content: rig::one_or_many::OneOrMany::one(
                                            rig::completion::message::UserContent::text(
                                                &msg.content,
                                            ),
                                        ),
                                    });
                                }
                                persistence::MessageRole::Assistant => {
                                    raw_chat_history.push(rig::completion::Message::Assistant {
                                        id: Some(msg.id.clone()),
                                        content: rig::one_or_many::OneOrMany::one(
                                            rig::completion::message::AssistantContent::text(
                                                &msg.content,
                                            ),
                                        ),
                                    });
                                }
                                persistence::MessageRole::System => {}
                            }
                        }

                        // Load into conversation_history for context tracking
                        for msg in &record.messages {
                            if msg.role == persistence::MessageRole::User {
                                // Find the next assistant message
                                let response = record
                                    .messages
                                    .iter()
                                    .skip_while(|m| m.id != msg.id)
                                    .skip(1)
                                    .find(|m| m.role == persistence::MessageRole::Assistant)
                                    .map(|m| m.content.clone())
                                    .unwrap_or_default();

                                conversation_history.add_turn(
                                    msg.content.clone(),
                                    response,
                                    vec![], // Tool calls not loaded for simplicity
                                );
                            }
                        }

                        println!(
                            "{}",
                            format!(
                                "  ✓ Loaded {} messages. You can now continue the conversation.",
                                record.messages.len()
                            )
                            .green()
                        );
                        println!();
                    }
                    continue;
                }
                Ok(false) => break, // /exit
                Err(e) => {
                    eprintln!("{}", format!("Error: {}", e).red());
                    continue;
                }
            }
        }

        // Check API key before making request (in case provider changed)
        if !ChatSession::has_api_key(session.provider) {
            eprintln!(
                "{}",
                "No API key configured. Use /provider to set one.".yellow()
            );
            continue;
        }

        // Check if compaction is needed before making the request
        if conversation_history.needs_compaction() {
            println!("{}", "  📦 Compacting conversation history...".dimmed());
            if let Some(summary) = conversation_history.compact() {
                println!(
                    "{}",
                    format!("  ✓ Compressed {} turns", summary.matches("Turn").count()).dimmed()
                );
            }
        }

        // Pre-request check: estimate if we're approaching context limit
        // Check raw_chat_history (actual messages) not conversation_history
        // because conversation_history may be out of sync
        let estimated_input_tokens = estimate_raw_history_tokens(&raw_chat_history)
            + input.len() / 4  // New input
            + 5000; // System prompt overhead estimate

        if estimated_input_tokens > 150_000 {
            println!(
                "{}",
                "  ⚠ Large context detected. Pre-truncating...".yellow()
            );

            let old_count = raw_chat_history.len();
            // Keep last 20 messages when approaching limit
            if raw_chat_history.len() > 20 {
                let drain_count = raw_chat_history.len() - 20;
                raw_chat_history.drain(0..drain_count);
                conversation_history.clear(); // Stay in sync
                println!(
                    "{}",
                    format!(
                        "  ✓ Truncated {} → {} messages",
                        old_count,
                        raw_chat_history.len()
                    )
                    .dimmed()
                );
            }
        }

        // Retry loop for automatic error recovery
        // MAX_RETRIES is for failures without progress
        // MAX_CONTINUATIONS is for truncations WITH progress (more generous)
        // TOOL_CALL_CHECKPOINT is the interval at which we ask user to confirm
        // MAX_TOOL_CALLS is the absolute maximum (300 = 6 checkpoints x 50)
        const MAX_RETRIES: u32 = 3;
        const MAX_CONTINUATIONS: u32 = 10;
        const _TOOL_CALL_CHECKPOINT: usize = 50;
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
                eprintln!("{}", "  📡 Sending continuation request...".dimmed());
            }

            // Create hook for Claude Code style tool display
            let hook = ToolDisplayHook::new();

            // Create progress indicator for visual feedback during generation
            let progress = ui::GenerationIndicator::new();
            // Layout connection disabled - using inline progress mode
            // progress.state().set_layout(layout_state.clone());
            hook.set_progress_state(progress.state()).await;

            let project_path_buf = session.project_path.clone();
            // Select prompt based on query type (analysis vs generation) and plan mode
            let preamble = get_system_prompt(
                &session.project_path,
                Some(&current_input),
                session.plan_mode,
            );
            let is_generation = prompts::is_generation_query(&current_input);
            let is_planning = session.plan_mode.is_planning();

            // Note: using raw_chat_history directly which preserves Reasoning blocks
            // This is needed for extended thinking to work with multi-turn conversations

            // Get progress state for interrupt detection
            let progress_state = progress.state();

            // Use tokio::select! to race the API call against Ctrl+C
            // This allows immediate cancellation, not just between tool calls
            let mut user_interrupted = false;

            // API call with Ctrl+C interrupt support
            let response = tokio::select! {
                biased; // Check ctrl_c first for faster response

                _ = tokio::signal::ctrl_c() => {
                    user_interrupted = true;
                    Err::<String, String>("User cancelled".to_string())
                }

                result = async {
                    match session.provider {
                ProviderType::OpenAI => {
                    let client = openai::Client::from_env();
                    // For GPT-5.x reasoning models, enable reasoning with summary output
                    // so we can see the model's thinking process
                    let reasoning_params =
                        if session.model.starts_with("gpt-5") || session.model.starts_with("o1") {
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
                        .tool(DclintTool::new(project_path_buf.clone()))
                        .tool(KubelintTool::new(project_path_buf.clone()))
                        .tool(HelmlintTool::new(project_path_buf.clone()))
                        .tool(TerraformFmtTool::new(project_path_buf.clone()))
                        .tool(TerraformValidateTool::new(project_path_buf.clone()))
                        .tool(TerraformInstallTool::new())
                        .tool(ReadFileTool::new(project_path_buf.clone()))
                        .tool(ListDirectoryTool::new(project_path_buf.clone()))
                        .tool(WebFetchTool::new());

                    // Add tools based on mode
                    if is_planning {
                        // Plan mode: read-only shell + plan creation tools
                        builder = builder
                            .tool(ShellTool::new(project_path_buf.clone()).with_read_only(true))
                            .tool(PlanCreateTool::new(project_path_buf.clone()))
                            .tool(PlanListTool::new(project_path_buf.clone()));
                    } else if is_generation {
                        // Standard mode + generation query: all tools including file writes and plan execution
                        let (mut write_file_tool, mut write_files_tool) =
                            if let Some(ref client) = ide_client {
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
                        // Disable confirmations if auto-accept mode is enabled (from plan menu)
                        if auto_accept_writes {
                            write_file_tool = write_file_tool.without_confirmation();
                            write_files_tool = write_files_tool.without_confirmation();
                        }
                        builder = builder
                            .tool(write_file_tool)
                            .tool(write_files_tool)
                            .tool(ShellTool::new(project_path_buf.clone()))
                            .tool(PlanListTool::new(project_path_buf.clone()))
                            .tool(PlanNextTool::new(project_path_buf.clone()))
                            .tool(PlanUpdateTool::new(project_path_buf.clone()));
                    }

                    if let Some(params) = reasoning_params {
                        builder = builder.additional_params(params);
                    }

                    let agent = builder.build();
                    // Allow up to 50 tool call turns for complex generation tasks
                    // Use hook to display tool calls as they happen
                    // Pass conversation history for context continuity
                    agent
                        .prompt(&current_input)
                        .with_history(&mut raw_chat_history)
                        .with_hook(hook.clone())
                        .multi_turn(50)
                        .await
                }
                ProviderType::Anthropic => {
                    let client = anthropic::Client::from_env();

                    // TODO: Extended thinking for Claude is disabled because rig-bedrock/rig-anthropic
                    // don't properly handle thinking blocks in multi-turn conversations with tool use.
                    // When thinking is enabled, ALL assistant messages must start with thinking blocks
                    // BEFORE tool_use blocks, but rig doesn't preserve/replay these.
                    // See: forge/crates/forge_services/src/provider/bedrock/provider.rs for reference impl.

                    let mut builder = client
                        .agent(&session.model)
                        .preamble(&preamble)
                        .max_tokens(4096)
                        .tool(AnalyzeTool::new(project_path_buf.clone()))
                        .tool(SecurityScanTool::new(project_path_buf.clone()))
                        .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                        .tool(HadolintTool::new(project_path_buf.clone()))
                        .tool(DclintTool::new(project_path_buf.clone()))
                        .tool(KubelintTool::new(project_path_buf.clone()))
                        .tool(HelmlintTool::new(project_path_buf.clone()))
                        .tool(TerraformFmtTool::new(project_path_buf.clone()))
                        .tool(TerraformValidateTool::new(project_path_buf.clone()))
                        .tool(TerraformInstallTool::new())
                        .tool(ReadFileTool::new(project_path_buf.clone()))
                        .tool(ListDirectoryTool::new(project_path_buf.clone()))
                        .tool(WebFetchTool::new());

                    // Add tools based on mode
                    if is_planning {
                        // Plan mode: read-only shell + plan creation tools
                        builder = builder
                            .tool(ShellTool::new(project_path_buf.clone()).with_read_only(true))
                            .tool(PlanCreateTool::new(project_path_buf.clone()))
                            .tool(PlanListTool::new(project_path_buf.clone()));
                    } else if is_generation {
                        // Standard mode + generation query: all tools including file writes and plan execution
                        let (mut write_file_tool, mut write_files_tool) =
                            if let Some(ref client) = ide_client {
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
                        // Disable confirmations if auto-accept mode is enabled (from plan menu)
                        if auto_accept_writes {
                            write_file_tool = write_file_tool.without_confirmation();
                            write_files_tool = write_files_tool.without_confirmation();
                        }
                        builder = builder
                            .tool(write_file_tool)
                            .tool(write_files_tool)
                            .tool(ShellTool::new(project_path_buf.clone()))
                            .tool(PlanListTool::new(project_path_buf.clone()))
                            .tool(PlanNextTool::new(project_path_buf.clone()))
                            .tool(PlanUpdateTool::new(project_path_buf.clone()));
                    }

                    let agent = builder.build();

                    // Allow up to 50 tool call turns for complex generation tasks
                    // Use hook to display tool calls as they happen
                    // Pass conversation history for context continuity
                    agent
                        .prompt(&current_input)
                        .with_history(&mut raw_chat_history)
                        .with_hook(hook.clone())
                        .multi_turn(50)
                        .await
                }
                ProviderType::Bedrock => {
                    // Bedrock provider via rig-bedrock - same pattern as OpenAI/Anthropic
                    let client = crate::bedrock::client::Client::from_env();

                    // Extended thinking for Claude models via Bedrock
                    // This enables Claude to show its reasoning process before responding.
                    // Requires vendored rig-bedrock that preserves Reasoning blocks with tool calls.
                    // Extended thinking budget - reduced to help with rate limits
                    // 8000 is enough for most tasks, increase to 16000 for complex analysis
                    let thinking_params = serde_json::json!({
                        "thinking": {
                            "type": "enabled",
                            "budget_tokens": 8000
                        }
                    });

                    let mut builder = client
                        .agent(&session.model)
                        .preamble(&preamble)
                        .max_tokens(64000)  // Max output tokens for Claude Sonnet on Bedrock
                        .tool(AnalyzeTool::new(project_path_buf.clone()))
                        .tool(SecurityScanTool::new(project_path_buf.clone()))
                        .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                        .tool(HadolintTool::new(project_path_buf.clone()))
                        .tool(DclintTool::new(project_path_buf.clone()))
                        .tool(KubelintTool::new(project_path_buf.clone()))
                        .tool(HelmlintTool::new(project_path_buf.clone()))
                        .tool(TerraformFmtTool::new(project_path_buf.clone()))
                        .tool(TerraformValidateTool::new(project_path_buf.clone()))
                        .tool(TerraformInstallTool::new())
                        .tool(ReadFileTool::new(project_path_buf.clone()))
                        .tool(ListDirectoryTool::new(project_path_buf.clone()))
                        .tool(WebFetchTool::new());

                    // Add tools based on mode
                    if is_planning {
                        // Plan mode: read-only shell + plan creation tools
                        builder = builder
                            .tool(ShellTool::new(project_path_buf.clone()).with_read_only(true))
                            .tool(PlanCreateTool::new(project_path_buf.clone()))
                            .tool(PlanListTool::new(project_path_buf.clone()));
                    } else if is_generation {
                        // Standard mode + generation query: all tools including file writes and plan execution
                        let (mut write_file_tool, mut write_files_tool) =
                            if let Some(ref client) = ide_client {
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
                        // Disable confirmations if auto-accept mode is enabled (from plan menu)
                        if auto_accept_writes {
                            write_file_tool = write_file_tool.without_confirmation();
                            write_files_tool = write_files_tool.without_confirmation();
                        }
                        builder = builder
                            .tool(write_file_tool)
                            .tool(write_files_tool)
                            .tool(ShellTool::new(project_path_buf.clone()))
                            .tool(PlanListTool::new(project_path_buf.clone()))
                            .tool(PlanNextTool::new(project_path_buf.clone()))
                            .tool(PlanUpdateTool::new(project_path_buf.clone()));
                    }

                    // Add thinking params for extended reasoning
                    builder = builder.additional_params(thinking_params);

                    let agent = builder.build();

                    // Use same multi-turn pattern as OpenAI/Anthropic
                    agent
                        .prompt(&current_input)
                        .with_history(&mut raw_chat_history)
                        .with_hook(hook.clone())
                        .multi_turn(50)
                        .await
                    }
                }.map_err(|e| e.to_string())
            } => result
            };

            // Stop the progress indicator before handling the response
            progress.stop().await;

            // Suppress unused variable warnings
            let _ = (&progress_state, user_interrupted);

            match response {
                Ok(text) => {
                    // Show final response
                    println!();
                    ResponseFormatter::print_response(&text);

                    // Track token usage - use actual from hook if available, else estimate
                    let hook_usage = hook.get_usage().await;
                    if hook_usage.has_data() {
                        // Use actual token counts from API response
                        session
                            .token_usage
                            .add_actual(hook_usage.input_tokens, hook_usage.output_tokens);
                    } else {
                        // Fall back to estimation when API doesn't provide usage
                        let prompt_tokens = TokenUsage::estimate_tokens(&input);
                        let completion_tokens = TokenUsage::estimate_tokens(&text);
                        session
                            .token_usage
                            .add_estimated(prompt_tokens, completion_tokens);
                    }
                    // Reset hook usage for next request batch
                    hook.reset_usage().await;

                    // Show context indicator like Forge: [model/~tokens]
                    let model_short = session
                        .model
                        .split('/')
                        .next_back()
                        .unwrap_or(&session.model)
                        .split(':')
                        .next()
                        .unwrap_or(&session.model);
                    println!();
                    println!(
                        "  {}[{}/{}]{}",
                        ui::colors::ansi::DIM,
                        model_short,
                        session.token_usage.format_compact(),
                        ui::colors::ansi::RESET
                    );

                    // Extract tool calls from the hook state for history tracking
                    let tool_calls = extract_tool_calls_from_hook(&hook).await;
                    let batch_tool_count = tool_calls.len();
                    total_tool_calls += batch_tool_count;

                    // Show tool call summary if significant
                    if batch_tool_count > 10 {
                        println!(
                            "{}",
                            format!(
                                "  ✓ Completed with {} tool calls ({} total this session)",
                                batch_tool_count, total_tool_calls
                            )
                            .dimmed()
                        );
                    }

                    // Add to conversation history with tool call records
                    conversation_history.add_turn(input.clone(), text.clone(), tool_calls.clone());

                    // Check if this heavy turn requires immediate compaction
                    // This helps prevent context overflow in subsequent requests
                    if conversation_history.needs_compaction() {
                        println!("{}", "  📦 Compacting conversation history...".dimmed());
                        if let Some(summary) = conversation_history.compact() {
                            println!(
                                "{}",
                                format!("  ✓ Compressed {} turns", summary.matches("Turn").count())
                                    .dimmed()
                            );
                        }
                    }

                    // Also update legacy session history for compatibility
                    session.history.push(("user".to_string(), input.clone()));
                    session
                        .history
                        .push(("assistant".to_string(), text.clone()));

                    // Record to persistent session storage
                    session_recorder.record_user_message(&input);
                    session_recorder.record_assistant_message(&text, Some(&tool_calls));
                    if let Err(e) = session_recorder.save() {
                        eprintln!(
                            "{}",
                            format!("  Warning: Failed to save session: {}", e).dimmed()
                        );
                    }

                    // Check if plan_create was called - show interactive menu
                    if let Some(plan_info) = find_plan_create_call(&tool_calls) {
                        println!(); // Space before menu

                        // Show the plan action menu (don't switch modes yet - let user choose)
                        match ui::show_plan_action_menu(&plan_info.0, plan_info.1) {
                            ui::PlanActionResult::ExecuteAutoAccept => {
                                // Now switch to standard mode for execution
                                if session.plan_mode.is_planning() {
                                    session.plan_mode = session.plan_mode.toggle();
                                }
                                auto_accept_writes = true;
                                pending_input = Some(format!(
                                    "Execute the plan at '{}'. Use plan_next to get tasks and execute them in order. Auto-accept all file writes.",
                                    plan_info.0
                                ));
                                succeeded = true;
                            }
                            ui::PlanActionResult::ExecuteWithReview => {
                                // Now switch to standard mode for execution
                                if session.plan_mode.is_planning() {
                                    session.plan_mode = session.plan_mode.toggle();
                                }
                                pending_input = Some(format!(
                                    "Execute the plan at '{}'. Use plan_next to get tasks and execute them in order.",
                                    plan_info.0
                                ));
                                succeeded = true;
                            }
                            ui::PlanActionResult::ChangePlan(feedback) => {
                                // Stay in plan mode for modifications
                                pending_input = Some(format!(
                                    "Please modify the plan at '{}'. User feedback: {}",
                                    plan_info.0, feedback
                                ));
                                succeeded = true;
                            }
                            ui::PlanActionResult::Cancel => {
                                // Just complete normally, don't execute
                                succeeded = true;
                            }
                        }
                    } else {
                        succeeded = true;
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();

                    println!();

                    // Check if this was a user-initiated cancellation (Ctrl+C)
                    if err_str.contains("cancelled") || err_str.contains("Cancelled") {
                        // Extract any completed work before cancellation
                        let completed_tools = extract_tool_calls_from_hook(&hook).await;
                        let tool_count = completed_tools.len();

                        eprintln!("{}", "⚠ Generation interrupted.".yellow());
                        if tool_count > 0 {
                            eprintln!(
                                "{}",
                                format!("  {} tool calls completed before interrupt.", tool_count)
                                    .dimmed()
                            );
                            // Add partial progress to history
                            conversation_history.add_turn(
                                current_input.clone(),
                                format!("[Interrupted after {} tool calls]", tool_count),
                                completed_tools,
                            );
                        }
                        eprintln!("{}", "  Type your next message to continue.".dimmed());

                        // Don't retry, don't mark as succeeded - just break to return to prompt
                        break;
                    }

                    // Check if this is a max depth error - handle as checkpoint
                    if err_str.contains("MaxDepth")
                        || err_str.contains("max_depth")
                        || err_str.contains("reached limit")
                    {
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
                            eprintln!(
                                "{}",
                                format!("Maximum tool call limit ({}) reached.", MAX_TOOL_CALLS)
                                    .red()
                            );
                            eprintln!(
                                "{}",
                                "The task is too complex. Try breaking it into smaller parts."
                                    .dimmed()
                            );
                            break;
                        }

                        // Ask user if they want to continue (unless auto-continue is enabled)
                        let should_continue = if auto_continue_tools {
                            eprintln!(
                                "{}",
                                "  Auto-continuing (you selected 'always')...".dimmed()
                            );
                            true
                        } else {
                            eprintln!(
                                "{}",
                                "Excessive tool calls used. Want to continue?".yellow()
                            );
                            eprintln!(
                                "{}",
                                "  [y] Yes, continue  [n] No, stop  [a] Always continue".dimmed()
                            );
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
                            eprintln!(
                                "{}",
                                "Stopped by user. Type 'continue' to resume later.".dimmed()
                            );
                            // Add partial progress to history
                            if !completed_tools.is_empty() {
                                conversation_history.add_turn(
                                    current_input.clone(),
                                    format!(
                                        "[Stopped at checkpoint - {} tools completed]",
                                        batch_tool_count
                                    ),
                                    vec![],
                                );
                            }
                            break;
                        }

                        // Continue from checkpoint
                        eprintln!(
                            "{}",
                            format!(
                                "  → Continuing... {} remaining tool calls available",
                                MAX_TOOL_CALLS - total_tool_calls
                            )
                            .dimmed()
                        );

                        // Add partial progress to history (without duplicating tool calls)
                        conversation_history.add_turn(
                            current_input.clone(),
                            format!(
                                "[Checkpoint - {} tools completed, continuing...]",
                                batch_tool_count
                            ),
                            vec![],
                        );

                        // Build continuation prompt
                        current_input =
                            build_continuation_prompt(&input, &completed_tools, &agent_thinking);

                        // Brief delay before continuation
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        continue; // Continue the loop without incrementing retry_attempt
                    } else if err_str.contains("rate")
                        || err_str.contains("Rate")
                        || err_str.contains("429")
                        || err_str.contains("Too many tokens")
                        || err_str.contains("please wait")
                        || err_str.contains("throttl")
                        || err_str.contains("Throttl")
                    {
                        eprintln!("{}", "⚠ Rate limited by API provider.".yellow());
                        // Wait before retry for rate limits (longer wait for "too many tokens")
                        retry_attempt += 1;
                        let wait_secs = if err_str.contains("Too many tokens") {
                            30
                        } else {
                            5
                        };
                        eprintln!(
                            "{}",
                            format!(
                                "  Waiting {} seconds before retry ({}/{})...",
                                wait_secs, retry_attempt, MAX_RETRIES
                            )
                            .dimmed()
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;
                    } else if is_input_too_long_error(&err_str) {
                        // Context too large - truncate raw_chat_history directly
                        // NOTE: We truncate raw_chat_history (actual messages) not conversation_history
                        // because conversation_history may be empty/stale during errors
                        eprintln!(
                            "{}",
                            "⚠ Context too large for model. Truncating history...".yellow()
                        );

                        let old_token_count = estimate_raw_history_tokens(&raw_chat_history);
                        let old_msg_count = raw_chat_history.len();

                        // Strategy: Keep only the last N messages (user/assistant pairs)
                        // More aggressive truncation on each retry: 10 → 6 → 4 messages
                        let keep_count = match retry_attempt {
                            0 => 10,
                            1 => 6,
                            _ => 4,
                        };

                        if raw_chat_history.len() > keep_count {
                            // Drain older messages, keep the most recent ones
                            let drain_count = raw_chat_history.len() - keep_count;
                            raw_chat_history.drain(0..drain_count);
                        }

                        let new_token_count = estimate_raw_history_tokens(&raw_chat_history);
                        eprintln!("{}", format!(
                            "  ✓ Truncated: {} messages (~{} tokens) → {} messages (~{} tokens)",
                            old_msg_count, old_token_count, raw_chat_history.len(), new_token_count
                        ).green());

                        // Also clear conversation_history to stay in sync
                        conversation_history.clear();

                        // Retry with truncated context
                        retry_attempt += 1;
                        if retry_attempt < MAX_RETRIES {
                            eprintln!(
                                "{}",
                                format!(
                                    "  → Retrying with truncated context ({}/{})...",
                                    retry_attempt, MAX_RETRIES
                                )
                                .dimmed()
                            );
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        } else {
                            eprintln!(
                                "{}",
                                "Context still too large after truncation. Try /clear to reset."
                                    .red()
                            );
                            break;
                        }
                    } else if is_truncation_error(&err_str) {
                        // Truncation error - try intelligent continuation
                        let completed_tools = extract_tool_calls_from_hook(&hook).await;
                        let agent_thinking = extract_agent_messages_from_hook(&hook).await;

                        // Count actually completed tools (not in-progress)
                        let completed_count = completed_tools
                            .iter()
                            .filter(|t| !t.result_summary.contains("IN PROGRESS"))
                            .count();
                        let in_progress_count = completed_tools.len() - completed_count;

                        if !completed_tools.is_empty() && continuation_count < MAX_CONTINUATIONS {
                            // We have partial progress - continue from where we left off
                            continuation_count += 1;
                            let status_msg = if in_progress_count > 0 {
                                format!(
                                    "⚠ Response truncated. {} completed, {} in-progress. Auto-continuing ({}/{})...",
                                    completed_count,
                                    in_progress_count,
                                    continuation_count,
                                    MAX_CONTINUATIONS
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
                                eprintln!(
                                    "{}",
                                    "  📦 Compacting history before continuation...".dimmed()
                                );
                                if let Some(summary) = conversation_history.compact() {
                                    eprintln!(
                                        "{}",
                                        format!(
                                            "  ✓ Compressed {} turns",
                                            summary.matches("Turn").count()
                                        )
                                        .dimmed()
                                    );
                                }
                            }

                            // Build continuation prompt with context
                            current_input = build_continuation_prompt(
                                &input,
                                &completed_tools,
                                &agent_thinking,
                            );

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
                            eprintln!(
                                "{}",
                                format!(
                                    "⚠ Response error (attempt {}/{}). Retrying...",
                                    retry_attempt, MAX_RETRIES
                                )
                                .yellow()
                            );
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        } else {
                            // Max retries/continuations reached
                            eprintln!("{}", format!("Error: {}", e).red());
                            if continuation_count >= MAX_CONTINUATIONS {
                                eprintln!("{}", format!("Max continuations ({}) reached. The task is too complex for one request.", MAX_CONTINUATIONS).dimmed());
                            } else {
                                eprintln!(
                                    "{}",
                                    "Max retries reached. The response may be too complex."
                                        .dimmed()
                                );
                            }
                            eprintln!(
                                "{}",
                                "Try breaking your request into smaller parts.".dimmed()
                            );
                            break;
                        }
                    } else if err_str.contains("timeout") || err_str.contains("Timeout") {
                        // Timeout - simple retry
                        retry_attempt += 1;
                        if retry_attempt < MAX_RETRIES {
                            eprintln!(
                                "{}",
                                format!(
                                    "⚠ Request timed out (attempt {}/{}). Retrying...",
                                    retry_attempt, MAX_RETRIES
                                )
                                .yellow()
                            );
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        } else {
                            eprintln!("{}", "Request timed out. Please try again.".red());
                            break;
                        }
                    } else {
                        // Unknown error - show details and break
                        eprintln!("{}", format!("Error: {}", e).red());
                        if continuation_count > 0 {
                            eprintln!(
                                "{}",
                                format!(
                                    "  (occurred during continuation attempt {})",
                                    continuation_count
                                )
                                .dimmed()
                            );
                        }
                        eprintln!("{}", "Error details for debugging:".dimmed());
                        eprintln!(
                            "{}",
                            format!("  - retry_attempt: {}/{}", retry_attempt, MAX_RETRIES)
                                .dimmed()
                        );
                        eprintln!(
                            "{}",
                            format!(
                                "  - continuation_count: {}/{}",
                                continuation_count, MAX_CONTINUATIONS
                            )
                            .dimmed()
                        );
                        break;
                    }
                }
            }
        }
        println!();
    }

    // Clean up terminal layout before exiting (disabled - layout not initialized)
    // if let Err(e) = terminal_layout.cleanup() {
    //     eprintln!(
    //         "{}",
    //         format!("Warning: Terminal cleanup failed: {}", e).dimmed()
    //     );
    // }

    Ok(())
}

// NOTE: wait_for_interrupt function removed - ESC interrupt feature disabled
// due to terminal corruption issues with spawn_blocking raw mode handling.
// TODO: Re-implement using tool hook callbacks for cleaner interruption.

/// Extract tool call records from the hook state for history tracking
async fn extract_tool_calls_from_hook(hook: &ToolDisplayHook) -> Vec<ToolCallRecord> {
    let state = hook.state();
    let guard = state.lock().await;

    guard
        .tool_calls
        .iter()
        .enumerate()
        .map(|(i, tc)| {
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
                // Mark read-only tools as droppable (their results can be re-fetched)
                droppable: matches!(
                    tc.name.as_str(),
                    "read_file" | "list_directory" | "analyze_project"
                ),
            }
        })
        .collect()
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

/// Estimate token count from raw rig Messages
/// This is used for context length management to prevent "input too long" errors.
/// Estimates ~4 characters per token.
fn estimate_raw_history_tokens(messages: &[rig::completion::Message]) -> usize {
    use rig::completion::message::{AssistantContent, UserContent};

    messages
        .iter()
        .map(|msg| -> usize {
            match msg {
                rig::completion::Message::User { content } => {
                    content
                        .iter()
                        .map(|c| -> usize {
                            match c {
                                UserContent::Text(t) => t.text.len() / 4,
                                _ => 100, // Estimate for images/documents
                            }
                        })
                        .sum::<usize>()
                }
                rig::completion::Message::Assistant { content, .. } => {
                    content
                        .iter()
                        .map(|c| -> usize {
                            match c {
                                AssistantContent::Text(t) => t.text.len() / 4,
                                AssistantContent::ToolCall(tc) => {
                                    // arguments is serde_json::Value, convert to string for length estimate
                                    let args_len = tc.function.arguments.to_string().len();
                                    (tc.function.name.len() + args_len) / 4
                                }
                                _ => 100,
                            }
                        })
                        .sum::<usize>()
                }
            }
        })
        .sum()
}

/// Find a plan_create tool call in the list and extract plan info
/// Returns (plan_path, task_count) if found
fn find_plan_create_call(tool_calls: &[ToolCallRecord]) -> Option<(String, usize)> {
    for tc in tool_calls {
        if tc.tool_name == "plan_create" {
            // Try to parse the result_summary as JSON to extract plan_path
            // Note: result_summary may be truncated, so we have multiple fallbacks
            let plan_path =
                if let Ok(result) = serde_json::from_str::<serde_json::Value>(&tc.result_summary) {
                    result
                        .get("plan_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                };

            // If JSON parsing failed, find the most recently created plan file
            // This is more reliable than trying to reconstruct the path from truncated args
            let plan_path = plan_path.unwrap_or_else(|| {
                find_most_recent_plan_file().unwrap_or_else(|| "plans/plan.md".to_string())
            });

            // Count tasks by reading the plan file directly
            let task_count = count_tasks_in_plan_file(&plan_path).unwrap_or(0);

            return Some((plan_path, task_count));
        }
    }
    None
}

/// Find the most recently created plan file in the plans directory
fn find_most_recent_plan_file() -> Option<String> {
    let plans_dir = std::env::current_dir().ok()?.join("plans");
    if !plans_dir.exists() {
        return None;
    }

    let mut newest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;

    for entry in std::fs::read_dir(&plans_dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "md")
            && let Ok(metadata) = entry.metadata()
            && let Ok(modified) = metadata.modified()
            && newest.as_ref().map(|(_, t)| modified > *t).unwrap_or(true)
        {
            newest = Some((path, modified));
        }
    }

    newest.map(|(path, _)| {
        // Return relative path
        path.strip_prefix(std::env::current_dir().unwrap_or_default())
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.display().to_string())
    })
}

/// Count tasks (checkbox items) in a plan file
fn count_tasks_in_plan_file(plan_path: &str) -> Option<usize> {
    use regex::Regex;

    // Try both relative and absolute paths
    let path = std::path::Path::new(plan_path);
    let content = if path.exists() {
        std::fs::read_to_string(path).ok()?
    } else {
        // Try with current directory
        std::fs::read_to_string(std::env::current_dir().ok()?.join(plan_path)).ok()?
    };

    // Count task checkboxes: - [ ], - [x], - [~], - [!]
    let task_regex = Regex::new(r"^\s*-\s*\[[ x~!]\]").ok()?;
    let count = content
        .lines()
        .filter(|line| task_regex.is_match(line))
        .count();

    Some(count)
}

/// Check if an error is a truncation/JSON parsing error that can be recovered via continuation
fn is_truncation_error(err_str: &str) -> bool {
    err_str.contains("JsonError")
        || err_str.contains("EOF while parsing")
        || err_str.contains("JSON")
        || err_str.contains("unexpected end")
}

/// Check if error is "input too long" - context exceeds model limit
/// This happens when conversation history grows beyond what the model can handle.
/// Recovery: compact history and retry with reduced context.
fn is_input_too_long_error(err_str: &str) -> bool {
    err_str.contains("too long")
        || err_str.contains("Too long")
        || err_str.contains("context length")
        || err_str.contains("maximum context")
        || err_str.contains("exceeds the model")
        || err_str.contains("Input is too long")
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
                other_tools.push(format!(
                    "{}({})",
                    tool.tool_name,
                    truncate_string(&tool.args_summary, 40)
                ));
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
    if let Some(last_thought) = agent_thinking.last() {
        prompt.push_str(&format!(
            "\n== YOUR LAST THOUGHTS ==\n\"{}\"\n",
            truncate_string(last_thought, 300)
        ));
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
    // For single queries (non-interactive), always use standard mode
    let preamble = get_system_prompt(project_path, Some(query), PlanMode::default());
    let is_generation = prompts::is_generation_query(query);

    match provider {
        ProviderType::OpenAI => {
            let client = openai::Client::from_env();
            let model_name = model.as_deref().unwrap_or("gpt-5.2");

            // For GPT-5.x reasoning models, enable reasoning with summary output
            let reasoning_params =
                if model_name.starts_with("gpt-5") || model_name.starts_with("o1") {
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
                .tool(DclintTool::new(project_path_buf.clone()))
                .tool(KubelintTool::new(project_path_buf.clone()))
                .tool(HelmlintTool::new(project_path_buf.clone()))
                .tool(TerraformFmtTool::new(project_path_buf.clone()))
                .tool(TerraformValidateTool::new(project_path_buf.clone()))
                .tool(TerraformInstallTool::new())
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf.clone()))
                .tool(WebFetchTool::new());

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
            let model_name = model.as_deref().unwrap_or("claude-sonnet-4-5-20250929");

            // TODO: Extended thinking for Claude is disabled because rig doesn't properly
            // handle thinking blocks in multi-turn conversations with tool use.
            // See: forge/crates/forge_services/src/provider/bedrock/provider.rs for reference.

            let mut builder = client
                .agent(model_name)
                .preamble(&preamble)
                .max_tokens(4096)
                .tool(AnalyzeTool::new(project_path_buf.clone()))
                .tool(SecurityScanTool::new(project_path_buf.clone()))
                .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                .tool(HadolintTool::new(project_path_buf.clone()))
                .tool(DclintTool::new(project_path_buf.clone()))
                .tool(KubelintTool::new(project_path_buf.clone()))
                .tool(HelmlintTool::new(project_path_buf.clone()))
                .tool(TerraformFmtTool::new(project_path_buf.clone()))
                .tool(TerraformValidateTool::new(project_path_buf.clone()))
                .tool(TerraformInstallTool::new())
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf.clone()))
                .tool(WebFetchTool::new());

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
        ProviderType::Bedrock => {
            // Bedrock provider via rig-bedrock - same pattern as Anthropic
            let client = crate::bedrock::client::Client::from_env();
            let model_name = model
                .as_deref()
                .unwrap_or("global.anthropic.claude-sonnet-4-5-20250929-v1:0");

            // Extended thinking for Claude via Bedrock
            let thinking_params = serde_json::json!({
                "thinking": {
                    "type": "enabled",
                    "budget_tokens": 16000
                }
            });

            let mut builder = client
                .agent(model_name)
                .preamble(&preamble)
                .max_tokens(64000)  // Max output tokens for Claude Sonnet on Bedrock
                .tool(AnalyzeTool::new(project_path_buf.clone()))
                .tool(SecurityScanTool::new(project_path_buf.clone()))
                .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                .tool(HadolintTool::new(project_path_buf.clone()))
                .tool(DclintTool::new(project_path_buf.clone()))
                .tool(KubelintTool::new(project_path_buf.clone()))
                .tool(HelmlintTool::new(project_path_buf.clone()))
                .tool(TerraformFmtTool::new(project_path_buf.clone()))
                .tool(TerraformValidateTool::new(project_path_buf.clone()))
                .tool(TerraformInstallTool::new())
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf.clone()))
                .tool(WebFetchTool::new());

            // Add generation tools if this is a generation query
            if is_generation {
                builder = builder
                    .tool(WriteFileTool::new(project_path_buf.clone()))
                    .tool(WriteFilesTool::new(project_path_buf.clone()))
                    .tool(ShellTool::new(project_path_buf.clone()));
            }

            let agent = builder.additional_params(thinking_params).build();

            agent
                .prompt(query)
                .multi_turn(50)
                .await
                .map_err(|e| AgentError::ProviderError(e.to_string()))
        }
    }
}
