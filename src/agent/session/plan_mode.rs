//! Plan mode utilities and incomplete plan tracking
//!
//! This module provides:
//! - `PlanMode` enum for toggling between standard and planning modes
//! - `IncompletePlan` struct for tracking plan progress
//! - `find_incomplete_plans` function to discover incomplete plans

use regex::Regex;

/// Information about an incomplete plan
#[derive(Debug, Clone)]
pub struct IncompletePlan {
    pub path: String,
    pub filename: String,
    pub done: usize,
    pub pending: usize,
    pub total: usize,
}

/// Find incomplete plans in the plans/ directory
pub fn find_incomplete_plans(project_path: &std::path::Path) -> Vec<IncompletePlan> {
    let plans_dir = project_path.join("plans");
    if !plans_dir.exists() {
        return Vec::new();
    }

    let task_regex = Regex::new(r"^\s*-\s*\[([ x~!])\]").unwrap();
    let mut incomplete = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&plans_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false)
                && let Ok(content) = std::fs::read_to_string(&path)
            {
                let mut done = 0;
                let mut pending = 0;
                let mut in_progress = 0;

                for line in content.lines() {
                    if let Some(caps) = task_regex.captures(line) {
                        match caps.get(1).map(|m| m.as_str()) {
                            Some("x") => done += 1,
                            Some(" ") => pending += 1,
                            Some("~") => in_progress += 1,
                            Some("!") => done += 1, // Failed counts as "attempted"
                            _ => {}
                        }
                    }
                }

                let total = done + pending + in_progress;
                if total > 0 && (pending > 0 || in_progress > 0) {
                    let rel_path = path
                        .strip_prefix(project_path)
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| path.display().to_string());

                    incomplete.push(IncompletePlan {
                        path: rel_path,
                        filename: path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        done,
                        pending: pending + in_progress,
                        total,
                    });
                }
            }
        }
    }

    // Sort by most recently modified (newest first)
    incomplete.sort_by(|a, b| b.filename.cmp(&a.filename));
    incomplete
}

/// Planning mode state - toggles between standard and plan mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlanMode {
    /// Standard mode - all tools available, normal operation
    #[default]
    Standard,
    /// Planning mode - read-only exploration, no file modifications
    Planning,
}

impl PlanMode {
    /// Toggle between Standard and Planning mode
    pub fn toggle(&self) -> Self {
        match self {
            PlanMode::Standard => PlanMode::Planning,
            PlanMode::Planning => PlanMode::Standard,
        }
    }

    /// Check if in planning mode
    pub fn is_planning(&self) -> bool {
        matches!(self, PlanMode::Planning)
    }

    /// Get display name for the mode
    pub fn display_name(&self) -> &'static str {
        match self {
            PlanMode::Standard => "standard mode",
            PlanMode::Planning => "plan mode",
        }
    }
}
