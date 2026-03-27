//! Output formatting for optimization results.
//!
//! Supports multiple output formats: table, JSON, and plain text.

use crate::analyzer::k8s_optimize::types::{OptimizationResult, Severity};
use colored::Colorize;
use serde::{Deserialize, Serialize};

// ============================================================================
// Output Format
// ============================================================================

/// Output format for optimization results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// ASCII table format (default)
    #[default]
    Table,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Plain text summary
    Summary,
}

impl OutputFormat {
    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "table" => Some(Self::Table),
            "json" => Some(Self::Json),
            "yaml" => Some(Self::Yaml),
            "summary" => Some(Self::Summary),
            _ => None,
        }
    }
}

// ============================================================================
// Formatting Functions
// ============================================================================

/// Format optimization result to string.
pub fn format_result_to_string(result: &OptimizationResult, format: OutputFormat) -> String {
    match format {
        OutputFormat::Table => format_table(result),
        OutputFormat::Json => format_json(result),
        OutputFormat::Yaml => format_yaml(result),
        OutputFormat::Summary => format_summary(result),
    }
}

/// Format and print optimization result.
pub fn format_result(result: &OptimizationResult, format: OutputFormat) {
    println!("{}", format_result_to_string(result, format));
}

// ============================================================================
// Table Format
// ============================================================================

fn format_table(result: &OptimizationResult) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "\n{}\n",
        "═══════════════════════════════════════════════════════════════════════════════════════════════════"
            .bright_blue()
    ));
    output.push_str(&format!(
        "{}\n",
        "💰 KUBERNETES RESOURCE OPTIMIZATION REPORT"
            .bright_white()
            .bold()
    ));
    output.push_str(&format!(
        "{}\n\n",
        "═══════════════════════════════════════════════════════════════════════════════════════════════════"
            .bright_blue()
    ));

    // Summary section
    output.push_str(&format_summary_section(result));

    // Recommendations section
    if result.has_recommendations() {
        output.push_str(&format!(
            "\n{}\n",
            "┌─ Recommendations ─────────────────────────────────────────────────────────────────────────────┐"
                .bright_blue()
        ));

        for (i, rec) in result.recommendations.iter().enumerate() {
            let severity_icon = match rec.severity {
                Severity::Critical => "🔴",
                Severity::High => "🟠",
                Severity::Medium => "🟡",
                Severity::Low => "🟢",
                Severity::Info => "ℹ️ ",
            };

            let severity_str = match rec.severity {
                Severity::Critical => rec.severity.as_str().bright_red(),
                Severity::High => rec.severity.as_str().red(),
                Severity::Medium => rec.severity.as_str().yellow(),
                Severity::Low => rec.severity.as_str().green(),
                Severity::Info => rec.severity.as_str().blue(),
            };

            output.push_str(&format!(
                "│\n│ {} {} {} {}\n",
                severity_icon,
                format!("[{}]", rec.rule_code).bright_cyan(),
                severity_str.bold(),
                rec.resource_identifier().bright_white()
            ));

            output.push_str(&format!(
                "│   {} {} / {}\n",
                "Resource:".dimmed(),
                rec.resource_kind.cyan(),
                rec.container.yellow()
            ));

            output.push_str(&format!("│   {} {}\n", "Issue:".dimmed(), rec.message));

            // Show current vs recommended
            if rec.current.has_any() || rec.recommended.has_any() {
                output.push_str(&format!("│   {}\n", "Current:".dimmed()));
                if let Some(cpu) = &rec.current.cpu_request {
                    output.push_str(&format!("│     CPU request: {}\n", cpu.red()));
                }
                if let Some(mem) = &rec.current.memory_request {
                    output.push_str(&format!("│     Memory request: {}\n", mem.red()));
                }

                output.push_str(&format!("│   {}\n", "Recommended:".dimmed()));
                if let Some(cpu) = &rec.recommended.cpu_request {
                    output.push_str(&format!("│     CPU request: {}\n", cpu.green()));
                }
                if let Some(mem) = &rec.recommended.memory_request {
                    output.push_str(&format!("│     Memory request: {}\n", mem.green()));
                }
            }

            if i < result.recommendations.len() - 1 {
                output.push_str(&format!(
                    "│{}",
                    "────────────────────────────────────────────────────────────────────────────────────────────\n"
                        .dimmed()
                ));
            }
        }

        output.push_str(&format!(
            "{}\n",
            "└────────────────────────────────────────────────────────────────────────────────────────────────┘"
                .bright_blue()
        ));
    } else {
        output.push_str(&format!(
            "\n{}\n",
            "✅ No optimization issues found! Your resources look well-configured.".green()
        ));
    }

    // Footer
    output.push_str(&format!(
        "\n{}\n",
        "═══════════════════════════════════════════════════════════════════════════════════════════════════"
            .bright_blue()
    ));

    output
}

