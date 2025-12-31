//! Session persistence for conversation history
//!
//! This module provides functionality to save and restore chat sessions,
//! enabling users to resume previous conversations.
//!
//! ## Storage Location
//! Sessions are stored in `~/.syncable/sessions/<project_hash>/session-{timestamp}-{uuid}.json`
//!
//! ## Features
//! - Automatic session recording on each turn
//! - Session listing and selection by UUID or index
//! - Resume from "latest" or specific session

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::history::ToolCallRecord;

/// Represents a complete conversation record stored on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRecord {
    /// Unique session identifier (UUID)
    pub session_id: String,
    /// Hash of the project path (for organizing sessions by project)
    pub project_hash: String,
    /// When the session started
    pub start_time: DateTime<Utc>,
    /// When the session was last updated
    pub last_updated: DateTime<Utc>,
    /// All messages in the conversation
    pub messages: Vec<MessageRecord>,
    /// Optional AI-generated summary
    pub summary: Option<String>,
}

/// A single message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    /// Unique message ID
    pub id: String,
    /// When the message was created
    pub timestamp: DateTime<Utc>,
    /// Who sent the message
    pub role: MessageRole,
    /// The message content
    pub content: String,
    /// Tool calls made during this message (for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<SerializableToolCall>>,
}

/// Simplified tool call record for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableToolCall {
    pub name: String,
    pub args_summary: String,
    pub result_summary: String,
}

impl From<&ToolCallRecord> for SerializableToolCall {
    fn from(tc: &ToolCallRecord) -> Self {
        Self {
            name: tc.tool_name.clone(),
            args_summary: tc.args_summary.clone(),
            result_summary: tc.result_summary.clone(),
        }
    }
}

/// Role of the message sender
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Session information for display and selection
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Unique session ID
    pub id: String,
    /// Path to the session file
    pub file_path: PathBuf,
    /// When the session started
    pub start_time: DateTime<Utc>,
    /// When the session was last updated
    pub last_updated: DateTime<Utc>,
    /// Number of messages
    pub message_count: usize,
    /// Display name (first user message or summary)
    pub display_name: String,
    /// 1-based index for selection
    pub index: usize,
}

/// Records conversations to disk
pub struct SessionRecorder {
    session_id: String,
    file_path: PathBuf,
    record: ConversationRecord,
}

impl SessionRecorder {
    /// Create a new session recorder for the given project
    pub fn new(project_path: &Path) -> Self {
        let session_id = Uuid::new_v4().to_string();
        let project_hash = hash_project_path(project_path);
        let start_time = Utc::now();

        // Format: session-{timestamp}-{uuid_short}.json
        let timestamp = start_time.format("%Y%m%d-%H%M%S").to_string();
        let uuid_short = &session_id[..8];
        let filename = format!("session-{}-{}.json", timestamp, uuid_short);

        // Storage location: ~/.syncable/sessions/<project_hash>/
        let sessions_dir = get_sessions_dir(&project_hash);
        let file_path = sessions_dir.join(filename);

        let record = ConversationRecord {
            session_id: session_id.clone(),
            project_hash,
            start_time,
            last_updated: start_time,
            messages: Vec::new(),
            summary: None,
        };

        Self {
            session_id,
            file_path,
            record,
        }
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Record a user message
    pub fn record_user_message(&mut self, content: &str) {
        let message = MessageRecord {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            role: MessageRole::User,
            content: content.to_string(),
            tool_calls: None,
        };
        self.record.messages.push(message);
        self.record.last_updated = Utc::now();
    }

    /// Record an assistant message with optional tool calls
    pub fn record_assistant_message(
        &mut self,
        content: &str,
        tool_calls: Option<&[ToolCallRecord]>,
    ) {
        let serializable_tools =
            tool_calls.map(|calls| calls.iter().map(SerializableToolCall::from).collect());

        let message = MessageRecord {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            role: MessageRole::Assistant,
            content: content.to_string(),
            tool_calls: serializable_tools,
        };
        self.record.messages.push(message);
        self.record.last_updated = Utc::now();
    }

    /// Save the session to disk
    pub fn save(&self) -> io::Result<()> {
        // Ensure directory exists
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write JSON
        let json = serde_json::to_string_pretty(&self.record)?;
        fs::write(&self.file_path, json)?;
        Ok(())
    }

    /// Check if the session has any messages
    pub fn has_messages(&self) -> bool {
        !self.record.messages.is_empty()
    }

    /// Get the number of messages
    pub fn message_count(&self) -> usize {
        self.record.messages.len()
    }
}

/// Selects and loads sessions
pub struct SessionSelector {
    #[allow(dead_code)]
    project_path: PathBuf,
    project_hash: String,
}

impl SessionSelector {
    /// Create a new session selector for the given project
    pub fn new(project_path: &Path) -> Self {
        let project_hash = hash_project_path(project_path);
        Self {
            project_path: project_path.to_path_buf(),
            project_hash,
        }
    }

    /// List all available sessions for this project, sorted by most recent first
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions_dir = get_sessions_dir(&self.project_hash);
        if !sessions_dir.exists() {
            return Vec::new();
        }

        let mut sessions: Vec<SessionInfo> = fs::read_dir(&sessions_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            .filter_map(|entry| self.load_session_info(&entry.path()))
            .collect();

        // Sort by last_updated, most recent first
        sessions.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));

        // Assign 1-based indices
        for (i, session) in sessions.iter_mut().enumerate() {
            session.index = i + 1;
        }

