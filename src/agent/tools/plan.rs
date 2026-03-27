//! Plan tools for Forge-style planning workflow
//!
//! Provides tools for creating and executing structured plans:
//! - `PlanCreateTool` - Create plan files with task checkboxes
//! - `PlanNextTool` - Get next pending task and mark it in-progress
//! - `PlanUpdateTool` - Update task status (done, failed)
//!
//! ## Task Status Format
//!
//! ```markdown
//! - [ ] Task description (PENDING)
//! - [~] Task description (IN_PROGRESS)
//! - [x] Task description (DONE)
//! - [!] Task description (FAILED: reason)
//! ```

use chrono::Local;
use regex::Regex;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

// ============================================================================
// Task Status Types
// ============================================================================

/// Task status in a plan file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,    // [ ]
    InProgress, // [~]
    Done,       // [x]
    Failed,     // [!]
}

impl TaskStatus {
    fn marker(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "[ ]",
            TaskStatus::InProgress => "[~]",
            TaskStatus::Done => "[x]",
            TaskStatus::Failed => "[!]",
        }
    }

    #[allow(dead_code)]
    fn from_marker(s: &str) -> Option<Self> {
        match s {
            "[ ]" => Some(TaskStatus::Pending),
            "[~]" => Some(TaskStatus::InProgress),
            "[x]" => Some(TaskStatus::Done),
            "[!]" => Some(TaskStatus::Failed),
            _ => None,
        }
    }
}

/// A task parsed from a plan file
#[derive(Debug, Clone)]
pub struct PlanTask {
    pub index: usize, // 1-based index
    pub status: TaskStatus,
    pub description: String,
    #[allow(dead_code)]
    pub line_number: usize, // Line number in file (1-based)
}

// ============================================================================
// Plan Parser
// ============================================================================

/// Parse tasks from plan file content
fn parse_plan_tasks(content: &str) -> Vec<PlanTask> {
    let task_regex = Regex::new(r"^(\s*)-\s*\[([ x~!])\]\s*(.+)$").unwrap();
    let mut tasks = Vec::new();
    let mut task_index = 0;

    for (line_idx, line) in content.lines().enumerate() {
        if let Some(caps) = task_regex.captures(line) {
            task_index += 1;
            let marker_char = caps.get(2).map(|m| m.as_str()).unwrap_or(" ");
            let description = caps.get(3).map(|m| m.as_str()).unwrap_or("").to_string();

            let status = match marker_char {
                " " => TaskStatus::Pending,
                "~" => TaskStatus::InProgress,
                "x" => TaskStatus::Done,
                "!" => TaskStatus::Failed,
                _ => TaskStatus::Pending,
            };

            tasks.push(PlanTask {
                index: task_index,
                status,
                description,
                line_number: line_idx + 1,
            });
        }
    }

    tasks
}

/// Update a task's status in the plan file content
fn update_task_status(
    content: &str,
    task_index: usize,
    new_status: TaskStatus,
    note: Option<&str>,
) -> Option<String> {
    let task_regex = Regex::new(r"^(\s*)-\s*\[[ x~!]\]\s*(.+)$").unwrap();
    let mut current_index = 0;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();

    for (line_idx, line) in content.lines().enumerate() {
        if task_regex.is_match(line) {
            current_index += 1;
            if current_index == task_index {
                // Found the task to update
                let caps = task_regex.captures(line)?;
                let indent = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let desc = caps.get(2).map(|m| m.as_str()).unwrap_or("");

                // Build new line with updated status
                let new_line = if new_status == TaskStatus::Failed {
                    let fail_note = note.unwrap_or("unknown reason");
                    format!(
                        "{}- {} {} (FAILED: {})",
                        indent,
                        new_status.marker(),
                        desc,
                        fail_note
                    )
                } else {
                    format!("{}- {} {}", indent, new_status.marker(), desc)
                };

                lines[line_idx] = new_line;
                return Some(lines.join("\n"));
            }
        }
    }

    None // Task not found
}

// ============================================================================
// Plan Create Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PlanCreateArgs {
    /// Short name for the plan (e.g., "auth-feature", "refactor-db")
    pub plan_name: String,
    /// Version identifier (e.g., "v1", "draft")
    pub version: Option<String>,
    /// Markdown content with task checkboxes (- [ ] task description)
    pub content: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Plan create error: {0}")]
pub struct PlanCreateError(String);

#[derive(Debug, Clone)]
pub struct PlanCreateTool {
    project_path: PathBuf,
}

impl PlanCreateTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }
}

impl Tool for PlanCreateTool {
    const NAME: &'static str = "plan_create";

