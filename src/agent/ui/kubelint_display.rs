//! Kubelint result display for terminal output
//!
//! Provides colored, formatted output for Kubernetes manifest lint results
//! using Syncable brand styling with box-drawing characters.

use crate::agent::ui::colors::icons;
use crate::agent::ui::response::brand;
use std::io::{self, Write};

/// Box width for consistent display
const BOX_WIDTH: usize = 72;

/// Display kubelint results in a formatted, colored terminal output
pub struct KubelintDisplay;

impl KubelintDisplay {
    /// Format and print kubelint results from the JSON output
    pub fn print_result(json_result: &str) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_result) {
            Self::print_formatted(&parsed);
        } else {
            // Fallback: just print the raw result
            println!("{}", json_result);
        }
    }

    /// Print formatted kubelint output with Syncable brand styling
    fn print_formatted(result: &serde_json::Value) {
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        // Source path
        let source = result["source"].as_str().unwrap_or("kubernetes manifests");

        // Header
        let _ = writeln!(handle);
        let _ = writeln!(
            handle,
            "{}{}╭─ {} Kubelint {}{}╮{}",
            brand::PURPLE,
            brand::BOLD,
            icons::KUBERNETES,
            "─".repeat(BOX_WIDTH - 16),
            brand::DIM,
            brand::RESET
        );

        // Source path line
        let _ = writeln!(
            handle,
            "{}│  {}{}{}{}",
            brand::DIM,
            brand::CYAN,
            source,
            " ".repeat((BOX_WIDTH - 4 - source.len()).max(0)),
            brand::RESET
        );

        // Empty line
        let _ = writeln!(handle, "{}│{}", brand::DIM, " ".repeat(BOX_WIDTH - 1));

        // Decision context
        if let Some(context) = result["decision_context"].as_str() {
            let context_color = if context.contains("CRITICAL") {
                brand::CORAL
            } else if context.contains("High") || context.contains("high") {
                brand::PEACH
            } else if context.contains("Good") || context.contains("No issues") {
                brand::SUCCESS
            } else {
                brand::PEACH
            };

            // Truncate context if too long
            let display_context = if context.len() > BOX_WIDTH - 6 {
                &context[..BOX_WIDTH - 9]
            } else {
                context
            };

            let _ = writeln!(
                handle,
                "{}│  {}{}{}{}",
                brand::DIM,
                context_color,
                display_context,
                " ".repeat((BOX_WIDTH - 4 - display_context.len()).max(0)),
                brand::RESET
            );
        }

        // Empty line
        let _ = writeln!(handle, "{}│{}", brand::DIM, " ".repeat(BOX_WIDTH - 1));

        // Summary counts
        if let Some(summary) = result.get("summary") {
            let total = summary["total_issues"].as_u64().unwrap_or(0);

            if total == 0 {
                let _ = writeln!(
                    handle,
                    "{}│  {}{} All checks passed! No issues found.{}{}",
                    brand::DIM,
                    brand::SUCCESS,
                    icons::SUCCESS,
                    " ".repeat(BOX_WIDTH - 42),
                    brand::RESET
                );

                // Objects analyzed
                let objects = summary["objects_analyzed"].as_u64().unwrap_or(0);
                let checks = summary["checks_run"].as_u64().unwrap_or(0);
                let stats = format!("{} objects analyzed • {} checks run", objects, checks);
                let _ = writeln!(handle, "{}│{}", brand::DIM, " ".repeat(BOX_WIDTH - 1));
                let _ = writeln!(
                    handle,
                    "{}│  {}{}{}{}",
                    brand::DIM,
                    brand::DIM,
                    stats,
                    " ".repeat((BOX_WIDTH - 4 - stats.len()).max(0)),
                    brand::RESET
                );
            } else {
                // Priority breakdown
                if let Some(by_priority) = summary.get("by_priority") {
                    let critical = by_priority["critical"].as_u64().unwrap_or(0);
                    let high = by_priority["high"].as_u64().unwrap_or(0);
                    let medium = by_priority["medium"].as_u64().unwrap_or(0);
                    let low = by_priority["low"].as_u64().unwrap_or(0);

                    let mut counts = String::new();
                    if critical > 0 {
                        counts.push_str(&format!("{} {} critical  ", icons::CRITICAL, critical));
                    }
                    if high > 0 {
                        counts.push_str(&format!("{} {} high  ", icons::HIGH, high));
                    }
                    if medium > 0 {
                        counts.push_str(&format!("{} {} medium  ", icons::MEDIUM, medium));
                    }
                    if low > 0 {
                        counts.push_str(&format!("{} {} low", icons::LOW, low));
                    }

                    let padding = if counts.len() < BOX_WIDTH - 4 {
                        (BOX_WIDTH - 4 - counts.chars().count()).max(0)
                    } else {
                        0
                    };
                    let _ = writeln!(
                        handle,
                        "{}│  {}{}{}",
                        brand::DIM,
                        counts,
                        " ".repeat(padding),
                        brand::RESET
                    );
                }
            }
        }

        // Quick fixes section
        if let Some(quick_fixes) = result.get("quick_fixes").and_then(|f| f.as_array())
            && !quick_fixes.is_empty()
        {
            let _ = writeln!(handle, "{}│{}", brand::DIM, " ".repeat(BOX_WIDTH - 1));
            let _ = writeln!(
                handle,
                "{}│  {}{} Quick Fixes:{}{}",
                brand::DIM,
                brand::PURPLE,
                icons::FIX,
                " ".repeat(BOX_WIDTH - 18),
                brand::RESET
            );

            for fix in quick_fixes.iter().take(5) {
                if let Some(fix_str) = fix.as_str() {
                    // Split fix into parts if it contains " - "
                    let (issue, remediation) = if let Some(pos) = fix_str.find(" - ") {
                        (&fix_str[..pos], &fix_str[pos + 3..])
                    } else {
                        (fix_str, "")
                    };

                    let issue_display = if issue.len() > BOX_WIDTH - 10 {
                        format!("{}...", &issue[..BOX_WIDTH - 13])
                    } else {
                        issue.to_string()
                    };

                    let _ = writeln!(
                        handle,
                        "{}│    {}→ {}{}{}{}",
                        brand::DIM,
                        brand::CYAN,
                        issue_display,
                        " ".repeat((BOX_WIDTH - 8 - issue_display.len()).max(0)),
                        brand::RESET,
                        brand::RESET
                    );

                    if !remediation.is_empty() {
                        let rem_display = if remediation.len() > BOX_WIDTH - 10 {
                            format!("{}...", &remediation[..BOX_WIDTH - 13])
                        } else {
                            remediation.to_string()
                        };
                        let _ = writeln!(
                            handle,
                            "{}│      {}{}{}{}",
                            brand::DIM,
                            brand::DIM,
                            rem_display,
                            " ".repeat((BOX_WIDTH - 8 - rem_display.len()).max(0)),
                            brand::RESET
                        );
                    }
                }
            }
        }

        // Critical and High priority issues with details
        Self::print_priority_section(
            &mut handle,
            result,
            "critical",
            "Critical Issues",
            brand::CORAL,
        );
        Self::print_priority_section(&mut handle, result, "high", "High Priority", brand::PEACH);

        // Medium/Low summary
        let medium_count = result["action_plan"]["medium"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        let low_count = result["action_plan"]["low"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        let other_count = medium_count + low_count;

        if other_count > 0 {
            let _ = writeln!(handle, "{}│{}", brand::DIM, " ".repeat(BOX_WIDTH - 1));
            let msg = format!(
                "{} {} priority issue{} (use --verbose to see all)",
                other_count,
                if medium_count > 0 {
                    "medium/low"
                } else {
                    "low"
                },
                if other_count == 1 { "" } else { "s" }
            );
            let _ = writeln!(
                handle,
                "{}│  {}{}{}{}",
                brand::DIM,
                brand::DIM,
                msg,
                " ".repeat((BOX_WIDTH - 4 - msg.len()).max(0)),
                brand::RESET
            );
        }

        // Footer
        let _ = writeln!(
            handle,
            "{}╰{}╯{}",
            brand::DIM,
            "─".repeat(BOX_WIDTH - 2),
            brand::RESET
        );
        let _ = writeln!(handle);

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

            let _ = writeln!(handle, "{}│{}", brand::DIM, " ".repeat(BOX_WIDTH - 1));
            let _ = writeln!(
                handle,
                "{}│  {}{}:{}{}",
                brand::DIM,
                color,
                title,
                " ".repeat((BOX_WIDTH - 4 - title.len() - 1).max(0)),
                brand::RESET
            );

            for issue in issues.iter().take(5) {
                let code = issue["check"].as_str().unwrap_or("???");
                let line = issue["line"].as_u64().unwrap_or(0);
                let message = issue["message"].as_str().unwrap_or("");
                let category = issue["category"].as_str().unwrap_or("");

                // Category badge
                let badge = Self::get_category_badge(category);

                // Issue header line
                let header = format!("Line {} • {} {}", line, code, badge);
                let _ = writeln!(
                    handle,
                    "{}│    {}{}{}{}",
                    brand::DIM,
                    brand::CYAN,
                    header,
                    " ".repeat((BOX_WIDTH - 6 - header.chars().count()).max(0)),
                    brand::RESET
                );

                // Message
                let msg_display = if message.len() > BOX_WIDTH - 8 {
                    format!("{}...", &message[..BOX_WIDTH - 11])
                } else {
                    message.to_string()
                };
                let _ = writeln!(
                    handle,
                    "{}│    {}{}{}",
                    brand::DIM,
                    msg_display,
                    " ".repeat((BOX_WIDTH - 6 - msg_display.len()).max(0)),
                    brand::RESET
                );

                // Remediation
                if let Some(remediation) = issue["remediation"].as_str() {
                    let rem_display = if remediation.len() > BOX_WIDTH - 12 {
                        format!("{}...", &remediation[..BOX_WIDTH - 15])
                    } else {
                        remediation.to_string()
                    };
                    let _ = writeln!(
                        handle,
                        "{}│    {}→ {}{}{}",
                        brand::DIM,
                        brand::CYAN,
                        rem_display,
                        " ".repeat((BOX_WIDTH - 8 - rem_display.len()).max(0)),
                        brand::RESET
                    );
                }
            }

            if issues.len() > 5 {
                let more_msg = format!("... and {} more", issues.len() - 5);
                let _ = writeln!(
                    handle,
                    "{}│    {}{}{}{}",
                    brand::DIM,
                    brand::DIM,
                    more_msg,
                    " ".repeat((BOX_WIDTH - 6 - more_msg.len()).max(0)),
                    brand::RESET
                );
            }
        }
    }

    /// Get category badge with color
    fn get_category_badge(category: &str) -> String {
        match category {
            "security" => format!("{}[SEC]{}", brand::CORAL, brand::RESET),
            "rbac" => format!("{}[RBAC]{}", brand::CORAL, brand::RESET),
            "best-practice" => format!("{}[BP]{}", brand::CYAN, brand::RESET),
            "validation" => format!("{}[VAL]{}", brand::PEACH, brand::RESET),
            "ports" => format!("{}[PORT]{}", brand::PEACH, brand::RESET),
            "disruption-budget" => format!("{}[PDB]{}", brand::DIM, brand::RESET),
            "autoscaling" => format!("{}[HPA]{}", brand::DIM, brand::RESET),
            "deprecated-api" => format!("{}[DEP]{}", brand::PEACH, brand::RESET),
            "service" => format!("{}[SVC]{}", brand::DIM, brand::RESET),
            _ => String::new(),
        }
    }

    /// Format a compact single-line summary for tool call display
    pub fn format_summary(json_result: &str) -> String {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_result) {
            let success = parsed["success"].as_bool().unwrap_or(false);
            let total = parsed["summary"]["total_issues"].as_u64().unwrap_or(0);

            if success && total == 0 {
                format!(
                    "{}{} {} K8s manifests OK - no issues{}",
                    brand::SUCCESS,
                    icons::SUCCESS,
                    icons::KUBERNETES,
                    brand::RESET
                )
            } else {
                let critical = parsed["summary"]["by_priority"]["critical"]
                    .as_u64()
                    .unwrap_or(0);
                let high = parsed["summary"]["by_priority"]["high"]
                    .as_u64()
                    .unwrap_or(0);

                if critical > 0 {
                    format!(
                        "{}{} {} {} critical, {} high priority issues{}",
                        brand::CORAL,
                        icons::CRITICAL,
                        icons::KUBERNETES,
                        critical,
                        high,
                        brand::RESET
                    )
                } else if high > 0 {
                    format!(
                        "{}{} {} {} high priority issues{}",
                        brand::PEACH,
                        icons::HIGH,
                        icons::KUBERNETES,
                        high,
                        brand::RESET
                    )
                } else {
                    format!(
                        "{}{} {} {} issues (medium/low){}",
                        brand::PEACH,
                        icons::MEDIUM,
                        icons::KUBERNETES,
                        total,
                        brand::RESET
                    )
                }
            }
        } else {
            format!("{} Kubelint analysis complete", icons::KUBERNETES)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_summary_success() {
        let json = r#"{"success": true, "summary": {"total_issues": 0, "by_priority": {"critical": 0, "high": 0, "medium": 0, "low": 0}}}"#;
        let summary = KubelintDisplay::format_summary(json);
        assert!(summary.contains("OK"));
    }

    #[test]
    fn test_format_summary_critical() {
        let json = r#"{"success": false, "summary": {"total_issues": 3, "by_priority": {"critical": 1, "high": 2, "medium": 0, "low": 0}}}"#;
        let summary = KubelintDisplay::format_summary(json);
        assert!(summary.contains("critical"));
    }

    #[test]
    fn test_category_badge() {
        let badge = KubelintDisplay::get_category_badge("security");
        assert!(badge.contains("SEC"));
    }

    #[test]
    fn test_print_result_with_issues() {
        // Test that print doesn't panic with real data
        let json = r#"{
            "source": "test.yaml",
            "success": false,
            "decision_context": "CRITICAL security issues found.",
            "summary": {
                "total_issues": 2,
                "objects_analyzed": 1,
                "checks_run": 63,
                "by_priority": {"critical": 1, "high": 1, "medium": 0, "low": 0}
            },
            "action_plan": {
                "critical": [{
                    "check": "privileged-container",
                    "severity": "error",
                    "priority": "critical",
                    "category": "security",
                    "message": "Container running in privileged mode",
                    "line": 20,
                    "remediation": "Set privileged: false"
                }],
                "high": [{
                    "check": "latest-tag",
                    "severity": "warning",
                    "priority": "high",
                    "category": "best-practice",
                    "message": "Image uses :latest tag",
                    "line": 18,
                    "remediation": "Use specific tag"
                }],
                "medium": [],
                "low": []
            },
            "quick_fixes": ["Deployment/nginx: privileged-container - Set privileged: false"]
        }"#;

        // Just test it doesn't panic
        KubelintDisplay::print_result(json);
    }

    #[test]
    fn test_print_result_success() {
        let json = r#"{
            "source": "secure.yaml",
            "success": true,
            "decision_context": "No issues found.",
            "summary": {
                "total_issues": 0,
                "objects_analyzed": 3,
                "checks_run": 63,
                "by_priority": {"critical": 0, "high": 0, "medium": 0, "low": 0}
            },
            "action_plan": {"critical": [], "high": [], "medium": [], "low": []}
        }"#;

        // Just test it doesn't panic
        KubelintDisplay::print_result(json);
    }
}
