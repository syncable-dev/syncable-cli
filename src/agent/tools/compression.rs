//! Smart Context Compression for Tool Outputs
//!
//! Implements multi-layer semantic compression with RAG retrieval pattern:
//! 1. Semantic Deduplication - Group identical patterns
//! 2. Importance-Weighted Output - Critical=full, Low=counts
//! 3. Hierarchical Summaries - Multi-level detail
//! 4. RAG Pattern - Store full data, return summary with retrieval reference

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

use super::output_store;

/// Severity levels for importance-weighted filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "critical" | "error" => Severity::Critical,
            "high" | "warning" => Severity::High,
            "medium" => Severity::Medium,
            "low" | "hint" => Severity::Low,
            _ => Severity::Info,
        }
    }
}

/// A deduplicated pattern representing multiple similar issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicatedPattern {
    /// The issue code/type (e.g., "no-resource-limits", "DL3008")
    pub code: String,
    /// Number of occurrences
    pub count: usize,
    /// Severity level
    pub severity: Severity,
    /// Brief description of the issue
    pub message: String,
    /// List of affected files (truncated if too many)
    pub affected_files: Vec<String>,
    /// One full example for context
    pub example: Option<Value>,
    /// Suggested fix template
    pub fix_template: Option<String>,
}

/// Compressed output ready for LLM context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedOutput {
    /// Tool that generated this output
    pub tool: String,
    /// Overall status
    pub status: String,
    /// Summary counts by severity
    pub summary: SeveritySummary,
    /// Critical issues - always shown in full
    pub critical_issues: Vec<Value>,
    /// High severity issues - shown in full if few, otherwise patterns
    pub high_issues: Vec<Value>,
    /// Deduplicated patterns for medium/low issues
    pub patterns: Vec<DeduplicatedPattern>,
    /// Reference ID for retrieving full data
    pub full_data_ref: String,
    /// Hint for agent on how to retrieve more details
    pub retrieval_hint: String,
}

/// Summary counts by severity level
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeveritySummary {
    pub total: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

/// Configuration for compression behavior
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Maximum high-severity issues to show in full (default: 10)
    pub max_high_full: usize,
    /// Maximum files to list per pattern (default: 5)
    pub max_files_per_pattern: usize,
    /// Target output size in bytes (default: 15KB)
    pub target_size_bytes: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            max_high_full: 10,
            max_files_per_pattern: 5,
            target_size_bytes: 15_000,
        }
    }
}