    type Error = PlanCreateError;
    type Args = PlanCreateArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Create a structured plan file with task checkboxes. Use this in plan mode to document implementation steps.

The plan file will be created in the `plans/` directory with format: {date}-{plan_name}-{version}.md

IMPORTANT: Each task MUST use the checkbox format: `- [ ] Task description`

Example content:
```markdown
# Authentication Feature Plan

## Overview
Add user authentication to the application.

## Tasks

- [ ] Create User model in src/models/user.rs
- [ ] Add password hashing with bcrypt
- [ ] Create login endpoint at POST /api/login
- [ ] Add JWT token generation
- [ ] Create authentication middleware
- [ ] Write tests for auth flow
```

The task status markers are:
- `[ ]` - PENDING (not started)
- `[~]` - IN_PROGRESS (currently being worked on)
- `[x]` - DONE (completed)
- `[!]` - FAILED (failed with reason)"#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "plan_name": {
                        "type": "string",
                        "description": "Short kebab-case name for the plan (e.g., 'auth-feature', 'refactor-db')"
                    },
                    "version": {
                        "type": "string",
                        "description": "Optional version identifier (e.g., 'v1', 'draft'). Defaults to 'v1'"
                    },
                    "content": {
                        "type": "string",
                        "description": "Markdown content with task checkboxes. Each task must be: '- [ ] Task description'"
                    }
                },
                "required": ["plan_name", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate plan name (kebab-case)
        let plan_name = args.plan_name.trim().to_lowercase().replace(' ', "-");
        if plan_name.is_empty() {
            return Err(PlanCreateError("Plan name cannot be empty".to_string()));
        }

        // Validate content has at least one task
        let tasks = parse_plan_tasks(&args.content);
        if tasks.is_empty() {
            return Err(PlanCreateError(
                "Plan must contain at least one task with format: '- [ ] Task description'"
                    .to_string(),
            ));
        }

        // Build filename: {date}-{plan_name}-{version}.md
        let version = args.version.unwrap_or_else(|| "v1".to_string());
        let date = Local::now().format("%Y-%m-%d");
        let filename = format!("{}-{}-{}.md", date, plan_name, version);

        // Create plans directory if it doesn't exist
        let plans_dir = self.project_path.join("plans");
        if !plans_dir.exists() {
            fs::create_dir_all(&plans_dir)
                .map_err(|e| PlanCreateError(format!("Failed to create plans directory: {}", e)))?;
        }

        // Check if file already exists
        let file_path = plans_dir.join(&filename);
        if file_path.exists() {
            return Err(PlanCreateError(format!(
                "Plan file already exists: {}. Use a different name or version.",
                filename
            )));
        }

        // Write the plan file
        fs::write(&file_path, &args.content)
            .map_err(|e| PlanCreateError(format!("Failed to write plan file: {}", e)))?;

        // Get relative path for display
        let rel_path = file_path
            .strip_prefix(&self.project_path)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| file_path.display().to_string());

        let result = json!({
            "success": true,
            "plan_path": rel_path,
            "filename": filename,
            "task_count": tasks.len(),
            "tasks": tasks.iter().map(|t| json!({
                "index": t.index,
                "description": t.description,
                "status": "pending"
            })).collect::<Vec<_>>(),
            "next_steps": "Plan created successfully. Choose an execution option from the menu."
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| PlanCreateError(format!("Failed to serialize: {}", e)))
    }
}

// ============================================================================
// Plan Next Tool - Get next pending task
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PlanNextArgs {
    /// Path to the plan file (relative or absolute)
    pub plan_path: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Plan next error: {0}")]
pub struct PlanNextError(String);

#[derive(Debug, Clone)]
pub struct PlanNextTool {
    project_path: PathBuf,
}

impl PlanNextTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        let p = PathBuf::from(path);
        if p.is_absolute() {
            p
        } else {
            self.project_path.join(p)
        }
    }
}

impl Tool for PlanNextTool {
    const NAME: &'static str = "plan_next";

