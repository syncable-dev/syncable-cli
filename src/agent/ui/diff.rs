//! Diff rendering for file change confirmation
//!
//! Provides visual diff display for file modifications, showing
//! additions in green and deletions in red with line numbers.
//!
//! When an IDE companion extension is connected, diffs can be shown
//! in the IDE's native diff viewer for a better experience.

use colored::Colorize;
use inquire::ui::{Color, IndexPrefix, RenderConfig, StyleSheet, Styled};
use similar::{ChangeTag, TextDiff};
use std::io::{self, Write};

use crate::agent::ide::{DiffResult, IdeClient};

/// Safely truncate a string to a maximum number of characters (not bytes)
/// This avoids panics when slicing UTF-8 strings with multi-byte characters
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Get custom render config for file confirmation prompts
fn get_file_confirmation_render_config() -> RenderConfig<'static> {
    RenderConfig::default()
        .with_highlighted_option_prefix(Styled::new("> ").with_fg(Color::LightCyan))
        .with_option_index_prefix(IndexPrefix::Simple)
        .with_selected_option(Some(StyleSheet::new().with_fg(Color::LightCyan)))
}

/// Render a diff between old and new content
pub fn render_diff(old_content: &str, new_content: &str, filename: &str) {
    let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
    let box_width = term_width.min(80);
    let inner_width = box_width - 4;

    // Header
    let header = format!(" {} ", filename);
    let header_len = header.len();
    let left_dashes = (inner_width.saturating_sub(header_len)) / 2;
    let right_dashes = inner_width.saturating_sub(header_len).saturating_sub(left_dashes);

    println!(
        "{}{}{}{}{}",
        "┌".dimmed(),
        "─".repeat(left_dashes).dimmed(),
        header.white().bold(),
        "─".repeat(right_dashes).dimmed(),
        "┐".dimmed()
    );

    let diff = TextDiff::from_lines(old_content, new_content);
    let mut old_line = 1usize;
    let mut new_line = 1usize;

    for change in diff.iter_all_changes() {
        let (line_num_display, prefix, content, style) = match change.tag() {
            ChangeTag::Delete => {
                let ln = format!("{:>4}", old_line);
                old_line += 1;
                (ln, "-", change.value().trim_end(), "red")
            }
            ChangeTag::Insert => {
                let ln = format!("{:>4}", new_line);
                new_line += 1;
                (ln, "+", change.value().trim_end(), "green")
            }
            ChangeTag::Equal => {
                let ln = format!("{:>4}", new_line);
                old_line += 1;
                new_line += 1;
                (ln, " ", change.value().trim_end(), "normal")
            }
        };

        // Truncate content if needed (using character count, not bytes)
        let max_content_len = inner_width.saturating_sub(8); // line num + prefix + spaces
        let truncated = truncate_str(content, max_content_len);

        match style {
            "red" => println!(
                "{} {} {} {}",
                "│".dimmed(),
                line_num_display.dimmed(),
                prefix.red().bold(),
                truncated.red()
            ),
            "green" => println!(
                "{} {} {} {}",
                "│".dimmed(),
                line_num_display.dimmed(),
                prefix.green().bold(),
                truncated.green()
            ),
            _ => println!(
                "{} {} {} {}",
                "│".dimmed(),
                line_num_display.dimmed(),
                prefix,
                truncated
            ),
        }
    }

    // Footer
    println!(
        "{}{}{}",
        "└".dimmed(),
        "─".repeat(box_width - 2).dimmed(),
        "┘".dimmed()
    );
    println!();

    let _ = io::stdout().flush();
}

