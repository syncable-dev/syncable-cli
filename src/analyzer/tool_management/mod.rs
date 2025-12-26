//! # Tool Management Module
//!
//! Handles detection, installation, and management of external tools required
//! for vulnerability scanning and other analysis tasks.

pub mod detector;
pub mod installer;
pub mod installers;
pub mod status;

pub use detector::{InstallationSource, ToolDetector, ToolStatus};
pub use installer::{ToolInstallationError, ToolInstaller};
pub use status::ToolStatusReporter;

/// Re-export common types
pub use detector::ToolDetectionConfig;
pub use installers::InstallationStrategy;
