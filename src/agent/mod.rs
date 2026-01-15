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

    // Shared background process manager for Prometheus port-forwards
    let bg_manager = Arc::new(BackgroundProcessManager::new());

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

                        // Try to restore from history_snapshot (new format with full context)
                        let restored_from_snapshot = if let Some(history_json) =
                            &record.history_snapshot
                        {
                            match ConversationHistory::from_json(history_json) {
                                Ok(restored) => {
                                    conversation_history = restored;
                                    // Rebuild raw_chat_history from restored conversation_history
                                    raw_chat_history = conversation_history.to_messages();
                                    println!(
                                            "{}",
                                            "  ✓ Restored full conversation context (including compacted history)".green()
                                        );
                                    true
                                }
                                Err(e) => {
                                    eprintln!(
                                        "{}",
                                        format!(
                                            "  Warning: Failed to restore history snapshot: {}",
                                            e
                                        )
                                        .yellow()
                                    );
                                    false
                                }
                            }
                        } else {
                            false
                        };

                        // Fallback: Load from messages (old format or if snapshot failed)
                        if !restored_from_snapshot {
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
                                        raw_chat_history
                                            .push(rig::completion::Message::Assistant {
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

                            // Load into conversation_history with tool calls from message records
                            for msg in &record.messages {
                                if msg.role == persistence::MessageRole::User {
                                    // Find the next assistant message
                                    let (response, tool_calls) = record
                                        .messages
                                        .iter()
                                        .skip_while(|m| m.id != msg.id)
                                        .skip(1)
                                        .find(|m| m.role == persistence::MessageRole::Assistant)
                                        .map(|m| {
                                            let tcs = m.tool_calls.as_ref().map(|calls| {
                                                calls
                                                    .iter()
                                                    .map(|tc| history::ToolCallRecord {
                                                        tool_name: tc.name.clone(),
                                                        args_summary: tc.args_summary.clone(),
                                                        result_summary: tc.result_summary.clone(),
                                                        tool_id: None,
                                                        droppable: false,
                                                    })
                                                    .collect::<Vec<_>>()
                                            });
                                            (m.content.clone(), tcs.unwrap_or_default())
                                        })
                                        .unwrap_or_default();

                                    conversation_history.add_turn(
                                        msg.content.clone(),
                                        response,
                                        tool_calls,
                                    );
                                }
                            }
                            println!(
                                "{}",
                                format!(
                                    "  ✓ Loaded {} messages (legacy format).",
                                    record.messages.len()
                                )
                                .green()
                            );
                        }
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
                // Ensure history starts with User message for OpenAI Responses API compatibility
                ensure_history_starts_with_user(&mut raw_chat_history);
                // Preserve compacted summary while clearing turns to stay in sync
                conversation_history.clear_turns_preserve_context();
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
                    // Use Responses API (default) for reasoning model support.
                    // rig-core 0.28+ handles Reasoning items properly in multi-turn.
                    let client = openai::Client::from_env();

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
                        .tool(K8sOptimizeTool::new(project_path_buf.clone()))
                        .tool(K8sCostsTool::new(project_path_buf.clone()))
                        .tool(K8sDriftTool::new(project_path_buf.clone()))
                        .tool(HelmlintTool::new(project_path_buf.clone()))
                        .tool(TerraformFmtTool::new(project_path_buf.clone()))
                        .tool(TerraformValidateTool::new(project_path_buf.clone()))
                        .tool(TerraformInstallTool::new())
                        .tool(ReadFileTool::new(project_path_buf.clone()))
                        .tool(ListDirectoryTool::new(project_path_buf.clone()))
                        .tool(WebFetchTool::new())
                        // Prometheus discovery and connection tools for live K8s analysis
                        .tool(PrometheusDiscoverTool::new())
                        .tool(PrometheusConnectTool::new(bg_manager.clone()))
                        // RAG retrieval tools for compressed tool outputs
                        .tool(RetrieveOutputTool::new())
                        .tool(ListOutputsTool::new());

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

                    // Enable reasoning for OpenAI reasoning models (GPT-5.x, O1, O3, O4)
                    let model_lower = session.model.to_lowercase();
                    let is_reasoning_model = model_lower.starts_with("gpt-5")
                        || model_lower.starts_with("gpt5")
                        || model_lower.starts_with("o1")
                        || model_lower.starts_with("o3")
                        || model_lower.starts_with("o4");

                    let agent = if is_reasoning_model {
                        let reasoning_params = serde_json::json!({
                            "reasoning": {
                                "effort": "medium",
                                "summary": "detailed"
                            }
                        });
                        builder.additional_params(reasoning_params).build()
                    } else {
                        builder.build()
                    };

                    // Use multi_turn with Responses API
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
                        .tool(K8sOptimizeTool::new(project_path_buf.clone()))
                        .tool(K8sCostsTool::new(project_path_buf.clone()))
                        .tool(K8sDriftTool::new(project_path_buf.clone()))
                        .tool(HelmlintTool::new(project_path_buf.clone()))
                        .tool(TerraformFmtTool::new(project_path_buf.clone()))
                        .tool(TerraformValidateTool::new(project_path_buf.clone()))
                        .tool(TerraformInstallTool::new())
                        .tool(ReadFileTool::new(project_path_buf.clone()))
                        .tool(ListDirectoryTool::new(project_path_buf.clone()))
                        .tool(WebFetchTool::new())
                        // Prometheus discovery and connection tools for live K8s analysis
                        .tool(PrometheusDiscoverTool::new())
                        .tool(PrometheusConnectTool::new(bg_manager.clone()))
                        // RAG retrieval tools for compressed tool outputs
                        .tool(RetrieveOutputTool::new())
                        .tool(ListOutputsTool::new());

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
                        .tool(K8sOptimizeTool::new(project_path_buf.clone()))
                        .tool(K8sCostsTool::new(project_path_buf.clone()))
                        .tool(K8sDriftTool::new(project_path_buf.clone()))
                        .tool(HelmlintTool::new(project_path_buf.clone()))
                        .tool(TerraformFmtTool::new(project_path_buf.clone()))
                        .tool(TerraformValidateTool::new(project_path_buf.clone()))
                        .tool(TerraformInstallTool::new())
                        .tool(ReadFileTool::new(project_path_buf.clone()))
                        .tool(ListDirectoryTool::new(project_path_buf.clone()))
                        .tool(WebFetchTool::new())
                        // Prometheus discovery and connection tools for live K8s analysis
                        .tool(PrometheusDiscoverTool::new())
                        .tool(PrometheusConnectTool::new(bg_manager.clone()))
                        // RAG retrieval tools for compressed tool outputs
                        .tool(RetrieveOutputTool::new())
                        .tool(ListOutputsTool::new());

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

                    // Simplify history for OpenAI Responses API reasoning models
                    // Keep only User text and Assistant text - strip reasoning, tool calls, tool results
                    // This prevents pairing errors like "rs_... without its required following item"
                    // and "fc_... without its required reasoning item"
                    if session.provider == ProviderType::OpenAI {
                        simplify_history_for_openai_reasoning(&mut raw_chat_history);
                    }

                    // Also update legacy session history for compatibility
                    session.history.push(("user".to_string(), input.clone()));
                    session
                        .history
                        .push(("assistant".to_string(), text.clone()));

                    // Record to persistent session storage (includes full history snapshot)
                    session_recorder.record_user_message(&input);
                    session_recorder.record_assistant_message(&text, Some(&tool_calls));
                    if let Err(e) = session_recorder.save_with_history(&conversation_history) {
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

                        // Strategy 1: Keep only the last N messages (user/assistant pairs)
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
                            // Ensure history starts with User message for OpenAI Responses API compatibility
                            ensure_history_starts_with_user(&mut raw_chat_history);
                        }

                        // Strategy 2: Compact large tool outputs to temp files + summaries
                        // This preserves data (agent can read file if needed) while reducing context
                        let max_output_chars = match retry_attempt {
                            0 => 50_000, // 50KB on first try
                            1 => 20_000, // 20KB on second
                            _ => 5_000,  // 5KB on third (aggressive)
                        };
                        compact_large_tool_outputs(&mut raw_chat_history, max_output_chars);

                        let new_token_count = estimate_raw_history_tokens(&raw_chat_history);
                        eprintln!("{}", format!(
                            "  ✓ Truncated: {} messages (~{} tokens) → {} messages (~{} tokens)",
                            old_msg_count, old_token_count, raw_chat_history.len(), new_token_count
                        ).green());

                        // Preserve compacted summary while clearing turns to stay in sync
                        conversation_history.clear_turns_preserve_context();

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

/// Compact large tool outputs by saving them to temp files and replacing with summaries.
/// This preserves all data (agent can read the file) while reducing context size.
fn compact_large_tool_outputs(messages: &mut [rig::completion::Message], max_chars: usize) {
    use rig::completion::message::{Text, ToolResultContent, UserContent};
    use std::fs;

    // Create temp directory for compacted outputs
    let temp_dir = std::env::temp_dir().join("syncable-agent-outputs");
    let _ = fs::create_dir_all(&temp_dir);

    for msg in messages.iter_mut() {
        if let rig::completion::Message::User { content } = msg {
            for item in content.iter_mut() {
                if let UserContent::ToolResult(tr) = item {
                    for trc in tr.content.iter_mut() {
                        if let ToolResultContent::Text(text) = trc
                            && text.text.len() > max_chars
                        {
                            // Save full output to temp file
                            let file_id = format!(
                                "{}_{}.txt",
                                tr.id,
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            );
                            let file_path = temp_dir.join(&file_id);

                            if let Ok(()) = fs::write(&file_path, &text.text) {
                                // Create a smart summary
                                let summary = create_output_summary(
                                    &text.text,
                                    &file_path.display().to_string(),
                                    max_chars / 2, // Use half max for summary
                                );

                                // Replace with summary
                                *trc = ToolResultContent::Text(Text { text: summary });
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Create a smart summary of a large output using incremental chunk processing.
/// Processes output in logical sections, summarizes each, then combines into actionable summary.
fn create_output_summary(full_output: &str, file_path: &str, max_summary_len: usize) -> String {
    let total_lines = full_output.lines().count();
    let total_chars = full_output.len();

    let summary_content =
        if full_output.trim_start().starts_with('{') || full_output.trim_start().starts_with('[') {
            // JSON output - extract structured summary
            summarize_json_incrementally(full_output, max_summary_len)
        } else {
            // Text output - chunk and summarize
            summarize_text_incrementally(full_output, max_summary_len)
        };

    format!(
        "[COMPACTED OUTPUT]\n\
        Full data: {}\n\
        Size: {} chars, {} lines\n\
        \n\
        {}\n\
        \n\
        [Read file with offset/limit for specific sections if needed]",
        file_path, total_chars, total_lines, summary_content
    )
}

/// Incrementally summarize JSON output, extracting key fields and prioritizing important items.
fn summarize_json_incrementally(json_str: &str, max_len: usize) -> String {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return "Failed to parse JSON".to_string();
    };

    let mut parts: Vec<String> = Vec::new();
    let mut current_len = 0;

    match &json {
        serde_json::Value::Object(obj) => {
            // Priority 1: Summary/stats fields
            for key in ["summary", "stats", "metadata", "status"] {
                if let Some(v) = obj.get(key) {
                    let s = format!("{}:\n{}", key, indent_json(v, 2, 500));
                    if current_len + s.len() < max_len {
                        parts.push(s.clone());
                        current_len += s.len();
                    }
                }
            }

            // Priority 2: Error/critical items (summarize each)
            for key in [
                "errors",
                "critical",
                "failures",
                "issues",
                "findings",
                "recommendations",
            ] {
                if let Some(serde_json::Value::Array(arr)) = obj.get(key) {
                    if arr.is_empty() {
                        continue;
                    }
                    parts.push(format!("\n{} ({} items):", key, arr.len()));

                    // Group by severity/type if present
                    let mut by_severity: std::collections::HashMap<
                        String,
                        Vec<&serde_json::Value>,
                    > = std::collections::HashMap::new();

                    for item in arr {
                        let severity = item
                            .get("severity")
                            .or_else(|| item.get("level"))
                            .or_else(|| item.get("type"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("other")
                            .to_string();
                        by_severity.entry(severity).or_default().push(item);
                    }

                    // Show critical/high first, summarize others
                    for sev in [
                        "critical", "high", "error", "warning", "medium", "low", "info", "other",
                    ] {
                        if let Some(items) = by_severity.get(sev) {
                            let show_count = match sev {
                                "critical" | "high" | "error" => 5.min(items.len()),
                                "warning" | "medium" => 3.min(items.len()),
                                _ => 2.min(items.len()),
                            };

                            if !items.is_empty() {
                                let s =
                                    format!("  [{}] {} items:", sev.to_uppercase(), items.len());
                                if current_len + s.len() < max_len {
                                    parts.push(s.clone());
                                    current_len += s.len();

                                    for item in items.iter().take(show_count) {
                                        let item_summary = summarize_single_item(item);
                                        if current_len + item_summary.len() < max_len {
                                            parts.push(format!("    • {}", item_summary));
                                            current_len += item_summary.len();
                                        }
                                    }

                                    if items.len() > show_count {
                                        parts.push(format!(
                                            "    ... and {} more",
                                            items.len() - show_count
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Priority 3: Show remaining top-level keys
            let shown_keys: std::collections::HashSet<&str> = [
                "summary",
                "stats",
                "metadata",
                "status",
                "errors",
                "critical",
                "failures",
                "issues",
                "findings",
                "recommendations",
            ]
            .iter()
            .cloned()
            .collect();

            let other_keys: Vec<_> = obj
                .keys()
                .filter(|k| !shown_keys.contains(k.as_str()))
                .collect();
            if !other_keys.is_empty() && current_len < max_len - 200 {
                parts.push(format!("\nOther fields: {:?}", other_keys));
            }
        }
        serde_json::Value::Array(arr) => {
            parts.push(format!("Array with {} items", arr.len()));

            // Try to group by type/severity
            for (i, item) in arr.iter().take(10).enumerate() {
                let s = format!("[{}] {}", i, summarize_single_item(item));
                if current_len + s.len() < max_len {
                    parts.push(s.clone());
                    current_len += s.len();
                }
            }
            if arr.len() > 10 {
                parts.push(format!("... and {} more items", arr.len() - 10));
            }
        }
        _ => {
            parts.push(truncate_json_value(&json, max_len));
        }
    }

    parts.join("\n")
}

/// Summarize a single JSON item (issue, error, etc.) into a one-liner.
fn summarize_single_item(item: &serde_json::Value) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Extract common fields
    for key in [
        "message",
        "description",
        "title",
        "name",
        "file",
        "path",
        "code",
        "rule",
    ] {
        if let Some(v) = item.get(key)
            && let Some(s) = v.as_str()
        {
            parts.push(truncate_string(s, 80));
            break; // Only take first descriptive field
        }
    }

    // Add location if present
    if let Some(file) = item
        .get("file")
        .or_else(|| item.get("path"))
        .and_then(|v| v.as_str())
    {
        if let Some(line) = item.get("line").and_then(|v| v.as_u64()) {
            parts.push(format!("at {}:{}", file, line));
        } else {
            parts.push(format!("in {}", truncate_string(file, 40)));
        }
    }

    if parts.is_empty() {
        truncate_json_value(item, 100)
    } else {
        parts.join(" ")
    }
}

/// Indent JSON for display.
fn indent_json(v: &serde_json::Value, indent: usize, max_len: usize) -> String {
    let s = serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string());
    let prefix = " ".repeat(indent);
    let indented: String = s
        .lines()
        .map(|l| format!("{}{}", prefix, l))
        .collect::<Vec<_>>()
        .join("\n");
    if indented.len() > max_len {
        format!("{}...", &indented[..max_len.saturating_sub(3)])
    } else {
        indented
    }
}

/// Incrementally summarize text output by processing in chunks.
fn summarize_text_incrementally(text: &str, max_len: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut parts: Vec<String> = Vec::new();
    let mut current_len = 0;

    // Look for section headers or key patterns
    let mut sections: Vec<(usize, &str)> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        // Detect headers (lines that look like titles)
        if line.starts_with('#')
            || line.starts_with("==")
            || line.starts_with("--")
            || (line.ends_with(':') && line.len() < 50)
            || line.chars().all(|c| c.is_uppercase() || c.is_whitespace())
        {
            sections.push((i, line));
        }
    }

    if !sections.is_empty() {
        // Summarize by sections
        parts.push(format!("Found {} sections:", sections.len()));
        for (i, (line_num, header)) in sections.iter().enumerate() {
            let next_section = sections.get(i + 1).map(|(n, _)| *n).unwrap_or(lines.len());
            let section_lines = next_section - line_num;

            let s = format!(
                "  [L{}] {} ({} lines)",
                line_num + 1,
                header.trim(),
                section_lines
            );
            if current_len + s.len() < max_len / 2 {
                parts.push(s.clone());
                current_len += s.len();
            }
        }
        parts.push("".to_string());
    }

    // Show first chunk
    let preview_lines = 15.min(lines.len());
    parts.push("Content preview:".to_string());
    for line in lines.iter().take(preview_lines) {
        let s = format!("  {}", truncate_string(line, 120));
        if current_len + s.len() < max_len * 3 / 4 {
            parts.push(s.clone());
            current_len += s.len();
        }
    }

    if lines.len() > preview_lines {
        parts.push(format!(
            "  ... ({} more lines)",
            lines.len() - preview_lines
        ));
    }

    // Show last few lines if space permits
    if lines.len() > preview_lines * 2 && current_len < max_len - 500 {
        parts.push("\nEnd of output:".to_string());
        for line in lines.iter().skip(lines.len() - 5) {
            let s = format!("  {}", truncate_string(line, 120));
            if current_len + s.len() < max_len {
                parts.push(s.clone());
                current_len += s.len();
            }
        }
    }

    parts.join("\n")
}

/// Truncate a JSON value for display
fn truncate_json_value(v: &serde_json::Value, max_len: usize) -> String {
    let s = v.to_string();
    if s.len() <= max_len {
        s
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Simplify history for OpenAI Responses API compatibility with reasoning models.
///
/// OpenAI's Responses API has strict pairing requirements:
/// - Reasoning items must be followed by their output (text or function_call)
/// - Function_call items must be preceded by their reasoning item
///
/// When passing history across user turns, these pairings get broken, causing errors like:
/// - "Item 'rs_...' of type 'reasoning' was provided without its required following item"
/// - "Item 'fc_...' of type 'function_call' was provided without its required 'reasoning' item"
///
/// Solution: Keep only User messages and final Assistant Text responses.
/// This preserves conversation context without the complex internal tool/reasoning structure.
fn simplify_history_for_openai_reasoning(history: &mut Vec<rig::completion::Message>) {
    use rig::completion::message::{AssistantContent, UserContent};
    use rig::one_or_many::OneOrMany;

    // Filter to keep only User text messages and Assistant text messages
    let simplified: Vec<rig::completion::Message> = history
        .iter()
        .filter_map(|msg| match msg {
            // Keep User messages, but only text content (not tool results)
            rig::completion::Message::User { content } => {
                let text_only: Vec<UserContent> = content
                    .iter()
                    .filter(|c| matches!(c, UserContent::Text(_)))
                    .cloned()
                    .collect();
                if text_only.is_empty() {
                    None
                } else {
                    let mut iter = text_only.into_iter();
                    let first = iter.next().unwrap();
                    let rest: Vec<_> = iter.collect();
                    let new_content = if rest.is_empty() {
                        OneOrMany::one(first)
                    } else {
                        OneOrMany::many(std::iter::once(first).chain(rest)).unwrap()
                    };
                    Some(rig::completion::Message::User {
                        content: new_content,
                    })
                }
            }
            // Keep Assistant messages, but only text content (not reasoning, tool calls)
            rig::completion::Message::Assistant { content, id } => {
                let text_only: Vec<AssistantContent> = content
                    .iter()
                    .filter(|c| matches!(c, AssistantContent::Text(_)))
                    .cloned()
                    .collect();
                if text_only.is_empty() {
                    None
                } else {
                    let mut iter = text_only.into_iter();
                    let first = iter.next().unwrap();
                    let rest: Vec<_> = iter.collect();
                    let new_content = if rest.is_empty() {
                        OneOrMany::one(first)
                    } else {
                        OneOrMany::many(std::iter::once(first).chain(rest)).unwrap()
                    };
                    Some(rig::completion::Message::Assistant {
                        content: new_content,
                        id: id.clone(),
                    })
                }
            }
        })
        .collect();

    *history = simplified;
}

/// Ensure history starts with a User message for OpenAI Responses API compatibility.
///
/// OpenAI's Responses API requires that reasoning items are properly structured within
/// a conversation. When history truncation leaves an Assistant message (containing
/// Reasoning blocks) at the start, OpenAI rejects it with:
/// "Item 'rs_...' of type 'reasoning' was provided without its required following item."
///
/// This function inserts a synthetic User message at the beginning if history starts
/// with an Assistant message, preserving the context while maintaining valid structure.
fn ensure_history_starts_with_user(history: &mut Vec<rig::completion::Message>) {
    if !history.is_empty()
        && matches!(
            history.first(),
            Some(rig::completion::Message::Assistant { .. })
        )
    {
        // Insert synthetic User message at the beginning to maintain valid conversation structure
        history.insert(
            0,
            rig::completion::Message::User {
                content: rig::one_or_many::OneOrMany::one(
                    rig::completion::message::UserContent::text("(Conversation continued)"),
                ),
            },
        );
    }
}

/// Estimate token count from raw rig Messages
/// This is used for context length management to prevent "input too long" errors.
/// Estimates ~4 characters per token.
fn estimate_raw_history_tokens(messages: &[rig::completion::Message]) -> usize {
    use rig::completion::message::{AssistantContent, ToolResultContent, UserContent};

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
                                UserContent::ToolResult(tr) => {
                                    // Tool results can be HUGE - properly estimate them
                                    tr.content
                                        .iter()
                                        .map(|trc| match trc {
                                            ToolResultContent::Text(t) => t.text.len() / 4,
                                            _ => 100,
                                        })
                                        .sum::<usize>()
                                }
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

    // Background process manager for Prometheus port-forwards (single query context)
    let bg_manager = Arc::new(BackgroundProcessManager::new());
    // Select prompt based on query type (analysis vs generation)
    // For single queries (non-interactive), always use standard mode
    let preamble = get_system_prompt(project_path, Some(query), PlanMode::default());
    let is_generation = prompts::is_generation_query(query);

    match provider {
        ProviderType::OpenAI => {
            // Use Responses API (default) for reasoning model support
            let client = openai::Client::from_env();
            let model_name = model.as_deref().unwrap_or("gpt-5.2");

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
                .tool(K8sOptimizeTool::new(project_path_buf.clone()))
                .tool(K8sCostsTool::new(project_path_buf.clone()))
                .tool(K8sDriftTool::new(project_path_buf.clone()))
                .tool(HelmlintTool::new(project_path_buf.clone()))
                .tool(TerraformFmtTool::new(project_path_buf.clone()))
                .tool(TerraformValidateTool::new(project_path_buf.clone()))
                .tool(TerraformInstallTool::new())
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf.clone()))
                .tool(WebFetchTool::new())
                // Prometheus discovery and connection tools for live K8s analysis
                .tool(PrometheusDiscoverTool::new())
                .tool(PrometheusConnectTool::new(bg_manager.clone()))
                // RAG retrieval tools for compressed tool outputs
                .tool(RetrieveOutputTool::new())
                .tool(ListOutputsTool::new());

            // Add generation tools if this is a generation query
            if is_generation {
                builder = builder
                    .tool(WriteFileTool::new(project_path_buf.clone()))
                    .tool(WriteFilesTool::new(project_path_buf.clone()))
                    .tool(ShellTool::new(project_path_buf.clone()));
            }

            // Enable reasoning for OpenAI reasoning models
            let model_lower = model_name.to_lowercase();
            let is_reasoning_model = model_lower.starts_with("gpt-5")
                || model_lower.starts_with("gpt5")
                || model_lower.starts_with("o1")
                || model_lower.starts_with("o3")
                || model_lower.starts_with("o4");

            let agent = if is_reasoning_model {
                let reasoning_params = serde_json::json!({
                    "reasoning": {
                        "effort": "medium",
                        "summary": "detailed"
                    }
                });
                builder.additional_params(reasoning_params).build()
            } else {
                builder.build()
            };

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
                .tool(K8sOptimizeTool::new(project_path_buf.clone()))
                .tool(K8sCostsTool::new(project_path_buf.clone()))
                .tool(K8sDriftTool::new(project_path_buf.clone()))
                .tool(HelmlintTool::new(project_path_buf.clone()))
                .tool(TerraformFmtTool::new(project_path_buf.clone()))
                .tool(TerraformValidateTool::new(project_path_buf.clone()))
                .tool(TerraformInstallTool::new())
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf.clone()))
                .tool(WebFetchTool::new())
                // Prometheus discovery and connection tools for live K8s analysis
                .tool(PrometheusDiscoverTool::new())
                .tool(PrometheusConnectTool::new(bg_manager.clone()))
                // RAG retrieval tools for compressed tool outputs
                .tool(RetrieveOutputTool::new())
                .tool(ListOutputsTool::new());

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
                .tool(K8sOptimizeTool::new(project_path_buf.clone()))
                .tool(K8sCostsTool::new(project_path_buf.clone()))
                .tool(K8sDriftTool::new(project_path_buf.clone()))
                .tool(HelmlintTool::new(project_path_buf.clone()))
                .tool(TerraformFmtTool::new(project_path_buf.clone()))
                .tool(TerraformValidateTool::new(project_path_buf.clone()))
                .tool(TerraformInstallTool::new())
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf.clone()))
                .tool(WebFetchTool::new())
                // Prometheus discovery and connection tools for live K8s analysis
                .tool(PrometheusDiscoverTool::new())
                .tool(PrometheusConnectTool::new(bg_manager.clone()))
                // RAG retrieval tools for compressed tool outputs
                .tool(RetrieveOutputTool::new())
                .tool(ListOutputsTool::new());

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
