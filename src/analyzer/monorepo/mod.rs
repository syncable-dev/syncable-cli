pub mod analysis;
pub mod config;
mod detection;
mod helpers;
mod project_info;
mod summary;

pub use analysis::{analyze_monorepo, analyze_monorepo_with_config};
pub use config::MonorepoDetectionConfig; 