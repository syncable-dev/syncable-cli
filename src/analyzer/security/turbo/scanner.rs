//! # Scanner Module
//! 
//! High-performance file scanning with memory-mapped I/O and parallel processing.

use std::path::PathBuf;
use std::sync::Arc;
use std::fs::File;
use std::io::{self, Read, BufReader};

use memmap2::MmapOptions;
use crossbeam::channel::{Receiver, Sender};
use parking_lot::{Mutex, RwLock};
use log::{debug, trace, warn};

use super::file_discovery::FileMetadata;
use super::pattern_engine::{PatternEngine, PatternMatch};
use super::cache::SecurityCache;
use crate::analyzer::security::{SecurityFinding, SecuritySeverity, SecurityCategory};

/// Scan task for a worker thread
#[derive(Debug)]
pub struct ScanTask {
    pub id: usize,
    pub file: FileMetadata,
    pub quick_reject: bool,
}

/// Scan result from a worker thread
#[derive(Debug)]
pub enum ScanResult {
    Findings(Vec<SecurityFinding>),
    Skipped,
    Error(String),
}

/// File scanner worker
pub struct FileScanner {
    thread_id: usize,
    pattern_engine: Arc<PatternEngine>,
    cache: Arc<SecurityCache>,
    use_mmap: bool,
}

impl FileScanner {
    pub fn new(
        thread_id: usize,
        pattern_engine: Arc<PatternEngine>,
        cache: Arc<SecurityCache>,
        use_mmap: bool,
    ) -> Self {
        Self {
            thread_id,
            pattern_engine,
            cache,
            use_mmap,
        }
    }
    
    /// Run the scanner worker
    pub fn run(
        &self,
        task_receiver: Receiver<ScanTask>,
        result_sender: Sender<ScanResult>,
        critical_count: Arc<Mutex<usize>>,
        should_terminate: Arc<RwLock<bool>>,
        max_critical: Option<usize>,
    ) {
        debug!("Scanner thread {} started", self.thread_id);
        
        while let Ok(task) = task_receiver.recv() {
            // Check for early termination
            if *should_terminate.read() {
                debug!("Scanner thread {} terminating early", self.thread_id);
                break;
            }
            
            // Process the scan task
            let result = self.scan_file(task);
            
            // Check for critical findings
            if let ScanResult::Findings(ref findings) = result {
                let critical_findings = findings.iter()
                    .filter(|f| f.severity == SecuritySeverity::Critical)
                    .count();
                
                if critical_findings > 0 {
                    let mut count = critical_count.lock();
                    *count += critical_findings;
                    
                    if let Some(max) = max_critical {
                        if *count >= max {
                            *should_terminate.write() = true;
                            debug!("Critical findings limit reached, triggering early termination");
                        }
                    }
                }
            }
            
            // Send result
            if result_sender.send(result).is_err() {
                break; // Channel closed
            }
        }
        
        debug!("Scanner thread {} finished", self.thread_id);
    }
    
    /// Scan a single file
    fn scan_file(&self, task: ScanTask) -> ScanResult {
        trace!("Thread {} scanning: {}", self.thread_id, task.file.path.display());
        
        // Check cache first
        if let Some(cached_result) = self.cache.get(&task.file.path) {
            trace!("Cache hit for: {}", task.file.path.display());
            return ScanResult::Findings(cached_result);
        }
        
        // Read file content
        let content = match self.read_file_content(&task.file) {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to read file {}: {}", task.file.path.display(), e);
                return ScanResult::Error(e.to_string());
            }
        };
        
        // Skip if content is empty
        if content.is_empty() {
            return ScanResult::Skipped;
        }
        
        // Scan content for patterns
        let matches = self.pattern_engine.scan_content(&content, task.quick_reject);
        
        // Convert matches to findings
        let findings = self.convert_matches_to_findings(matches, &task.file);
        
        // Cache the result
        self.cache.insert(task.file.path.clone(), findings.clone());
        
