//! UI helpers for the chat session
//!
//! Contains display functions for help, logo, and welcome banner.

use super::{find_incomplete_plans, ChatSession};
use crate::agent::commands::SLASH_COMMANDS;
use crate::agent::ui::ansi;
use colored::Colorize;

const ROBOT: &str = "🤖";

/// Print help with available commands
pub fn print_help() {
    println!();
    println!(
        "  {}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}",
        ansi::PURPLE,
        ansi::RESET
    );
    println!("  {}📖 Available Commands{}", ansi::PURPLE, ansi::RESET);
    println!(
        "  {}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}",
        ansi::PURPLE,
        ansi::RESET
    );
    println!();

    for cmd in SLASH_COMMANDS.iter() {
        let alias = cmd.alias.map(|a| format!(" ({})", a)).unwrap_or_default();
        println!(
            "  {}/{:<12}{}{} - {}{}{}",
            ansi::CYAN,
            cmd.name,
            alias,
            ansi::RESET,
            ansi::DIM,
            cmd.description,
            ansi::RESET
        );
    }

    println!();
    println!(
        "  {}Tip: Type / to see interactive command picker!{}",
        ansi::DIM,
        ansi::RESET
    );
    println!();
}

/// Print session banner with colorful SYNCABLE ASCII art
pub fn print_logo() {
    // Colors matching the logo gradient: purple → orange → pink
    // Using ANSI 256 colors for better gradient

    // Purple shades for S, y
    let purple = "\x1b[38;5;141m"; // Light purple
    // Orange shades for n, c
    let orange = "\x1b[38;5;216m"; // Peach/orange
    // Pink shades for a, b, l, e
    let pink = "\x1b[38;5;212m"; // Hot pink
    let magenta = "\x1b[38;5;207m"; // Magenta
    let reset = "\x1b[0m";

    println!();
    println!(
        "{}  ███████╗{}{} ██╗   ██╗{}{}███╗   ██╗{}{} ██████╗{}{}  █████╗ {}{}██████╗ {}{}██╗     {}{}███████╗{}",
        purple,
        reset,
        purple,
        reset,
        orange,
        reset,
        orange,
        reset,
        pink,
        reset,
        pink,
        reset,
        magenta,
        reset,
        magenta,
        reset
    );
    println!(
        "{}  ██╔════╝{}{} ╚██╗ ██╔╝{}{}████╗  ██║{}{} ██╔════╝{}{} ██╔══██╗{}{}██╔══██╗{}{}██║     {}{}██╔════╝{}",
        purple,
        reset,
        purple,
        reset,
        orange,
        reset,
        orange,
        reset,
        pink,
        reset,
        pink,
        reset,
        magenta,
        reset,
        magenta,
        reset
    );
    println!(
        "{}  ███████╗{}{}  ╚████╔╝ {}{}██╔██╗ ██║{}{} ██║     {}{} ███████║{}{}██████╔╝{}{}██║     {}{}█████╗  {}",
        purple,
        reset,
        purple,
        reset,
        orange,
        reset,
        orange,
        reset,
        pink,
        reset,
        pink,
        reset,
        magenta,
        reset,
        magenta,
        reset
    );
    println!(
        "{}  ╚════██║{}{}   ╚██╔╝  {}{}██║╚██╗██║{}{} ██║     {}{} ██╔══██║{}{}██╔══██╗{}{}██║     {}{}██╔══╝  {}",
        purple,
        reset,
        purple,
        reset,
        orange,
        reset,
        orange,
        reset,
        pink,
        reset,
        pink,
        reset,
        magenta,
        reset,
        magenta,
        reset
    );
    println!(
        "{}  ███████║{}{}    ██║   {}{}██║ ╚████║{}{} ╚██████╗{}{} ██║  ██║{}{}██████╔╝{}{}███████╗{}{}███████╗{}",
        purple,
        reset,
        purple,
        reset,
        orange,
        reset,
        orange,
        reset,
        pink,
        reset,
        pink,
        reset,
        magenta,
        reset,
        magenta,
        reset
    );
    println!(
        "{}  ╚══════╝{}{}    ╚═╝   {}{}╚═╝  ╚═══╝{}{}  ╚═════╝{}{} ╚═╝  ╚═╝{}{}╚═════╝ {}{}╚══════╝{}{}╚══════╝{}",
        purple,
        reset,
        purple,
        reset,
        orange,
        reset,
        orange,
        reset,
        pink,
        reset,
        pink,
        reset,
        magenta,
        reset,
        magenta,
        reset
    );
    println!();
}

/// Print the welcome banner
pub fn print_banner(session: &ChatSession) {
    // Print the gradient ASCII logo
    print_logo();

    // Platform promo
    println!(
        "  {} {}",
        "🚀".dimmed(),
        "Want to deploy? Deploy instantly from Syncable Platform → https://syncable.dev".dimmed()
    );
    println!();

    // Print agent info
    println!(
        "  {} {} powered by {}: {}",
        ROBOT,
        "Syncable Agent".white().bold(),
        session.provider.to_string().cyan(),
        session.model.cyan()
    );
    println!("  {}", "Your AI-powered code analysis assistant".dimmed());

    // Check for incomplete plans and show a hint
    let incomplete_plans = find_incomplete_plans(&session.project_path);
    if !incomplete_plans.is_empty() {
        println!();
        if incomplete_plans.len() == 1 {
            let plan = &incomplete_plans[0];
            println!(
                "  {} {} ({}/{} done)",
                "📋 Incomplete plan:".yellow(),
                plan.filename.white(),
                plan.done,
                plan.total
            );
            println!(
                "     {} \"{}\" {}",
                "→".cyan(),
                "continue".cyan().bold(),
                "to resume".dimmed()
            );
        } else {
            println!(
                "  {} {} incomplete plans found. Use {} to see them.",
                "📋".yellow(),
                incomplete_plans.len(),
                "/plans".cyan()
            );
        }
    }

    println!();
    println!(
        "  {} Type your questions. Use {} to exit.\n",
        "→".cyan(),
        "exit".yellow().bold()
    );
}
