//! RAG Storage Layer for Tool Outputs
//!
//! Stores full tool outputs to disk for later retrieval by the agent.
//! Implements the storage part of the RAG (Retrieval-Augmented Generation) pattern.
//!
//! ## Session Tracking
//!
//! All stored outputs are tracked in a session registry, so the agent always knows
//! what data is available for retrieval. Every compressed output includes the full
//! list of available refs.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Directory where outputs are stored
const OUTPUT_DIR: &str = "/tmp/syncable-cli/outputs";

/// Maximum age of stored outputs in seconds (1 hour)
const MAX_AGE_SECS: u64 = 3600;

/// Session registry entry - tracks what's available for retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRef {
    /// Reference ID for retrieval
    pub ref_id: String,
    /// Tool that generated this output
    pub tool: String,
    /// What this output contains (brief description)
    pub contains: String,
    /// Summary counts (e.g., "47 issues: 3 critical, 12 high")
    pub summary: String,
    /// Timestamp when stored
    pub timestamp: u64,
    /// Size in bytes
    pub size_bytes: usize,
}

/// Global session registry - tracks all stored outputs in current session
static SESSION_REGISTRY: Mutex<Vec<SessionRef>> = Mutex::new(Vec::new());

/// Register a new output in the session registry
pub fn register_session_ref(
    ref_id: &str,
    tool: &str,
    contains: &str,
    summary: &str,
    size_bytes: usize,
) {
    if let Ok(mut registry) = SESSION_REGISTRY.lock() {
        // Remove any existing entry for this ref_id (in case of re-runs)
        registry.retain(|r| r.ref_id != ref_id);

        registry.push(SessionRef {
            ref_id: ref_id.to_string(),
            tool: tool.to_string(),
            contains: contains.to_string(),
            summary: summary.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            size_bytes,
        });
    }
}

/// Get all session refs for inclusion in compressed outputs
pub fn get_session_refs() -> Vec<SessionRef> {
    SESSION_REGISTRY
        .lock()
        .map(|r| r.clone())
        .unwrap_or_default()
}

/// Clear old entries from session registry (called periodically)
pub fn cleanup_session_registry() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Ok(mut registry) = SESSION_REGISTRY.lock() {
        registry.retain(|r| now - r.timestamp < MAX_AGE_SECS);
    }
}

/// Format session refs as a user-friendly string for the agent
pub fn format_session_refs_for_agent() -> String {
    let refs = get_session_refs();

    if refs.is_empty() {
        return String::new();
    }

    let mut output = String::from("\n📦 AVAILABLE DATA FOR RETRIEVAL:\n");
    output.push_str("─────────────────────────────────\n");

    for r in &refs {
        let age = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            .saturating_sub(r.timestamp);

        let age_str = if age < 60 {
            format!("{}s ago", age)
        } else {
            format!("{}m ago", age / 60)
        };

        output.push_str(&format!(
            "\n• {} [{}]\n  Contains: {}\n  Summary: {}\n  Retrieve: retrieve_output(\"{}\") or with query\n",
            r.ref_id, age_str, r.contains, r.summary, r.ref_id
        ));
    }

    output.push_str("\n─────────────────────────────────\n");
    output.push_str(
        "Query examples: \"severity:critical\", \"file:deployment.yaml\", \"code:DL3008\"\n",
    );

    output
}

/// Generate a short unique reference ID
fn generate_ref_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    // Use last 8 chars of timestamp + random suffix
    let ts_part = format!("{:x}", timestamp)
        .chars()
        .rev()
        .take(6)
        .collect::<String>();
    let rand_part: String = (0..4)
        .map(|_| {
            let idx = (timestamp as usize + rand_simple()) % 36;
            "abcdefghijklmnopqrstuvwxyz0123456789"
                .chars()
                .nth(idx)
                .unwrap()
        })
        .collect();

    format!("{}_{}", ts_part, rand_part)
}

/// Simple pseudo-random number (no external deps)
fn rand_simple() -> usize {
    let ptr = Box::into_raw(Box::new(0u8));
    let addr = ptr as usize;
    unsafe { drop(Box::from_raw(ptr)) };
    addr.wrapping_mul(1103515245).wrapping_add(12345) % (1 << 31)
}

/// Ensure output directory exists
fn ensure_output_dir() -> std::io::Result<PathBuf> {
    let path = PathBuf::from(OUTPUT_DIR);
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    Ok(path)
}

/// Store output to disk and return reference ID
///
/// # Arguments
/// * `output` - The JSON value to store
/// * `tool_name` - Name of the tool (used as prefix in ref_id)
///
/// # Returns
/// Reference ID that can be used to retrieve the output later
pub fn store_output(output: &Value, tool_name: &str) -> String {
    let ref_id = format!("{}_{}", tool_name, generate_ref_id());

    if let Ok(dir) = ensure_output_dir() {
        let path = dir.join(format!("{}.json", ref_id));

        // Store with metadata
        let stored = serde_json::json!({
            "ref_id": ref_id,
            "tool": tool_name,
            "timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            "data": output
        });

        if let Ok(json_str) = serde_json::to_string(&stored) {
            let _ = fs::write(&path, json_str);
        }
    }

    ref_id
}

/// Retrieve stored output by reference ID
///
/// # Arguments
/// * `ref_id` - The reference ID returned from `store_output`
///
/// # Returns
/// The stored JSON value, or None if not found
pub fn retrieve_output(ref_id: &str) -> Option<Value> {
    let path = PathBuf::from(OUTPUT_DIR).join(format!("{}.json", ref_id));

    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    let stored: Value = serde_json::from_str(&content).ok()?;

    // Return just the data portion
    stored.get("data").cloned()
}