        ScanResult::Findings(findings)
    }
    
    /// Read file content with optimal method
    fn read_file_content(&self, file_meta: &FileMetadata) -> io::Result<String> {
        // Use memory mapping for larger files if enabled
        if self.use_mmap && file_meta.size > 4096 {
            self.read_file_mmap(&file_meta.path)
        } else {
            self.read_file_buffered(&file_meta.path)
        }
    }
    
    /// Read file using memory mapping
    fn read_file_mmap(&self, path: &PathBuf) -> io::Result<String> {
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        
        // Validate UTF-8 using SIMD if available
        match simdutf8::basic::from_utf8(&mmap) {
            Ok(content) => Ok(content.to_string()),
            Err(_) => {
                // Fallback to lossy conversion for non-UTF8 files
                Ok(String::from_utf8_lossy(&mmap).to_string())
            }
        }
    }
    
    /// Read file using buffered I/O
    fn read_file_buffered(&self, path: &PathBuf) -> io::Result<String> {
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(8192, file);
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        Ok(content)
    }
    
    /// Convert pattern matches to security findings
    fn convert_matches_to_findings(&self, matches: Vec<PatternMatch>, file_meta: &FileMetadata) -> Vec<SecurityFinding> {
        matches.into_iter()
            .map(|match_| {
                SecurityFinding {
                    id: format!("{}-{}-{}", match_.pattern.id, file_meta.path.display(), match_.line_number),
                    title: match_.pattern.name.clone(),
                    description: self.enhance_description(&match_.pattern.description, file_meta),
                    severity: self.adjust_severity(&match_.pattern.severity, file_meta, match_.confidence),
                    category: match_.pattern.category.clone(),
                    file_path: Some(file_meta.path.clone()),
                    line_number: Some(match_.line_number),
                    column_number: Some(match_.column_number),
                    evidence: Some(match_.evidence),
                    remediation: match_.pattern.remediation.clone(),
                    references: match_.pattern.references.clone(),
                    cwe_id: match_.pattern.cwe_id.clone(),
                    compliance_frameworks: self.get_compliance_frameworks(&match_.pattern.category),
                }
            })
            .collect()
    }
    
    /// Enhance description with file context and proper gitignore status
    fn enhance_description(&self, base_description: &str, file_meta: &FileMetadata) -> String {
        let mut description = base_description.to_string();
        
        // Add comprehensive gitignore context for status determination
        if file_meta.is_gitignored {
            // File is properly protected
            if file_meta.priority_hints.is_env_file || 
               file_meta.priority_hints.is_config_file ||
               base_description.to_lowercase().contains("secret") ||
               base_description.to_lowercase().contains("key") ||
               base_description.to_lowercase().contains("token") {
                description.push_str(" (File is protected by .gitignore)");
            } else {
                description.push_str(" (File appears safe for version control)");
            }
        } else {
            // File is NOT gitignored - determine risk level
            if self.file_contains_secrets(file_meta) {
                // Check if tracked by git using git command
                if self.is_file_tracked_by_git(&file_meta.path) {
                    description.push_str(" (File is tracked by git and may expose secrets in version history - CRITICAL RISK)");
                } else {
                    description.push_str(" (File is NOT in .gitignore but contains secrets - HIGH RISK)");
                }
            } else {
                description.push_str(" (File appears safe for version control)");
            }
        }
        
        // Add file type context
        if file_meta.priority_hints.is_env_file {
            description.push_str(" [Environment file]");
        } else if file_meta.priority_hints.is_config_file {
            description.push_str(" [Configuration file]");
        }
        
        description
    }
    
    /// Check if file likely contains secrets based on patterns
    fn file_contains_secrets(&self, file_meta: &FileMetadata) -> bool {
        // Check file name patterns
        if let Some(file_name) = file_meta.path.file_name().and_then(|n| n.to_str()) {
            let file_name_lower = file_name.to_lowercase();
            let secret_file_patterns = [
                ".env", ".key", ".pem", ".p12", ".pfx", 
                "id_rsa", "id_dsa", "id_ecdsa", "id_ed25519",
                "credentials", "secrets", "private", "secret.json",
                "service-account", "auth.json", "config.json"
            ];
            
            if secret_file_patterns.iter().any(|pattern| file_name_lower.contains(pattern)) {
                return true;
            }
        }
        
        // Check if it's a priority file (likely to contain secrets)
        file_meta.priority_hints.is_env_file || 
        file_meta.priority_hints.is_config_file ||
        file_meta.is_critical()
    }
    
    /// Check if file is tracked by git
    fn is_file_tracked_by_git(&self, file_path: &std::path::PathBuf) -> bool {
        use std::process::Command;
        
        Command::new("git")
            .args(&["ls-files", "--error-unmatch"])
            .arg(file_path)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    /// Adjust severity based on context
    fn adjust_severity(&self, base_severity: &SecuritySeverity, file_meta: &FileMetadata, confidence: f32) -> SecuritySeverity {
        let mut severity = base_severity.clone();
        
        // Upgrade severity for unprotected files
        if !file_meta.is_gitignored && matches!(severity, SecuritySeverity::Medium | SecuritySeverity::High) {
            severity = match severity {
                SecuritySeverity::Medium => SecuritySeverity::High,
                SecuritySeverity::High => SecuritySeverity::Critical,
                _ => severity,
            };
        }
        
        // Downgrade for low confidence
        if confidence < 0.5 && matches!(severity, SecuritySeverity::High | SecuritySeverity::Critical) {
            severity = match severity {
                SecuritySeverity::Critical => SecuritySeverity::High,
                SecuritySeverity::High => SecuritySeverity::Medium,
                _ => severity,
            };
        }
        
        severity
    }
    
    /// Get compliance frameworks based on category
    fn get_compliance_frameworks(&self, category: &SecurityCategory) -> Vec<String> {
        match category {
            SecurityCategory::SecretsExposure => vec!["SOC2".to_string(), "GDPR".to_string(), "PCI-DSS".to_string()],
            SecurityCategory::InsecureConfiguration => vec!["SOC2".to_string(), "OWASP".to_string()],
            SecurityCategory::AuthenticationSecurity => vec!["SOC2".to_string(), "OWASP".to_string()],
            SecurityCategory::DataProtection => vec!["GDPR".to_string(), "CCPA".to_string()],
            _ => vec!["SOC2".to_string()],
        }
    }
}

