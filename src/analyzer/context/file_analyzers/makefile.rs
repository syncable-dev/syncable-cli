use crate::analyzer::{context::helpers::create_regex, BuildScript};
use crate::common::file_utils::is_readable_file;
use crate::error::Result;
use std::path::Path;

/// Analyzes Makefile for build scripts
pub(crate) fn analyze_makefile(
    root: &Path,
    build_scripts: &mut Vec<BuildScript>,
) -> Result<()> {
    let makefiles = ["Makefile", "makefile"];

    for makefile in &makefiles {
        let path = root.join(makefile);
        if is_readable_file(&path) {
            let content = std::fs::read_to_string(&path)?;

            // Simple Makefile target extraction
            let target_regex = create_regex(r"^([a-zA-Z0-9_-]+):\s*(?:[^\n]*)?$")?;
            let mut in_recipe = false;
            let mut current_target = String::new();
            let mut current_command = String::new();

            for line in content.lines() {
                if let Some(cap) = target_regex.captures(line) {
                    // Save previous target if any
                    if !current_target.is_empty() && !current_command.is_empty() {
                        build_scripts.push(BuildScript {
                            name: current_target.clone(),
                            command: format!("make {}", current_target),
                            description: None,
                            is_default: current_target == "run" || current_target == "start",
                        });
                    }

                    if let Some(target) = cap.get(1) {
                        current_target = target.as_str().to_string();
                        current_command.clear();
                        in_recipe = true;
                    }
                } else if in_recipe && line.starts_with('\t') {
                    if current_command.is_empty() {
                        current_command = line.trim().to_string();
                    }
                } else if !line.trim().is_empty() {
                    in_recipe = false;
                }
            }

            // Save last target
            if !current_target.is_empty() && !current_command.is_empty() {
                build_scripts.push(BuildScript {
                    name: current_target.clone(),
                    command: format!("make {}", current_target),
                    description: None,
                    is_default: current_target == "run" || current_target == "start",
                });
            }

            break;
        }
    }

    Ok(())
} 