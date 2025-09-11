use super::javascript::{RuntimeDetectionResult};
use std::path::Path;

/// Generic runtime detection engine that can be extended for other languages
pub struct RuntimeDetectionEngine;

impl RuntimeDetectionEngine {
    /// Detect the primary runtime and package manager for a project
    pub fn detect_primary_runtime(project_path: &Path) -> RuntimeDetectionResult {
        use super::javascript::RuntimeDetector;
        
        let js_detector = RuntimeDetector::new(project_path.to_path_buf());
        js_detector.detect_js_runtime_and_package_manager()
    }
    
    /// Get all available package managers in a project
    pub fn get_all_package_managers(project_path: &Path) -> Vec<super::javascript::PackageManager> {
        use super::javascript::RuntimeDetector;
        
        let js_detector = RuntimeDetector::new(project_path.to_path_buf());
        js_detector.detect_all_package_managers()
    }
    
    /// Check if a project uses a specific runtime
    pub fn uses_runtime(project_path: &Path, runtime: &str) -> bool {
        let detection = Self::detect_primary_runtime(project_path);
        detection.runtime.as_str() == runtime
    }
}