/// Specialized scanner for .env files
pub struct EnvFileScanner;

impl EnvFileScanner {
    /// Fast scan of .env files without regex
    pub fn scan_env_file(path: &PathBuf) -> Result<Vec<SecurityFinding>, io::Error> {
        let content = std::fs::read_to_string(path)?;
        let mut findings = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Parse key=value pairs
            if let Some(eq_pos) = line.find('=') {
                let key = &line[..eq_pos].trim();
                let value = &line[eq_pos + 1..].trim_matches('"').trim_matches('\'');
                
                // Check for sensitive keys with actual values
                if is_sensitive_env_key(key) && !value.is_empty() && !is_placeholder_value(value) {
                    findings.push(SecurityFinding {
                        id: format!("env-secret-{}-{}", path.display(), line_num),
                        title: format!("Sensitive Environment Variable: {}", key),
                        description: format!("Environment variable '{}' contains a potentially sensitive value", key),
                        severity: determine_env_severity(key, value),
                        category: SecurityCategory::SecretsExposure,
                        file_path: Some(path.clone()),
                        line_number: Some(line_num + 1),
                        column_number: Some(eq_pos + 1),
                        evidence: Some(format!("{}=***", key)),
                        remediation: vec![
                            "Ensure .env files are in .gitignore".to_string(),
                            "Use .env.example for documentation".to_string(),
                            "Consider using a secure secret management service".to_string(),
                        ],
                        references: vec![
                            "https://12factor.net/config".to_string(),
                        ],
                        cwe_id: Some("CWE-798".to_string()),
                        compliance_frameworks: vec!["SOC2".to_string(), "GDPR".to_string()],
                    });
                }
            }
        }
        
        Ok(findings)
    }
}

/// Check if an environment variable key is sensitive
fn is_sensitive_env_key(key: &str) -> bool {
    let key_upper = key.to_uppercase();
    let sensitive_patterns = [
        "PASSWORD", "SECRET", "KEY", "TOKEN", "API", "AUTH",
        "PRIVATE", "CREDENTIAL", "ACCESS", "CLIENT", "STRIPE",
        "AWS", "GOOGLE", "AZURE", "DATABASE", "DB_", "JWT",
    ];
    
    sensitive_patterns.iter().any(|pattern| key_upper.contains(pattern))
}

/// Check if a value is likely a placeholder
fn is_placeholder_value(value: &str) -> bool {
    let placeholders = [
        "your_", "change_me", "xxx", "placeholder", "example",
        "test", "demo", "fake", "dummy", "<", ">", "${", "}",
    ];
    
    let value_lower = value.to_lowercase();
    placeholders.iter().any(|p| value_lower.contains(p))
}

/// Determine severity based on the type of secret
fn determine_env_severity(key: &str, _value: &str) -> SecuritySeverity {
    let key_upper = key.to_uppercase();
    
    // Critical: API keys, database credentials
    if key_upper.contains("DATABASE") || key_upper.contains("DB_PASS") ||
       key_upper.contains("AWS_SECRET") || key_upper.contains("STRIPE_SECRET") {
        return SecuritySeverity::Critical;
    }
    
    // High: Most API keys and secrets
    if key_upper.contains("API") || key_upper.contains("SECRET") ||
       key_upper.contains("PRIVATE") || key_upper.contains("TOKEN") {
        return SecuritySeverity::High;
    }
    
    // Medium: General passwords and auth
    if key_upper.contains("PASSWORD") || key_upper.contains("AUTH") {
        return SecuritySeverity::Medium;
    }
    
    SecuritySeverity::Low
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_env_file_scanner() {
        let temp_dir = TempDir::new().unwrap();
        let env_file = temp_dir.path().join(".env");
        
        fs::write(&env_file, r#"
# Database config
DATABASE_URL=postgres://user:password@localhost/db
API_KEY=sk-1234567890abcdef
PUBLIC_URL=https://example.com
TEST_VAR=placeholder_value
"#).unwrap();
        
        let findings = EnvFileScanner::scan_env_file(&env_file).unwrap();
        
        // Should find DATABASE_URL and API_KEY but not PUBLIC_URL or TEST_VAR
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.title.contains("DATABASE_URL")));
        assert!(findings.iter().any(|f| f.title.contains("API_KEY")));
    }
    
    #[test]
    fn test_placeholder_detection() {
        assert!(is_placeholder_value("your_api_key_here"));
        assert!(is_placeholder_value("<YOUR_TOKEN>"));
        assert!(is_placeholder_value("xxx"));
        assert!(!is_placeholder_value("sk-1234567890"));
    }
} 