/// Retrieve and filter output by query
///
/// # Arguments
/// * `ref_id` - The reference ID
/// * `query` - Optional filter query (e.g., "severity:critical", "file:path", "code:DL3008")
///
/// # Returns
/// Filtered JSON value, or None if not found
pub fn retrieve_filtered(ref_id: &str, query: Option<&str>) -> Option<Value> {
    let data = retrieve_output(ref_id)?;

    let query = match query {
        Some(q) if !q.is_empty() => q,
        _ => return Some(data),
    };

    // Parse query
    let (filter_type, filter_value) = parse_query(query);

    // Find issues/findings array in data
    let issues = find_issues_array(&data)?;

    // Filter issues
    let filtered: Vec<Value> = issues
        .iter()
        .filter(|issue| matches_filter(issue, &filter_type, &filter_value))
        .cloned()
        .collect();

    Some(serde_json::json!({
        "query": query,
        "total_matches": filtered.len(),
        "results": filtered
    }))
}

/// Parse a query string into type and value
fn parse_query(query: &str) -> (String, String) {
    if let Some(idx) = query.find(':') {
        let (t, v) = query.split_at(idx);
        (t.to_lowercase(), v[1..].to_string())
    } else {
        // Treat as general search term
        ("any".to_string(), query.to_string())
    }
}

/// Find issues/findings array in a JSON value
fn find_issues_array(data: &Value) -> Option<Vec<Value>> {
    let issue_fields = [
        "issues",
        "findings",
        "violations",
        "warnings",
        "errors",
        "recommendations",
        "results",
    ];

    for field in &issue_fields {
        if let Some(arr) = data.get(field).and_then(|v| v.as_array()) {
            return Some(arr.clone());
        }
    }

    // Check if data itself is an array
    if let Some(arr) = data.as_array() {
        return Some(arr.clone());
    }

    None
}

/// Check if an issue matches a filter
fn matches_filter(issue: &Value, filter_type: &str, filter_value: &str) -> bool {
    match filter_type {
        "severity" | "level" => {
            let sev = issue
                .get("severity")
                .or_else(|| issue.get("level"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            sev.to_lowercase().contains(&filter_value.to_lowercase())
        }
        "file" | "path" => {
            let file = issue
                .get("file")
                .or_else(|| issue.get("path"))
                .or_else(|| issue.get("filename"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            file.to_lowercase().contains(&filter_value.to_lowercase())
        }
        "code" | "rule" => {
            let code = issue
                .get("code")
                .or_else(|| issue.get("rule"))
                .or_else(|| issue.get("rule_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            code.to_lowercase().contains(&filter_value.to_lowercase())
        }
        "container" | "resource" => {
            let container = issue
                .get("container")
                .or_else(|| issue.get("resource"))
                .or_else(|| issue.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            container
                .to_lowercase()
                .contains(&filter_value.to_lowercase())
        }
        "any" | _ => {
            // Search in all string values
            let issue_str = serde_json::to_string(issue).unwrap_or_default();
            issue_str
                .to_lowercase()
                .contains(&filter_value.to_lowercase())
        }
    }
}

/// List all stored outputs
pub fn list_outputs() -> Vec<OutputInfo> {
    let dir = match ensure_output_dir() {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut outputs = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.ends_with(".json") {
                    let ref_id = filename.trim_end_matches(".json").to_string();

                    // Read metadata
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(stored) = serde_json::from_str::<Value>(&content) {
                            let tool = stored
                                .get("tool")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let timestamp = stored
                                .get("timestamp")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            let size = content.len();

                            outputs.push(OutputInfo {
                                ref_id,
                                tool,
                                timestamp,
                                size_bytes: size,
                            });
                        }
                    }
                }
            }
        }
    }

    // Sort by timestamp (newest first)
    outputs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    outputs
}

/// Information about a stored output
#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub ref_id: String,
    pub tool: String,
    pub timestamp: u64,
    pub size_bytes: usize,
}

/// Clean up old stored outputs
pub fn cleanup_old_outputs() {
    let dir = match ensure_output_dir() {
        Ok(d) => d,
        Err(_) => return,
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                if let Ok(stored) = serde_json::from_str::<Value>(&content) {
                    let timestamp = stored
                        .get("timestamp")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    if now - timestamp > MAX_AGE_SECS {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve() {
        let data = serde_json::json!({
            "issues": [
                { "code": "test1", "severity": "high", "file": "test.yaml" }
            ]
        });

        let ref_id = store_output(&data, "test_tool");
        assert!(ref_id.starts_with("test_tool_"));

        let retrieved = retrieve_output(&ref_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), data);
    }

    #[test]
    fn test_filtered_retrieval() {
        let data = serde_json::json!({
            "issues": [
                { "code": "DL3008", "severity": "warning", "file": "Dockerfile1" },
                { "code": "DL3009", "severity": "info", "file": "Dockerfile2" },
                { "code": "DL3008", "severity": "warning", "file": "Dockerfile3" }
            ]
        });

        let ref_id = store_output(&data, "filter_test");

        // Filter by code
        let filtered = retrieve_filtered(&ref_id, Some("code:DL3008"));
        assert!(filtered.is_some());
        let results = filtered.unwrap();
        assert_eq!(results["total_matches"], 2);

        // Filter by severity
        let filtered = retrieve_filtered(&ref_id, Some("severity:info"));
        assert!(filtered.is_some());
        let results = filtered.unwrap();
        assert_eq!(results["total_matches"], 1);
    }

    #[test]
    fn test_parse_query() {
        assert_eq!(
            parse_query("severity:critical"),
            ("severity".to_string(), "critical".to_string())
        );
        assert_eq!(
            parse_query("searchterm"),
            ("any".to_string(), "searchterm".to_string())
        );
    }
}
