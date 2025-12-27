//! Truncation utilities for tool outputs
//!
//! Limits the size of tool outputs to prevent context overflow.
//! Based on Forge's approach: truncate proactively BEFORE sending to the LLM.

/// Configuration for output truncation limits
pub struct TruncationLimits {
    /// Maximum lines to return from file reads (default: 2000)
    pub max_file_lines: usize,
    /// Lines to keep from start of shell output (default: 200)
    pub shell_prefix_lines: usize,
    /// Lines to keep from end of shell output (default: 200)
    pub shell_suffix_lines: usize,
    /// Maximum characters per line (default: 2000)
    pub max_line_length: usize,
    /// Maximum directory entries to return (default: 500)
    pub max_dir_entries: usize,
}

impl Default for TruncationLimits {
    fn default() -> Self {
        Self {
            max_file_lines: 2000,
            shell_prefix_lines: 200,
            shell_suffix_lines: 200,
            max_line_length: 2000,
            max_dir_entries: 500,
        }
    }
}

/// Result of truncating file content
pub struct TruncatedFileContent {
    /// The (possibly truncated) content
    pub content: String,
    /// Total lines in original file
    pub total_lines: usize,
    /// Lines actually returned
    pub returned_lines: usize,
    /// Whether content was truncated
    pub was_truncated: bool,
    /// Number of lines with truncated characters
    #[allow(dead_code)]
    pub lines_char_truncated: usize,
}

/// Truncate file content to max lines, with per-line character limit
pub fn truncate_file_content(content: &str, limits: &TruncationLimits) -> TruncatedFileContent {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    let (selected_lines, was_truncated) = if total_lines <= limits.max_file_lines {
        (lines.clone(), false)
    } else {
        // Take first max_file_lines lines
        (lines[..limits.max_file_lines].to_vec(), true)
    };

    let mut lines_char_truncated = 0;
    let processed: Vec<String> = selected_lines
        .iter()
        .map(|line| {
            if line.chars().count() > limits.max_line_length {
                lines_char_truncated += 1;
                let truncated: String = line.chars().take(limits.max_line_length).collect();
                let extra = line.chars().count() - limits.max_line_length;
                format!("{}...[{} chars truncated]", truncated, extra)
            } else {
                line.to_string()
            }
        })
        .collect();

    let returned_lines = processed.len();
    let mut result = processed.join("\n");

    // Add truncation notice at the end
    if was_truncated {
        result.push_str(&format!(
            "\n\n[OUTPUT TRUNCATED: Showing first {} of {} lines. Use start_line/end_line to read specific sections.]",
            returned_lines, total_lines
        ));
    }

    TruncatedFileContent {
        content: result,
        total_lines,
        returned_lines,
        was_truncated,
        lines_char_truncated,
    }
}

/// Result of truncating shell output
pub struct TruncatedShellOutput {
    /// The truncated stdout
    pub stdout: String,
    /// The truncated stderr
    pub stderr: String,
    /// Total stdout lines
    pub stdout_total_lines: usize,
    /// Total stderr lines
    pub stderr_total_lines: usize,
    /// Whether stdout was truncated
    pub stdout_truncated: bool,
    /// Whether stderr was truncated
    pub stderr_truncated: bool,
}

/// Truncate shell output using prefix/suffix strategy
/// Shows first N lines + last M lines, hiding the middle
pub fn truncate_shell_output(
    stdout: &str,
    stderr: &str,
    limits: &TruncationLimits,
) -> TruncatedShellOutput {
    let stdout_result = truncate_stream(
        stdout,
        limits.shell_prefix_lines,
        limits.shell_suffix_lines,
        limits.max_line_length,
    );

    let stderr_result = truncate_stream(
        stderr,
        limits.shell_prefix_lines,
        limits.shell_suffix_lines,
        limits.max_line_length,
    );

    TruncatedShellOutput {
        stdout: stdout_result.0,
        stderr: stderr_result.0,
        stdout_total_lines: stdout_result.1,
        stderr_total_lines: stderr_result.1,
        stdout_truncated: stdout_result.2,
        stderr_truncated: stderr_result.2,
    }
}

