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
/// For analyze_project outputs, supports:
/// - section:summary - Top-level info
/// - section:projects - List projects
/// - section:frameworks - All frameworks
/// - section:languages - All languages
/// - section:services - All services
/// - project:name - Specific project details
/// - service:name - Specific service
/// - language:Go - Language details
/// - framework:* - Framework details
/// - compact:true - Compacted output (default for analyze_project)
///
/// # Returns
/// Filtered JSON value, or None if not found
pub fn retrieve_filtered(ref_id: &str, query: Option<&str>) -> Option<Value> {
    let data = retrieve_output(ref_id)?;

    // Check if this is an analyze_project output
    if is_analyze_project_output(&data) {
        return retrieve_analyze_project(&data, query);
    }

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

// ============================================================================
// Smart Retrieval for different output types
// ============================================================================

/// Output type detection for smart retrieval
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    /// MonorepoAnalysis - has "projects" array and/or "is_monorepo"
    MonorepoAnalysis,
    /// ProjectAnalysis - flat structure with "languages" + "analysis_metadata"
    ProjectAnalysis,
    /// LintResult - has "failures" array (kubelint, hadolint, dclint, helmlint)
    LintResult,
    /// OptimizationResult - has "recommendations" array (k8s_optimize)
    OptimizationResult,
    /// Generic - fallback for unknown structures
    Generic,
}

/// Detect the output type for smart retrieval routing
pub fn detect_output_type(data: &Value) -> OutputType {
    // MonorepoAnalysis: has projects array or is_monorepo flag
    if data.get("projects").is_some() || data.get("is_monorepo").is_some() {
        return OutputType::MonorepoAnalysis;
    }

    // ProjectAnalysis: has languages array + analysis_metadata (flat structure)
    if data.get("languages").is_some() && data.get("analysis_metadata").is_some() {
        return OutputType::ProjectAnalysis;
    }

    // LintResult: has failures array
    if data.get("failures").is_some() {
        return OutputType::LintResult;
    }

    // OptimizationResult: has recommendations array
    if data.get("recommendations").is_some() {
        return OutputType::OptimizationResult;
    }

    OutputType::Generic
}

/// Check if data is an analyze_project output (either type)
fn is_analyze_project_output(data: &Value) -> bool {
    matches!(
        detect_output_type(data),
        OutputType::MonorepoAnalysis | OutputType::ProjectAnalysis
    )
}

/// Smart retrieval for analyze_project outputs
/// Supports queries like:
/// - section:summary - Top-level info without nested data
/// - section:projects - List project names and categories
/// - project:name - Get specific project details (compacted)
/// - service:name - Get specific service details
/// - language:Go - Get language details for a specific language
/// - framework:* - List all detected frameworks
/// - compact:true - Strip file arrays, return counts
pub fn retrieve_analyze_project(data: &Value, query: Option<&str>) -> Option<Value> {
    let query = query.unwrap_or("compact:true");
    let (query_type, query_value) = parse_query(query);

    match query_type.as_str() {
        "section" => match query_value.as_str() {
            "summary" => Some(extract_summary(data)),
            "projects" => Some(extract_projects_list(data)),
            "frameworks" => Some(extract_all_frameworks(data)),
            "languages" => Some(extract_all_languages(data)),
            "services" => Some(extract_all_services(data)),
            _ => Some(compact_analyze_output(data)),
        },
        "project" => extract_project_by_name(data, &query_value),
        "service" => extract_service_by_name(data, &query_value),
        "language" => extract_language_details(data, &query_value),
        "framework" => extract_framework_details(data, &query_value),
        "compact" => Some(compact_analyze_output(data)),
        _ => {
            // Default: return compacted output
            Some(compact_analyze_output(data))
        }
    }
}