        sessions
    }

    /// Find a session by identifier (UUID, partial UUID, or numeric index)
    pub fn find_session(&self, identifier: &str) -> Option<SessionInfo> {
        let sessions = self.list_sessions();

        // Try to parse as numeric index first
        if let Ok(index) = identifier.parse::<usize>()
            && index > 0
            && index <= sessions.len()
        {
            return sessions.into_iter().nth(index - 1);
        }

        // Try to find by UUID or partial UUID
        sessions
            .into_iter()
            .find(|s| s.id == identifier || s.id.starts_with(identifier))
    }

    /// Resolve "latest" or specific identifier to a session
    pub fn resolve_session(&self, arg: &str) -> Option<SessionInfo> {
        if arg == "latest" {
            self.list_sessions().into_iter().next()
        } else {
            self.find_session(arg)
        }
    }

    /// Load a full conversation record from a session
    pub fn load_conversation(&self, session_info: &SessionInfo) -> io::Result<ConversationRecord> {
        let content = fs::read_to_string(&session_info.file_path)?;
        serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Load session info from a file path
    fn load_session_info(&self, file_path: &Path) -> Option<SessionInfo> {
        let content = fs::read_to_string(file_path).ok()?;
        let record: ConversationRecord = serde_json::from_str(&content).ok()?;

        // Get display name from summary or first user message
        let display_name = record.summary.clone().unwrap_or_else(|| {
            record
                .messages
                .iter()
                .find(|m| m.role == MessageRole::User)
                .map(|m| truncate_message(&m.content, 60))
                .unwrap_or_else(|| "Empty session".to_string())
        });

        Some(SessionInfo {
            id: record.session_id,
            file_path: file_path.to_path_buf(),
            start_time: record.start_time,
            last_updated: record.last_updated,
            message_count: record.messages.len(),
            display_name,
            index: 0, // Will be set by list_sessions
        })
    }
}

/// Get the sessions directory for a project
fn get_sessions_dir(project_hash: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".syncable")
        .join("sessions")
        .join(project_hash)
}

/// Hash a project path to create a consistent directory name
fn hash_project_path(project_path: &Path) -> String {
    let canonical = project_path
        .canonicalize()
        .unwrap_or_else(|_| project_path.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    format!("{:016x}", hasher.finish())[..8].to_string()
}

/// Truncate a message for display
fn truncate_message(msg: &str, max_len: usize) -> String {
    // Clean up the message
    let clean = msg.lines().next().unwrap_or(msg).trim();

    if clean.len() <= max_len {
        clean.to_string()
    } else {
        format!("{}...", &clean[..max_len.saturating_sub(3)])
    }
}

/// Format relative time for display
pub fn format_relative_time(time: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(time);

    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        let mins = duration.num_minutes();
        format!("{}m ago", mins)
    } else if duration.num_hours() < 24 {
        let hours = duration.num_hours();
        format!("{}h ago", hours)
    } else if duration.num_days() < 30 {
        let days = duration.num_days();
        format!("{}d ago", days)
    } else {
        time.format("%Y-%m-%d").to_string()
    }
}

/// Display an interactive session browser and return the selected session
pub fn browse_sessions(project_path: &Path) -> Option<SessionInfo> {
    use colored::Colorize;

    let selector = SessionSelector::new(project_path);
    let sessions = selector.list_sessions();

    if sessions.is_empty() {
        println!(
            "{}",
            "No previous sessions found for this project.".yellow()
        );
        return None;
    }

    // Show sessions
    println!();
    println!(
        "{}",
        format!("Recent Sessions ({})", sessions.len())
            .cyan()
            .bold()
    );
    println!();

    for session in &sessions {
        let time = format_relative_time(session.last_updated);
        let msg_count = session.message_count;

        println!(
            "  {} {} {}",
            format!("[{}]", session.index).cyan(),
            session.display_name.white(),
            format!("({})", time).dimmed()
        );
        println!("      {} messages", msg_count.to_string().dimmed());
    }

    println!();
    print!(
        "{}",
        "Enter number to resume, or press Enter to cancel: ".dimmed()
    );
    io::stdout().flush().ok()?;

    // Read user input
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input).ok()?;
    let input = input.trim();

    if input.is_empty() {
        return None;
    }

    selector.find_session(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_session_recorder() {
        let temp_dir = tempdir().unwrap();
        let project_path = temp_dir.path();

        let mut recorder = SessionRecorder::new(project_path);
        assert!(!recorder.has_messages());

        recorder.record_user_message("Hello, world!");
        assert!(recorder.has_messages());
        assert_eq!(recorder.message_count(), 1);

        recorder.record_assistant_message("Hello! How can I help?", None);
        assert_eq!(recorder.message_count(), 2);

        // Save and verify
        recorder.save().unwrap();
        assert!(recorder.file_path.exists());
    }

    #[test]
    fn test_project_hash() {
        let hash1 = hash_project_path(Path::new("/tmp/project1"));
        let hash2 = hash_project_path(Path::new("/tmp/project2"));
        let hash3 = hash_project_path(Path::new("/tmp/project1"));

        assert_eq!(hash1.len(), 8);
        assert_ne!(hash1, hash2);
        assert_eq!(hash1, hash3);
    }

    #[test]
    fn test_truncate_message() {
        assert_eq!(truncate_message("short", 10), "short");
        assert_eq!(truncate_message("this is a long message", 10), "this is...");
        assert_eq!(truncate_message("line1\nline2\nline3", 100), "line1");
    }

    #[test]
    fn test_format_relative_time() {
        let now = Utc::now();
        assert_eq!(format_relative_time(now), "just now");

        let hour_ago = now - chrono::Duration::hours(1);
        assert_eq!(format_relative_time(hour_ago), "1h ago");

        let day_ago = now - chrono::Duration::days(1);
        assert_eq!(format_relative_time(day_ago), "1d ago");
    }
}