/// Main compression function - compresses tool output and stores full data for retrieval
///
/// # Arguments
/// * `output` - The raw JSON output from a tool
/// * `tool_name` - Name of the tool (e.g., "kubelint", "k8s_optimize")
/// * `config` - Compression configuration
///
/// # Returns
/// JSON string of compressed output, or original if compression not applicable
pub fn compress_tool_output(output: &Value, tool_name: &str, config: &CompressionConfig) -> String {
    // Check if output is small enough - no compression needed
    let raw_str = serde_json::to_string(output).unwrap_or_default();
    if raw_str.len() <= config.target_size_bytes {
        return raw_str;
    }

    // Store full output for later retrieval
    let ref_id = output_store::store_output(output, tool_name);

    // Extract issues/findings array from the output
    let issues = extract_issues(output);

    if issues.is_empty() {
        // Register in session with description
        let contains = format!("{} analysis data (no issues)", tool_name);
        output_store::register_session_ref(
            &ref_id,
            tool_name,
            &contains,
            "0 issues",
            raw_str.len(),
        );

        // No issues to compress, just store and return summary
        let mut result = serde_json::to_string_pretty(&json!({
            "tool": tool_name,
            "status": "NO_ISSUES",
            "summary": { "total": 0 },
            "full_data_ref": ref_id,
            "retrieval_hint": format!("Use retrieve_output('{}') for full analysis data", ref_id)
        }))
        .unwrap_or(raw_str.clone());

        // Append ALL session refs so agent always knows what's available
        result.push_str(&output_store::format_session_refs_for_agent());
        return result;
    }

    // Classify issues by severity
    let (critical, high, medium, low, info) = classify_by_severity(&issues);

    // Build summary
    let summary = SeveritySummary {
        total: issues.len(),
        critical: critical.len(),
        high: high.len(),
        medium: medium.len(),
        low: low.len(),
        info: info.len(),
    };

    // Critical issues: always full detail
    let critical_issues: Vec<Value> = critical.clone();

    // High issues: full detail if few, otherwise deduplicate
    let high_issues: Vec<Value> = if high.len() <= config.max_high_full {
        high.clone()
    } else {
        // Show first few + pattern for rest
        high.iter().take(config.max_high_full).cloned().collect()
    };

    // Deduplicate medium/low/info issues into patterns
    let mut all_lower: Vec<Value> = Vec::new();
    all_lower.extend(medium.clone());
    all_lower.extend(low.clone());
    all_lower.extend(info.clone());

    // Also add remaining high issues if there were too many
    if high.len() > config.max_high_full {
        all_lower.extend(high.iter().skip(config.max_high_full).cloned());
    }

    let patterns = deduplicate_to_patterns(&all_lower, config);

    // Determine status
    let status = if summary.critical > 0 {
        "CRITICAL_ISSUES_FOUND"
    } else if summary.high > 0 {
        "HIGH_ISSUES_FOUND"
    } else if summary.total > 0 {
        "ISSUES_FOUND"
    } else {
        "CLEAN"
    };

    // Register in session registry with meaningful description
    let contains = match tool_name {
        "kubelint" => "Kubernetes manifest lint issues (security, best practices)",
        "k8s_optimize" => "K8s resource optimization recommendations",
        "analyze" => "Project analysis (languages, frameworks, dependencies)",
        _ => "Tool analysis results",
    };
    let summary_str = format!(
        "{} issues: {} critical, {} high, {} medium",
        summary.total, summary.critical, summary.high, summary.medium
    );
    output_store::register_session_ref(&ref_id, tool_name, contains, &summary_str, raw_str.len());

    let compressed = CompressedOutput {
        tool: tool_name.to_string(),
        status: status.to_string(),
        summary,
        critical_issues,
        high_issues,
        patterns,
        full_data_ref: ref_id.clone(),
        retrieval_hint: format!(
            "Use retrieve_output('{}', query) to get full details. Query options: 'severity:critical', 'file:path', 'code:DL3008'",
            ref_id
        ),
    };

    let mut result = serde_json::to_string_pretty(&compressed).unwrap_or(raw_str);

    // Append ALL session refs so agent always knows what's available
    result.push_str(&output_store::format_session_refs_for_agent());
    result
}

/// Extract issues/findings array from various output formats
fn extract_issues(output: &Value) -> Vec<Value> {
    // Try common field names for issues/findings
    let issue_fields = [
        "issues",
        "findings",
        "violations",
        "warnings",
        "errors",
        "recommendations",
        "results",
        "diagnostics",
    ];

    for field in &issue_fields {
        if let Some(arr) = output.get(field).and_then(|v| v.as_array()) {
            return arr.clone();
        }
    }

    // Check if output itself is an array
    if let Some(arr) = output.as_array() {
        return arr.clone();
    }

    // Try nested structures
    if let Some(obj) = output.as_object() {
        for (_, v) in obj {
            if let Some(arr) = v.as_array() {
                if !arr.is_empty() && is_issue_like(&arr[0]) {
                    return arr.clone();
                }
            }
        }
    }

    Vec::new()
}

/// Check if a value looks like an issue/finding
fn is_issue_like(value: &Value) -> bool {
    if let Some(obj) = value.as_object() {
        // Issues typically have severity, code, message, or file fields
        obj.contains_key("severity")
            || obj.contains_key("code")
            || obj.contains_key("message")
            || obj.contains_key("rule")
            || obj.contains_key("level")
    } else {
        false
    }
}

/// Classify issues by severity level
fn classify_by_severity(
    issues: &[Value],
) -> (Vec<Value>, Vec<Value>, Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut critical = Vec::new();
    let mut high = Vec::new();
    let mut medium = Vec::new();
    let mut low = Vec::new();
    let mut info = Vec::new();

    for issue in issues {
        let severity = get_severity(issue);
        match severity {
            Severity::Critical => critical.push(issue.clone()),
            Severity::High => high.push(issue.clone()),
            Severity::Medium => medium.push(issue.clone()),
            Severity::Low => low.push(issue.clone()),
            Severity::Info => info.push(issue.clone()),
        }
    }

    (critical, high, medium, low, info)
}

