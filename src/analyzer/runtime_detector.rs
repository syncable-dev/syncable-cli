use std::path::{PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};
use log::{debug, info};

/// JavaScript runtime types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JavaScriptRuntime {
    Bun,
    Node,
    Deno,
    Unknown,
}

impl JavaScriptRuntime {
    pub fn as_str(&self) -> &str {
        match self {
            JavaScriptRuntime::Bun => "bun",
            JavaScriptRuntime::Node => "node",
            JavaScriptRuntime::Deno => "deno",
            JavaScriptRuntime::Unknown => "unknown",
        }
    }
}

/// Package manager types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageManager {
    Bun,
    Npm,
    Yarn,
    Pnpm,
    Unknown,
}

impl PackageManager {
    pub fn as_str(&self) -> &str {
        match self {
            PackageManager::Bun => "bun",
            PackageManager::Npm => "npm",
            PackageManager::Yarn => "yarn",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Unknown => "unknown",
        }
    }
    
    pub fn lockfile_name(&self) -> &str {
        match self {
            PackageManager::Bun => "bun.lockb",
            PackageManager::Npm => "package-lock.json",
            PackageManager::Yarn => "yarn.lock",
            PackageManager::Pnpm => "pnpm-lock.yaml",
            PackageManager::Unknown => "",
        }
    }
    
    pub fn audit_command(&self) -> &str {
        match self {
            PackageManager::Bun => "bun audit",
            PackageManager::Npm => "npm audit",
            PackageManager::Yarn => "yarn audit",
            PackageManager::Pnpm => "pnpm audit",
            PackageManager::Unknown => "",
        }
    }
}

/// Runtime detection result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDetectionResult {
    pub runtime: JavaScriptRuntime,
    pub package_manager: PackageManager,
    pub detected_lockfiles: Vec<String>,
    pub has_package_json: bool,
    pub has_engines_field: bool,
    pub confidence: DetectionConfidence,
}

/// Confidence level for runtime detection
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DetectionConfidence {
    High,    // Lock file present or explicit engine specification
    Medium,  // Inferred from package.json or common patterns
    Low,     // Default assumptions
}

/// Runtime detector for JavaScript/TypeScript projects
pub struct RuntimeDetector {
    project_path: PathBuf,
}

