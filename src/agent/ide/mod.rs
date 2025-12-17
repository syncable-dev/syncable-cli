//! IDE Integration Module
//!
//! Provides integration with IDEs (VS Code, Cursor, etc.) via MCP (Model Context Protocol).
//! This enables showing file diffs in the IDE's native diff viewer instead of terminal.

pub mod detect;
pub mod types;
pub mod client;

pub use client::{IdeClient, DiffResult, IdeError};
pub use detect::{IdeInfo, detect_ide, get_ide_process_info};
