//! Hadolint result display for terminal output
//!
//! Provides colored, formatted output for Dockerfile lint results
//! that's visually distinct and easy to recognize.

use crate::agent::ui::colors::{ansi, icons};
use std::io::{self, Write};

/// Display hadolint results in a formatted, colored terminal output
pub struct HadolintDisplay;

impl HadolintDisplay {
    /// Format and print hadolint results from the JSON output
    pub fn print_result(json_result: &str) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_result) {
            Self::print_formatted(&parsed);
        } else {
            // Fallback: just print the raw result
            println!("{}", json_result);
        }
    }

    /// Print formatted hadolint output
    fn print_formatted(result: &serde_json::Value) {
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        // Header with Docker icon and file name
        let file = result["file"].as_str().unwrap_or("Dockerfile");
        let _ = writeln!(
            handle,
            "\n{}{}━━━ {} Hadolint: {} ━━━{}",
            ansi::DOCKER_BLUE,
            ansi::BOLD,
            icons::DOCKER,
            file,
            ansi::RESET
        );

        // Decision context
        if let Some(context) = result["decision_context"].as_str() {
            let context_color = if context.contains("Critical") {
                ansi::CRITICAL
            } else if context.contains("High") {
                ansi::HIGH
            } else if context.contains("Medium") || context.contains("improvements") {
                ansi::MEDIUM
            } else {
                ansi::LOW
            };
            let _ = writeln!(
                handle,
                "{}  {} {}{}",
                context_color,
                icons::ARROW,
                context,
                ansi::RESET
            );
        }

        // Summary counts
        if let Some(summary) = result.get("summary") {
            let total = summary["total"].as_u64().unwrap_or(0);
            if total == 0 {
                let _ = writeln!(
                    handle,
                    "\n{}  {} No issues found!{}",
                    ansi::SUCCESS,
                    icons::SUCCESS,
                    ansi::RESET
                );
            } else {
                let _ = writeln!(handle);

                // Priority breakdown
                if let Some(by_priority) = summary.get("by_priority") {
                    let critical = by_priority["critical"].as_u64().unwrap_or(0);
                    let high = by_priority["high"].as_u64().unwrap_or(0);
                    let medium = by_priority["medium"].as_u64().unwrap_or(0);
                    let low = by_priority["low"].as_u64().unwrap_or(0);

                    let _ = write!(handle, "  ");
                    if critical > 0 {
                        let _ = write!(
                            handle,
                            "{}{} {} critical{}  ",
                            ansi::CRITICAL,
                            icons::CRITICAL,
                            critical,
                            ansi::RESET
                        );
                    }
                    if high > 0 {
                        let _ = write!(
                            handle,
                            "{}{} {} high{}  ",
                            ansi::HIGH,
                            icons::HIGH,
                            high,
                            ansi::RESET
                        );
                    }
                    if medium > 0 {
                        let _ = write!(
                            handle,
                            "{}{} {} medium{}  ",
                            ansi::MEDIUM,
                            icons::MEDIUM,
                            medium,
                            ansi::RESET
                        );
                    }
                    if low > 0 {
                        let _ = write!(
                            handle,
                            "{}{} {} low{}",
                            ansi::LOW,
                            icons::LOW,
                            low,
                            ansi::RESET
                        );
                    }
                    let _ = writeln!(handle);
                }
            }
        }

        // Quick fixes (most important)
        if let Some(quick_fixes) = result.get("quick_fixes").and_then(|f| f.as_array()) {
            if !quick_fixes.is_empty() {
                let _ = writeln!(
                    handle,
                    "\n{}{}  Quick Fixes:{}",
                    ansi::DOCKER_BLUE,
                    icons::FIX,
                    ansi::RESET
                );
                for fix in quick_fixes.iter().take(5) {
                    if let Some(fix_str) = fix.as_str() {
                        let _ = writeln!(
                            handle,
                            "{}    {} {}{}",
                            ansi::INFO_BLUE,
                            icons::ARROW,
                            fix_str,
                            ansi::RESET
                        );
                    }
                }
            }
        }

        // Critical and High priority issues with details
        Self::print_priority_section(&mut handle, result, "critical", "Critical Issues", ansi::CRITICAL);
        Self::print_priority_section(&mut handle, result, "high", "High Priority", ansi::HIGH);

        // Optionally show medium (collapsed)
        if let Some(medium_issues) = result["action_plan"]["medium"].as_array() {
            if !medium_issues.is_empty() {
                let _ = writeln!(
                    handle,
                    "\n{}  {} {} medium priority issue{} (run with --verbose to see all){}",
                    ansi::MEDIUM,
                    icons::MEDIUM,
                    medium_issues.len(),
                    if medium_issues.len() == 1 { "" } else { "s" },
                    ansi::RESET
                );
            }
        }

        // Footer separator
        let _ = writeln!(
            handle,
            "{}{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}",
            ansi::DOCKER_BLUE,
            ansi::DIM,
            ansi::RESET
        );

        let _ = handle.flush();
    }

    /// Print a section for a priority level
    fn print_priority_section(
        handle: &mut io::StdoutLock,
        result: &serde_json::Value,
        priority: &str,
        title: &str,
        color: &str,
    ) {
        if let Some(issues) = result["action_plan"][priority].as_array() {
            if issues.is_empty() {
                return;
            }

            let _ = writeln!(handle, "\n{}  {}:{}", color, title, ansi::RESET);

            for issue in issues.iter().take(10) {
                let code = issue["code"].as_str().unwrap_or("???");
                let line = issue["line"].as_u64().unwrap_or(0);
                let message = issue["message"].as_str().unwrap_or("");
                let category = issue["category"].as_str().unwrap_or("");

                // Category badge
                let category_badge = match category {
                    "security" => format!("{}[SEC]{}", ansi::CRITICAL, ansi::RESET),
                    "best-practice" => format!("{}[BP]{}", ansi::INFO_BLUE, ansi::RESET),
                    "deprecated" => format!("{}[DEP]{}", ansi::MEDIUM, ansi::RESET),
                    "performance" => format!("{}[PERF]{}", ansi::CYAN, ansi::RESET),
                    "maintainability" => format!("{}[MAINT]{}", ansi::GRAY, ansi::RESET),
                    _ => String::new(),
                };

                let _ = writeln!(
                    handle,
                    "    {}{}:{}{} {}{}{} {} {}",
                    ansi::DIM,
                    line,
                    ansi::RESET,
                    ansi::DOCKER_BLUE,
                    code,
                    ansi::RESET,
                    category_badge,
                    ansi::GRAY,
                    message,
                );

                // Show fix recommendation
                if let Some(fix) = issue["fix"].as_str() {
                    let _ = writeln!(
                        handle,
                        "       {}→ {}{}",
                        ansi::INFO_BLUE,
                        fix,
                        ansi::RESET
                    );
                }
            }

            if issues.len() > 10 {
                let _ = writeln!(
                    handle,
                    "    {}... and {} more{}",
                    ansi::DIM,
                    issues.len() - 10,
                    ansi::RESET
                );
            }
        }
    }

    /// Format a compact single-line summary for tool call display
    pub fn format_summary(json_result: &str) -> String {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_result) {
            let success = parsed["success"].as_bool().unwrap_or(false);
            let total = parsed["summary"]["total"].as_u64().unwrap_or(0);

            if success && total == 0 {
                format!(
                    "{}{} {} Dockerfile OK - no issues{}",
                    ansi::SUCCESS,
                    icons::SUCCESS,
                    icons::DOCKER,
                    ansi::RESET
                )
            } else {
                let critical = parsed["summary"]["by_priority"]["critical"].as_u64().unwrap_or(0);
                let high = parsed["summary"]["by_priority"]["high"].as_u64().unwrap_or(0);

                if critical > 0 {
                    format!(
                        "{}{} {} {} critical, {} high priority issues{}",
                        ansi::CRITICAL,
                        icons::ERROR,
                        icons::DOCKER,
                        critical,
                        high,
                        ansi::RESET
                    )
                } else if high > 0 {
                    format!(
                        "{}{} {} {} high priority issues{}",
                        ansi::HIGH,
                        icons::WARNING,
                        icons::DOCKER,
                        high,
                        ansi::RESET
                    )
                } else {
                    format!(
                        "{}{} {} {} issues (medium/low){}",
                        ansi::MEDIUM,
                        icons::WARNING,
                        icons::DOCKER,
                        total,
                        ansi::RESET
                    )
                }
            }
        } else {
            format!("{} Hadolint analysis complete", icons::DOCKER)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_summary_success() {
        let json = r#"{"success": true, "summary": {"total": 0, "by_priority": {"critical": 0, "high": 0, "medium": 0, "low": 0}}}"#;
        let summary = HadolintDisplay::format_summary(json);
        assert!(summary.contains("OK"));
    }

    #[test]
    fn test_format_summary_critical() {
        let json = r#"{"success": false, "summary": {"total": 3, "by_priority": {"critical": 1, "high": 2, "medium": 0, "low": 0}}}"#;
        let summary = HadolintDisplay::format_summary(json);
        assert!(summary.contains("critical"));
    }
}