    type Error = PlanNextError;
    type Args = PlanNextArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Get the next pending task from a plan file and mark it as in-progress.

This tool:
1. Reads the plan file
2. Finds the first `[ ]` (PENDING) task
3. Updates it to `[~]` (IN_PROGRESS) in the file
4. Returns the task description for you to execute

After executing the task, use `plan_update` to mark it as done or failed.

Returns null task if all tasks are complete."#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "plan_path": {
                        "type": "string",
                        "description": "Path to the plan file (e.g., 'plans/2025-01-15-auth-feature-v1.md')"
                    }
                },
                "required": ["plan_path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let file_path = self.resolve_path(&args.plan_path);

        // Read plan file
        let content = fs::read_to_string(&file_path)
            .map_err(|e| PlanNextError(format!("Failed to read plan file: {}", e)))?;

        // Parse tasks
        let tasks = parse_plan_tasks(&content);
        if tasks.is_empty() {
            return Err(PlanNextError("No tasks found in plan file".to_string()));
        }

        // Find first pending task
        let pending_task = tasks.iter().find(|t| t.status == TaskStatus::Pending);

        match pending_task {
            Some(task) => {
                // Update task to in-progress
                let updated_content =
                    update_task_status(&content, task.index, TaskStatus::InProgress, None)
                        .ok_or_else(|| PlanNextError("Failed to update task status".to_string()))?;

                // Write updated content
                fs::write(&file_path, &updated_content)
                    .map_err(|e| PlanNextError(format!("Failed to write plan file: {}", e)))?;

                // Count task states
                let done_count = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Done)
                    .count();
                let pending_count = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Pending)
                    .count()
                    - 1; // -1 for current
                let failed_count = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Failed)
                    .count();

                let result = json!({
                    "has_task": true,
                    "task_index": task.index,
                    "task_description": task.description,
                    "total_tasks": tasks.len(),
                    "completed": done_count,
                    "pending": pending_count,
                    "failed": failed_count,
                    "progress": format!("{}/{}", done_count, tasks.len()),
                    "instructions": "Execute this task using appropriate tools, then call plan_update to mark it done."
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| PlanNextError(format!("Failed to serialize: {}", e)))
            }
            None => {
                // No pending tasks - check if all done
                let done_count = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Done)
                    .count();
                let failed_count = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Failed)
                    .count();
                let in_progress = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::InProgress)
                    .count();

                let result = json!({
                    "has_task": false,
                    "total_tasks": tasks.len(),
                    "completed": done_count,
                    "failed": failed_count,
                    "in_progress": in_progress,
                    "status": if in_progress > 0 {
                        "Task in progress - complete it before getting next"
                    } else if failed_count > 0 {
                        "Plan completed with failures"
                    } else {
                        "All tasks completed successfully!"
                    }
                });

                serde_json::to_string_pretty(&result)
                    .map_err(|e| PlanNextError(format!("Failed to serialize: {}", e)))
            }
        }
    }
}

// ============================================================================
// Plan Update Tool - Update task status
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PlanUpdateArgs {
    /// Path to the plan file
    pub plan_path: String,
    /// 1-based task index to update
    pub task_index: usize,
    /// New status: "done", "failed", or "pending"
    pub status: String,
    /// Optional note for failed tasks
    pub note: Option<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("Plan update error: {0}")]
pub struct PlanUpdateError(String);

#[derive(Debug, Clone)]
pub struct PlanUpdateTool {
    project_path: PathBuf,
}

impl PlanUpdateTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        let p = PathBuf::from(path);
        if p.is_absolute() {
            p
        } else {
            self.project_path.join(p)
        }
    }
}

impl Tool for PlanUpdateTool {
    const NAME: &'static str = "plan_update";

    type Error = PlanUpdateError;
    type Args = PlanUpdateArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Update the status of a task in a plan file.

Use this after completing or failing a task to update its status:
- "done" - Mark task as completed `[x]`
- "failed" - Mark task as failed `[!]` (include a note explaining why)
- "pending" - Reset task to pending `[ ]`

After marking a task done, call `plan_next` to get the next task."#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "plan_path": {
                        "type": "string",
                        "description": "Path to the plan file"
                    },
                    "task_index": {
                        "type": "integer",
                        "description": "1-based index of the task to update"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["done", "failed", "pending"],
                        "description": "New status for the task"
                    },
                    "note": {
                        "type": "string",
                        "description": "Optional note explaining failure (required for 'failed' status)"
                    }
                },
                "required": ["plan_path", "task_index", "status"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let file_path = self.resolve_path(&args.plan_path);

        // Read plan file
        let content = fs::read_to_string(&file_path)
            .map_err(|e| PlanUpdateError(format!("Failed to read plan file: {}", e)))?;

        // Parse status
        let new_status = match args.status.to_lowercase().as_str() {
            "done" => TaskStatus::Done,
            "failed" => TaskStatus::Failed,
            "pending" => TaskStatus::Pending,
            _ => {
                return Err(PlanUpdateError(format!(
                    "Invalid status '{}'. Use: done, failed, or pending",
                    args.status
                )));
            }
        };

        // Require note for failed status
        if new_status == TaskStatus::Failed && args.note.is_none() {
            return Err(PlanUpdateError(
                "A note is required when marking a task as failed".to_string(),
            ));
        }

        // Update task status
        let updated_content =
            update_task_status(&content, args.task_index, new_status, args.note.as_deref())
                .ok_or_else(|| {
                    PlanUpdateError(format!("Task {} not found in plan", args.task_index))
                })?;

        // Write updated content
        fs::write(&file_path, &updated_content)
            .map_err(|e| PlanUpdateError(format!("Failed to write plan file: {}", e)))?;

        // Parse updated tasks for summary
        let tasks = parse_plan_tasks(&updated_content);
        let done_count = tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Done)
            .count();
        let pending_count = tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .count();
        let failed_count = tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Failed)
            .count();

        let result = json!({
            "success": true,
            "task_index": args.task_index,
            "new_status": args.status,
            "progress": format!("{}/{}", done_count, tasks.len()),
            "summary": {
                "total": tasks.len(),
                "done": done_count,
                "pending": pending_count,
                "failed": failed_count
            },
            "next_action": if pending_count > 0 {
                "Call plan_next to get the next pending task"
            } else if failed_count > 0 {
                "Plan complete with failures. Review failed tasks."
            } else {
                "All tasks completed! Plan execution finished."
            }
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| PlanUpdateError(format!("Failed to serialize: {}", e)))
    }
}

