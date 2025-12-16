//! Agent module for interactive AI-powered CLI assistance
//!
//! This module provides an agent layer using the Rig library that allows users
//! to interact with the CLI through natural language conversations.
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
pub mod session;
pub mod tools;
pub mod ui;

use colored::Colorize;
use rig::{
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::{anthropic, openai},
};
use session::ChatSession;
use commands::TokenUsage;
use std::path::Path;
use std::sync::Arc;
use ui::{ResponseFormatter, Spinner, ToolDisplayHook, spawn_tool_display_handler};

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

/// Get the system prompt for the agent
fn get_system_prompt(project_path: &Path) -> String {
    format!(
        r#"You are a helpful AI assistant integrated into the Syncable CLI tool. You help developers understand and improve their codebases.

## Project Context
You are currently working with a project located at: {}

## Your Capabilities
You have access to tools to help analyze and understand the project:

1. **analyze_project** - Analyze the project to detect languages, frameworks, dependencies, and architecture
2. **security_scan** - Perform security analysis to find potential vulnerabilities and secrets
3. **check_vulnerabilities** - Check dependencies for known security vulnerabilities
4. **read_file** - Read the contents of a file in the project
5. **list_directory** - List files and directories in a path

## Guidelines
- Use the available tools to gather information before answering questions about the project
- Be concise but thorough in your explanations
- When you find issues, suggest specific fixes
- Format code examples using markdown code blocks"#,
        project_path.display()
    )
}

/// Run the agent in interactive mode with custom REPL supporting /model and /provider commands
pub async fn run_interactive(
    project_path: &Path,
    provider: ProviderType,
    model: Option<String>,
) -> AgentResult<()> {
    use tools::*;

    let mut session = ChatSession::new(project_path, provider, model);
    
    // Load API key from config file to env if not already set
    ChatSession::load_api_key_to_env(session.provider);
    
    // Check if API key is configured, prompt if not
    if !ChatSession::has_api_key(session.provider) {
        ChatSession::prompt_api_key(session.provider)?;
    }
    
    session.print_banner();
    
    loop {
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
        
        // Start spinner for visual feedback
        println!();
        let spinner = Arc::new(Spinner::new("Thinking..."));
        
        // Create hook for tool display
        let (hook, receiver) = ToolDisplayHook::new();
        let spinner_clone = spinner.clone();
        let _tool_display_handle = spawn_tool_display_handler(receiver, spinner_clone);
        
        let project_path_buf = session.project_path.clone();
        let preamble = get_system_prompt(&session.project_path);
        
        let response = match session.provider {
            ProviderType::OpenAI => {
                let client = openai::Client::from_env();
                // For GPT-5.x reasoning models, explicitly set reasoning_effort to avoid
                // deserialization errors (Rig's ReasoningEffort enum lacks "none" variant)
                let reasoning_params = if session.model.starts_with("gpt-5") || session.model.starts_with("o1") {
                    Some(serde_json::json!({
                        "reasoning": {
                            "effort": "medium"
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
                    .tool(ListDirectoryTool::new(project_path_buf));
                
                if let Some(params) = reasoning_params {
                    builder = builder.additional_params(params);
                }
                
                let agent = builder.build();
                // Allow up to 10 tool call turns for thorough analysis
                // Use hook to display tool calls as they happen
                agent.prompt(&input).with_hook(hook.clone()).multi_turn(10).await
            }
            ProviderType::Anthropic => {
                let client = anthropic::Client::from_env();
                let agent = client
                    .agent(&session.model)
                    .preamble(&preamble)
                    .max_tokens(4096)
                    .tool(AnalyzeTool::new(project_path_buf.clone()))
                    .tool(SecurityScanTool::new(project_path_buf.clone()))
                    .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                    .tool(ReadFileTool::new(project_path_buf.clone()))
                    .tool(ListDirectoryTool::new(project_path_buf))
                    .build();
                
                // Allow up to 10 tool call turns for thorough analysis
                // Use hook to display tool calls as they happen
                agent.prompt(&input).with_hook(hook.clone()).multi_turn(10).await
            }
        };
        
        match response {
            Ok(text) => {
                // Stop spinner and show beautifully formatted response
                spinner.stop().await;
                ResponseFormatter::print_response(&text);
                
                // Track token usage (estimate since Rig doesn't expose exact counts)
                let prompt_tokens = TokenUsage::estimate_tokens(&input);
                let completion_tokens = TokenUsage::estimate_tokens(&text);
                session.token_usage.add_request(prompt_tokens, completion_tokens);
                
                session.history.push(("user".to_string(), input));
                session.history.push(("assistant".to_string(), text));
            }
            Err(e) => {
                spinner.stop().await;
                eprintln!("{}", format!("Error: {}", e).red());
            }
        }
        println!();
    }

    Ok(())
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
    let preamble = get_system_prompt(project_path);

    match provider {
        ProviderType::OpenAI => {
            let client = openai::Client::from_env();
            let model_name = model.as_deref().unwrap_or("gpt-5.2");
            
            // For GPT-5.x reasoning models, explicitly set reasoning_effort
            let reasoning_params = if model_name.starts_with("gpt-5") || model_name.starts_with("o1") {
                Some(serde_json::json!({
                    "reasoning": {
                        "effort": "medium"
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
                .tool(ListDirectoryTool::new(project_path_buf));
            
            if let Some(params) = reasoning_params {
                builder = builder.additional_params(params);
            }
            
            let agent = builder.build();

            agent
                .prompt(query)
                .multi_turn(10)
                .await
                .map_err(|e| AgentError::ProviderError(e.to_string()))
        }
        ProviderType::Anthropic => {
            let client = anthropic::Client::from_env();
            let model_name = model.as_deref().unwrap_or("claude-sonnet-4-20250514");

            let agent = client
                .agent(model_name)
                .preamble(&preamble)
                .max_tokens(4096)
                .tool(AnalyzeTool::new(project_path_buf.clone()))
                .tool(SecurityScanTool::new(project_path_buf.clone()))
                .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf))
                .build();

            agent
                .prompt(query)
                .multi_turn(10)
                .await
                .map_err(|e| AgentError::ProviderError(e.to_string()))
        }
    }
}
