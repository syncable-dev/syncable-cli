// Handler modules
pub mod analyze;
pub mod dependencies;
pub mod generate;
pub mod optimize;
pub mod security;
pub mod tools;
pub mod utils;
pub mod vulnerabilities;

// Re-export all handler functions
pub use analyze::handle_analyze;
pub use dependencies::handle_dependencies;
pub use generate::{handle_generate, handle_validate};
pub use optimize::{OptimizeOptions, handle_optimize};
pub use security::handle_security;
pub use tools::handle_tools;
pub use utils::{format_project_category, handle_support};
pub use vulnerabilities::handle_vulnerabilities;
