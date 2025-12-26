//! Interactive menu for post-plan actions
//!
//! Displays after a plan is created with options:
//! 1. Execute and auto-accept changes
//! 2. Execute and review each change
//! 3. Change something - provide feedback

use colored::Colorize;
use inquire::ui::{Color, IndexPrefix, RenderConfig, StyleSheet, Styled};
use inquire::{InquireError, Select, Text};

/// Result of the plan action menu
#[derive(Debug, Clone)]
pub enum PlanActionResult {
    /// Execute plan, auto-accept all file writes
    ExecuteAutoAccept,
    /// Execute plan, require confirmation for each file write
    ExecuteWithReview,
    /// User wants to change the plan, includes feedback
    ChangePlan(String),
    /// User cancelled (Esc or Ctrl+C)
    Cancel,
}

/// Get custom render config for plan menu
fn get_plan_menu_render_config() -> RenderConfig<'static> {
    RenderConfig::default()
        .with_highlighted_option_prefix(Styled::new("▸ ").with_fg(Color::LightCyan))
        .with_option_index_prefix(IndexPrefix::Simple)
        .with_selected_option(Some(StyleSheet::new().with_fg(Color::LightCyan)))
        .with_scroll_up_prefix(Styled::new("▲ "))
        .with_scroll_down_prefix(Styled::new("▼ "))
}

/// Display plan summary box
fn display_plan_box(plan_path: &str, task_count: usize) {
    let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
    let box_width = term_width.min(70);
    let inner_width = box_width - 4;

    // Top border with title
    println!(
        "{}",
        format!(
            "{}{}{}",
            "┌─ Plan Created ".bright_green(),
            "─".repeat(inner_width.saturating_sub(15)).dimmed(),
            "┐".dimmed()
        )
    );

    // Plan path
    let path_display = format!("  {}", plan_path);
    println!(
        "{}{}{}{}",
        "│".dimmed(),
        path_display.cyan(),
        " ".repeat(inner_width.saturating_sub(path_display.len())),
        "│".dimmed()
    );

    // Task count
    let tasks_display = format!("  {} tasks ready to execute", task_count);
    println!(
        "{}{}{}{}",
        "│".dimmed(),
        tasks_display.white(),
        " ".repeat(inner_width.saturating_sub(tasks_display.len())),
        "│".dimmed()
    );

    // Bottom border
    println!(
        "{}",
        format!(
            "{}{}{}",
            "└".dimmed(),
            "─".repeat(box_width - 2).dimmed(),
            "┘".dimmed()
        )
    );
    println!();
}

/// Show the post-plan action menu
///
/// Displays after a plan is created, offering execution options:
/// 1. Execute and auto-accept - runs all tasks without confirmation prompts
/// 2. Execute and review - requires confirmation for each file write
/// 3. Change something - lets user provide feedback to modify the plan
pub fn show_plan_action_menu(plan_path: &str, task_count: usize) -> PlanActionResult {
    display_plan_box(plan_path, task_count);

    let options = vec![
        "Execute and auto-accept changes".to_string(),
        "Execute and review each change".to_string(),
        "Change something in the plan".to_string(),
    ];

    println!("{}", "What would you like to do?".white());

    let selection = Select::new("", options.clone())
        .with_render_config(get_plan_menu_render_config())
        .with_page_size(3)
        .with_help_message("↑↓ to move, Enter to select, Esc to cancel")
        .prompt();

    match selection {
        Ok(answer) => {
            if answer == options[0] {
                println!("{}", "→ Will execute plan with auto-accept".green());
                PlanActionResult::ExecuteAutoAccept
            } else if answer == options[1] {
                println!("{}", "→ Will execute plan with review for each change".yellow());
                PlanActionResult::ExecuteWithReview
            } else {
                // User wants to change the plan
                println!();
                match Text::new("What should be changed in the plan?")
                    .with_help_message("Press Enter to submit, Esc to cancel")
                    .prompt()
                {
                    Ok(feedback) if !feedback.trim().is_empty() => {
                        PlanActionResult::ChangePlan(feedback)
                    }
                    _ => PlanActionResult::Cancel,
                }
            }
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            println!("{}", "Plan execution cancelled.".dimmed());
            PlanActionResult::Cancel
        }
        Err(_) => PlanActionResult::Cancel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Interactive tests require manual testing
    // These are placeholder tests for non-interactive functionality

    #[test]
    fn test_plan_action_result_variants() {
        // Ensure all variants are constructible
        let _ = PlanActionResult::ExecuteAutoAccept;
        let _ = PlanActionResult::ExecuteWithReview;
        let _ = PlanActionResult::ChangePlan("test".to_string());
        let _ = PlanActionResult::Cancel;
    }
}