/// Truncate a single stream (stdout or stderr) with prefix/suffix strategy
fn truncate_stream(
    content: &str,
    prefix_lines: usize,
    suffix_lines: usize,
    max_line_length: usize,
) -> (String, usize, bool) {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let max_total = prefix_lines + suffix_lines;

    if total_lines <= max_total {
        // No truncation needed, just apply character limit
        let processed: Vec<String> = lines
            .iter()
            .map(|line| truncate_line(line, max_line_length))
            .collect();
        return (processed.join("\n"), total_lines, false);
    }

    // Need truncation: take prefix + suffix
    let mut result = Vec::new();

    // Add prefix lines
    for line in lines.iter().take(prefix_lines) {
        result.push(truncate_line(line, max_line_length));
    }

    // Add truncation marker
    let hidden = total_lines - prefix_lines - suffix_lines;
    result.push(format!(
        "\n... [{} lines hidden, showing first {} and last {} of {} total] ...\n",
        hidden, prefix_lines, suffix_lines, total_lines
    ));

    // Add suffix lines
    for line in lines.iter().skip(total_lines - suffix_lines) {
        result.push(truncate_line(line, max_line_length));
    }

    (result.join("\n"), total_lines, true)
}

/// Truncate a single line if it exceeds max length
fn truncate_line(line: &str, max_length: usize) -> String {
    if line.chars().count() <= max_length {
        line.to_string()
    } else {
        let truncated: String = line.chars().take(max_length).collect();
        let extra = line.chars().count() - max_length;
        format!("{}...[{} chars]", truncated, extra)
    }
}

/// Result of truncating directory listing
pub struct TruncatedDirListing {
    /// The (possibly truncated) entries
    pub entries: Vec<serde_json::Value>,
    /// Total entries in directory
    pub total_entries: usize,
    /// Whether list was truncated
    pub was_truncated: bool,
}

/// Truncate directory listing to max entries
pub fn truncate_dir_listing(
    entries: Vec<serde_json::Value>,
    max_entries: usize,
) -> TruncatedDirListing {
    let total_entries = entries.len();

    if total_entries <= max_entries {
        TruncatedDirListing {
            entries,
            total_entries,
            was_truncated: false,
        }
    } else {
        TruncatedDirListing {
            entries: entries.into_iter().take(max_entries).collect(),
            total_entries,
            was_truncated: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_file_no_truncation_needed() {
        let content = "line1\nline2\nline3";
        let limits = TruncationLimits::default();
        let result = truncate_file_content(content, &limits);

        assert_eq!(result.total_lines, 3);
        assert_eq!(result.returned_lines, 3);
        assert!(!result.was_truncated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_truncate_file_exceeds_limit() {
        let lines: Vec<String> = (0..100).map(|i| format!("line {}", i)).collect();
        let content = lines.join("\n");
        let limits = TruncationLimits {
            max_file_lines: 10,
            ..Default::default()
        };
        let result = truncate_file_content(&content, &limits);

        assert_eq!(result.total_lines, 100);
        assert_eq!(result.returned_lines, 10);
        assert!(result.was_truncated);
        assert!(result.content.contains("[OUTPUT TRUNCATED"));
    }

    #[test]
    fn test_truncate_shell_prefix_suffix() {
        let lines: Vec<String> = (0..500).map(|i| format!("output line {}", i)).collect();
        let stdout = lines.join("\n");
        let limits = TruncationLimits {
            shell_prefix_lines: 5,
            shell_suffix_lines: 5,
            ..Default::default()
        };
        let result = truncate_shell_output(&stdout, "", &limits);

        assert_eq!(result.stdout_total_lines, 500);
        assert!(result.stdout_truncated);
        assert!(result.stdout.contains("output line 0"));
        assert!(result.stdout.contains("output line 499"));
        assert!(result.stdout.contains("lines hidden"));
    }

    #[test]
    fn test_truncate_long_line() {
        let long_line = "x".repeat(3000);
        let result = truncate_line(&long_line, 100);

        assert!(result.len() < 200); // Should be truncated
        assert!(result.contains("chars]"));
    }

    #[test]
    fn test_truncate_dir_listing() {
        let entries: Vec<serde_json::Value> = (0..100)
            .map(|i| serde_json::json!({"name": format!("file{}", i)}))
            .collect();

        let result = truncate_dir_listing(entries, 10);

        assert_eq!(result.total_entries, 100);
        assert_eq!(result.entries.len(), 10);
        assert!(result.was_truncated);
    }
}
