use crate::common::file_utils::is_readable_file;
use crate::error::Result;
use std::collections::HashMap;
use std::path::Path;

/// Analyzes .env files
pub(crate) fn analyze_env_files(
    root: &Path,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
) -> Result<()> {
    let env_files = [
        ".env",
        ".env.example",
        ".env.local",
        ".env.development",
        ".env.production",
    ];

    for env_file in &env_files {
        let path = root.join(env_file);
        if is_readable_file(&path) {
            let content = std::fs::read_to_string(&path)?;

            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Some(eq_pos) = line.find('=') {
                    let (key, value) = line.split_at(eq_pos);
                    let key = key.trim();
                    let value = value[1..].trim(); // Skip the '='

                    // Check if it's marked as required (common convention)
                    let required = value.is_empty() || value == "required" || value == "REQUIRED";
                    let actual_value = if required {
                        None
                    } else {
                        Some(value.to_string())
                    };

                    env_vars
                        .entry(key.to_string())
                        .or_insert((actual_value, required, None));
                }
            }
        }
    }

    Ok(())
}
