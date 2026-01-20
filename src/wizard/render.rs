//! Shared rendering utilities for wizard prompts

use colored::Colorize;
use inquire::ui::{Color, IndexPrefix, RenderConfig, StyleSheet, Styled};

/// Get the standard render config for wizard prompts
pub fn wizard_render_config() -> RenderConfig<'static> {
    RenderConfig::default()
        .with_highlighted_option_prefix(Styled::new("▸ ").with_fg(Color::LightCyan))
        .with_option_index_prefix(IndexPrefix::Simple)
        .with_selected_option(Some(StyleSheet::new().with_fg(Color::LightCyan)))
        .with_scroll_up_prefix(Styled::new("▲ "))
        .with_scroll_down_prefix(Styled::new("▼ "))
}

/// Display a wizard step header box
pub fn display_step_header(step_number: u8, step_name: &str, description: &str) {
    let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
    let box_width = term_width.min(70);
    let inner_width = box_width - 4;

    println!();
    // Top border with step indicator
    let header = format!("─ Step {} · {} ", step_number, step_name);
    println!(
        "{}{}{}",
        "┌".bright_cyan(),
        header.bright_cyan(),
        "─".repeat(inner_width.saturating_sub(header.len())).bright_cyan()
    );

    // Description
    let desc_lines = textwrap::wrap(description, inner_width - 2);
    for line in &desc_lines {
        println!(
            "{}  {}",
            "│".dimmed(),
            line.white()
        );
    }

    // Bottom border
    println!(
        "{}{}",
        "└".dimmed(),
        "─".repeat(box_width - 1).dimmed()
    );
    println!();
}

/// Format a status indicator (checkmark or X)
pub fn status_indicator(connected: bool) -> String {
    if connected {
        "✓".green().to_string()
    } else {
        "✗".red().to_string()
    }
}

/// Format a count badge
pub fn count_badge(count: usize, label: &str) -> String {
    if count > 0 {
        format!("{} {}", count.to_string().cyan(), label.dimmed())
    } else {
        format!("{} {}", "0".dimmed(), label.dimmed())
    }
}