/// Extract severity from an issue value
fn get_severity(issue: &Value) -> Severity {
    // Try common severity field names
    let severity_fields = ["severity", "level", "priority", "type"];

    for field in &severity_fields {
        if let Some(s) = issue.get(field).and_then(|v| v.as_str()) {
            return Severity::from_str(s);
        }
    }

    // Check for error/warning in code field
    if let Some(code) = issue.get("code").and_then(|v| v.as_str()) {
        if code.to_lowercase().contains("error") {
            return Severity::Critical;
        }
        if code.to_lowercase().contains("warn") {
            return Severity::High;
        }
    }

    Severity::Medium // Default
}

/// Get issue code/type for deduplication grouping
fn get_issue_code(issue: &Value) -> String {
    // Try common code field names
    let code_fields = ["code", "rule", "rule_id", "type", "check", "id"];

    for field in &code_fields {
        if let Some(s) = issue.get(field).and_then(|v| v.as_str()) {
            return s.to_string();
        }
    }

    // Fall back to message hash
    if let Some(msg) = issue.get("message").and_then(|v| v.as_str()) {
        return format!("msg:{}", &msg[..msg.len().min(30)]);
    }

    "unknown".to_string()
}

/// Get file path from an issue
fn get_issue_file(issue: &Value) -> Option<String> {
    let file_fields = ["file", "path", "filename", "location", "source"];

    for field in &file_fields {
        if let Some(s) = issue.get(field).and_then(|v| v.as_str()) {
            return Some(s.to_string());
        }
        // Handle nested location objects
        if let Some(loc) = issue.get(field).and_then(|v| v.as_object()) {
            if let Some(f) = loc.get("file").and_then(|v| v.as_str()) {
                return Some(f.to_string());
            }
        }
    }

    None
}

/// Get message from an issue
fn get_issue_message(issue: &Value) -> String {
    let msg_fields = ["message", "msg", "description", "text", "detail"];

    for field in &msg_fields {
        if let Some(s) = issue.get(field).and_then(|v| v.as_str()) {
            return s.to_string();
        }
    }

    "No message".to_string()
}

/// Deduplicate issues into patterns
fn deduplicate_to_patterns(
    issues: &[Value],
    config: &CompressionConfig,
) -> Vec<DeduplicatedPattern> {
    // Group by issue code
    let mut groups: HashMap<String, Vec<&Value>> = HashMap::new();

    for issue in issues {
        let code = get_issue_code(issue);
        groups.entry(code).or_default().push(issue);
    }

    // Convert groups to patterns
    let mut patterns: Vec<DeduplicatedPattern> = groups
        .into_iter()
        .map(|(code, group)| {
            let first = group[0];
            let severity = get_severity(first);
            let message = get_issue_message(first);

            // Collect affected files
            let mut files: Vec<String> = group.iter().filter_map(|i| get_issue_file(i)).collect();
            files.dedup();

            let total_files = files.len();
            let truncated_files: Vec<String> = if files.len() > config.max_files_per_pattern {
                let mut truncated: Vec<String> = files
                    .iter()
                    .take(config.max_files_per_pattern)
                    .cloned()
                    .collect();
                truncated.push(format!(
                    "...+{} more",
                    total_files - config.max_files_per_pattern
                ));
                truncated
            } else {
                files
            };

            // Extract fix template if available
            let fix_template = first
                .get("fix")
                .or_else(|| first.get("suggestion"))
                .or_else(|| first.get("recommendation"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            DeduplicatedPattern {
                code,
                count: group.len(),
                severity,
                message,
                affected_files: truncated_files,
                example: if group.len() > 1 {
                    Some(first.clone())
                } else {
                    None
                },
                fix_template,
            }
        })
        .collect();

    // Sort by severity (critical first) then by count
    patterns.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| b.count.cmp(&a.count))
    });

    patterns
}

