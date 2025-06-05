pub mod types;

use crate::error::Result;
use std::path::Path;

/// Load configuration from file or use defaults
pub fn load_config(_path: Option<&Path>) -> Result<types::Config> {
    // TODO: Implement configuration loading
    Ok(types::Config::default())
} 