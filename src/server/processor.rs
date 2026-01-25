//! Agent Processor - Routes frontend messages to agent for processing.
//!
//! This module provides the `AgentProcessor` which consumes messages from
//! the frontend (via WebSocket/POST) and processes them through the LLM,
//! emitting AG-UI events for the response.
//!
//! # Architecture
//!
//! ```text
//! Frontend → WebSocket/POST → message channel → AgentProcessor
//!                                                     ↓
//!                                              LLM (multi_turn)
//!                                                     ↓
//!                                              EventBridge → SSE/WS → Frontend
//! ```

use std::collections::HashMap;
use std::time::Instant;

use ag_ui_core::{Role, RunId, ThreadId};
use rig::completion::message::{AssistantContent, UserContent};
use rig::completion::Message as RigMessage;
use rig::one_or_many::OneOrMany;
use tokio::sync::mpsc;
use tracing::{debug, info};

use super::{AgentMessage, EventBridge};

/// Configuration for the agent processor.
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// LLM provider to use (openai, anthropic, bedrock).
    pub provider: String,
    /// Model name/ID.
    pub model: String,
    /// Maximum number of tool call iterations.
    pub max_turns: usize,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            max_turns: 50,
        }
    }
}

impl ProcessorConfig {
    /// Creates a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the provider.
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = provider.into();
        self
    }

    /// Sets the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Sets the maximum number of turns.
    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns;
        self
    }
}

/// Per-thread session state for conversation isolation.
#[derive(Debug)]
pub struct ThreadSession {
    /// Thread ID for this session.
    pub thread_id: ThreadId,
    /// Raw chat history for multi-turn conversations.
    pub history: Vec<RigMessage>,
    /// When this session was created.
    pub created_at: Instant,
    /// Number of turns in this session.
    pub turn_count: usize,
}

impl ThreadSession {
    /// Creates a new thread session.
    pub fn new(thread_id: ThreadId) -> Self {
        Self {
            thread_id,
            history: Vec::new(),
            created_at: Instant::now(),
            turn_count: 0,
        }
    }

    /// Adds a user message to history.
    pub fn add_user_message(&mut self, content: &str) {
        self.history.push(RigMessage::User {
            content: OneOrMany::one(UserContent::text(content)),
        });
    }

    /// Adds an assistant message to history.
    pub fn add_assistant_message(&mut self, content: &str) {
        self.history.push(RigMessage::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::text(content)),
        });
        self.turn_count += 1;
    }
}

/// Processes frontend messages through the LLM agent.
///
/// The processor maintains per-thread sessions for conversation isolation
/// and emits AG-UI events via the EventBridge during processing.
pub struct AgentProcessor {
    /// Receiver for messages from frontend.
    message_rx: mpsc::Receiver<AgentMessage>,
    /// Event bridge for emitting AG-UI events.
    event_bridge: EventBridge,
    /// Per-thread session state.
    sessions: HashMap<ThreadId, ThreadSession>,
    /// Processor configuration.
    config: ProcessorConfig,
}

impl AgentProcessor {
    /// Creates a new agent processor.
    ///
    /// # Arguments
    /// * `message_rx` - Receiver for messages from frontend
    /// * `event_bridge` - Bridge for emitting AG-UI events
    /// * `config` - Processor configuration
    pub fn new(
        message_rx: mpsc::Receiver<AgentMessage>,
        event_bridge: EventBridge,
        config: ProcessorConfig,
    ) -> Self {
        Self {
            message_rx,
            event_bridge,
            sessions: HashMap::new(),
            config,
        }
    }

    /// Creates a processor with default configuration.
    pub fn with_defaults(
        message_rx: mpsc::Receiver<AgentMessage>,
        event_bridge: EventBridge,
    ) -> Self {
        Self::new(message_rx, event_bridge, ProcessorConfig::default())
    }

    /// Gets or creates a session for the given thread ID.
    fn get_or_create_session(&mut self, thread_id: &ThreadId) -> &mut ThreadSession {
        self.sessions
            .entry(thread_id.clone())
            .or_insert_with(|| ThreadSession::new(thread_id.clone()))
    }

    /// Gets the current session count.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Gets the configuration.
    pub fn config(&self) -> &ProcessorConfig {
        &self.config
    }

    /// Extracts the user message content from RunAgentInput messages.
    ///
    /// Returns the last user message content, or None if no user messages.
    fn extract_user_input(
        &self,
        messages: &[ag_ui_core::types::Message],
    ) -> Option<String> {
        // Find the last user message and extract its content
        messages
            .iter()
            .rev()
            .find(|m| m.role() == Role::User)
            .and_then(|m| m.content().map(|s| s.to_string()))
    }