// ============================================================================
// Plan List Tool - List available plans
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PlanListArgs {
    /// Optional filter by status (e.g., "incomplete" to show plans with pending tasks)
    pub filter: Option<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("Plan list error: {0}")]
pub struct PlanListError(String);

#[derive(Debug, Clone)]
pub struct PlanListTool {
    project_path: PathBuf,
}

impl PlanListTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }
}

impl Tool for PlanListTool {
    const NAME: &'static str = "plan_list";

    type Error = PlanListError;
    type Args = PlanListArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"List all plan files in the plans/ directory with their status summary.

Shows each plan with:
- Filename and path
- Task counts (done/pending/failed)
- Overall status"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "enum": ["all", "incomplete", "complete"],
                        "description": "Filter plans: 'all' (default), 'incomplete' (has pending), 'complete' (no pending)"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let plans_dir = self.project_path.join("plans");

        if !plans_dir.exists() {
            let result = json!({
                "plans": [],
                "message": "No plans directory found. Create a plan first with plan_create."
            });
            return serde_json::to_string_pretty(&result)
                .map_err(|e| PlanListError(format!("Failed to serialize: {}", e)));
        }

        let filter = args.filter.as_deref().unwrap_or("all");
        let mut plans = Vec::new();

        let entries = fs::read_dir(&plans_dir)
            .map_err(|e| PlanListError(format!("Failed to read plans directory: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false)
                && let Ok(content) = fs::read_to_string(&path)
            {
                let tasks = parse_plan_tasks(&content);
                let done = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Done)
                    .count();
                let pending = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Pending)
                    .count();
                let in_progress = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::InProgress)
                    .count();
                let failed = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Failed)
                    .count();

                // Apply filter
                let include = match filter {
                    "incomplete" => pending > 0 || in_progress > 0,
                    "complete" => pending == 0 && in_progress == 0,
                    _ => true,
                };

                if include {
                    let rel_path = path
                        .strip_prefix(&self.project_path)
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| path.display().to_string());

                    plans.push(json!({
                        "path": rel_path,
                        "filename": path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                        "tasks": {
                            "total": tasks.len(),
                            "done": done,
                            "pending": pending,
                            "in_progress": in_progress,
                            "failed": failed
                        },
                        "progress": format!("{}/{}", done, tasks.len()),
                        "status": if pending == 0 && in_progress == 0 {
                            if failed > 0 { "completed_with_failures" } else { "complete" }
                        } else if in_progress > 0 {
                            "in_progress"
                        } else {
                            "pending"
                        }
                    }));
                }
            }
        }

        // Sort by filename (most recent first due to date prefix)
        plans.sort_by(|a, b| {
            let a_name = a.get("filename").and_then(|v| v.as_str()).unwrap_or("");
            let b_name = b.get("filename").and_then(|v| v.as_str()).unwrap_or("");
            b_name.cmp(a_name)
        });

        let result = json!({
            "plans": plans,
            "total": plans.len(),
            "filter": filter
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| PlanListError(format!("Failed to serialize: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_list_plans_empty_directory() {
        let dir = tempdir().unwrap();
        let tool = PlanListTool::new(dir.path().to_path_buf());
        let args = PlanListArgs { filter: None };

        let result = tool.call(args).await.unwrap();
        // Should return valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_object());
        // No plans should mean total is 0 or plans is empty array
        if let Some(total) = parsed.get("total") {
            assert!(total.as_u64().unwrap_or(0) == 0);
        }
    }

    #[tokio::test]
    async fn test_list_plans_with_plans() {
        let dir = tempdir().unwrap();
        let plans_dir = dir.path().join(".plans");
        std::fs::create_dir(&plans_dir).unwrap();
        std::fs::write(
            plans_dir.join("2026-01-15-test.md"),
            "# Test Plan\n\nSome content",
        )
        .unwrap();

        let tool = PlanListTool::new(dir.path().to_path_buf());
        let args = PlanListArgs { filter: None };

        let result = tool.call(args).await.unwrap();
        // Should return valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_object());
    }
}
