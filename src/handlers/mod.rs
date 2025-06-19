// Handler modules
pub mod analyze;
pub mod dependencies;
pub mod generate;
pub mod security;
pub mod tools;
pub mod utils;
pub mod vulnerabilities;

// Re-export all handler functions
pub use analyze::handle_analyze;
pub use dependencies::handle_dependencies;
pub use generate::{handle_generate, handle_validate};
pub use security::handle_security;
pub use tools::handle_tools;
pub use utils::{handle_support, format_project_category};
pub use vulnerabilities::handle_vulnerabilities; 