/// Extract top-level summary without nested data
fn extract_summary(data: &Value) -> Value {
    let mut summary = serde_json::Map::new();

    // Handle MonorepoAnalysis structure
    if let Some(root) = data.get("root_path").and_then(|v| v.as_str()) {
        summary.insert("root_path".to_string(), Value::String(root.to_string()));
    }
    if let Some(mono) = data.get("is_monorepo").and_then(|v| v.as_bool()) {
        summary.insert("is_monorepo".to_string(), Value::Bool(mono));
    }

    // Handle ProjectAnalysis structure (flat)
    if let Some(root) = data.get("project_root").and_then(|v| v.as_str()) {
        summary.insert("project_root".to_string(), Value::String(root.to_string()));
    }
    if let Some(arch) = data.get("architecture_type").and_then(|v| v.as_str()) {
        summary.insert("architecture_type".to_string(), Value::String(arch.to_string()));
    }

    // Count projects (MonorepoAnalysis)
    if let Some(projects) = data.get("projects").and_then(|v| v.as_array()) {
        summary.insert("project_count".to_string(), Value::Number(projects.len().into()));

        // Extract project names
        let names: Vec<Value> = projects
            .iter()
            .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
            .map(|n| Value::String(n.to_string()))
            .collect();
        summary.insert("project_names".to_string(), Value::Array(names));
    }

    // Extract languages (ProjectAnalysis flat structure)
    if let Some(languages) = data.get("languages").and_then(|v| v.as_array()) {
        let names: Vec<Value> = languages
            .iter()
            .filter_map(|l| l.get("name").and_then(|n| n.as_str()))
            .map(|n| Value::String(n.to_string()))
            .collect();
        summary.insert("languages".to_string(), Value::Array(names));
    }

    // Extract technologies (ProjectAnalysis flat structure)
    if let Some(techs) = data.get("technologies").and_then(|v| v.as_array()) {
        let names: Vec<Value> = techs
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .map(|n| Value::String(n.to_string()))
            .collect();
        summary.insert("technologies".to_string(), Value::Array(names));
    }

    // Extract services (ProjectAnalysis flat structure) - include names, not just count
    if let Some(services) = data.get("services").and_then(|v| v.as_array()) {
        summary.insert("services_count".to_string(), Value::Number(services.len().into()));
        // Include service names so agent knows what microservices exist
        let service_names: Vec<Value> = services
            .iter()
            .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
            .map(|n| Value::String(n.to_string()))
            .collect();
        if !service_names.is_empty() {
            summary.insert("services".to_string(), Value::Array(service_names));
        }
    }

    Value::Object(summary)
}

/// Extract list of projects with basic info (no file arrays)
fn extract_projects_list(data: &Value) -> Value {
    let projects = data.get("projects").and_then(|v| v.as_array());

    let list: Vec<Value> = projects
        .map(|arr| {
            arr.iter()
                .map(|p| {
                    let mut proj = serde_json::Map::new();
                    if let Some(name) = p.get("name") {
                        proj.insert("name".to_string(), name.clone());
                    }
                    if let Some(path) = p.get("path") {
                        proj.insert("path".to_string(), path.clone());
                    }
                    if let Some(cat) = p.get("project_category") {
                        proj.insert("category".to_string(), cat.clone());
                    }
                    // Add language/framework counts
                    if let Some(analysis) = p.get("analysis") {
                        if let Some(langs) = analysis.get("languages").and_then(|v| v.as_array()) {
                            let lang_names: Vec<Value> = langs
                                .iter()
                                .filter_map(|l| l.get("name").and_then(|n| n.as_str()))
                                .map(|n| Value::String(n.to_string()))
                                .collect();
                            proj.insert("languages".to_string(), Value::Array(lang_names));
                        }
                        if let Some(fws) = analysis.get("frameworks").and_then(|v| v.as_array()) {
                            let fw_names: Vec<Value> = fws
                                .iter()
                                .filter_map(|f| f.get("name").and_then(|n| n.as_str()))
                                .map(|n| Value::String(n.to_string()))
                                .collect();
                            proj.insert("frameworks".to_string(), Value::Array(fw_names));
                        }
                    }
                    Value::Object(proj)
                })
                .collect()
        })
        .unwrap_or_default();

    serde_json::json!({
        "total_projects": list.len(),
        "projects": list
    })
}

/// Extract specific project by name
fn extract_project_by_name(data: &Value, name: &str) -> Option<Value> {
    let projects = data.get("projects").and_then(|v| v.as_array())?;

    let project = projects.iter().find(|p| {
        p.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n.to_lowercase().contains(&name.to_lowercase()))
            .unwrap_or(false)
    })?;

    Some(compact_project(project))
}

