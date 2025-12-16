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
//! sync-ctl chat --provider openai --model gpt-4o
//!
//! # Single query
//! sync-ctl chat -q "What security issues does this project have?"
//! ```

pub mod config;
pub mod tools;
pub mod ui;

use futures::StreamExt;
use rig::{
    agent::MultiTurnStreamItem,
    client::{CompletionClient, ProviderClient},
    completion::{Message, Prompt},
    providers::{anthropic, openai},
    streaming::{StreamedAssistantContent, StreamingChat},
};
use std::io::{self, BufRead, Write};
use std::path::Path;

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

    #[error("Client initialization error: {0}")]
    ClientError(String),
}

pub type AgentResult<T> = Result<T, AgentError>;

/// Get the system prompt for the agent
fn get_system_prompt(project_path: &Path) -> String {
    format!(
        r#"You are an expert AI coding assistant integrated into the Syncable CLI. You help developers understand, navigate, and improve their codebases through deep, thorough investigation.

## Project Context
Project location: {}

## Your Tools

### 🏗️ MONOREPO DISCOVERY (USE FIRST!)
- **discover_services** - **START HERE for monorepos!** Lists ALL services/packages with their:
  - Names, paths, types (Next.js, Express, Rust binary, etc.)
  - Frameworks detected (React, Prisma, tRPC, etc.)
  - Workspace configuration
  - Use `path: "apps"` or `path: "services"` to focus on specific areas

### 🔍 DEEP ANALYSIS
- **analyze_project** - Comprehensive analysis of a specific project
  - **ALWAYS specify `path`** to analyze individual services: `path: "apps/api"`
  - `mode: "json"` - Structured data (default, best for parsing)
  - `mode: "detailed"` - Full analysis with Docker info
  - **For monorepos: Call this MULTIPLE TIMES with different paths!**

### 🔎 CODE SEARCH
- **search_code** - Grep-like search across files
  - `pattern: "function_name"` - Find where things are defined/used
  - `path: "apps/api"` - Search within specific service
  - `regex: true` - Enable regex patterns
  - `extension: "ts"` - Filter by file type
  - `max_results: 100` - Increase for thorough search

- **find_files** - Find files by name/pattern
  - `pattern: "*.config.*"` - Find all config files
  - `pattern: "Dockerfile*"` - Find Dockerfiles
  - `include_dirs: true` - Include directories

- **read_file** - Read actual file contents
  - Use after finding files to see implementation details
  - `start_line`/`end_line` - Read specific sections

- **list_directory** - Explore directory structure
  - `recursive: true` - See nested structure

### 🛡️ SECURITY
- **security_scan** - Find secrets, hardcoded credentials, security issues
- **check_vulnerabilities** - Check dependencies for known CVEs

### 📦 GENERATION
- **generate_iac** - Generate Infrastructure as Code
  - `path: "apps/api"` - Generate for specific service
  - `generate_type: "dockerfile" | "compose" | "terraform" | "all"`

## AGENTIC INVESTIGATION PROTOCOL

You are a DEEPLY INVESTIGATIVE agent. You have up to 300 tool calls - USE THEM!

### For Monorepos (multiple services/packages):
1. **ALWAYS start with `discover_services`** to map the entire structure
2. **Analyze EACH relevant service individually** with `analyze_project(path: "service/path")`
3. **Search across the monorepo** for patterns, shared code, cross-service dependencies
4. **Read key files** in each service (entry points, configs, main logic)
5. **Cross-reference** - how do services communicate? What's shared?

### For Deep Investigation:
1. **Don't stop at surface level** - dig into implementation
2. **Follow the code** - if you find a function call, search for its definition
3. **Check configs** - look for .env files, config directories, environment setup
4. **Examine dependencies** - package.json, Cargo.toml, what's being used?
5. **Read actual source code** - use read_file to understand implementation

### Investigation Mindset:
- "I found 5 services, let me analyze each one..."
- "The API uses Express, let me find the route definitions..."
- "This imports from ../shared, let me explore that directory..."
- "There's a database connection, let me find the schema..."
- "I see tRPC, let me find the router definitions..."

## Response Guidelines
- NEVER answer without thorough investigation first
- Show your exploration: "Discovering services... Found 5 apps. Analyzing apps/api..."
- For each service: summarize its purpose, tech stack, key files
- When asked to investigate: USE MANY TOOLS, explore deeply
- Format code with ```language blocks
- Be specific: "In apps/api/src/routes/users.ts line 45..." 
- Don't guess - if you're uncertain, explore more!"#,
        project_path.display()
    )
}

/// Run the agent in interactive mode with beautiful UI
pub async fn run_interactive(
    project_path: &Path,
    provider: ProviderType,
    model: Option<String>,
) -> AgentResult<()> {
    use tools::*;
    use ui::AgentUI;

    let project_path_buf = project_path.to_path_buf();
    let preamble = get_system_prompt(project_path);
    let mut ui = AgentUI::new();
    let mut chat_history: Vec<Message> = Vec::new();

    let provider_name = match provider {
        ProviderType::OpenAI => "OpenAI",
        ProviderType::Anthropic => "Anthropic",
    };

    match provider {
        ProviderType::OpenAI => {
            let client = openai::Client::from_env();
            let model_name = model.as_deref().unwrap_or("gpt-4o");

            let agent = client
                .agent(model_name)
                .preamble(&preamble)
                .max_tokens(4096)
                .tool(DiscoverServicesTool::new(project_path_buf.clone()))
                .tool(AnalyzeTool::new(project_path_buf.clone()))
                .tool(SecurityScanTool::new(project_path_buf.clone()))
                .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf.clone()))
                .tool(SearchCodeTool::new(project_path_buf.clone()))
                .tool(FindFilesTool::new(project_path_buf.clone()))
                .tool(GenerateIaCTool::new(project_path_buf.clone()))
                .build();

            ui.print_welcome(provider_name, model_name);

            // Custom chat loop with streaming
            loop {
                ui.print_prompt();
                io::stdout().flush().ok();

                let mut input = String::new();
                if io::stdin().lock().read_line(&mut input).is_err() {
                    break;
                }

                let input = input.trim();
                if input.is_empty() {
                    continue;
                }
                if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                    println!("\n  {} Goodbye!\n", ui::SPARKLES);
                    break;
                }

                ui.start_thinking();

                // Use streaming chat with multi-turn enabled for tool calls
                let mut stream = agent.stream_chat(input, chat_history.clone()).multi_turn(300).await;
                ui.stop_thinking();
                ui.print_assistant_header();
                ui.start_streaming();

                let mut full_response = String::new();
                let mut had_tool_calls = false;
                let mut last_update = 0;

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(text))) => {
                            full_response.push_str(&text.text);
                            // Update progress every 50 chars
                            if full_response.len() - last_update > 50 {
                                ui.update_streaming(full_response.len());
                                last_update = full_response.len();
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall(tool_call))) => {
                            had_tool_calls = true;
                            ui.pause_spinner();
                            ui.print_tool_call_notification(&tool_call.function.name);
                            ui.print_tool_call_complete(&tool_call.function.name);
                            ui.start_streaming();
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(_)) => {}
                        Ok(MultiTurnStreamItem::StreamUserItem(_)) => {}
                        Ok(MultiTurnStreamItem::FinalResponse(_)) => {}
                        Err(e) => {
                            ui.print_error(&format!("Stream error: {}", e));
                            break;
                        }
                        _ => {}
                    }
                }

                // Render the complete response with markdown
                ui.finish_streaming_and_render(&full_response);

                // Update chat history
                if !full_response.is_empty() || had_tool_calls {
                    chat_history.push(Message::user(input));
                    chat_history.push(Message::assistant(&full_response));
                }
            }
        }
        ProviderType::Anthropic => {
            let client = anthropic::Client::from_env();
            let model_name = model.as_deref().unwrap_or("claude-3-5-sonnet-latest");

            let agent = client
                .agent(model_name)
                .preamble(&preamble)
                .max_tokens(4096)
                .tool(DiscoverServicesTool::new(project_path_buf.clone()))
                .tool(AnalyzeTool::new(project_path_buf.clone()))
                .tool(SecurityScanTool::new(project_path_buf.clone()))
                .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf.clone()))
                .tool(SearchCodeTool::new(project_path_buf.clone()))
                .tool(FindFilesTool::new(project_path_buf.clone()))
                .tool(GenerateIaCTool::new(project_path_buf.clone()))
                .build();

            ui.print_welcome(provider_name, model_name);

            // Custom chat loop with streaming
            loop {
                ui.print_prompt();
                io::stdout().flush().ok();

                let mut input = String::new();
                if io::stdin().lock().read_line(&mut input).is_err() {
                    break;
                }

                let input = input.trim();
                if input.is_empty() {
                    continue;
                }
                if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                    println!("\n  {} Goodbye!\n", ui::SPARKLES);
                    break;
                }

                ui.start_thinking();

                // Use streaming chat with multi-turn enabled for tool calls
                let mut stream = agent.stream_chat(input, chat_history.clone()).multi_turn(300).await;
                ui.stop_thinking();
                ui.print_assistant_header();
                ui.start_streaming();

                let mut full_response = String::new();
                let mut had_tool_calls = false;
                let mut last_update = 0;

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(text))) => {
                            full_response.push_str(&text.text);
                            // Update progress every 50 chars
                            if full_response.len() - last_update > 50 {
                                ui.update_streaming(full_response.len());
                                last_update = full_response.len();
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall(tool_call))) => {
                            had_tool_calls = true;
                            ui.pause_spinner();
                            ui.print_tool_call_notification(&tool_call.function.name);
                            ui.print_tool_call_complete(&tool_call.function.name);
                            ui.start_streaming();
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(_)) => {}
                        Ok(MultiTurnStreamItem::StreamUserItem(_)) => {}
                        Ok(MultiTurnStreamItem::FinalResponse(_)) => {}
                        Err(e) => {
                            ui.print_error(&format!("Stream error: {}", e));
                            break;
                        }
                        _ => {}
                    }
                }

                // Render the complete response with markdown
                ui.finish_streaming_and_render(&full_response);

                // Update chat history
                if !full_response.is_empty() || had_tool_calls {
                    chat_history.push(Message::user(input));
                    chat_history.push(Message::assistant(&full_response));
                }
            }
        }
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
            let model_name = model.as_deref().unwrap_or("gpt-4o");

            let agent = client
                .agent(model_name)
                .preamble(&preamble)
                .max_tokens(4096)
                .tool(DiscoverServicesTool::new(project_path_buf.clone()))
                .tool(AnalyzeTool::new(project_path_buf.clone()))
                .tool(SecurityScanTool::new(project_path_buf.clone()))
                .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf.clone()))
                .tool(SearchCodeTool::new(project_path_buf.clone()))
                .tool(FindFilesTool::new(project_path_buf.clone()))
                .tool(GenerateIaCTool::new(project_path_buf))
                .build();

            agent
                .prompt(query)
                .await
                .map_err(|e| AgentError::ProviderError(e.to_string()))
        }
        ProviderType::Anthropic => {
            let client = anthropic::Client::from_env();
            let model_name = model.as_deref().unwrap_or("claude-3-5-sonnet-latest");

            let agent = client
                .agent(model_name)
                .preamble(&preamble)
                .max_tokens(4096)
                .tool(DiscoverServicesTool::new(project_path_buf.clone()))
                .tool(AnalyzeTool::new(project_path_buf.clone()))
                .tool(SecurityScanTool::new(project_path_buf.clone()))
                .tool(VulnerabilitiesTool::new(project_path_buf.clone()))
                .tool(ReadFileTool::new(project_path_buf.clone()))
                .tool(ListDirectoryTool::new(project_path_buf.clone()))
                .tool(SearchCodeTool::new(project_path_buf.clone()))
                .tool(FindFilesTool::new(project_path_buf.clone()))
                .tool(GenerateIaCTool::new(project_path_buf))
                .build();

            agent
                .prompt(query)
                .await
                .map_err(|e| AgentError::ProviderError(e.to_string()))
        }
    }
}
