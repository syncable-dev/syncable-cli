//! IDE Integration Module
//!
//! Provides integration with IDEs (VS Code, Cursor, etc.) via MCP (Model Context Protocol).
//! This enables showing file diffs in the IDE's native diff viewer instead of terminal.

pub mod client;
pub mod detect;
pub mod types;

pub use client::{DiffResult, IdeClient, IdeError};
pub use detect::{IdeInfo, detect_ide, get_ide_process_info};
pub use types::{Diagnostic, DiagnosticSeverity, DiagnosticsResponse};