    /// Processes a single message through the agent.
    ///
    /// This is the core processing method that:
    /// 1. Emits RunStarted
    /// 2. Processes through LLM (simplified for now - echoes back)
    /// 3. Emits TextMessage events
    /// 4. Updates session history
    /// 5. Emits RunFinished
    async fn process_message(
        &mut self,
        thread_id: ThreadId,
        _run_id: RunId,
        user_input: String,
    ) {
        info!(
            thread_id = %thread_id,
            input_len = user_input.len(),
            "Processing message"
        );

        // Get or create session
        let session = self.get_or_create_session(&thread_id);
        session.add_user_message(&user_input);

        // Emit run started
        self.event_bridge.start_run().await;

        // Start thinking
        self.event_bridge.start_thinking(Some("Processing")).await;

        // TODO: In Phase 23+, this will be replaced with actual LLM call
        // For now, generate a simple response to verify the pipeline works
        let response = format!(
            "I received your message: \"{}\". \
             (This is a placeholder response - LLM integration coming in Phase 23+)",
            if user_input.len() > 50 {
                format!("{}...", &user_input[..50])
            } else {
                user_input.clone()
            }
        );

        self.event_bridge.end_thinking().await;

        // Emit the response as text message
        self.event_bridge.start_message().await;

        // Stream the response in chunks to demonstrate streaming capability
        for chunk in response.chars().collect::<Vec<_>>().chunks(10) {
            let chunk_str: String = chunk.iter().collect();
            self.event_bridge.emit_text_chunk(&chunk_str).await;
        }

        self.event_bridge.end_message().await;

        // Update session
        let session = self.get_or_create_session(&thread_id);
        session.add_assistant_message(&response);

        debug!(
            thread_id = %thread_id,
            turn_count = session.turn_count,
            "Message processed"
        );

        // Finish the run
        self.event_bridge.finish_run().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn create_test_processor() -> (AgentProcessor, mpsc::Sender<AgentMessage>) {
        let (msg_tx, msg_rx) = mpsc::channel(100);
        let (event_tx, _) = broadcast::channel(100);
        let bridge = EventBridge::new(
            event_tx,
            Arc::new(RwLock::new(ThreadId::random())),
            Arc::new(RwLock::new(None)),
        );
        let processor = AgentProcessor::with_defaults(msg_rx, bridge);
        (processor, msg_tx)
    }

    #[test]
    fn test_processor_config_default() {
        let config = ProcessorConfig::default();
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-4o");
        assert_eq!(config.max_turns, 50);
    }

    #[test]
    fn test_processor_config_builder() {
        let config = ProcessorConfig::new()
            .with_provider("anthropic")
            .with_model("claude-3-opus")
            .with_max_turns(100);
        assert_eq!(config.provider, "anthropic");
        assert_eq!(config.model, "claude-3-opus");
        assert_eq!(config.max_turns, 100);
    }

    #[test]
    fn test_thread_session_new() {
        let thread_id = ThreadId::random();
        let session = ThreadSession::new(thread_id.clone());
        assert_eq!(session.thread_id, thread_id);
        assert!(session.history.is_empty());
        assert_eq!(session.turn_count, 0);
    }

    #[test]
    fn test_thread_session_add_messages() {
        let mut session = ThreadSession::new(ThreadId::random());

        session.add_user_message("Hello");
        assert_eq!(session.history.len(), 1);
        assert_eq!(session.turn_count, 0); // User message doesn't increment turn

        session.add_assistant_message("Hi there!");
        assert_eq!(session.history.len(), 2);
        assert_eq!(session.turn_count, 1); // Assistant message increments turn
    }

    #[test]
    fn test_processor_creation() {
        let (processor, _tx) = create_test_processor();
        assert_eq!(processor.session_count(), 0);
        assert_eq!(processor.config().provider, "openai");
    }

    #[test]
    fn test_get_or_create_session() {
        let (mut processor, _tx) = create_test_processor();
        let thread_id = ThreadId::random();

        // First call creates new session
        let session = processor.get_or_create_session(&thread_id);
        assert_eq!(session.turn_count, 0);

        // Add a message
        session.add_user_message("test");

        // Second call returns same session
        let session = processor.get_or_create_session(&thread_id);
        assert_eq!(session.history.len(), 1);
    }

    #[tokio::test]
    async fn test_process_message() {
        let (mut processor, _tx) = create_test_processor();
        let thread_id = ThreadId::random();
        let run_id = RunId::random();

        processor.process_message(
            thread_id.clone(),
            run_id,
            "Hello, agent!".to_string(),
        ).await;

        // Check session was updated
        assert_eq!(processor.session_count(), 1);
        let session = processor.sessions.get(&thread_id).unwrap();
        assert_eq!(session.turn_count, 1);
        assert_eq!(session.history.len(), 2); // user + assistant
    }
}
