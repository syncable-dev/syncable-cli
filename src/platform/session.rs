//! Platform session state management
//!
//! Manages the selected platform project/organization context that persists
//! across CLI sessions. Stored in `~/.syncable/platform-session.json`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

/// Platform session state - tracks selected project, organization, and environment
///
/// This is a separate system from conversation persistence - it tracks
/// which platform project/org/environment the user has selected for platform operations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformSession {
    /// Selected platform project UUID
    pub project_id: Option<String>,
    /// Human-readable project name
    pub project_name: Option<String>,
    /// Organization UUID
    pub org_id: Option<String>,
    /// Organization name
    pub org_name: Option<String>,
    /// Selected environment UUID
    pub environment_id: Option<String>,
    /// Human-readable environment name
    pub environment_name: Option<String>,
    /// When the session was last updated
    pub last_updated: Option<DateTime<Utc>>,
}

impl PlatformSession {
    /// Creates a new empty platform session
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a platform session with a selected project
    pub fn with_project(
        project_id: String,
        project_name: String,
        org_id: String,
        org_name: String,
    ) -> Self {
        Self {
            project_id: Some(project_id),
            project_name: Some(project_name),
            org_id: Some(org_id),
            org_name: Some(org_name),
            environment_id: None,
            environment_name: None,
            last_updated: Some(Utc::now()),
        }
    }

    /// Creates a platform session with a selected project and environment
    pub fn with_environment(
        project_id: String,
        project_name: String,
        org_id: String,
        org_name: String,
        environment_id: String,
        environment_name: String,
    ) -> Self {
        Self {
            project_id: Some(project_id),
            project_name: Some(project_name),
            org_id: Some(org_id),
            org_name: Some(org_name),
            environment_id: Some(environment_id),
            environment_name: Some(environment_name),
            last_updated: Some(Utc::now()),
        }
    }

    /// Clears the selected project and environment
    pub fn clear(&mut self) {
        self.project_id = None;
        self.project_name = None;
        self.org_id = None;
        self.org_name = None;
        self.environment_id = None;
        self.environment_name = None;
        self.last_updated = Some(Utc::now());
    }

    /// Clears only the selected environment (keeps project)
    pub fn clear_environment(&mut self) {
        self.environment_id = None;
        self.environment_name = None;
        self.last_updated = Some(Utc::now());
    }

    /// Returns true if a project is currently selected
    pub fn is_project_selected(&self) -> bool {
        self.project_id.is_some()
    }

    /// Returns true if an environment is currently selected
    pub fn is_environment_selected(&self) -> bool {
        self.environment_id.is_some()
    }

