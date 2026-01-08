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
        "failures", // LintResult from kubelint, hadolint, dclint, helmlint
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
///
/// Handles both:
/// - MonorepoAnalysis: has "projects" array, "is_monorepo", "root_path"
/// - ProjectAnalysis: flat structure with "languages", "technologies" at top level
///
/// For large analysis, returns a minimal summary and stores full data for retrieval.
pub fn compress_analysis_output(output: &Value, config: &CompressionConfig) -> String {
    let raw_str = serde_json::to_string(output).unwrap_or_default();
    if raw_str.len() <= config.target_size_bytes {
        return raw_str;
    }

    // Store full output for later retrieval
    let ref_id = output_store::store_output(output, "analyze_project");

    // Build a MINIMAL summary - just enough to understand the project
    let mut summary = json!({
        "tool": "analyze_project",
        "status": "ANALYSIS_COMPLETE",
        "full_data_ref": ref_id.clone()
    });

    let summary_obj = summary.as_object_mut().unwrap();

    // Detect output type and extract accordingly
    let is_monorepo = output.get("projects").is_some() || output.get("is_monorepo").is_some();
    let is_project_analysis = output.get("languages").is_some() && output.get("analysis_metadata").is_some();

    if is_monorepo {
        // MonorepoAnalysis structure
        if let Some(mono) = output.get("is_monorepo").and_then(|v| v.as_bool()) {
            summary_obj.insert("is_monorepo".to_string(), json!(mono));
        }
        if let Some(root) = output.get("root_path").and_then(|v| v.as_str()) {
            summary_obj.insert("root_path".to_string(), json!(root));
        }

        if let Some(projects) = output.get("projects").and_then(|v| v.as_array()) {
            summary_obj.insert("project_count".to_string(), json!(projects.len()));

            let mut all_languages: Vec<String> = Vec::new();
            let mut all_frameworks: Vec<String> = Vec::new();
            let mut project_names: Vec<String> = Vec::new();

            for project in projects.iter().take(20) {
                if let Some(name) = project.get("name").and_then(|v| v.as_str()) {
                    project_names.push(name.to_string());
                }
                if let Some(analysis) = project.get("analysis") {
                    if let Some(langs) = analysis.get("languages").and_then(|v| v.as_array()) {
                        for lang in langs {
                            if let Some(name) = lang.get("name").and_then(|v| v.as_str()) {
                                if !all_languages.contains(&name.to_string()) {
                                    all_languages.push(name.to_string());
                                }
                            }
                        }
                    }
                    if let Some(fws) = analysis.get("frameworks").and_then(|v| v.as_array()) {
                        for fw in fws {
                            if let Some(name) = fw.get("name").and_then(|v| v.as_str()) {
                                if !all_frameworks.contains(&name.to_string()) {
                                    all_frameworks.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }

            summary_obj.insert("project_names".to_string(), json!(project_names));
            summary_obj.insert("languages_detected".to_string(), json!(all_languages));
            summary_obj.insert("frameworks_detected".to_string(), json!(all_frameworks));
        }
    } else if is_project_analysis {
        // ProjectAnalysis flat structure - languages/technologies at top level
        if let Some(root) = output.get("project_root").and_then(|v| v.as_str()) {
            summary_obj.insert("project_root".to_string(), json!(root));
        }
        if let Some(arch) = output.get("architecture_type").and_then(|v| v.as_str()) {
            summary_obj.insert("architecture_type".to_string(), json!(arch));
        }
        if let Some(proj_type) = output.get("project_type").and_then(|v| v.as_str()) {
            summary_obj.insert("project_type".to_string(), json!(proj_type));
        }

        // Extract languages (at top level)
        if let Some(langs) = output.get("languages").and_then(|v| v.as_array()) {
            let names: Vec<&str> = langs
                .iter()
                .filter_map(|l| l.get("name").and_then(|n| n.as_str()))
                .collect();
            summary_obj.insert("languages_detected".to_string(), json!(names));
        }

        // Extract technologies (at top level)
        if let Some(techs) = output.get("technologies").and_then(|v| v.as_array()) {
            let names: Vec<&str> = techs
                .iter()
                .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
                .collect();
            summary_obj.insert("technologies_detected".to_string(), json!(names));
        }

        // Extract services (include names, not just count)
        if let Some(services) = output.get("services").and_then(|v| v.as_array()) {
            summary_obj.insert("services_count".to_string(), json!(services.len()));
            // Include service names so agent knows what microservices exist
            let service_names: Vec<&str> = services
                .iter()
                .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                .collect();
            if !service_names.is_empty() {
                summary_obj.insert("services_detected".to_string(), json!(service_names));
            }
        }
    }

    // CRITICAL: Include retrieval instructions prominently
    summary_obj.insert(
        "retrieval_instructions".to_string(),
        json!({
            "message": "Full analysis stored. Use retrieve_output with queries to get specific sections.",
            "ref_id": ref_id,
            "available_queries": [
                "section:summary - Project overview",
                "section:languages - All detected languages",
                "section:frameworks - All detected frameworks/technologies",
                "section:services - All detected services",
                "language:<name> - Details for specific language (e.g., language:Rust)",
                "framework:<name> - Details for specific framework"
            ],
            "example": format!("retrieve_output('{}', 'section:summary')", ref_id)
        }),
    );

    // Build session summary
    let project_count = output
        .get("projects")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(1);
    let summary_str = format!(
        "{} project(s), {} bytes stored",
        project_count,
        raw_str.len()
    );

    // Register in session registry
    output_store::register_session_ref(
        &ref_id,
        "analyze_project",
        "Full project analysis (use section queries to retrieve specific data)",
        &summary_str,
        raw_str.len(),
    );

    // Return minimal JSON
    serde_json::to_string_pretty(&summary).unwrap_or_else(|_| {
        format!(
            r#"{{"tool":"analyze_project","status":"STORED","full_data_ref":"{}","message":"Analysis complete. Use retrieve_output('{}', 'section:summary') to view."}}"#,
            ref_id, ref_id
        )
    })
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