/// Extract specific service by name
fn extract_service_by_name(data: &Value, name: &str) -> Option<Value> {
    let projects = data.get("projects").and_then(|v| v.as_array())?;

    for project in projects {
        if let Some(services) = project
            .get("analysis")
            .and_then(|a| a.get("services"))
            .and_then(|s| s.as_array())
        {
            if let Some(service) = services.iter().find(|s| {
                s.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.to_lowercase().contains(&name.to_lowercase()))
                    .unwrap_or(false)
            }) {
                return Some(service.clone());
            }
        }
    }
    None
}

/// Extract language detection details (with file count instead of file list)
fn extract_language_details(data: &Value, lang_name: &str) -> Option<Value> {
    let mut results = Vec::new();

    // Helper to process a languages array
    let process_languages = |languages: &[Value], proj_name: &str, results: &mut Vec<Value>| {
        for lang in languages {
            let name = lang.get("name").and_then(|n| n.as_str()).unwrap_or("");
            if lang_name == "*" || name.to_lowercase().contains(&lang_name.to_lowercase()) {
                let mut compact_lang = serde_json::Map::new();
                if !proj_name.is_empty() {
                    compact_lang.insert("project".to_string(), Value::String(proj_name.to_string()));
                }
                compact_lang.insert("name".to_string(), lang.get("name").cloned().unwrap_or(Value::Null));
                compact_lang.insert("version".to_string(), lang.get("version").cloned().unwrap_or(Value::Null));
                compact_lang.insert("confidence".to_string(), lang.get("confidence").cloned().unwrap_or(Value::Null));

                // Replace file array with count
                if let Some(files) = lang.get("files").and_then(|f| f.as_array()) {
                    compact_lang.insert("file_count".to_string(), Value::Number(files.len().into()));
                }

                results.push(Value::Object(compact_lang));
            }
        }
    };

    // Handle ProjectAnalysis flat structure (languages at top level)
    if let Some(languages) = data.get("languages").and_then(|v| v.as_array()) {
        process_languages(languages, "", &mut results);
    }

    // Handle MonorepoAnalysis structure (languages nested in projects)
    if let Some(projects) = data.get("projects").and_then(|v| v.as_array()) {
        for project in projects {
            let proj_name = project
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");

            if let Some(languages) = project
                .get("analysis")
                .and_then(|a| a.get("languages"))
                .and_then(|l| l.as_array())
            {
                process_languages(languages, proj_name, &mut results);
            }
        }
    }

    Some(serde_json::json!({
        "query": format!("language:{}", lang_name),
        "total_matches": results.len(),
        "results": results
    }))
}

/// Extract framework/technology details
fn extract_framework_details(data: &Value, fw_name: &str) -> Option<Value> {
    let mut results = Vec::new();

    // Helper to process a frameworks/technologies array
    let process_techs = |techs: &[Value], proj_name: &str, results: &mut Vec<Value>| {
        for tech in techs {
            let name = tech.get("name").and_then(|n| n.as_str()).unwrap_or("");
            if fw_name == "*" || name.to_lowercase().contains(&fw_name.to_lowercase()) {
                let mut compact_fw = serde_json::Map::new();
                if !proj_name.is_empty() {
                    compact_fw.insert("project".to_string(), Value::String(proj_name.to_string()));
                }
                if let Some(v) = tech.get("name") {
                    compact_fw.insert("name".to_string(), v.clone());
                }
                if let Some(v) = tech.get("version") {
                    compact_fw.insert("version".to_string(), v.clone());
                }
                if let Some(v) = tech.get("category") {
                    compact_fw.insert("category".to_string(), v.clone());
                }
                results.push(Value::Object(compact_fw));
            }
        }
    };

    // Handle ProjectAnalysis flat structure (technologies at top level)
    if let Some(techs) = data.get("technologies").and_then(|v| v.as_array()) {
        process_techs(techs, "", &mut results);
    }

    // Also check frameworks field (deprecated but may exist)
    if let Some(fws) = data.get("frameworks").and_then(|v| v.as_array()) {
        process_techs(fws, "", &mut results);
    }

    // Handle MonorepoAnalysis structure (frameworks nested in projects)
    if let Some(projects) = data.get("projects").and_then(|v| v.as_array()) {
        for project in projects {
            let proj_name = project
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");

            if let Some(frameworks) = project
                .get("analysis")
                .and_then(|a| a.get("frameworks"))
                .and_then(|f| f.as_array())
            {
                process_techs(frameworks, proj_name, &mut results);
            }
        }
    }

    Some(serde_json::json!({
        "query": format!("framework:{}", fw_name),
        "total_matches": results.len(),
        "results": results
    }))
}

