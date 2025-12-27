//! # Tool Installation Strategies
//!
//! Language-specific installers for development and security tools

pub mod common;
pub mod go;
pub mod java;
pub mod javascript;
pub mod python;
pub mod rust;

pub use common::InstallationStrategy;

use crate::error::Result;

/// Common trait for tool installers
pub trait ToolInstaller {
    fn install(&self, tool_name: &str) -> Result<()>;
    fn is_installed(&self, tool_name: &str) -> bool;
    fn get_install_command(&self, tool_name: &str) -> Option<String>;
}