fn format_summary_section(result: &OptimizationResult) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "{}",
        "┌─ Summary ─────────────────────────────────────────────────────────────────────────────────────────┐\n"
            .bright_blue()
    ));

    output.push_str(&format!(
        "│ {} {:>6}     {} {:>6}     {} {:>6}\n",
        "Resources:".dimmed(),
        result.summary.resources_analyzed.to_string().bright_white(),
        "Containers:".dimmed(),
        result
            .summary
            .containers_analyzed
            .to_string()
            .bright_white(),
        "Mode:".dimmed(),
        result.metadata.mode.to_string().cyan(),
    ));

    output.push_str(&format!(
        "│ {} {:>6}     {} {:>6}     {} {:>6}\n",
        "Over-provisioned:".dimmed(),
        if result.summary.over_provisioned > 0 {
            result.summary.over_provisioned.to_string().red()
        } else {
            result.summary.over_provisioned.to_string().green()
        },
        "Missing requests:".dimmed(),
        if result.summary.missing_requests > 0 {
            result.summary.missing_requests.to_string().yellow()
        } else {
            result.summary.missing_requests.to_string().green()
        },
        "Optimal:".dimmed(),
        result.summary.optimal.to_string().green(),
    ));

    if result.summary.total_waste_percentage > 0.0 {
        output.push_str(&format!(
            "│ {} {:.1}%\n",
            "Estimated waste:".dimmed(),
            result.summary.total_waste_percentage.to_string().red(),
        ));
    }

    if let Some(savings) = result.summary.estimated_monthly_savings_usd {
        output.push_str(&format!(
            "│ {} ${:.2}/month\n",
            "Potential savings:".dimmed(),
            savings.to_string().green(),
        ));
    }

    output.push_str(&format!(
        "│ {} {}ms     {} {}\n",
        "Duration:".dimmed(),
        result.metadata.duration_ms.to_string().dimmed(),
        "Path:".dimmed(),
        result.metadata.path.display().to_string().dimmed(),
    ));

    output.push_str(&format!(
        "{}",
        "└───────────────────────────────────────────────────────────────────────────────────────────────────┘\n"
            .bright_blue()
    ));

    output
}

// ============================================================================
// JSON Format
// ============================================================================

fn format_json(result: &OptimizationResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".to_string())
}

// ============================================================================
// YAML Format
// ============================================================================

fn format_yaml(result: &OptimizationResult) -> String {
    serde_yaml::to_string(result).unwrap_or_else(|_| "".to_string())
}

// ============================================================================
// Summary Format
// ============================================================================

fn format_summary(result: &OptimizationResult) -> String {
    let mut output = String::new();

    output.push_str("▶ RESOURCE OPTIMIZATION SUMMARY\n");
    output.push_str("──────────────────────────────────────────────────\n");
    output.push_str(&format!(
        "│ Resources: {} ({})\n",
        result.summary.resources_analyzed, result.metadata.mode
    ));
    output.push_str(&format!(
        "│ Containers: {}\n",
        result.summary.containers_analyzed
    ));
    output.push_str(&format!(
        "│ Issues: {} over-provisioned, {} missing requests\n",
        result.summary.over_provisioned, result.summary.missing_requests
    ));
    output.push_str(&format!("│ Optimal: {}\n", result.summary.optimal));
    output.push_str(&format!(
        "│ Analysis Time: {}ms\n",
        result.metadata.duration_ms
    ));
    output.push_str("──────────────────────────────────────────────────\n");

    output
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::k8s_optimize::types::AnalysisMode;
    use std::path::PathBuf;

    #[test]
    fn test_output_format_parse() {
        assert_eq!(OutputFormat::parse("table"), Some(OutputFormat::Table));
        assert_eq!(OutputFormat::parse("JSON"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::parse("yaml"), Some(OutputFormat::Yaml));
        assert_eq!(OutputFormat::parse("summary"), Some(OutputFormat::Summary));
        assert_eq!(OutputFormat::parse("invalid"), None);
    }

    #[test]
    fn test_format_json() {
        let result = OptimizationResult::new(PathBuf::from("."), AnalysisMode::Static);
        let json = format_json(&result);
        assert!(json.contains("\"summary\""));
        assert!(json.contains("\"recommendations\""));
    }

    #[test]
    fn test_format_summary() {
        let result = OptimizationResult::new(PathBuf::from("."), AnalysisMode::Static);
        let summary = format_summary(&result);
        assert!(summary.contains("RESOURCE OPTIMIZATION SUMMARY"));
        assert!(summary.contains("Resources:"));
    }

    #[test]
    fn test_format_table() {
        let result = OptimizationResult::new(PathBuf::from("."), AnalysisMode::Static);
        let table = format_table(&result);
        assert!(table.contains("KUBERNETES RESOURCE OPTIMIZATION REPORT"));
        assert!(table.contains("Summary"));
    }
}