/// Extract all frameworks across all projects
fn extract_all_frameworks(data: &Value) -> Value {
    extract_framework_details(data, "*").unwrap_or(serde_json::json!({"results": []}))
}

/// Extract all languages across all projects
fn extract_all_languages(data: &Value) -> Value {
    extract_language_details(data, "*").unwrap_or(serde_json::json!({"results": []}))
}

/// Extract all services across all projects
fn extract_all_services(data: &Value) -> Value {
    let mut services = Vec::new();

    // Helper to extract compact service info from a ServiceAnalysis
    let compact_service = |svc: &Value, project_name: Option<&str>| -> Value {
        let mut svc_info = serde_json::Map::new();

        // Add project name if from monorepo
        if let Some(proj) = project_name {
            svc_info.insert("project".to_string(), Value::String(proj.to_string()));
        }

        // Core fields
        if let Some(v) = svc.get("name") {
            svc_info.insert("name".to_string(), v.clone());
        }
        if let Some(v) = svc.get("path") {
            svc_info.insert("path".to_string(), v.clone());
        }
        if let Some(v) = svc.get("service_type") {
            svc_info.insert("service_type".to_string(), v.clone());
        }

        // Extract language names (compact)
        if let Some(langs) = svc.get("languages").and_then(|l| l.as_array()) {
            let lang_names: Vec<Value> = langs
                .iter()
                .filter_map(|l| l.get("name").and_then(|n| n.as_str()))
                .map(|n| Value::String(n.to_string()))
                .collect();
            if !lang_names.is_empty() {
                svc_info.insert("languages".to_string(), Value::Array(lang_names));
            }
        }

        // Extract technology names (compact)
        if let Some(techs) = svc.get("technologies").and_then(|t| t.as_array()) {
            let tech_names: Vec<Value> = techs
                .iter()
                .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
                .map(|n| Value::String(n.to_string()))
                .collect();
            if !tech_names.is_empty() {
                svc_info.insert("technologies".to_string(), Value::Array(tech_names));
            }
        }

        // Extract ports
        if let Some(ports) = svc.get("ports").and_then(|p| p.as_array()) {
            let port_list: Vec<Value> = ports
                .iter()
                .filter_map(|p| p.get("port").and_then(|n| n.as_u64()))
                .map(|n| Value::Number(n.into()))
                .collect();
            if !port_list.is_empty() {
                svc_info.insert("ports".to_string(), Value::Array(port_list));
            }
        }

        Value::Object(svc_info)
    };

    // Handle ProjectAnalysis flat structure (services at top level)
    if let Some(svcs) = data.get("services").and_then(|s| s.as_array()) {
        for svc in svcs {
            services.push(compact_service(svc, None));
        }
    }

    // Handle MonorepoAnalysis structure (services nested in projects)
    if let Some(projects) = data.get("projects").and_then(|v| v.as_array()) {
        for project in projects {
            let proj_name = project
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");

            if let Some(svcs) = project
                .get("analysis")
                .and_then(|a| a.get("services"))
                .and_then(|s| s.as_array())
            {
                for svc in svcs {
                    services.push(compact_service(svc, Some(proj_name)));
                }
            }
        }
    }

    serde_json::json!({
        "total_services": services.len(),
        "services": services
    })
}

