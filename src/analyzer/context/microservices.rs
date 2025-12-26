use crate::error::Result;
use std::path::Path;

/// Represents a detected microservice within the project
#[derive(Debug)]
pub(crate) struct MicroserviceInfo {
    pub name: String,
    pub has_db: bool,
    pub has_api: bool,
}

/// Detects microservice structure based on directory patterns
pub(crate) fn detect_microservices_structure(project_root: &Path) -> Result<Vec<MicroserviceInfo>> {
    let mut microservices = Vec::new();

    // Common patterns for microservice directories
    let service_indicators = [
        "api",
        "service",
        "encore.service.ts",
        "main.ts",
        "main.go",
        "main.py",
    ];
    let db_indicators = ["db", "database", "migrations", "schema", "models"];

    // Check root-level directories
    if let Ok(entries) = std::fs::read_dir(project_root) {
        for entry in entries.flatten() {
            if entry.file_type()?.is_dir() {
                let dir_name = entry.file_name().to_string_lossy().to_string();
                let dir_path = entry.path();

                // Skip common non-service directories
                if dir_name.starts_with('.')
                    || [
                        "node_modules",
                        "target",
                        "dist",
                        "build",
                        "__pycache__",
                        "vendor",
                    ]
                    .contains(&dir_name.as_str())
                {
                    continue;
                }

                // Check if this directory looks like a service
                let mut has_api = false;
                let mut has_db = false;

                if let Ok(sub_entries) = std::fs::read_dir(&dir_path) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_name = sub_entry.file_name().to_string_lossy().to_string();

                        // Check for API indicators
                        if service_indicators.iter().any(|&ind| sub_name.contains(ind)) {
                            has_api = true;
                        }

                        // Check for DB indicators
                        if db_indicators.iter().any(|&ind| sub_name.contains(ind)) {
                            has_db = true;
                        }
                    }
                }

                // If it has service characteristics, add it as a microservice
                if has_api || has_db {
                    microservices.push(MicroserviceInfo {
                        name: dir_name,
                        has_db,
                        has_api,
                    });
                }
            }
        }
    }

    Ok(microservices)
}