/// Render a new file (all additions)
pub fn render_new_file(content: &str, filename: &str) {
    let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
    let box_width = term_width.min(80);
    let inner_width = box_width - 4;

    // Header with "new file" indicator
    let header = format!(" {} (new file) ", filename);
    let header_len = header.len();
    let left_dashes = (inner_width.saturating_sub(header_len)) / 2;
    let right_dashes = inner_width.saturating_sub(header_len).saturating_sub(left_dashes);

    println!(
        "{}{}{}{}{}",
        "┌".dimmed(),
        "─".repeat(left_dashes).dimmed(),
        header.green().bold(),
        "─".repeat(right_dashes).dimmed(),
        "┐".dimmed()
    );

    // Show first N lines as preview
    const MAX_PREVIEW_LINES: usize = 20;
    let lines: Vec<&str> = content.lines().collect();
    let show_truncation = lines.len() > MAX_PREVIEW_LINES;

    for (i, line) in lines.iter().take(MAX_PREVIEW_LINES).enumerate() {
        let line_num = format!("{:>4}", i + 1);
        let max_content_len = inner_width.saturating_sub(8);
        let truncated = truncate_str(line, max_content_len);

        println!(
            "{} {} {} {}",
            "│".dimmed(),
            line_num.dimmed(),
            "+".green().bold(),
            truncated.green()
        );
    }

    if show_truncation {
        let remaining = lines.len() - MAX_PREVIEW_LINES;
        println!(
            "{} {} {} {}",
            "│".dimmed(),
            "    ".dimmed(),
            "...".dimmed(),
            format!("({} more lines)", remaining).dimmed()
        );
    }

    // Footer
    println!(
        "{}{}{}",
        "└".dimmed(),
        "─".repeat(box_width - 2).dimmed(),
        "┘".dimmed()
    );
    println!();

    let _ = io::stdout().flush();
}

/// Confirm file write with diff display and optional IDE integration
pub fn confirm_file_write(
    path: &str,
    old_content: Option<&str>,
    new_content: &str,
) -> crate::agent::ui::confirmation::ConfirmationResult {
    use crate::agent::ui::confirmation::ConfirmationResult;
    use inquire::{InquireError, Select, Text};

    // Show terminal diff
    match old_content {
        Some(old) => render_diff(old, new_content, path),
        None => render_new_file(new_content, path),
    };

    let options = vec![
        "Yes, allow once".to_string(),
        "Yes, allow always".to_string(),
        "Type here to suggest changes".to_string(),
    ];

    println!("{}", "Apply this change?".white());

    let selection = Select::new("", options.clone())
        .with_render_config(get_file_confirmation_render_config())
        .with_page_size(3)  // Show all 3 options
        .with_help_message("↑↓ to move, Enter to select, Esc to cancel")
        .prompt();

    match selection {
        Ok(answer) => {
            if answer == options[0] {
                ConfirmationResult::Proceed
            } else if answer == options[1] {
                // Allow always for this file pattern
                let filename = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string());
                ConfirmationResult::ProceedAlways(filename)
            } else {
                // User wants to type feedback
                println!();
                match Text::new("What changes would you like?")
                    .with_help_message("Press Enter to submit, Esc to cancel")
                    .prompt()
                {
                    Ok(feedback) if !feedback.trim().is_empty() => {
                        ConfirmationResult::Modify(feedback)
                    }
                    _ => ConfirmationResult::Cancel,
                }
            }
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            ConfirmationResult::Cancel
        }
        Err(_) => ConfirmationResult::Cancel,
    }
}