/// Compact entire analyze_project output (strip file arrays)
fn compact_analyze_output(data: &Value) -> Value {
    let mut result = serde_json::Map::new();

    // Handle MonorepoAnalysis structure
    if let Some(v) = data.get("root_path") {
        result.insert("root_path".to_string(), v.clone());
    }
    if let Some(v) = data.get("is_monorepo") {
        result.insert("is_monorepo".to_string(), v.clone());
    }

    // Compact projects (MonorepoAnalysis)
    if let Some(projects) = data.get("projects").and_then(|v| v.as_array()) {
        let compacted: Vec<Value> = projects.iter().map(|p| compact_project(p)).collect();
        result.insert("projects".to_string(), Value::Array(compacted));
        return Value::Object(result);
    }

    // Handle ProjectAnalysis flat structure
    if let Some(v) = data.get("project_root") {
        result.insert("project_root".to_string(), v.clone());
    }
    if let Some(v) = data.get("architecture_type") {
        result.insert("architecture_type".to_string(), v.clone());
    }
    if let Some(v) = data.get("project_type") {
        result.insert("project_type".to_string(), v.clone());
    }

    // Compact languages (replace files array with count)
    if let Some(languages) = data.get("languages").and_then(|v| v.as_array()) {
        let compacted: Vec<Value> = languages
            .iter()
            .map(|lang| {
                let mut compact_lang = serde_json::Map::new();
                for key in &["name", "version", "confidence"] {
                    if let Some(v) = lang.get(*key) {
                        compact_lang.insert(key.to_string(), v.clone());
                    }
                }
                // Replace files array with count
                if let Some(files) = lang.get("files").and_then(|f| f.as_array()) {
                    compact_lang.insert("file_count".to_string(), Value::Number(files.len().into()));
                }
                Value::Object(compact_lang)
            })
            .collect();
        result.insert("languages".to_string(), Value::Array(compacted));
    }

    // Include technologies (usually not huge)
    if let Some(techs) = data.get("technologies").and_then(|v| v.as_array()) {
        let compacted: Vec<Value> = techs
            .iter()
            .map(|tech| {
                let mut compact_tech = serde_json::Map::new();
                for key in &["name", "version", "category", "confidence"] {
                    if let Some(v) = tech.get(*key) {
                        compact_tech.insert(key.to_string(), v.clone());
                    }
                }
                Value::Object(compact_tech)
            })
            .collect();
        result.insert("technologies".to_string(), Value::Array(compacted));
    }

    // Include services (usually small)
    if let Some(services) = data.get("services").and_then(|v| v.as_array()) {
        result.insert("services".to_string(), Value::Array(services.clone()));
    }

    // Include analysis_metadata
    if let Some(meta) = data.get("analysis_metadata") {
        result.insert("analysis_metadata".to_string(), meta.clone());
    }

    Value::Object(result)
}

