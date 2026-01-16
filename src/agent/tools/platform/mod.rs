//! Platform tools for managing Syncable platform resources
//!
//! This module provides agent tools for interacting with the Syncable Platform API:
//! - Listing organizations and projects
//! - Selecting and managing project context
//! - Querying current context state
//! - Cloud provider connection management
//! - Service deployment management
//!
//! ## Tools
//!
//! - `ListOrganizationsTool` - List organizations the user belongs to
//! - `ListProjectsTool` - List projects within an organization
//! - `SelectProjectTool` - Select a project as the current context
//! - `CurrentContextTool` - Get the currently selected project context
//! - `OpenProviderSettingsTool` - Open cloud provider settings in browser
//! - `CheckProviderConnectionTool` - Check if a cloud provider is connected
//! - `ListDeploymentConfigsTool` - List deployment configurations for a project
//! - `TriggerDeploymentTool` - Trigger a deployment using a config
//! - `GetDeploymentStatusTool` - Get deployment task status
//! - `ListDeploymentsTool` - List recent deployments for a project
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
//!
//! ## Cloud Provider Connection Flow
//!
//! 1. Agent calls `check_provider_connection` to see if GCP/AWS/etc is connected
//! 2. If not connected, agent calls `open_provider_settings` to open browser
//! 3. User completes OAuth flow in browser
//! 4. Agent calls `check_provider_connection` again to verify
//!
//! ## Deployment Flow
//!
//! 1. Agent calls `list_deployment_configs` to see available deployment configs
//! 2. Agent calls `trigger_deployment` with project_id and config_id
//! 3. Agent calls `get_deployment_status` with task_id to monitor progress
//! 4. Agent calls `list_deployments` to see deployment history and public URLs
//!
//! **SECURITY NOTE:** The agent NEVER handles actual credentials (OAuth tokens,
//! API keys). It only checks connection STATUS. All credential handling happens
//! securely in the browser through the platform's OAuth flow.

mod check_provider_connection;
mod current_context;
mod get_deployment_status;
mod list_deployment_configs;
mod list_deployments;
mod list_organizations;
mod list_projects;
mod open_provider_settings;
mod select_project;
mod trigger_deployment;

pub use check_provider_connection::CheckProviderConnectionTool;
pub use current_context::CurrentContextTool;
pub use get_deployment_status::GetDeploymentStatusTool;
pub use list_deployment_configs::ListDeploymentConfigsTool;
pub use list_deployments::ListDeploymentsTool;
pub use list_organizations::ListOrganizationsTool;
pub use list_projects::ListProjectsTool;
pub use open_provider_settings::OpenProviderSettingsTool;
pub use select_project::SelectProjectTool;
pub use trigger_deployment::TriggerDeploymentTool;
