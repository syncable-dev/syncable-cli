/// Configuration for monorepo detection
#[derive(Debug, Clone)]
pub struct MonorepoDetectionConfig {
    /// Maximum depth to search for projects
    pub max_depth: usize,
    /// Minimum confidence threshold for considering a directory as a project
    pub min_project_confidence: f32,
    /// Whether to analyze subdirectories that might be projects
    pub deep_scan: bool,
    /// Patterns to exclude from project detection
    pub exclude_patterns: Vec<String>,
}

impl Default for MonorepoDetectionConfig {
    fn default() -> Self {
        Self {
            // Monorepos often nest apps/libs 3–5 levels deep (e.g., apps/api/src)
            max_depth: 5,
            min_project_confidence: 0.6,
            deep_scan: true,
            exclude_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "build".to_string(),
                "dist".to_string(),
                ".next".to_string(),
                "__pycache__".to_string(),
                "vendor".to_string(),
                ".venv".to_string(),
                "venv".to_string(),
                ".env".to_string(),
                "coverage".to_string(),
                "docs".to_string(),
                "tmp".to_string(),
                "temp".to_string(),
            ],
        }
    }
} 