/// Compact a single project (strip file arrays, replace with counts)
fn compact_project(project: &Value) -> Value {
    let mut compact = serde_json::Map::new();

    // Copy basic fields
    for key in &["name", "path", "project_category"] {
        if let Some(v) = project.get(*key) {
            compact.insert(key.to_string(), v.clone());
        }
    }

    // Compact analysis
    if let Some(analysis) = project.get("analysis") {
        let mut compact_analysis = serde_json::Map::new();

        // Copy project_root
        if let Some(v) = analysis.get("project_root") {
            compact_analysis.insert("project_root".to_string(), v.clone());
        }

        // Compact languages (strip files, add file_count)
        if let Some(languages) = analysis.get("languages").and_then(|v| v.as_array()) {
            let compacted: Vec<Value> = languages
                .iter()
                .map(|lang| {
                    let mut compact_lang = serde_json::Map::new();
                    for key in &["name", "version", "confidence"] {
                        if let Some(v) = lang.get(*key) {
                            compact_lang.insert(key.to_string(), v.clone());
                        }
                    }
                    // Replace files array with count
                    if let Some(files) = lang.get("files").and_then(|f| f.as_array()) {
                        compact_lang.insert("file_count".to_string(), Value::Number(files.len().into()));
                    }
                    Value::Object(compact_lang)
                })
                .collect();
            compact_analysis.insert("languages".to_string(), Value::Array(compacted));
        }

        // Copy frameworks, databases, services as-is (usually not huge)
        for key in &["frameworks", "databases", "services", "build_tools", "package_managers"] {
            if let Some(v) = analysis.get(*key) {
                compact_analysis.insert(key.to_string(), v.clone());
            }
        }

        compact.insert("analysis".to_string(), Value::Object(compact_analysis));
    }

    Value::Object(compact)
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

    #[test]
    fn test_analyze_project_detection() {
        let analyze_data = serde_json::json!({
            "root_path": "/test",
            "is_monorepo": true,
            "projects": []
        });
        assert!(is_analyze_project_output(&analyze_data));

        let lint_data = serde_json::json!({
            "issues": [{ "code": "DL3008" }]
        });
        assert!(!is_analyze_project_output(&lint_data));
    }

    #[test]
    fn test_analyze_project_summary() {
        let data = serde_json::json!({
            "root_path": "/test/monorepo",
            "is_monorepo": true,
            "projects": [
                { "name": "api-gateway", "path": "services/api" },
                { "name": "web-app", "path": "apps/web" }
            ]
        });

        let summary = extract_summary(&data);
        assert_eq!(summary["root_path"], "/test/monorepo");
        assert_eq!(summary["is_monorepo"], true);
        assert_eq!(summary["project_count"], 2);
    }

    #[test]
    fn test_analyze_project_compact() {
        // Simulates massive analyze_project output with 1000s of files
        let files: Vec<String> = (0..1000).map(|i| format!("/src/file{}.ts", i)).collect();

        let data = serde_json::json!({
            "root_path": "/test",
            "is_monorepo": false,
            "projects": [{
                "name": "test-project",
                "path": "",
                "project_category": "Api",
                "analysis": {
                    "project_root": "/test",
                    "languages": [{
                        "name": "TypeScript",
                        "version": "5.0",
                        "confidence": 0.95,
                        "files": files
                    }],
                    "frameworks": [{
                        "name": "React",
                        "version": "18.0"
                    }]
                }
            }]
        });

        let ref_id = store_output(&data, "analyze_project_test");

        // Default retrieval should return compacted output
        let result = retrieve_filtered(&ref_id, None);
        assert!(result.is_some());

        let compacted = result.unwrap();

        // Verify files array was replaced with file_count
        let project = &compacted["projects"][0];
        let lang = &project["analysis"]["languages"][0];
        assert_eq!(lang["name"], "TypeScript");
        assert_eq!(lang["file_count"], 1000);
        assert!(lang.get("files").is_none()); // No files array

        // The compacted JSON should be much smaller
        let compacted_str = serde_json::to_string(&compacted).unwrap();
        let original_str = serde_json::to_string(&data).unwrap();
        assert!(compacted_str.len() < original_str.len() / 10); // At least 10x smaller
    }

    #[test]
    fn test_analyze_project_section_queries() {
        let data = serde_json::json!({
            "root_path": "/test",
            "is_monorepo": true,
            "projects": [{
                "name": "api-service",
                "path": "services/api",
                "project_category": "Api",
                "analysis": {
                    "languages": [{
                        "name": "Go",
                        "version": "1.21",
                        "confidence": 0.9,
                        "files": ["/main.go", "/handler.go"]
                    }],
                    "frameworks": [{
                        "name": "Gin",
                        "version": "1.9",
                        "category": "Web"
                    }],
                    "services": [{
                        "name": "api-http",
                        "type": "http",
                        "port": 8080
                    }]
                }
            }]
        });

        let ref_id = store_output(&data, "analyze_query_test");

        // Test section:projects
        let projects = retrieve_filtered(&ref_id, Some("section:projects"));
        assert!(projects.is_some());
        assert_eq!(projects.as_ref().unwrap()["total_projects"], 1);

        // Test section:frameworks
        let frameworks = retrieve_filtered(&ref_id, Some("section:frameworks"));
        assert!(frameworks.is_some());
        assert_eq!(frameworks.as_ref().unwrap()["total_matches"], 1);
        assert_eq!(frameworks.as_ref().unwrap()["results"][0]["name"], "Gin");

        // Test section:languages
        let languages = retrieve_filtered(&ref_id, Some("section:languages"));
        assert!(languages.is_some());
        assert_eq!(languages.as_ref().unwrap()["total_matches"], 1);
        assert_eq!(languages.as_ref().unwrap()["results"][0]["name"], "Go");
        // Files should be replaced with count
        assert_eq!(languages.as_ref().unwrap()["results"][0]["file_count"], 2);

        // Test language:Go specific query
        let go = retrieve_filtered(&ref_id, Some("language:Go"));
        assert!(go.is_some());
        assert_eq!(go.as_ref().unwrap()["total_matches"], 1);

        // Test framework:Gin specific query
        let gin = retrieve_filtered(&ref_id, Some("framework:Gin"));
        assert!(gin.is_some());
        assert_eq!(gin.as_ref().unwrap()["total_matches"], 1);
    }
}