/// Confirm file write with IDE integration
///
/// When an IDE client is connected, this shows BOTH:
/// 1. The diff in the IDE's native diff viewer
/// 2. A terminal menu for confirmation
///
/// The user can respond from either place - first response wins.
///
/// # Arguments
/// * `path` - Path to the file being modified
/// * `old_content` - Current file content (None for new files)
/// * `new_content` - Proposed new content
/// * `ide_client` - Optional IDE client for native diff viewing
///
/// # Returns
/// A `ConfirmationResult` indicating the user's decision
pub async fn confirm_file_write_with_ide(
    path: &str,
    old_content: Option<&str>,
    new_content: &str,
    ide_client: Option<&IdeClient>,
) -> crate::agent::ui::confirmation::ConfirmationResult {
    use crate::agent::ui::confirmation::ConfirmationResult;
    use inquire::{InquireError, Select, Text};
    use tokio::sync::oneshot;

    // Show terminal diff first
    match old_content {
        Some(old) => render_diff(old, new_content, path),
        None => render_new_file(new_content, path),
    };

    // Try to open IDE diff if connected (non-blocking)
    let ide_connected = ide_client.map(|c| c.is_connected()).unwrap_or(false);

    if ide_connected {
        let client = ide_client.unwrap();

        // Convert to absolute path for IDE
        let abs_path = std::path::Path::new(path)
            .canonicalize()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string());

        // Create channels for communication
        let (terminal_tx, terminal_rx) = oneshot::channel::<ConfirmationResult>();
        let (cancel_tx, _cancel_rx) = oneshot::channel::<()>();
        let (menu_ready_tx, menu_ready_rx) = oneshot::channel::<()>();

        // Spawn terminal input on blocking thread
        let path_owned = path.to_string();
        let ide_name = client.ide_name().unwrap_or("IDE").to_string();
        let terminal_handle = tokio::task::spawn_blocking(move || {
            let options = vec![
                "Yes, allow once".to_string(),
                "Yes, allow always".to_string(),
                "Type here to suggest changes".to_string(),
            ];

            println!(
                "{} Diff opened in {} - respond here or in the IDE",
                "→".cyan(),
                ide_name
            );
            println!("{}", "Apply this change?".white());

            // Signal that the menu is about to be displayed
            let _ = menu_ready_tx.send(());

            let selection = Select::new("", options.clone())
                .with_render_config(get_file_confirmation_render_config())
                .with_page_size(3)
                .with_help_message("↑↓ to move, Enter to select, Esc to cancel (or use IDE)")
                .prompt();

            let result = match selection {
                Ok(answer) => {
                    if answer == options[0] {
                        ConfirmationResult::Proceed
                    } else if answer == options[1] {
                        let filename = std::path::Path::new(&path_owned)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path_owned.clone());
                        ConfirmationResult::ProceedAlways(filename)
                    } else {
                        // User wants to type feedback
                        println!();
                        match Text::new("What changes would you like?")
                            .with_help_message("Press Enter to submit, Esc to cancel")
                            .prompt()
                        {
                            Ok(feedback) if !feedback.trim().is_empty() => {
                                ConfirmationResult::Modify(feedback)
                            }
                            _ => ConfirmationResult::Cancel,
                        }
                    }
                }
                Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
                    ConfirmationResult::Cancel
                }
                Err(_) => ConfirmationResult::Cancel,
            };

            let _ = terminal_tx.send(result);
        });

        // Wait for terminal menu to be ready before opening IDE diff
        let _ = menu_ready_rx.await;

        // Small delay to ensure terminal has rendered the Select prompt
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Now open IDE diff
        let ide_future = client.open_diff(&abs_path, new_content);

        // Race: first response wins
        tokio::select! {
            // IDE responded
            ide_result = ide_future => {
                // Cancel terminal input (it's blocking so we can't really cancel it,
                // but we'll ignore its result)
                let _ = cancel_tx.send(());
                terminal_handle.abort();

                match ide_result {
                    Ok(DiffResult::Accepted { content: _ }) => {
                        println!("\n{} Changes accepted in IDE", "✓".green());
                        return ConfirmationResult::Proceed;
                    }
                    Ok(DiffResult::Rejected) => {
                        println!("\n{} Changes rejected in IDE", "✗".red());
                        return ConfirmationResult::Cancel;
                    }
                    Err(e) => {
                        println!("\n{} IDE error: {}", "!".yellow(), e);
                        // IDE failed but terminal thread is already running
                        // Just return Cancel - user can retry
                        return ConfirmationResult::Cancel;
                    }
                }
            }
            // Terminal responded
            terminal_result = terminal_rx => {
                // Close IDE diff
                let _ = client.close_diff(&abs_path).await;

                match terminal_result {
                    Ok(result) => {
                        match &result {
                            ConfirmationResult::Proceed => {
                                println!("{} Changes accepted", "✓".green());
                            }
                            ConfirmationResult::ProceedAlways(_) => {
                                println!("{} Changes accepted (always for this file type)", "✓".green());
                            }
                            ConfirmationResult::Cancel => {
                                println!("{} Changes cancelled", "✗".red());
                            }
                            ConfirmationResult::Modify(_) => {
                                println!("{} Feedback provided", "→".cyan());
                            }
                        }
                        return result;
                    }
                    Err(_) => {
                        return ConfirmationResult::Cancel;
                    }
                }
            }
        }
    }

    // No IDE connection - just use terminal
    confirm_file_write(path, old_content, new_content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_render_doesnt_panic() {
        let old = "line 1\nline 2\nline 3";
        let new = "line 1\nmodified line 2\nline 3\nline 4";
        // Just verify it doesn't panic
        render_diff(old, new, "test.txt");
    }

    #[test]
    fn test_new_file_render_doesnt_panic() {
        let content = "new content\nline 2";
        render_new_file(content, "new_file.txt");
    }
}