impl RuntimeDetector {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }
    
    /// Detect JavaScript runtime and package manager for the project
    pub fn detect_js_runtime_and_package_manager(&self) -> RuntimeDetectionResult {
        debug!("Detecting JavaScript runtime and package manager for project: {}", self.project_path.display());
        
        let mut detected_lockfiles = Vec::new();
        let has_package_json = self.project_path.join("package.json").exists();
        
        debug!("Has package.json: {}", has_package_json);
        
        // Priority 1: Check for lock files (highest confidence)
        let lockfile_detection = self.detect_by_lockfiles(&mut detected_lockfiles);
        if let Some((runtime, manager)) = lockfile_detection {
            info!("Detected {} runtime with {} package manager via lockfile", runtime.as_str(), manager.as_str());
            return RuntimeDetectionResult {
                runtime,
                package_manager: manager,
                detected_lockfiles,
                has_package_json,
                has_engines_field: false, // Will be updated below
                confidence: DetectionConfidence::High,
            };
        }
        
        // Priority 2: Check package.json engines field (high confidence)
        let mut engines_result = self.detect_by_engines_field();
        if let Some((runtime, manager)) = engines_result.clone() {
            engines_result = Some((runtime.clone(), manager.clone()));
            info!("Detected {} runtime with {} package manager via engines field", runtime.as_str(), manager.as_str());
            return RuntimeDetectionResult {
                runtime,
                package_manager: manager,
                detected_lockfiles,
                has_package_json,
                has_engines_field: true,
                confidence: DetectionConfidence::High,
            };
        }
        
        // Priority 3: Check for common Bun-specific files (medium confidence)
        if self.has_bun_specific_files() {
            info!("Detected Bun-specific files, assuming Bun runtime");
            return RuntimeDetectionResult {
                runtime: JavaScriptRuntime::Bun,
                package_manager: PackageManager::Bun,
                detected_lockfiles,
                has_package_json,
                has_engines_field: false,
                confidence: DetectionConfidence::Medium,
            };
        }
        
        // Priority 4: Default behavior based on project type
        if has_package_json {
            // If package.json exists but no specific indicators, default to Node.js with npm
            debug!("Package.json exists but no specific runtime detected, defaulting to Node.js with npm");
            RuntimeDetectionResult {
                runtime: JavaScriptRuntime::Node,
                package_manager: PackageManager::Npm,
                detected_lockfiles,
                has_package_json,
                has_engines_field: false,
                confidence: DetectionConfidence::Low,
            }
        } else {
            // No package.json, not a JavaScript project
            debug!("No package.json found, not a JavaScript project");
            RuntimeDetectionResult {
                runtime: JavaScriptRuntime::Unknown,
                package_manager: PackageManager::Unknown,
                detected_lockfiles,
                has_package_json,
                has_engines_field: false,
                confidence: DetectionConfidence::Low,
            }
        }
    }
    
    /// Detect all available package managers in the project
    pub fn detect_all_package_managers(&self) -> Vec<PackageManager> {
        let mut managers = Vec::new();
        
        if self.project_path.join("bun.lockb").exists() {
            managers.push(PackageManager::Bun);
        }
        if self.project_path.join("pnpm-lock.yaml").exists() {
            managers.push(PackageManager::Pnpm);
        }
        if self.project_path.join("yarn.lock").exists() {
            managers.push(PackageManager::Yarn);
        }
        if self.project_path.join("package-lock.json").exists() {
            managers.push(PackageManager::Npm);
        }
        
        managers
    }
    
    /// Check if this is likely a Bun project
    pub fn is_bun_project(&self) -> bool {
        let result = self.detect_js_runtime_and_package_manager();
        matches!(result.runtime, JavaScriptRuntime::Bun) || 
        matches!(result.package_manager, PackageManager::Bun)
    }
    
    /// Check if this is a JavaScript/TypeScript project
    pub fn is_js_project(&self) -> bool {
        self.project_path.join("package.json").exists() ||
        self.project_path.join("bun.lockb").exists() ||
        self.project_path.join("package-lock.json").exists() ||
        self.project_path.join("yarn.lock").exists() ||
        self.project_path.join("pnpm-lock.yaml").exists()
    }
    
    /// Detect runtime by lock files
    fn detect_by_lockfiles(&self, detected_lockfiles: &mut Vec<String>) -> Option<(JavaScriptRuntime, PackageManager)> {
        // Check Bun first (as it's the most specific)
        if self.project_path.join("bun.lockb").exists() {
            detected_lockfiles.push("bun.lockb".to_string());
            debug!("Found bun.lockb, using Bun runtime and package manager");
            return Some((JavaScriptRuntime::Bun, PackageManager::Bun));
        }
        
        // Check pnpm-lock.yaml
        if self.project_path.join("pnpm-lock.yaml").exists() {
            detected_lockfiles.push("pnpm-lock.yaml".to_string());
            debug!("Found pnpm-lock.yaml, using Node.js runtime with pnpm");
            return Some((JavaScriptRuntime::Node, PackageManager::Pnpm));
        }
        
        // Check yarn.lock
        if self.project_path.join("yarn.lock").exists() {
            detected_lockfiles.push("yarn.lock".to_string());
            debug!("Found yarn.lock, using Node.js runtime with Yarn");
            return Some((JavaScriptRuntime::Node, PackageManager::Yarn));
        }
        
        // Check package-lock.json
        if self.project_path.join("package-lock.json").exists() {
            detected_lockfiles.push("package-lock.json".to_string());
            debug!("Found package-lock.json, using Node.js runtime with npm");
            return Some((JavaScriptRuntime::Node, PackageManager::Npm));
        }
        
        None
    }
    
    /// Detect runtime by engines field in package.json
    fn detect_by_engines_field(&self) -> Option<(JavaScriptRuntime, PackageManager)> {
        let package_json_path = self.project_path.join("package.json");
        if !package_json_path.exists() {
            return None;
        }
        
        match self.read_package_json() {
            Ok(package_json) => {
                if let Some(engines) = package_json.get("engines") {
                    debug!("Found engines field in package.json: {:?}", engines);
                    
                    // Check for Bun engine
                    if engines.get("bun").is_some() {
                        debug!("Found bun engine specification");
                        return Some((JavaScriptRuntime::Bun, PackageManager::Bun));
                    }
                    
                    // Check for Deno engine (less common but possible)
                    if engines.get("deno").is_some() {
                        debug!("Found deno engine specification");
                        return Some((JavaScriptRuntime::Deno, PackageManager::Unknown));
                    }
                    
                    // If only node is specified, default to npm
                    if engines.get("node").is_some() {
                        debug!("Found node engine specification, using npm as default");
                        return Some((JavaScriptRuntime::Node, PackageManager::Npm));
                    }
                }
                
                // Check packageManager field (newer npm/yarn feature)
                if let Some(package_manager) = package_json.get("packageManager").and_then(|pm| pm.as_str()) {
                    debug!("Found packageManager field: {}", package_manager);
                    
                    if package_manager.starts_with("bun") {
                        return Some((JavaScriptRuntime::Bun, PackageManager::Bun));
                    } else if package_manager.starts_with("pnpm") {
                        return Some((JavaScriptRuntime::Node, PackageManager::Pnpm));
                    } else if package_manager.starts_with("yarn") {
                        return Some((JavaScriptRuntime::Node, PackageManager::Yarn));
                    } else if package_manager.starts_with("npm") {
                        return Some((JavaScriptRuntime::Node, PackageManager::Npm));
                    }
                }
            }
            Err(e) => {
                debug!("Failed to read package.json: {}", e);
            }
        }
        
        None
    }
    
    /// Check for Bun-specific files
    fn has_bun_specific_files(&self) -> bool {
        // Check for bunfig.toml (Bun configuration file)
        if self.project_path.join("bunfig.toml").exists() {
            debug!("Found bunfig.toml");
            return true;
        }
        
        // Check for .bunfig.toml (alternative config name)
        if self.project_path.join(".bunfig.toml").exists() {
            debug!("Found .bunfig.toml");
            return true;
        }
        
        // Check for bun-specific scripts in package.json
        if let Ok(package_json) = self.read_package_json() {
            if let Some(scripts) = package_json.get("scripts").and_then(|s| s.as_object()) {
                for script in scripts.values() {
                    if let Some(script_str) = script.as_str() {
                        if script_str.contains("bun ") || script_str.starts_with("bun") {
                            debug!("Found Bun command in scripts: {}", script_str);
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }
    
    /// Read and parse package.json
    fn read_package_json(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let package_json_path = self.project_path.join("package.json");
        let content = fs::read_to_string(package_json_path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        Ok(json)
    }
    
    /// Get recommended audit commands for the project
    pub fn get_audit_commands(&self) -> Vec<String> {
        let result = self.detect_js_runtime_and_package_manager();
        let mut commands = Vec::new();
        
        // Primary command based on detection
        commands.push(result.package_manager.audit_command().to_string());
        
        // Add fallback commands for multiple package managers
        let all_managers = self.detect_all_package_managers();
        for manager in all_managers {
            let cmd = manager.audit_command().to_string();
            if !commands.contains(&cmd) {
                commands.push(cmd);
            }
        }
        
        commands
    }
    
    /// Get a human-readable summary of the detection
    pub fn get_detection_summary(&self) -> String {
        let result = self.detect_js_runtime_and_package_manager();
        
        let confidence_str = match result.confidence {
            DetectionConfidence::High => "high confidence",
            DetectionConfidence::Medium => "medium confidence", 
            DetectionConfidence::Low => "low confidence (default)",
        };
        
        let mut summary = format!(
            "Detected {} runtime with {} package manager ({})",
            result.runtime.as_str(),
            result.package_manager.as_str(),
            confidence_str
        );
        
        if !result.detected_lockfiles.is_empty() {
            summary.push_str(&format!(" - Lock files: {}", result.detected_lockfiles.join(", ")));
        }
        
        if result.has_engines_field {
            summary.push_str(" - Engines field present");
        }
        
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    fn create_test_project() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();
        (temp_dir, project_path)
    }
    
    #[test]
    fn test_bun_project_detection() {
        let (_temp_dir, project_path) = create_test_project();
        
        // Create bun.lockb file
        fs::write(project_path.join("bun.lockb"), b"").unwrap();
        
        let detector = RuntimeDetector::new(project_path);
        let result = detector.detect_js_runtime_and_package_manager();
        
        assert_eq!(result.runtime, JavaScriptRuntime::Bun);
        assert_eq!(result.package_manager, PackageManager::Bun);
        assert_eq!(result.confidence, DetectionConfidence::High);
        assert!(result.detected_lockfiles.contains(&"bun.lockb".to_string()));
    }
    
    #[test]
    fn test_bun_engines_detection() {
        let (_temp_dir, project_path) = create_test_project();
        
        // Create package.json with bun engine
        let package_json = serde_json::json!({
            "name": "test-project",
            "engines": {
                "bun": "^1.0.0"
            }
        });
        fs::write(
            project_path.join("package.json"), 
            serde_json::to_string_pretty(&package_json).unwrap()
        ).unwrap();
        
        let detector = RuntimeDetector::new(project_path);
        let result = detector.detect_js_runtime_and_package_manager();
        
        assert_eq!(result.runtime, JavaScriptRuntime::Bun);
        assert_eq!(result.package_manager, PackageManager::Bun);
        assert_eq!(result.confidence, DetectionConfidence::High);
        assert!(result.has_engines_field);
    }
    
    #[test]
    fn test_package_manager_field_detection() {
        let (_temp_dir, project_path) = create_test_project();
        
        // Create package.json with packageManager field
        let package_json = serde_json::json!({
            "name": "test-project",
            "packageManager": "bun@1.0.0"
        });
        fs::write(
            project_path.join("package.json"), 
            serde_json::to_string_pretty(&package_json).unwrap()
        ).unwrap();
        
        let detector = RuntimeDetector::new(project_path);
        let result = detector.detect_js_runtime_and_package_manager();
        
        assert_eq!(result.runtime, JavaScriptRuntime::Bun);
        assert_eq!(result.package_manager, PackageManager::Bun);
        assert_eq!(result.confidence, DetectionConfidence::High);
    }
    
    #[test]
    fn test_yarn_project_detection() {
        let (_temp_dir, project_path) = create_test_project();
        
        // Create yarn.lock file
        fs::write(project_path.join("yarn.lock"), b"").unwrap();
        
        let detector = RuntimeDetector::new(project_path);
        let result = detector.detect_js_runtime_and_package_manager();
        
        assert_eq!(result.runtime, JavaScriptRuntime::Node);
        assert_eq!(result.package_manager, PackageManager::Yarn);
        assert_eq!(result.confidence, DetectionConfidence::High);
    }
    
    #[test]
    fn test_npm_project_detection() {
        let (_temp_dir, project_path) = create_test_project();
        
        // Create package-lock.json file
        fs::write(project_path.join("package-lock.json"), b"{}").unwrap();
        
        let detector = RuntimeDetector::new(project_path);
        let result = detector.detect_js_runtime_and_package_manager();
        
        assert_eq!(result.runtime, JavaScriptRuntime::Node);
        assert_eq!(result.package_manager, PackageManager::Npm);
        assert_eq!(result.confidence, DetectionConfidence::High);
    }
    
    #[test]
    fn test_pnpm_project_detection() {
        let (_temp_dir, project_path) = create_test_project();
        
        // Create pnpm-lock.yaml file
        fs::write(project_path.join("pnpm-lock.yaml"), b"").unwrap();
        
        let detector = RuntimeDetector::new(project_path);
        let result = detector.detect_js_runtime_and_package_manager();
        
        assert_eq!(result.runtime, JavaScriptRuntime::Node);
        assert_eq!(result.package_manager, PackageManager::Pnpm);
        assert_eq!(result.confidence, DetectionConfidence::High);
    }
    
    #[test]
    fn test_bunfig_detection() {
        let (_temp_dir, project_path) = create_test_project();
        
        // Create bunfig.toml file
        fs::write(project_path.join("bunfig.toml"), b"[install]\nregistry = \"https://registry.npmjs.org/\"").unwrap();
        
        let detector = RuntimeDetector::new(project_path);
        let result = detector.detect_js_runtime_and_package_manager();
        
        assert_eq!(result.runtime, JavaScriptRuntime::Bun);
        assert_eq!(result.package_manager, PackageManager::Bun);
        assert_eq!(result.confidence, DetectionConfidence::Medium);
    }
    
    #[test]
    fn test_bun_script_detection() {
        let (_temp_dir, project_path) = create_test_project();
        
        // Create package.json with bun script
        let package_json = serde_json::json!({
            "name": "test-project",
            "scripts": {
                "start": "bun run index.ts",
                "dev": "bun --watch index.ts"
            }
        });
        fs::write(
            project_path.join("package.json"), 
            serde_json::to_string_pretty(&package_json).unwrap()
        ).unwrap();
        
        let detector = RuntimeDetector::new(project_path);
        let result = detector.detect_js_runtime_and_package_manager();
        
        assert_eq!(result.runtime, JavaScriptRuntime::Bun);
        assert_eq!(result.package_manager, PackageManager::Bun);
        assert_eq!(result.confidence, DetectionConfidence::Medium);
    }
    
    #[test]
    fn test_default_detection() {
        let (_temp_dir, project_path) = create_test_project();
        
        // Create only package.json with no special fields
        let package_json = serde_json::json!({
            "name": "test-project"
        });
        fs::write(
            project_path.join("package.json"), 
            serde_json::to_string_pretty(&package_json).unwrap()
        ).unwrap();
        
        let detector = RuntimeDetector::new(project_path);
        let result = detector.detect_js_runtime_and_package_manager();
        
        assert_eq!(result.runtime, JavaScriptRuntime::Node);
        assert_eq!(result.package_manager, PackageManager::Npm);
        assert_eq!(result.confidence, DetectionConfidence::Low);
    }
    
    #[test]
    fn test_multiple_lockfiles() {
        let (_temp_dir, project_path) = create_test_project();
        
        // Create multiple lock files - bun should take priority
        fs::write(project_path.join("bun.lockb"), b"").unwrap();
        fs::write(project_path.join("package-lock.json"), b"{}").unwrap();
        fs::write(project_path.join("yarn.lock"), b"").unwrap();
        
        let detector = RuntimeDetector::new(project_path);
        let result = detector.detect_js_runtime_and_package_manager();
        
        assert_eq!(result.runtime, JavaScriptRuntime::Bun);
        assert_eq!(result.package_manager, PackageManager::Bun);
        assert_eq!(result.confidence, DetectionConfidence::High);
        
        let all_managers = detector.detect_all_package_managers();
        assert!(all_managers.contains(&PackageManager::Bun));
        assert!(all_managers.contains(&PackageManager::Npm));
        assert!(all_managers.contains(&PackageManager::Yarn));
    }
    
    #[test]
    fn test_audit_commands() {
        let (_temp_dir, project_path) = create_test_project();
        
        // Create bun project
        fs::write(project_path.join("bun.lockb"), b"").unwrap();
        
        let detector = RuntimeDetector::new(project_path);
        let commands = detector.get_audit_commands();
        
        assert!(commands.contains(&"bun audit".to_string()));
    }
    
    #[test]
    fn test_detection_summary() {
        let (_temp_dir, project_path) = create_test_project();
        
        // Create bun project
        fs::write(project_path.join("bun.lockb"), b"").unwrap();
        
        let detector = RuntimeDetector::new(project_path);
        let summary = detector.get_detection_summary();
        
        assert!(summary.contains("bun"));
        assert!(summary.contains("high confidence"));
        assert!(summary.contains("bun.lockb"));
    }
}