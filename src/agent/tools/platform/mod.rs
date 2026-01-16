//! Platform tools for managing Syncable platform resources
//!
//! This module provides agent tools for interacting with the Syncable Platform API:
//! - Listing organizations and projects
//! - Selecting and managing project context
//! - Querying current context state
//!
//! ## Tools
//!
//! - `ListOrganizationsTool` - List organizations the user belongs to
//! - `ListProjectsTool` - List projects within an organization
//! - `SelectProjectTool` - Select a project as the current context
//! - `CurrentContextTool` - Get the currently selected project context
//!
//! ## Prerequisites
//!
//! All tools require the user to be authenticated via `sync-ctl auth login`.
//!
//! ## Example Flow
//!
//! 1. User asks: "What projects do I have access to?"
//! 2. Agent calls `list_organizations` to get available organizations
//! 3. Agent calls `list_projects` for each organization
//! 4. User asks: "Select the 'my-project' project"
//! 5. Agent calls `select_project` with the project and organization IDs
//! 6. Agent can then use `current_context` to verify the selection

mod current_context;
mod list_organizations;
mod list_projects;
mod select_project;

pub use current_context::CurrentContextTool;
pub use list_organizations::ListOrganizationsTool;
pub use list_projects::ListProjectsTool;
pub use select_project::SelectProjectTool;