/// Compress analyze_project output specifically
pub fn compress_analysis_output(output: &Value, config: &CompressionConfig) -> String {
    let raw_str = serde_json::to_string(output).unwrap_or_default();
    if raw_str.len() <= config.target_size_bytes {
        return raw_str;
    }

    // Store full output
    let ref_id = output_store::store_output(output, "analyze");

    // Extract key summary fields
    let mut compressed = json!({
        "tool": "analyze_project",
        "full_data_ref": ref_id,
        "retrieval_hint": format!("Use retrieve_output('{}') for full analysis", ref_id)
    });

    if let Some(obj) = output.as_object() {
        let compressed_obj = compressed.as_object_mut().unwrap();

        // Always include these summary fields
        let summary_fields = [
            "name",
            "languages",
            "frameworks",
            "build_tools",
            "package_managers",
            "ci_cd",
            "containerization",
            "cloud_providers",
            "databases",
        ];

        for field in &summary_fields {
            if let Some(v) = obj.get(*field) {
                // For arrays, include count + first few items
                if let Some(arr) = v.as_array() {
                    if arr.len() > 5 {
                        let truncated: Vec<Value> = arr.iter().take(5).cloned().collect();
                        compressed_obj.insert(
                            field.to_string(),
                            json!({
                                "items": truncated,
                                "total": arr.len(),
                                "note": format!("+{} more (use retrieve_output)", arr.len() - 5)
                            }),
                        );
                    } else {
                        compressed_obj.insert(field.to_string(), v.clone());
                    }
                } else {
                    compressed_obj.insert(field.to_string(), v.clone());
                }
            }
        }

        // Handle dependencies specially - just counts
        if let Some(deps) = obj.get("dependencies") {
            if let Some(deps_obj) = deps.as_object() {
                let mut dep_summary = json!({});
                for (lang, dep_list) in deps_obj {
                    if let Some(arr) = dep_list.as_array() {
                        dep_summary[lang] = json!({ "count": arr.len() });
                    }
                }
                compressed_obj.insert("dependencies_summary".to_string(), dep_summary);
            }
        }

        // Handle file structure - depth-limited
        if let Some(structure) = obj.get("structure") {
            compressed_obj.insert(
                "structure_note".to_string(),
                json!("Full structure available via retrieve_output"),
            );
            // Include just top-level directories
            if let Some(dirs) = structure.get("directories").and_then(|v| v.as_array()) {
                let top_dirs: Vec<&str> = dirs
                    .iter()
                    .filter_map(|v| v.as_str())
                    .filter(|s| !s.contains('/') || s.matches('/').count() == 1)
                    .take(10)
                    .collect();
                compressed_obj.insert("top_directories".to_string(), json!(top_dirs));
            }
        }
    }

    // Build summary for session registry
    let summary_parts: Vec<String> = output
        .as_object()
        .map(|obj| {
            let mut parts = Vec::new();
            if let Some(langs) = obj.get("languages").and_then(|v| v.as_array()) {
                parts.push(format!("{} languages", langs.len()));
            }
            if let Some(fws) = obj.get("frameworks").and_then(|v| v.as_array()) {
                parts.push(format!("{} frameworks", fws.len()));
            }
            parts
        })
        .unwrap_or_default();
    let summary_str = if summary_parts.is_empty() {
        "Project structure and dependencies".to_string()
    } else {
        summary_parts.join(", ")
    };

    // Register in session registry
    output_store::register_session_ref(
        &ref_id,
        "analyze",
        "Project analysis (languages, frameworks, dependencies, structure)",
        &summary_str,
        raw_str.len(),
    );

    let mut result = serde_json::to_string_pretty(&compressed).unwrap_or(raw_str);

    // Append ALL session refs so agent always knows what's available
    result.push_str(&output_store::format_session_refs_for_agent());
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);
    }

    #[test]
    fn test_extract_issues_from_array_field() {
        let output = json!({
            "issues": [
                { "code": "DL3008", "severity": "warning", "message": "Pin versions" },
                { "code": "DL3009", "severity": "info", "message": "Delete apt lists" }
            ]
        });

        let issues = extract_issues(&output);
        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn test_deduplication() {
        let issues = vec![
            json!({ "code": "DL3008", "severity": "warning", "file": "Dockerfile1" }),
            json!({ "code": "DL3008", "severity": "warning", "file": "Dockerfile2" }),
            json!({ "code": "DL3008", "severity": "warning", "file": "Dockerfile3" }),
            json!({ "code": "DL3009", "severity": "info", "file": "Dockerfile1" }),
        ];

        let config = CompressionConfig::default();
        let patterns = deduplicate_to_patterns(&issues, &config);

        assert_eq!(patterns.len(), 2);

        let dl3008 = patterns.iter().find(|p| p.code == "DL3008").unwrap();
        assert_eq!(dl3008.count, 3);
        assert_eq!(dl3008.affected_files.len(), 3);
    }

    #[test]
    fn test_small_output_not_compressed() {
        let small_output = json!({
            "issues": [
                { "code": "test", "severity": "low" }
            ]
        });

        let config = CompressionConfig {
            target_size_bytes: 10000,
            ..Default::default()
        };

        let result = compress_tool_output(&small_output, "test", &config);
        // Should return original (not compressed) since it's small
        assert!(!result.contains("full_data_ref"));
    }
}