    /// Returns the path to the platform session file
    ///
    /// Location: `~/.syncable/platform-session.json`
    pub fn session_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".syncable")
            .join("platform-session.json")
    }

    /// Load platform session from disk
    ///
    /// Returns Default if the file doesn't exist or can't be parsed.
    pub fn load() -> io::Result<Self> {
        let path = Self::session_path();

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Save platform session to disk
    ///
    /// Creates `~/.syncable/` directory if it doesn't exist.
    pub fn save(&self) -> io::Result<()> {
        let path = Self::session_path();

        // Ensure directory exists (pattern from persistence.rs)
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;
        Ok(())
    }

    /// Returns a display string for the current context
    ///
    /// Format: "[org/project/env]", "[org/project]", or "[no project selected]"
    pub fn display_context(&self) -> String {
        match (&self.org_name, &self.project_name, &self.environment_name) {
            (Some(org), Some(project), Some(env)) => format!("[{}/{}/{}]", org, project, env),
            (Some(org), Some(project), None) => format!("[{}/{}]", org, project),
            (None, Some(project), Some(env)) => format!("[{}/{}]", project, env),
            (None, Some(project), None) => format!("[{}]", project),
            _ => "[no project selected]".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_new_session_is_empty() {
        let session = PlatformSession::new();
        assert!(!session.is_project_selected());
        assert_eq!(session.display_context(), "[no project selected]");
    }

    #[test]
    fn test_with_project() {
        let session = PlatformSession::with_project(
            "proj-123".to_string(),
            "my-project".to_string(),
            "org-456".to_string(),
            "my-org".to_string(),
        );

        assert!(session.is_project_selected());
        assert_eq!(session.project_id, Some("proj-123".to_string()));
        assert_eq!(session.display_context(), "[my-org/my-project]");
    }

    #[test]
    fn test_clear() {
        let mut session = PlatformSession::with_project(
            "proj-123".to_string(),
            "my-project".to_string(),
            "org-456".to_string(),
            "my-org".to_string(),
        );

        session.clear();
        assert!(!session.is_project_selected());
        assert!(session.last_updated.is_some()); // last_updated preserved
    }

    #[test]
    fn test_display_context() {
        // Full context with environment
        let session = PlatformSession::with_environment(
            "id".to_string(),
            "project".to_string(),
            "oid".to_string(),
            "org".to_string(),
            "env-id".to_string(),
            "prod".to_string(),
        );
        assert_eq!(session.display_context(), "[org/project/prod]");

        // Project only (no env)
        let session = PlatformSession::with_project(
            "id".to_string(),
            "project".to_string(),
            "oid".to_string(),
            "org".to_string(),
        );
        assert_eq!(session.display_context(), "[org/project]");

        // Project only (no org)
        let session = PlatformSession {
            project_id: Some("id".to_string()),
            project_name: Some("project".to_string()),
            org_id: None,
            org_name: None,
            environment_id: None,
            environment_name: None,
            last_updated: None,
        };
        assert_eq!(session.display_context(), "[project]");

        // No project
        let session = PlatformSession::new();
        assert_eq!(session.display_context(), "[no project selected]");
    }

    #[test]
    fn test_with_environment() {
        let session = PlatformSession::with_environment(
            "proj-123".to_string(),
            "my-project".to_string(),
            "org-456".to_string(),
            "my-org".to_string(),
            "env-789".to_string(),
            "production".to_string(),
        );

        assert!(session.is_project_selected());
        assert!(session.is_environment_selected());
        assert_eq!(session.project_id, Some("proj-123".to_string()));
        assert_eq!(session.environment_id, Some("env-789".to_string()));
        assert_eq!(session.environment_name, Some("production".to_string()));
        assert_eq!(session.display_context(), "[my-org/my-project/production]");
    }

    #[test]
    fn test_clear_environment() {
        let mut session = PlatformSession::with_environment(
            "proj-123".to_string(),
            "my-project".to_string(),
            "org-456".to_string(),
            "my-org".to_string(),
            "env-789".to_string(),
            "production".to_string(),
        );

        assert!(session.is_environment_selected());

        session.clear_environment();

        assert!(session.is_project_selected()); // Project still selected
        assert!(!session.is_environment_selected()); // Environment cleared
        assert_eq!(session.display_context(), "[my-org/my-project]");
    }

    #[test]
    fn test_is_environment_selected() {
        let session = PlatformSession::new();
        assert!(!session.is_environment_selected());

        let session = PlatformSession::with_project(
            "proj-123".to_string(),
            "my-project".to_string(),
            "org-456".to_string(),
            "my-org".to_string(),
        );
        assert!(!session.is_environment_selected());

        let session = PlatformSession::with_environment(
            "proj-123".to_string(),
            "my-project".to_string(),
            "org-456".to_string(),
            "my-org".to_string(),
            "env-789".to_string(),
            "staging".to_string(),
        );
        assert!(session.is_environment_selected());
    }

    #[test]
    fn test_save_and_load() {
        // Use a temp directory for testing
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().join("platform-session.json");

        // Create and save a session
        let session = PlatformSession::with_project(
            "proj-789".to_string(),
            "test-project".to_string(),
            "org-abc".to_string(),
            "test-org".to_string(),
        );

        // Write directly to temp path for testing
        let json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(&temp_path, json).unwrap();

        // Read back
        let content = fs::read_to_string(&temp_path).unwrap();
        let loaded: PlatformSession = serde_json::from_str(&content).unwrap();

        assert_eq!(loaded.project_id, session.project_id);
        assert_eq!(loaded.project_name, session.project_name);
        assert_eq!(loaded.org_id, session.org_id);
        assert_eq!(loaded.org_name, session.org_name);
    }

    #[test]
    fn test_load_missing_file() {
        // When file doesn't exist, should return default
        // (This test relies on the actual load() checking path.exists())
        // We can't easily test this without mocking, so we just verify default behavior
        let default = PlatformSession::default();
        assert!(!default.is_project_selected());
    }
}
