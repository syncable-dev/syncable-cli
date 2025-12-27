//! Summary view display functionality

use crate::analyzer::MonorepoAnalysis;
use colored::*;

/// Display summary view only
pub fn display_summary_view(analysis: &MonorepoAnalysis) {
    println!(
        "\n{} {}",
        "▶".bright_blue(),
        "PROJECT ANALYSIS SUMMARY".bright_white().bold()
    );
    println!("{}", "─".repeat(50).dimmed());

    println!(
        "{} Architecture: {}",
        "│".dimmed(),
        if analysis.is_monorepo {
            format!("Monorepo ({} projects)", analysis.projects.len()).yellow()
        } else {
            "Single Project".to_string().yellow()
        }
    );

    println!(
        "{} Pattern: {}",
        "│".dimmed(),
        format!("{:?}", analysis.technology_summary.architecture_pattern).green()
    );
    println!(
        "{} Stack: {}",
        "│".dimmed(),
        analysis.technology_summary.languages.join(", ").blue()
    );

    if !analysis.technology_summary.frameworks.is_empty() {
        println!(
            "{} Frameworks: {}",
            "│".dimmed(),
            analysis.technology_summary.frameworks.join(", ").magenta()
        );
    }

    println!(
        "{} Analysis Time: {}ms",
        "│".dimmed(),
        analysis.metadata.analysis_duration_ms
    );
    println!(
        "{} Confidence: {:.0}%",
        "│".dimmed(),
        analysis.metadata.confidence_score * 100.0
    );

    println!("{}", "─".repeat(50).dimmed());
}

/// Display summary view - returns string
pub fn display_summary_view_to_string(analysis: &MonorepoAnalysis) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "\n{} {}\n",
        "▶".bright_blue(),
        "PROJECT ANALYSIS SUMMARY".bright_white().bold()
    ));
    output.push_str(&format!("{}\n", "─".repeat(50).dimmed()));

    output.push_str(&format!(
        "{} Architecture: {}\n",
        "│".dimmed(),
        if analysis.is_monorepo {
            format!("Monorepo ({} projects)", analysis.projects.len()).yellow()
        } else {
            "Single Project".to_string().yellow()
        }
    ));

    output.push_str(&format!(
        "{} Pattern: {}\n",
        "│".dimmed(),
        format!("{:?}", analysis.technology_summary.architecture_pattern).green()
    ));
    output.push_str(&format!(
        "{} Stack: {}\n",
        "│".dimmed(),
        analysis.technology_summary.languages.join(", ").blue()
    ));

    if !analysis.technology_summary.frameworks.is_empty() {
        output.push_str(&format!(
            "{} Frameworks: {}\n",
            "│".dimmed(),
            analysis.technology_summary.frameworks.join(", ").magenta()
        ));
    }

    output.push_str(&format!(
        "{} Analysis Time: {}ms\n",
        "│".dimmed(),
        analysis.metadata.analysis_duration_ms
    ));
    output.push_str(&format!(
        "{} Confidence: {:.0}%\n",
        "│".dimmed(),
        analysis.metadata.confidence_score * 100.0
    ));

    output.push_str(&format!("{}\n", "─".repeat(50).dimmed()));

    output
}
