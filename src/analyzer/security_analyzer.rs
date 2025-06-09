//! # Security Analyzer
//! 
//! Comprehensive security analysis module that performs multi-layered security assessment:
//! - Configuration security analysis (secrets, insecure settings)
//! - Code security patterns (language/framework-specific issues)
//! - Infrastructure security (Docker, compose configurations)
//! - Security policy recommendations and compliance guidance
//! - Security scoring with actionable remediation steps

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::Instant;
use std::process::Command;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use log::{info, debug};
use rayon::prelude::*;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};

use crate::analyzer::{ProjectAnalysis, DetectedLanguage, DetectedTechnology, EnvVar};
use crate::analyzer::dependency_parser::Language;


#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Security analysis failed: {0}")]
    AnalysisFailed(String),
    
    #[error("Configuration analysis error: {0}")]
    ConfigAnalysisError(String),
    
    #[error("Code pattern analysis error: {0}")]
    CodePatternError(String),
    
    #[error("Infrastructure analysis error: {0}")]
    InfrastructureError(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

/// Security finding severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecuritySeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Categories of security findings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SecurityCategory {
    /// Exposed secrets, API keys, passwords
    SecretsExposure,
    /// Insecure configuration settings
    InsecureConfiguration,
    /// Language/framework-specific security patterns
    CodeSecurityPattern,
    /// Infrastructure and deployment security
    InfrastructureSecurity,
    /// Authentication and authorization issues
    AuthenticationSecurity,
    /// Data protection and privacy concerns
    DataProtection,
    /// Network and communication security
    NetworkSecurity,
    /// Compliance and regulatory requirements
    Compliance,
}

/// A security finding with details and remediation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: SecuritySeverity,
    pub category: SecurityCategory,
    pub file_path: Option<PathBuf>,
    pub line_number: Option<usize>,
    pub column_number: Option<usize>,
    pub evidence: Option<String>,
    pub remediation: Vec<String>,
    pub references: Vec<String>,
    pub cwe_id: Option<String>,
    pub compliance_frameworks: Vec<String>,
}

/// Comprehensive security analysis report
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityReport {
    pub analyzed_at: chrono::DateTime<chrono::Utc>,
    pub overall_score: f32, // 0-100, higher is better
    pub risk_level: SecuritySeverity,
    pub total_findings: usize,
    pub findings_by_severity: HashMap<SecuritySeverity, usize>,
    pub findings_by_category: HashMap<SecurityCategory, usize>,
    pub findings: Vec<SecurityFinding>,
    pub recommendations: Vec<String>,
    pub compliance_status: HashMap<String, ComplianceStatus>,
}

/// Compliance framework status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub framework: String,
    pub coverage: f32, // 0-100%
    pub missing_controls: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Configuration for security analysis
#[derive(Debug, Clone)]
pub struct SecurityAnalysisConfig {
    pub include_low_severity: bool,
    pub check_secrets: bool,
    pub check_code_patterns: bool,
    pub check_infrastructure: bool,
    pub check_compliance: bool,
    pub frameworks_to_check: Vec<String>,
    pub ignore_patterns: Vec<String>,
    /// Whether to skip scanning files that are gitignored
    pub skip_gitignored_files: bool,
    /// Whether to downgrade severity for gitignored files instead of skipping
    pub downgrade_gitignored_severity: bool,
}

impl Default for SecurityAnalysisConfig {
    fn default() -> Self {
        Self {
            include_low_severity: false,
            check_secrets: true,
            check_code_patterns: true,
            check_infrastructure: true,
            check_compliance: true,
            frameworks_to_check: vec![
                "SOC2".to_string(),
                "GDPR".to_string(),
                "OWASP".to_string(),
            ],
            ignore_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "build".to_string(),
                ".next".to_string(),
                "dist".to_string(),
                "test".to_string(),
                "tests".to_string(),
                "*.json".to_string(), // Exclude JSON files that often contain hashes
                "*.lock".to_string(), // Exclude lock files with checksums
                "*_sample.*".to_string(), // Exclude sample files
                "*audit*".to_string(), // Exclude audit reports
            ],
            skip_gitignored_files: true, // Default to skipping gitignored files
            downgrade_gitignored_severity: false, // Skip entirely by default
        }
    }
}

pub struct SecurityAnalyzer {
    config: SecurityAnalysisConfig,
    secret_patterns: Vec<SecretPattern>,
    security_rules: HashMap<Language, Vec<SecurityRule>>,
    git_ignore_cache: std::sync::Mutex<HashMap<PathBuf, bool>>,
    project_root: Option<PathBuf>,
}

/// Pattern for detecting secrets and sensitive data
struct SecretPattern {
    name: String,
    pattern: Regex,
    severity: SecuritySeverity,
    description: String,
}

/// Security rule for code pattern analysis
struct SecurityRule {
    id: String,
    name: String,
    pattern: Regex,
    severity: SecuritySeverity,
    category: SecurityCategory,
    description: String,
    remediation: Vec<String>,
    cwe_id: Option<String>,
}

impl SecurityAnalyzer {
    pub fn new() -> Result<Self, SecurityError> {
        Self::with_config(SecurityAnalysisConfig::default())
    }
    
    pub fn with_config(config: SecurityAnalysisConfig) -> Result<Self, SecurityError> {
        let secret_patterns = Self::initialize_secret_patterns()?;
        let security_rules = Self::initialize_security_rules()?;
        
        Ok(Self {
            config,
            secret_patterns,
            security_rules,
            git_ignore_cache: std::sync::Mutex::new(HashMap::new()),
            project_root: None,
        })
    }
    

    
    /// Perform comprehensive security analysis with appropriate progress for verbosity level
    pub fn analyze_security(&mut self, analysis: &ProjectAnalysis) -> Result<SecurityReport, SecurityError> {
        let start_time = Instant::now();
        info!("Starting comprehensive security analysis");
        
        // Set project root for gitignore checking
        self.project_root = Some(analysis.project_root.clone());
        
        // Check if we're in verbose mode by checking log level
        let is_verbose = log::max_level() >= log::LevelFilter::Info;
        
        // Set up progress tracking appropriate for verbosity
        let multi_progress = MultiProgress::new();
        
        // In verbose mode, we'll completely skip adding progress bars to avoid visual conflicts
        
        // Count enabled analysis phases
        let mut total_phases = 0;
        if self.config.check_secrets { total_phases += 1; }
        if self.config.check_code_patterns { total_phases += 1; }
        if self.config.check_infrastructure { total_phases += 1; }
        total_phases += 2; // env vars and framework analysis always run
        
        // Create appropriate progress indicator based on verbosity
        let main_pb = if is_verbose {
            None // No main progress bar in verbose mode to avoid conflicts with logs
        } else {
            // Normal mode: Rich progress bar
            let pb = multi_progress.add(ProgressBar::new(100));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("🛡️  {msg} {bar:50.cyan/blue} {percent}% [{elapsed_precise}]")
                    .unwrap()
                    .progress_chars("██▉▊▋▌▍▎▏  "),
            );
            Some(pb)
        };
        
        let mut findings = Vec::new();
        let phase_weight = if is_verbose { 1u64 } else { 100 / total_phases as u64 };
        let mut current_progress = 0u64;
        
        // 1. Configuration Security Analysis
        if self.config.check_secrets {
            if let Some(ref pb) = main_pb {
                pb.set_message("Analyzing configuration & secrets...");
                pb.set_position(current_progress);
            }
            
            if is_verbose {
                findings.extend(self.analyze_configuration_security(&analysis.project_root)?);
            } else {
                findings.extend(self.analyze_configuration_security_with_progress(&analysis.project_root, &multi_progress)?);
            }
            
            if let Some(ref pb) = main_pb {
                current_progress += phase_weight;
                pb.set_position(current_progress);
            }
        }
        
        // 2. Code Security Patterns
        if self.config.check_code_patterns {
            if let Some(ref pb) = main_pb {
                pb.set_message("Analyzing code security patterns...");
            }
            
            if is_verbose {
                findings.extend(self.analyze_code_security_patterns(&analysis.project_root, &analysis.languages)?);
            } else {
                findings.extend(self.analyze_code_security_patterns_with_progress(&analysis.project_root, &analysis.languages, &multi_progress)?);
            }
            
            if let Some(ref pb) = main_pb {
                current_progress += phase_weight;
                pb.set_position(current_progress);
            }
        }
        
        // 3. Infrastructure Security (skipped - not implemented yet)
        // TODO: Implement infrastructure security analysis
        // Currently all infrastructure analysis methods return empty results
        
        // 4. Environment Variables Security
        if let Some(ref pb) = main_pb {
            pb.set_message("Analyzing environment variables...");
        }
        
        findings.extend(self.analyze_environment_security(&analysis.environment_variables));
        if let Some(ref pb) = main_pb {
            current_progress += phase_weight;
            pb.set_position(current_progress);
        }
        
        // 5. Framework-specific Security (skipped - not implemented yet)
        // TODO: Implement framework-specific security analysis
        // Currently all framework analysis methods return empty results
        
        if let Some(ref pb) = main_pb {
            current_progress = 100;
            pb.set_position(current_progress);
        }
        
        // Processing phase
        if let Some(ref pb) = main_pb {
            pb.set_message("Processing findings & generating report...");
        }
        
        // DEDUPLICATION: Remove duplicate findings for the same secret/issue
        let pre_dedup_count = findings.len();
        findings = self.deduplicate_findings(findings);
        let post_dedup_count = findings.len();
        
        if pre_dedup_count != post_dedup_count {
            info!("Deduplicated {} redundant findings, {} unique findings remain", 
                  pre_dedup_count - post_dedup_count, post_dedup_count);
        }
        
        // Filter findings based on configuration
        let pre_filter_count = findings.len();
        if !self.config.include_low_severity {
            findings.retain(|f| f.severity != SecuritySeverity::Low && f.severity != SecuritySeverity::Info);
        }
        
        // Sort by severity (most critical first)
        findings.sort_by(|a, b| a.severity.cmp(&b.severity));
        
        // Calculate metrics
        let total_findings = findings.len();
        let findings_by_severity = self.count_by_severity(&findings);
        let findings_by_category = self.count_by_category(&findings);
        let overall_score = self.calculate_security_score(&findings);
        let risk_level = self.determine_risk_level(&findings);
        
        // Generate compliance status (disabled - not implemented yet)
        // TODO: Implement compliance assessment
        let compliance_status = HashMap::new();
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(&findings, &analysis.technologies);
        
        // Complete with summary
        let duration = start_time.elapsed().as_secs_f32();
        if let Some(pb) = main_pb {
            pb.finish_with_message(format!("✅ Security analysis completed in {:.1}s - Found {} issues", duration, total_findings));
        }
        
        // Print summary
        if pre_filter_count != total_findings {
            info!("Found {} total findings, showing {} after filtering", pre_filter_count, total_findings);
        } else {
            info!("Found {} security findings", total_findings);
        }
        
        Ok(SecurityReport {
            analyzed_at: chrono::Utc::now(),
            overall_score,
            risk_level,
            total_findings,
            findings_by_severity,
            findings_by_category,
            findings,
            recommendations,
            compliance_status,
        })
    }
    
    /// Check if a file is gitignored using git check-ignore command
    fn is_file_gitignored(&self, file_path: &Path) -> bool {
        // Return false if we don't have project root set
        let project_root = match &self.project_root {
            Some(root) => root,
            None => return false,
        };
        
        // Use cache to avoid repeated git calls
        if let Ok(cache) = self.git_ignore_cache.lock() {
            if let Some(&cached_result) = cache.get(file_path) {
                return cached_result;
            }
        }
        
        // Check if this is a git repository
        if !project_root.join(".git").exists() {
            debug!("Not a git repository, treating all files as tracked");
            return false;
        }
        
        // First, try git check-ignore for the most accurate result
        let git_result = Command::new("git")
            .args(&["check-ignore", "--quiet"])
            .arg(file_path)
            .current_dir(project_root)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        
        // If git check-ignore says it's ignored, trust it
        if git_result {
            if let Ok(mut cache) = self.git_ignore_cache.lock() {
                cache.insert(file_path.to_path_buf(), true);
            }
            return true;
        }
        
        // Fallback: Parse .gitignore files manually for common patterns
        // This helps when git check-ignore might not work perfectly in all scenarios
        let manual_result = self.check_gitignore_patterns(file_path, project_root);
        
        // Cache the result (prefer git result, fallback to manual)
        let final_result = git_result || manual_result;
        if let Ok(mut cache) = self.git_ignore_cache.lock() {
            cache.insert(file_path.to_path_buf(), final_result);
        }
        
        final_result
    }
    
    /// Manually check gitignore patterns as a fallback
    fn check_gitignore_patterns(&self, file_path: &Path, project_root: &Path) -> bool {
        // Get relative path from project root
        let relative_path = match file_path.strip_prefix(project_root) {
            Ok(rel) => rel,
            Err(_) => return false,
        };
        
        let path_str = relative_path.to_string_lossy();
        let file_name = relative_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        // Read .gitignore file
        let gitignore_path = project_root.join(".gitignore");
        if let Ok(gitignore_content) = fs::read_to_string(&gitignore_path) {
            for line in gitignore_content.lines() {
                let pattern = line.trim();
                if pattern.is_empty() || pattern.starts_with('#') {
                    continue;
                }
                
                // Check if this pattern matches our file
                if self.matches_gitignore_pattern(pattern, &path_str, file_name) {
                    debug!("File {} matches gitignore pattern: {}", path_str, pattern);
                    return true;
                }
            }
        }
        
        // Also check global gitignore patterns for common .env patterns
        self.matches_common_env_patterns(file_name)
    }
    
    /// Check if a file matches a specific gitignore pattern
    fn matches_gitignore_pattern(&self, pattern: &str, path_str: &str, file_name: &str) -> bool {
        // Handle different types of patterns
        if pattern.contains('*') {
            // Wildcard patterns
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                // Try matching both full path and just filename
                if glob_pattern.matches(path_str) || glob_pattern.matches(file_name) {
                    return true;
                }
            }
        } else if pattern.starts_with('/') {
            // Absolute path from repo root
            let abs_pattern = &pattern[1..];
            if path_str == abs_pattern {
                return true;
            }
        } else {
            // Simple pattern - could match anywhere in path
            if path_str == pattern || 
               file_name == pattern || 
               path_str.ends_with(&format!("/{}", pattern)) {
                return true;
            }
        }
        
        false
    }
    
    /// Check against common .env file patterns that should typically be ignored
    fn matches_common_env_patterns(&self, file_name: &str) -> bool {
        let common_env_patterns = [
            ".env",
            ".env.local",
            ".env.development", 
            ".env.production",
            ".env.staging",
            ".env.test",
            ".env.example", // Usually committed but should be treated carefully
        ];
        
        // Exact matches
        if common_env_patterns.contains(&file_name) {
            return file_name != ".env.example"; // .env.example is usually committed
        }
        
        // Pattern matches
        if file_name.starts_with(".env.") || 
           file_name.ends_with(".env") ||
           (file_name.starts_with(".") && file_name.contains("env")) {
            // Be conservative - only ignore if it's clearly a local/environment specific file
            return !file_name.contains("example") && 
                   !file_name.contains("sample") && 
                   !file_name.contains("template");
        }
        
        false
    }
    
    /// Check if a file is actually tracked by git
    fn is_file_tracked(&self, file_path: &Path) -> bool {
        let project_root = match &self.project_root {
            Some(root) => root,
            None => return true, // Assume tracked if no project root
        };
        
        // Check if this is a git repository
        if !project_root.join(".git").exists() {
            return true; // Not a git repo, treat as tracked
        }
        
        // Use git ls-files to check if file is tracked
        Command::new("git")
            .args(&["ls-files", "--error-unmatch"])
            .arg(file_path)
            .current_dir(project_root)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(true) // Default to tracked if git command fails
    }
    
    /// Determine the appropriate severity for a secret finding based on git status
    fn determine_secret_severity(&self, file_path: &Path, original_severity: SecuritySeverity) -> (SecuritySeverity, Vec<String>) {
        let mut additional_remediation = Vec::new();
        
        // Check if file is gitignored
        if self.is_file_gitignored(file_path) {
            if self.config.skip_gitignored_files {
                // Return Info level to indicate this should be skipped
                return (SecuritySeverity::Info, vec!["File is properly gitignored".to_string()]);
            } else if self.config.downgrade_gitignored_severity {
                // Downgrade severity for gitignored files
                let downgraded = match original_severity {
                    SecuritySeverity::Critical => SecuritySeverity::Medium,
                    SecuritySeverity::High => SecuritySeverity::Low,
                    SecuritySeverity::Medium => SecuritySeverity::Low,
                    SecuritySeverity::Low => SecuritySeverity::Info,
                    SecuritySeverity::Info => SecuritySeverity::Info,
                };
                additional_remediation.push("Note: File is gitignored, reducing severity".to_string());
                return (downgraded, additional_remediation);
            }
        }
        
        // Check if file is tracked by git
        if !self.is_file_tracked(file_path) {
            additional_remediation.push("Ensure this file is added to .gitignore to prevent accidental commits".to_string());
        } else {
            // File is tracked - this is a serious issue
            additional_remediation.push("⚠️  CRITICAL: This file is tracked by git! Secrets may be in version history.".to_string());
            additional_remediation.push("Consider using git-filter-branch or BFG Repo-Cleaner to remove from history".to_string());
            additional_remediation.push("Rotate any exposed secrets immediately".to_string());
            
            // Upgrade severity for tracked files
            let upgraded = match original_severity {
                SecuritySeverity::High => SecuritySeverity::Critical,
                SecuritySeverity::Medium => SecuritySeverity::High,
                SecuritySeverity::Low => SecuritySeverity::Medium,
                other => other,
            };
            return (upgraded, additional_remediation);
        }
        
        (original_severity, additional_remediation)
    }
    
    /// Initialize secret detection patterns
    fn initialize_secret_patterns() -> Result<Vec<SecretPattern>, SecurityError> {
        let patterns = vec![
            // API Keys and Tokens - Specific patterns first
            ("AWS Access Key", r"AKIA[0-9A-Z]{16}", SecuritySeverity::Critical),
            ("AWS Secret Key", r#"(?i)(aws[_-]?secret|secret[_-]?access[_-]?key)["']?\s*[:=]\s*["']?[A-Za-z0-9/+=]{40}["']?"#, SecuritySeverity::Critical),
            ("S3 Secret Key", r#"(?i)(s3[_-]?secret[_-]?key|linode[_-]?s3[_-]?secret)["']?\s*[:=]\s*["']?[A-Za-z0-9/+=]{20,}["']?"#, SecuritySeverity::High),
            ("GitHub Token", r"gh[pousr]_[A-Za-z0-9_]{36,255}", SecuritySeverity::High),
            ("OpenAI API Key", r"sk-[A-Za-z0-9]{48}", SecuritySeverity::High),
            ("Stripe API Key", r"sk_live_[0-9a-zA-Z]{24}", SecuritySeverity::Critical),
            ("Stripe Publishable Key", r"pk_live_[0-9a-zA-Z]{24}", SecuritySeverity::Medium),
            
            // Database URLs and Passwords - Enhanced to avoid env var false positives
            ("Hardcoded Database URL", r#"(?i)(database_url|db_url)["']?\s*[:=]\s*["']?(postgresql|mysql|mongodb)://[^"'\s]+"#, SecuritySeverity::Critical),
            ("Hardcoded Password", r#"(?i)(password|passwd|pwd)["']?\s*[:=]\s*["']?[^"']{6,}["']?"#, SecuritySeverity::High),
            ("JWT Secret", r#"(?i)(jwt[_-]?secret)["']?\s*[:=]\s*["']?[A-Za-z0-9_\-+/=]{20,}"#, SecuritySeverity::High),
            
            // Private Keys
            ("RSA Private Key", r"-----BEGIN RSA PRIVATE KEY-----", SecuritySeverity::Critical),
            ("SSH Private Key", r"-----BEGIN OPENSSH PRIVATE KEY-----", SecuritySeverity::Critical),
            ("PGP Private Key", r"-----BEGIN PGP PRIVATE KEY BLOCK-----", SecuritySeverity::Critical),
            
            // Cloud Provider Keys
            ("Google Cloud Service Account", r#""type":\s*"service_account""#, SecuritySeverity::High),
            ("Azure Storage Key", r"DefaultEndpointsProtocol=https;AccountName=", SecuritySeverity::High),
            
            // Client-side exposed environment variables (these are the real security issues)
            ("Client-side Exposed Secret", r#"(?i)(REACT_APP_|NEXT_PUBLIC_|VUE_APP_|VITE_)[A-Z_]*(?:SECRET|KEY|TOKEN|PASSWORD|API)[A-Z_]*["']?\s*[:=]\s*["']?[A-Za-z0-9_\-+/=]{10,}"#, SecuritySeverity::High),
            
            // Hardcoded API keys (not environment variable access)
            ("Hardcoded API Key", r#"(?i)(api[_-]?key|apikey)["']?\s*[:=]\s*["']?[A-Za-z0-9_\-]{20,}["']?"#, SecuritySeverity::High),
            
            // Generic secrets that are clearly hardcoded (not env var access)
            ("Hardcoded Secret", r#"(?i)(secret|token)["']?\s*[:=]\s*["']?[A-Za-z0-9_\-+/=]{24,}["']?"#, SecuritySeverity::Medium),
        ];
        
        patterns.into_iter()
            .map(|(name, pattern, severity)| {
                Ok(SecretPattern {
                    name: name.to_string(),
                    pattern: Regex::new(pattern)?,
                    severity,
                    description: format!("Potential {} found in code", name),
                })
            })
            .collect()
    }
    
    /// Initialize language-specific security rules
    fn initialize_security_rules() -> Result<HashMap<Language, Vec<SecurityRule>>, SecurityError> {
        let mut rules = HashMap::new();
        
        // JavaScript/TypeScript Rules
        rules.insert(Language::JavaScript, vec![
            SecurityRule {
                id: "js-001".to_string(),
                name: "Eval Usage".to_string(),
                pattern: Regex::new(r"\beval\s*\(")?,
                severity: SecuritySeverity::High,
                category: SecurityCategory::CodeSecurityPattern,
                description: "Use of eval() can lead to code injection vulnerabilities".to_string(),
                remediation: vec![
                    "Avoid using eval() with user input".to_string(),
                    "Use JSON.parse() for parsing JSON data".to_string(),
                    "Consider using safer alternatives like Function constructor with validation".to_string(),
                ],
                cwe_id: Some("CWE-95".to_string()),
            },
            SecurityRule {
                id: "js-002".to_string(),
                name: "innerHTML Usage".to_string(),
                pattern: Regex::new(r"\.innerHTML\s*=")?,
                severity: SecuritySeverity::Medium,
                category: SecurityCategory::CodeSecurityPattern,
                description: "innerHTML can lead to XSS vulnerabilities if used with unsanitized data".to_string(),
                remediation: vec![
                    "Use textContent instead of innerHTML for text".to_string(),
                    "Sanitize HTML content before setting innerHTML".to_string(),
                    "Consider using secure templating libraries".to_string(),
                ],
                cwe_id: Some("CWE-79".to_string()),
            },
        ]);
        
        // Python Rules
        rules.insert(Language::Python, vec![
            SecurityRule {
                id: "py-001".to_string(),
                name: "SQL Injection Risk".to_string(),
                pattern: Regex::new(r#"\.execute\s*\(\s*[f]?["'][^"']*%[sd]"#)?,
                severity: SecuritySeverity::High,
                category: SecurityCategory::CodeSecurityPattern,
                description: "String formatting in SQL queries can lead to SQL injection".to_string(),
                remediation: vec![
                    "Use parameterized queries instead of string formatting".to_string(),
                    "Use ORM query builders where possible".to_string(),
                    "Validate and sanitize all user inputs".to_string(),
                ],
                cwe_id: Some("CWE-89".to_string()),
            },
            SecurityRule {
                id: "py-002".to_string(),
                name: "Pickle Usage".to_string(),
                pattern: Regex::new(r"\bpickle\.loads?\s*\(")?,
                severity: SecuritySeverity::High,
                category: SecurityCategory::CodeSecurityPattern,
                description: "Pickle can execute arbitrary code during deserialization".to_string(),
                remediation: vec![
                    "Avoid pickle for untrusted data".to_string(),
                    "Use JSON or other safe serialization formats".to_string(),
                    "If pickle is necessary, validate data sources".to_string(),
                ],
                cwe_id: Some("CWE-502".to_string()),
            },
        ]);
        
        // Add more language rules as needed...
        
        Ok(rules)
    }
    
    /// Analyze configuration files for security issues with appropriate progress tracking
    fn analyze_configuration_security_with_progress(&self, project_root: &Path, multi_progress: &MultiProgress) -> Result<Vec<SecurityFinding>, SecurityError> {
        debug!("Analyzing configuration security");
        let mut findings = Vec::new();
        
        // Collect relevant files
        let config_files = self.collect_config_files(project_root)?;
        
        if config_files.is_empty() {
            info!("No configuration files found");
            return Ok(findings);
        }
        
        let is_verbose = log::max_level() >= log::LevelFilter::Info;
        
        info!("📁 Found {} configuration files to analyze", config_files.len());
        
        // Create appropriate progress tracking - completely skip in verbose mode
        let file_pb = if is_verbose {
            None // No progress bars at all in verbose mode
        } else {
            // Normal mode: Show detailed progress
            let pb = multi_progress.add(ProgressBar::new(config_files.len() as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("  🔍 {msg} {bar:40.cyan/blue} {pos}/{len} files ({percent}%)")
                    .unwrap()
                    .progress_chars("████▉▊▋▌▍▎▏  "),
            );
            pb.set_message("Scanning configuration files...");
            Some(pb)
        };
        
        // Use atomic counter for progress updates if needed
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let processed_count = Arc::new(AtomicUsize::new(0));
        
        // Analyze each file with appropriate progress tracking
        let file_findings: Vec<Vec<SecurityFinding>> = config_files
            .par_iter()
            .map(|file_path| {
                let result = self.analyze_file_for_secrets(file_path);
                
                // Update progress only in non-verbose mode
                if let Some(ref pb) = file_pb {
                    let current = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                    if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                        // Truncate long filenames for better display
                        let display_name = if file_name.len() > 30 {
                            format!("...{}", &file_name[file_name.len()-27..])
                        } else {
                            file_name.to_string()
                        };
                        pb.set_message(format!("Scanning {}", display_name));
                    }
                    pb.set_position(current as u64);
                }
                
                result
            })
            .filter_map(|result| result.ok())
            .collect();
        
        // Finish progress tracking
        if let Some(pb) = file_pb {
            pb.finish_with_message(format!("✅ Scanned {} configuration files", config_files.len()));
        }
        
        for mut file_findings in file_findings {
            findings.append(&mut file_findings);
        }
        
        // Check for common insecure configurations
        findings.extend(self.check_insecure_configurations(project_root)?);
        
        info!("🔍 Found {} configuration security findings", findings.len());
        Ok(findings)
    }
    
    /// Direct configuration security analysis without progress bars
    fn analyze_configuration_security(&self, project_root: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        debug!("Analyzing configuration security");
        let mut findings = Vec::new();
        
        // Collect relevant files
        let config_files = self.collect_config_files(project_root)?;
        
        if config_files.is_empty() {
            info!("No configuration files found");
            return Ok(findings);
        }
        
        info!("📁 Found {} configuration files to analyze", config_files.len());
        
        // Analyze each file directly without progress tracking
        let file_findings: Vec<Vec<SecurityFinding>> = config_files
            .par_iter()
            .map(|file_path| self.analyze_file_for_secrets(file_path))
            .filter_map(|result| result.ok())
            .collect();
        
        for mut file_findings in file_findings {
            findings.append(&mut file_findings);
        }
        
        // Check for common insecure configurations
        findings.extend(self.check_insecure_configurations(project_root)?);
        
        info!("🔍 Found {} configuration security findings", findings.len());
        Ok(findings)
    }
    
    /// Analyze code for security patterns with appropriate progress tracking
    fn analyze_code_security_patterns_with_progress(&self, project_root: &Path, languages: &[DetectedLanguage], multi_progress: &MultiProgress) -> Result<Vec<SecurityFinding>, SecurityError> {
        debug!("Analyzing code security patterns");
        let mut findings = Vec::new();
        
        // Count total source files across all languages
        let mut total_files = 0;
        let mut language_files = Vec::new();
        
        for language in languages {
            if let Some(lang) = Language::from_string(&language.name) {
                if let Some(_rules) = self.security_rules.get(&lang) {
                    let source_files = self.collect_source_files(project_root, &language.name)?;
                    total_files += source_files.len();
                    language_files.push((language, source_files));
                }
            }
        }
        
        if total_files == 0 {
            info!("No source files found for code pattern analysis");
            return Ok(findings);
        }
        
        let is_verbose = log::max_level() >= log::LevelFilter::Info;
        
        info!("📄 Found {} source files across {} languages", total_files, language_files.len());
        
        // Create appropriate progress tracking
        let code_pb = if is_verbose {
            // Verbose mode: No sub-progress to avoid visual clutter
            None
        } else {
            // Normal mode: Show detailed progress
            let pb = multi_progress.add(ProgressBar::new(total_files as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("  📄 {msg} {bar:40.yellow/white} {pos}/{len} files ({percent}%)")
                    .unwrap()
                    .progress_chars("████▉▊▋▌▍▎▏  "),
            );
            pb.set_message("Scanning source code...");
            Some(pb)
        };
    
        
        // Use atomic counter for progress if needed
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let processed_count = Arc::new(AtomicUsize::new(0));
        
        // Process all languages
        for (language, source_files) in language_files {
            if let Some(lang) = Language::from_string(&language.name) {
                if let Some(rules) = self.security_rules.get(&lang) {
                let file_findings: Vec<Vec<SecurityFinding>> = source_files
                    .par_iter()
                    .map(|file_path| {
                        let result = self.analyze_file_with_rules(file_path, rules);
                        
                        // Update progress only in non-verbose mode
                        if let Some(ref pb) = code_pb {
                            let current = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                            if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                                let display_name = if file_name.len() > 25 {
                                    format!("...{}", &file_name[file_name.len()-22..])
                                } else {
                                    file_name.to_string()
                                };
                                pb.set_message(format!("Scanning {} ({})", display_name, language.name));
                            }
                            pb.set_position(current as u64);
                        }
                        
                        result
                    })
                    .filter_map(|result| result.ok())
                    .collect();
                
                for mut file_findings in file_findings {
                    findings.append(&mut file_findings);
                }
                }
            }
        }
        
        // Finish progress tracking
        if let Some(pb) = code_pb {
            pb.finish_with_message(format!("✅ Scanned {} source files", total_files));
        }
        
        info!("🔍 Found {} code security findings", findings.len());
        Ok(findings)
    }
    
    /// Direct code security analysis without progress bars
    fn analyze_code_security_patterns(&self, project_root: &Path, languages: &[DetectedLanguage]) -> Result<Vec<SecurityFinding>, SecurityError> {
        debug!("Analyzing code security patterns");
        let mut findings = Vec::new();
        
        // Count total source files across all languages
        let mut total_files = 0;
        let mut language_files = Vec::new();
        
        for language in languages {
            if let Some(lang) = Language::from_string(&language.name) {
                if let Some(_rules) = self.security_rules.get(&lang) {
                    let source_files = self.collect_source_files(project_root, &language.name)?;
                    total_files += source_files.len();
                    language_files.push((language, source_files));
                }
            }
        }
        
        if total_files == 0 {
            info!("No source files found for code pattern analysis");
            return Ok(findings);
        }
        
        info!("📄 Found {} source files across {} languages", total_files, language_files.len());
        
        // Process all languages without progress tracking
        for (language, source_files) in language_files {
            if let Some(lang) = Language::from_string(&language.name) {
                if let Some(rules) = self.security_rules.get(&lang) {
                let file_findings: Vec<Vec<SecurityFinding>> = source_files
                    .par_iter()
                    .map(|file_path| self.analyze_file_with_rules(file_path, rules))
                    .filter_map(|result| result.ok())
                    .collect();
                
                for mut file_findings in file_findings {
                    findings.append(&mut file_findings);
                }
                }
            }
        }

        info!("🔍 Found {} code security findings", findings.len());
        Ok(findings)
    }
    
    /// Analyze infrastructure configurations with appropriate progress tracking
    fn analyze_infrastructure_security_with_progress(&self, project_root: &Path, _technologies: &[DetectedTechnology], multi_progress: &MultiProgress) -> Result<Vec<SecurityFinding>, SecurityError> {
        debug!("Analyzing infrastructure security");
        let mut findings = Vec::new();
        
        let is_verbose = log::max_level() >= log::LevelFilter::Info;
        
        // Create appropriate progress indicator
        let infra_pb = if is_verbose {
            // Verbose mode: No spinner to avoid conflicts with logs
            None
        } else {
            // Normal mode: Show spinner
            let pb = multi_progress.add(ProgressBar::new_spinner());
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("  🏗️  {msg} {spinner:.magenta}")
                    .unwrap()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            Some(pb)
        };
        
        // Check Dockerfile security
        if let Some(ref pb) = infra_pb {
            pb.set_message("Checking Dockerfiles & Compose files...");
        }
        findings.extend(self.analyze_dockerfile_security(project_root)?);
        findings.extend(self.analyze_compose_security(project_root)?);
        
        // Check CI/CD configurations
        if let Some(ref pb) = infra_pb {
            pb.set_message("Checking CI/CD configurations...");
        }
        findings.extend(self.analyze_cicd_security(project_root)?);
        
        // Finish progress tracking
        if let Some(pb) = infra_pb {
            pb.finish_with_message("✅ Infrastructure analysis complete");
        }
        info!("🔍 Found {} infrastructure security findings", findings.len());
        
        Ok(findings)
    }
    
    /// Direct infrastructure security analysis without progress bars
    fn analyze_infrastructure_security(&self, project_root: &Path, _technologies: &[DetectedTechnology]) -> Result<Vec<SecurityFinding>, SecurityError> {
        debug!("Analyzing infrastructure security");
        let mut findings = Vec::new();
        
        // Check Dockerfile security
        findings.extend(self.analyze_dockerfile_security(project_root)?);
        findings.extend(self.analyze_compose_security(project_root)?);
        
        // Check CI/CD configurations
        findings.extend(self.analyze_cicd_security(project_root)?);
        
        info!("🔍 Found {} infrastructure security findings", findings.len());
        Ok(findings)
    }
    
    /// Analyze environment variables for security issues
    fn analyze_environment_security(&self, env_vars: &[EnvVar]) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();
        
        for env_var in env_vars {
            // Check for sensitive variable names without proper protection
            if self.is_sensitive_env_var(&env_var.name) && env_var.default_value.is_some() {
                findings.push(SecurityFinding {
                    id: format!("env-{}", env_var.name.to_lowercase()),
                    title: "Sensitive Environment Variable with Default Value".to_string(),
                    description: format!("Environment variable '{}' appears to contain sensitive data but has a default value", env_var.name),
                    severity: SecuritySeverity::Medium,
                    category: SecurityCategory::SecretsExposure,
                    file_path: None,
                    line_number: None,
                    column_number: None,
                    evidence: Some(format!("Variable: {} = {:?}", env_var.name, env_var.default_value)),
                    remediation: vec![
                        "Remove default value for sensitive environment variables".to_string(),
                        "Use a secure secret management system".to_string(),
                        "Document required environment variables separately".to_string(),
                    ],
                    references: vec![
                        "https://owasp.org/www-project-top-ten/2021/A05_2021-Security_Misconfiguration/".to_string(),
                    ],
                    cwe_id: Some("CWE-200".to_string()),
                    compliance_frameworks: vec!["SOC2".to_string(), "GDPR".to_string()],
                });
            }
        }
        
        findings
    }
    
    /// Analyze framework-specific security configurations with appropriate progress
    fn analyze_framework_security_with_progress(&self, project_root: &Path, technologies: &[DetectedTechnology], multi_progress: &MultiProgress) -> Result<Vec<SecurityFinding>, SecurityError> {
        debug!("Analyzing framework-specific security");
        let mut findings = Vec::new();
        
        let framework_count = technologies.len();
        if framework_count == 0 {
            info!("No frameworks detected for security analysis");
            return Ok(findings);
        }
        
        let is_verbose = log::max_level() >= log::LevelFilter::Info;
        
        info!("🔧 Found {} frameworks to analyze", framework_count);
        
        // Create appropriate progress indicator
        let fw_pb = if is_verbose {
            // Verbose mode: No spinner to avoid conflicts with logs
            None
        } else {
            // Normal mode: Show spinner
            let pb = multi_progress.add(ProgressBar::new_spinner());
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("  🔧 {msg} {spinner:.cyan}")
                    .unwrap()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(120));
            Some(pb)
        };
        
        for tech in technologies {
            if let Some(ref pb) = fw_pb {
                pb.set_message(format!("Checking {} configuration...", tech.name));
            }
            
            match tech.name.as_str() {
                "Express.js" | "Express" => {
                    findings.extend(self.analyze_express_security(project_root)?);
                },
                "Django" => {
                    findings.extend(self.analyze_django_security(project_root)?);
                },
                "Spring Boot" => {
                    findings.extend(self.analyze_spring_security(project_root)?);
                },
                "Next.js" => {
                    findings.extend(self.analyze_nextjs_security(project_root)?);
                },
                // Add more frameworks as needed
                _ => {}
            }
        }
        
        // Finish progress tracking
        if let Some(pb) = fw_pb {
            pb.finish_with_message("✅ Framework analysis complete");
        }
        info!("🔍 Found {} framework security findings", findings.len());
        
        Ok(findings)
    }
    
    /// Direct framework security analysis without progress bars
    fn analyze_framework_security(&self, project_root: &Path, technologies: &[DetectedTechnology]) -> Result<Vec<SecurityFinding>, SecurityError> {
        debug!("Analyzing framework-specific security");
        let mut findings = Vec::new();
        
        let framework_count = technologies.len();
        if framework_count == 0 {
            info!("No frameworks detected for security analysis");
            return Ok(findings);
        }
        
        info!("🔧 Found {} frameworks to analyze", framework_count);
        
        for tech in technologies {
            match tech.name.as_str() {
                "Express.js" | "Express" => {
                    findings.extend(self.analyze_express_security(project_root)?);
                },
                "Django" => {
                    findings.extend(self.analyze_django_security(project_root)?);
                },
                "Spring Boot" => {
                    findings.extend(self.analyze_spring_security(project_root)?);
                },
                "Next.js" => {
                    findings.extend(self.analyze_nextjs_security(project_root)?);
                },
                // Add more frameworks as needed
                _ => {}
            }
        }
        
        info!("🔍 Found {} framework security findings", findings.len());
        Ok(findings)
    }
    
    // Helper methods for specific analyses...
    
    fn collect_config_files(&self, project_root: &Path) -> Result<Vec<PathBuf>, SecurityError> {
        let patterns = vec![
            "*.env*", "*.conf", "*.config", "*.ini", "*.yaml", "*.yml", 
            "*.toml", "docker-compose*.yml", "Dockerfile*",
            ".github/**/*.yml", ".gitlab-ci.yml", "package.json",
            "requirements.txt", "Cargo.toml", "go.mod", "pom.xml",
        ];
        
        let mut files = crate::common::file_utils::find_files_by_patterns(project_root, &patterns)
            .map_err(|e| SecurityError::Io(e))?;
        
        // Filter out files matching ignore patterns
        files.retain(|file| {
            let file_name = file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let file_path = file.to_string_lossy();
            
            !self.config.ignore_patterns.iter().any(|pattern| {
                if pattern.contains('*') {
                    // Use glob matching for wildcard patterns
                    glob::Pattern::new(pattern)
                        .map(|p| p.matches(&file_path) || p.matches(file_name))
                        .unwrap_or(false)
                } else {
                    // Exact string matching
                    file_path.contains(pattern) || file_name.contains(pattern)
                }
            })
        });
        
        Ok(files)
    }
    
    fn analyze_file_for_secrets(&self, file_path: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        let content = fs::read_to_string(file_path)?;
        let mut findings = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            for pattern in &self.secret_patterns {
                if let Some(match_) = pattern.pattern.find(line) {
                    // Skip if it looks like a placeholder or example
                    if self.is_likely_placeholder(line) {
                        continue;
                    }
                    
                    // NEW: Skip if this is legitimate environment variable usage
                    if self.is_legitimate_env_var_usage(line, file_path) {
                        debug!("Skipping legitimate env var usage: {}", line.trim());
                        continue;
                    }
                    
                    // Determine severity based on git status
                    let (severity, additional_remediation) = self.determine_secret_severity(file_path, pattern.severity.clone());
                    
                    // Skip if severity is Info (indicates gitignored and should be skipped)
                    if self.config.skip_gitignored_files && severity == SecuritySeverity::Info {
                        debug!("Skipping secret in gitignored file: {}", file_path.display());
                        continue;
                    }
                    
                    // Build base remediation steps
                    let mut remediation = vec![
                        "Remove sensitive data from source code".to_string(),
                        "Use environment variables for secrets".to_string(),
                        "Consider using a secure secret management service".to_string(),
                    ];
                    
                    // Add git-specific remediation based on file status
                    remediation.extend(additional_remediation);
                    
                    // Add generic gitignore advice if not already covered
                    if !self.is_file_gitignored(file_path) && !self.is_file_tracked(file_path) {
                        remediation.push("Add this file to .gitignore to prevent accidental commits".to_string());
                    }
                    
                    // Create enhanced finding with git-aware severity and remediation
                    let mut description = pattern.description.clone();
                    if self.is_file_tracked(file_path) {
                        description.push_str(" (⚠️  WARNING: File is tracked by git - secrets may be in version history!)");
                    } else if self.is_file_gitignored(file_path) {
                        description.push_str(" (ℹ️  Note: File is gitignored)");
                    }
                    
                    findings.push(SecurityFinding {
                        id: format!("secret-{}-{}", pattern.name.to_lowercase().replace(' ', "-"), line_num),
                        title: format!("Potential {} Exposure", pattern.name),
                        description,
                        severity,
                        category: SecurityCategory::SecretsExposure,
                        file_path: Some(file_path.to_path_buf()),
                        line_number: Some(line_num + 1),
                        column_number: Some(match_.start() + 1), // 1-indexed column position
                        evidence: Some(format!("Line: {}", line.trim())),
                        remediation,
                        references: vec![
                            "https://owasp.org/www-project-top-ten/2021/A05_2021-Security_Misconfiguration/".to_string(),
                        ],
                        cwe_id: Some("CWE-200".to_string()),
                        compliance_frameworks: vec!["SOC2".to_string(), "GDPR".to_string()],
                    });
                }
            }
        }
        
        Ok(findings)
    }
    
    /// Check if a line represents legitimate environment variable usage (not a security issue)
    fn is_legitimate_env_var_usage(&self, line: &str, file_path: &Path) -> bool {
        let line_trimmed = line.trim();
        
        // Check for common legitimate environment variable access patterns
        let legitimate_env_patterns = [
            // Node.js/JavaScript patterns
            r"process\.env\.[A-Z_]+",
            r#"process\.env\[['""][A-Z_]+['"]\]"#,
            
            // Vite/Modern JS patterns  
            r"import\.meta\.env\.[A-Z_]+",
            r#"import\.meta\.env\[['""][A-Z_]+['"]\]"#,
            
            // Python patterns
            r#"os\.environ\.get\(["'][A-Z_]+["']\)"#,
            r#"os\.environ\[["'][A-Z_]+["']\]"#,
            r#"getenv\(["'][A-Z_]+["']\)"#,
            
            // Rust patterns
            r#"env::var\("([A-Z_]+)"\)"#,
            r#"std::env::var\("([A-Z_]+)"\)"#,
            
            // Go patterns
            r#"os\.Getenv\(["'][A-Z_]+["']\)"#,
            
            // Java patterns
            r#"System\.getenv\(["'][A-Z_]+["']\)"#,
            
            // Shell/Docker patterns
            r"\$\{?[A-Z_]+\}?",
            r"ENV [A-Z_]+",
            
            // Config file access patterns
            r"config\.[a-z_]+\.[A-Z_]+",
            r"settings\.[A-Z_]+",
            r"env\.[A-Z_]+",
        ];
        
        // Check if the line matches any legitimate environment variable access pattern
        for pattern_str in &legitimate_env_patterns {
            if let Ok(pattern) = Regex::new(pattern_str) {
                if pattern.is_match(line_trimmed) {
                    // Additional context checks to make sure this is really legitimate
                    
                    // Check if this is in a server-side context (not client-side)
                    if self.is_server_side_file(file_path) {
                        return true;
                    }
                    
                    // Check if this is NOT a client-side exposed variable
                    if !self.is_client_side_exposed_env_var(line_trimmed) {
                        return true;
                    }
                }
            }
        }
        
        // Check for assignment vs access - assignments might be setting up environment variables
        // which could be legitimate in certain contexts
        if self.is_env_var_assignment_context(line_trimmed, file_path) {
            return true;
        }
        
        false
    }
    
    /// Check if a file is likely server-side code (vs client-side)
    fn is_server_side_file(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy().to_lowercase();
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Server-side indicators
        let server_indicators = [
            "/server/", "/api/", "/backend/", "/src/app/api/", "/pages/api/",
            "/routes/", "/controllers/", "/middleware/", "/models/",
            "/lib/", "/utils/", "/services/", "/config/",
            "server.js", "index.js", "app.js", "main.js",
            ".env", "dockerfile", "docker-compose",
        ];
        
        // Client-side indicators (these should return false)
        let client_indicators = [
            "/public/", "/static/", "/assets/", "/components/", "/pages/",
            "/src/components/", "/src/pages/", "/client/", "/frontend/",
            "index.html", ".html", "/dist/", "/build/",
            "dist/", "build/", "public/", "static/", "assets/",
        ];
        
        // If it's clearly client-side, return false
        if client_indicators.iter().any(|indicator| path_str.contains(indicator)) {
            return false;
        }
        
        // If it has server-side indicators, return true
        if server_indicators.iter().any(|indicator| 
            path_str.contains(indicator) || file_name.contains(indicator)
        ) {
            return true;
        }
        
        // Default to true for ambiguous cases (be conservative about flagging env var usage)
        true
    }
    
    /// Check if an environment variable is exposed to client-side (security issue)
    fn is_client_side_exposed_env_var(&self, line: &str) -> bool {
        let client_prefixes = [
            "REACT_APP_", "NEXT_PUBLIC_", "VUE_APP_", "VITE_", 
            "GATSBY_", "PUBLIC_", "NUXT_PUBLIC_",
        ];
        
        client_prefixes.iter().any(|prefix| line.contains(prefix))
    }
    
    /// Check if this is a legitimate environment variable assignment context
    fn is_env_var_assignment_context(&self, line: &str, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy().to_lowercase();
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Only very specific configuration files where env var assignments are expected
        // Be more restrictive to avoid false positives
        let env_config_files = [
            ".env", 
            "docker-compose.yml", "docker-compose.yaml",
            ".env.example", ".env.sample", ".env.template",
            ".env.local", ".env.development", ".env.production", ".env.staging",
        ];
        
        // Check for exact filename matches for .env files (most common legitimate case)
        if env_config_files.iter().any(|pattern| file_name == *pattern) {
            return true;
        }
        
        // Docker files are also legitimate for environment variable assignment
        if file_name.starts_with("dockerfile") || file_name == "dockerfile" {
            return true;
        }
        
        // Shell scripts or CI/CD files
        if file_name.ends_with(".sh") || 
           file_name.ends_with(".bash") ||
           path_str.contains(".github/workflows/") ||
           path_str.contains(".gitlab-ci") {
            return true;
        }
        
        // Lines that are clearly setting up environment variables for child processes
        // Only match very specific patterns that indicate legitimate environment setup
        let setup_patterns = [
            r"export [A-Z_]+=",           // Shell export
            r"ENV [A-Z_]+=",              // Dockerfile ENV
            r"^\s*environment:\s*$",      // Docker Compose environment section header
            r"^\s*env:\s*$",              // Kubernetes env section header
            r"process\.env\.[A-Z_]+ =",   // Explicitly setting process.env (rare but legitimate)
        ];
        
        for pattern_str in &setup_patterns {
            if let Ok(pattern) = Regex::new(pattern_str) {
                if pattern.is_match(line) {
                    return true;
                }
            }
        }
        
        false
    }
    
    fn is_likely_placeholder(&self, line: &str) -> bool {
        let placeholder_indicators = [
            "example", "placeholder", "your_", "insert_", "replace_",
            "xxx", "yyy", "zzz", "fake", "dummy", "test_key",
            "sk-xxxxxxxx", "AKIA00000000",
        ];
        
        let hash_indicators = [
            "checksum", "hash", "sha1", "sha256", "md5", "commit",
            "fingerprint", "digest", "advisory", "ghsa-", "cve-",
            "rustc_fingerprint", "last-commit", "references",
        ];
        
        let line_lower = line.to_lowercase();
        
        // Check for placeholder indicators
        if placeholder_indicators.iter().any(|indicator| line_lower.contains(indicator)) {
            return true;
        }
        
        // Check for hash/checksum context
        if hash_indicators.iter().any(|indicator| line_lower.contains(indicator)) {
            return true;
        }
        
        // Check if it's a URL or path (often contains hash-like strings)
        if line_lower.contains("http") || line_lower.contains("github.com") {
            return true;
        }
        
        // Check if it's likely a hex-only string (git commits, checksums)
        if let Some(potential_hash) = self.extract_potential_hash(line) {
            if potential_hash.len() >= 32 && self.is_hex_only(&potential_hash) {
                return true; // Likely a SHA hash
            }
        }
        
        false
    }
    
    fn extract_potential_hash(&self, line: &str) -> Option<String> {
        // Look for quoted strings that might be hashes
        if let Some(start) = line.find('"') {
            if let Some(end) = line[start + 1..].find('"') {
                let potential = &line[start + 1..start + 1 + end];
                if potential.len() >= 32 {
                    return Some(potential.to_string());
                }
            }
        }
        None
    }
    
    fn is_hex_only(&self, s: &str) -> bool {
        s.chars().all(|c| c.is_ascii_hexdigit())
    }
    
    fn is_sensitive_env_var(&self, name: &str) -> bool {
        let sensitive_patterns = [
            "password", "secret", "key", "token", "auth", "api",
            "private", "credential", "cert", "ssl", "tls",
        ];
        
        let name_lower = name.to_lowercase();
        sensitive_patterns.iter().any(|pattern| name_lower.contains(pattern))
    }
    
    // Placeholder implementations for specific framework analysis
    fn analyze_express_security(&self, _project_root: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        // TODO: Implement Express.js specific security checks
        Ok(vec![])
    }
    
    fn analyze_django_security(&self, _project_root: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        // TODO: Implement Django specific security checks
        Ok(vec![])
    }
    
    fn analyze_spring_security(&self, _project_root: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        // TODO: Implement Spring Boot specific security checks
        Ok(vec![])
    }
    
    fn analyze_nextjs_security(&self, _project_root: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        // TODO: Implement Next.js specific security checks
        Ok(vec![])
    }
    
    fn analyze_dockerfile_security(&self, _project_root: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        // TODO: Implement Dockerfile security analysis
        Ok(vec![])
    }
    
    fn analyze_compose_security(&self, _project_root: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        // TODO: Implement Docker Compose security analysis
        Ok(vec![])
    }
    
    fn analyze_cicd_security(&self, _project_root: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        // TODO: Implement CI/CD security analysis
        Ok(vec![])
    }
    
    // Additional helper methods...
    fn collect_source_files(&self, project_root: &Path, language: &str) -> Result<Vec<PathBuf>, SecurityError> {
        // TODO: Implement source file collection based on language
        Ok(vec![])
    }
    
    fn analyze_file_with_rules(&self, _file_path: &Path, _rules: &[SecurityRule]) -> Result<Vec<SecurityFinding>, SecurityError> {
        // TODO: Implement rule-based file analysis
        Ok(vec![])
    }
    
    fn check_insecure_configurations(&self, _project_root: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        // TODO: Implement insecure configuration checks
        Ok(vec![])
    }
    
    /// Deduplicate findings to avoid multiple reports for the same secret/issue
    fn deduplicate_findings(&self, mut findings: Vec<SecurityFinding>) -> Vec<SecurityFinding> {
        use std::collections::HashSet;
        
        let mut seen_secrets: HashSet<String> = HashSet::new();
        let mut deduplicated = Vec::new();
        
        // Sort by priority: more specific patterns first, then by severity
        findings.sort_by(|a, b| {
            // First, prioritize specific patterns over generic ones
            let a_priority = self.get_pattern_priority(&a.title);
            let b_priority = self.get_pattern_priority(&b.title);
            
            match a_priority.cmp(&b_priority) {
                std::cmp::Ordering::Equal => {
                    // If same priority, sort by severity (most critical first)
                    a.severity.cmp(&b.severity)
                }
                other => other
            }
        });
        
        for finding in findings {
            let key = self.generate_finding_key(&finding);
            
            if !seen_secrets.contains(&key) {
                seen_secrets.insert(key);
                deduplicated.push(finding);
            }
        }
        
        deduplicated
    }
    
    /// Generate a unique key for deduplication based on the type of finding
    fn generate_finding_key(&self, finding: &SecurityFinding) -> String {
        match finding.category {
            SecurityCategory::SecretsExposure => {
                // For secrets, deduplicate based on file path and the actual secret content
                if let Some(evidence) = &finding.evidence {
                    if let Some(file_path) = &finding.file_path {
                        // Extract the secret value from the evidence line
                        if let Some(secret_value) = self.extract_secret_value(evidence) {
                            return format!("secret:{}:{}", file_path.display(), secret_value);
                        }
                        // Fallback to file + line if we can't extract the value
                        if let Some(line_num) = finding.line_number {
                            return format!("secret:{}:{}", file_path.display(), line_num);
                        }
                    }
                }
                // Fallback for environment variables or other secrets without file paths
                format!("secret:{}", finding.title)
            }
            _ => {
                // For non-secret findings, use file path + line number + title
                if let Some(file_path) = &finding.file_path {
                    if let Some(line_num) = finding.line_number {
                        format!("other:{}:{}:{}", file_path.display(), line_num, finding.title)
                    } else {
                        format!("other:{}:{}", file_path.display(), finding.title)
                    }
                } else {
                    format!("other:{}", finding.title)
                }
            }
        }
    }
    
    /// Extract secret value from evidence line for deduplication
    fn extract_secret_value(&self, evidence: &str) -> Option<String> {
        // Look for patterns like "KEY=value" or "KEY: value"
        if let Some(pos) = evidence.find('=') {
            let value = evidence[pos + 1..].trim();
            // Remove quotes if present
            let value = value.trim_matches('"').trim_matches('\'');
            if value.len() > 10 { // Only consider substantial values
                return Some(value.to_string());
            }
        }
        
        // Look for patterns like "key: value" in YAML/JSON
        if let Some(pos) = evidence.find(':') {
            let value = evidence[pos + 1..].trim();
            let value = value.trim_matches('"').trim_matches('\'');
            if value.len() > 10 {
                return Some(value.to_string());
            }
        }
        
        None
    }
    
    /// Get pattern priority for deduplication (lower number = higher priority)
    fn get_pattern_priority(&self, title: &str) -> u8 {
        // Most specific patterns get highest priority (lowest number)
        if title.contains("AWS Access Key") { return 1; }
        if title.contains("AWS Secret Key") { return 1; }
        if title.contains("S3 Secret Key") { return 1; }
        if title.contains("GitHub Token") { return 1; }
        if title.contains("OpenAI API Key") { return 1; }
        if title.contains("Stripe") { return 1; }
        if title.contains("RSA Private Key") { return 1; }
        if title.contains("SSH Private Key") { return 1; }
        
        // JWT and specific API keys are more specific than generic
        if title.contains("JWT Secret") { return 2; }
        if title.contains("Database URL") { return 2; }
        
        // Generic API key patterns are less specific
        if title.contains("API Key") { return 3; }
        
        // Environment variable findings are less specific
        if title.contains("Environment Variable") { return 4; }
        
        // Generic patterns get lowest priority (highest number)
        if title.contains("Generic Secret") { return 5; }
        
        // Default priority for other patterns
        3
    }
    
    fn count_by_severity(&self, findings: &[SecurityFinding]) -> HashMap<SecuritySeverity, usize> {
        let mut counts = HashMap::new();
        for finding in findings {
            *counts.entry(finding.severity.clone()).or_insert(0) += 1;
        }
        counts
    }
    
    fn count_by_category(&self, findings: &[SecurityFinding]) -> HashMap<SecurityCategory, usize> {
        let mut counts = HashMap::new();
        for finding in findings {
            *counts.entry(finding.category.clone()).or_insert(0) += 1;
        }
        counts
    }
    
    fn calculate_security_score(&self, findings: &[SecurityFinding]) -> f32 {
        if findings.is_empty() {
            return 100.0;
        }
        
        let total_penalty = findings.iter().map(|f| match f.severity {
            SecuritySeverity::Critical => 25.0,
            SecuritySeverity::High => 15.0,
            SecuritySeverity::Medium => 8.0,
            SecuritySeverity::Low => 3.0,
            SecuritySeverity::Info => 1.0,
        }).sum::<f32>();
        
        (100.0 - total_penalty).max(0.0)
    }
    
    fn determine_risk_level(&self, findings: &[SecurityFinding]) -> SecuritySeverity {
        if findings.iter().any(|f| f.severity == SecuritySeverity::Critical) {
            SecuritySeverity::Critical
        } else if findings.iter().any(|f| f.severity == SecuritySeverity::High) {
            SecuritySeverity::High
        } else if findings.iter().any(|f| f.severity == SecuritySeverity::Medium) {
            SecuritySeverity::Medium
        } else if !findings.is_empty() {
            SecuritySeverity::Low
        } else {
            SecuritySeverity::Info
        }
    }
    
    fn assess_compliance(&self, _findings: &[SecurityFinding], _technologies: &[DetectedTechnology]) -> HashMap<String, ComplianceStatus> {
        // TODO: Implement compliance assessment
        HashMap::new()
    }
    
    fn generate_recommendations(&self, findings: &[SecurityFinding], _technologies: &[DetectedTechnology]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if findings.iter().any(|f| f.category == SecurityCategory::SecretsExposure) {
            recommendations.push("Implement a secure secret management strategy".to_string());
        }
        
        if findings.iter().any(|f| f.severity == SecuritySeverity::Critical) {
            recommendations.push("Address critical security findings immediately".to_string());
        }
        
        recommendations
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_security_score_calculation() {
        let analyzer = SecurityAnalyzer::new().unwrap();
        
        let findings = vec![
            SecurityFinding {
                id: "test-1".to_string(),
                title: "Test Critical".to_string(),
                description: "Test".to_string(),
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::SecretsExposure,
                file_path: None,
                line_number: None,
                column_number: None,
                evidence: None,
                remediation: vec![],
                references: vec![],
                cwe_id: None,
                compliance_frameworks: vec![],
            }
        ];
        
        let score = analyzer.calculate_security_score(&findings);
        assert_eq!(score, 75.0); // 100 - 25 (critical penalty)
    }
    
    #[test]
    fn test_secret_pattern_matching() {
        let analyzer = SecurityAnalyzer::new().unwrap();
        
        // Test if placeholder detection works
        assert!(analyzer.is_likely_placeholder("API_KEY=sk-xxxxxxxxxxxxxxxx"));
        assert!(!analyzer.is_likely_placeholder("API_KEY=sk-1234567890abcdef"));
    }
    
    #[test]
    fn test_sensitive_env_var_detection() {
        let analyzer = SecurityAnalyzer::new().unwrap();
        
        assert!(analyzer.is_sensitive_env_var("DATABASE_PASSWORD"));
        assert!(analyzer.is_sensitive_env_var("JWT_SECRET"));
        assert!(!analyzer.is_sensitive_env_var("PORT"));
        assert!(!analyzer.is_sensitive_env_var("NODE_ENV"));
    }
    
    #[test]
    fn test_gitignore_aware_severity() {
        use tempfile::TempDir;
        use std::fs;
        use std::process::Command;
        
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        
        // Initialize a real git repo
        let git_init = Command::new("git")
            .args(&["init"])
            .current_dir(project_root)
            .output();
        
        // Skip test if git is not available
        if git_init.is_err() {
            println!("Skipping gitignore test - git not available");
            return;
        }
        
        // Create .gitignore file
        fs::write(project_root.join(".gitignore"), ".env\n.env.local\n").unwrap();
        
        // Stage and commit .gitignore to make it effective
        let _ = Command::new("git")
            .args(&["add", ".gitignore"])
            .current_dir(project_root)
            .output();
        let _ = Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(project_root)
            .output();
        let _ = Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(project_root)
            .output();
        let _ = Command::new("git")
            .args(&["commit", "-m", "Add gitignore"])
            .current_dir(project_root)
            .output();
        
        let mut analyzer = SecurityAnalyzer::new().unwrap();
        analyzer.project_root = Some(project_root.to_path_buf());
        
        // Test file that would be gitignored
        let env_file = project_root.join(".env");
        fs::write(&env_file, "API_KEY=sk-1234567890abcdef").unwrap();
        
        // Test severity determination for gitignored file
        let (severity, remediation) = analyzer.determine_secret_severity(&env_file, SecuritySeverity::High);
        
        // With default config, gitignored files should be marked as Info (skipped)
        assert_eq!(severity, SecuritySeverity::Info);
        assert!(remediation.iter().any(|r| r.contains("gitignored")));
    }
    
    #[test]
    fn test_gitignore_config_options() {
        let mut config = SecurityAnalysisConfig::default();
        
        // Test default configuration
        assert!(config.skip_gitignored_files);
        assert!(!config.downgrade_gitignored_severity);
        
        // Test downgrade mode
        config.skip_gitignored_files = false;
        config.downgrade_gitignored_severity = true;
        
        let analyzer = SecurityAnalyzer::with_config(config).unwrap();
        // Additional test logic could be added here for downgrade behavior
    }
    
    #[test]
    fn test_gitignore_pattern_matching() {
        let analyzer = SecurityAnalyzer::new().unwrap();
        
        // Test wildcard patterns - *.env matches files ending with .env
        assert!(!analyzer.matches_gitignore_pattern("*.env", ".env.local", ".env.local")); // Doesn't end with .env
        assert!(analyzer.matches_gitignore_pattern("*.env", "production.env", "production.env")); // Ends with .env
        assert!(analyzer.matches_gitignore_pattern(".env*", ".env.production", ".env.production")); // Starts with .env
        assert!(analyzer.matches_gitignore_pattern("*.log", "app.log", "app.log"));
        
        // Test exact patterns
        assert!(analyzer.matches_gitignore_pattern(".env", ".env", ".env"));
        assert!(!analyzer.matches_gitignore_pattern(".env", ".env.local", ".env.local"));
        
        // Test directory patterns
        assert!(analyzer.matches_gitignore_pattern("/config.json", "config.json", "config.json"));
        assert!(!analyzer.matches_gitignore_pattern("/config.json", "src/config.json", "config.json"));
        
        // Test common .env patterns that should work
        assert!(analyzer.matches_gitignore_pattern(".env*", ".env", ".env"));
        assert!(analyzer.matches_gitignore_pattern(".env*", ".env.local", ".env.local"));
        assert!(analyzer.matches_gitignore_pattern(".env.*", ".env.production", ".env.production"));
    }
    
    #[test]
    fn test_common_env_patterns() {
        let analyzer = SecurityAnalyzer::new().unwrap();
        
        // Should match common .env files
        assert!(analyzer.matches_common_env_patterns(".env"));
        assert!(analyzer.matches_common_env_patterns(".env.local"));
        assert!(analyzer.matches_common_env_patterns(".env.production"));
        assert!(analyzer.matches_common_env_patterns(".env.development"));
        assert!(analyzer.matches_common_env_patterns(".env.test"));
        
        // Should NOT match example/template files (usually committed)
        assert!(!analyzer.matches_common_env_patterns(".env.example"));
        assert!(!analyzer.matches_common_env_patterns(".env.sample"));
        assert!(!analyzer.matches_common_env_patterns(".env.template"));
        
        // Should not match non-env files
        assert!(!analyzer.matches_common_env_patterns("config.json"));
        assert!(!analyzer.matches_common_env_patterns("package.json"));
    }
    
    #[test]
    fn test_legitimate_env_var_usage() {
        let analyzer = SecurityAnalyzer::new().unwrap();
        
        // Create mock file paths
        let server_file = Path::new("src/server/config.js");
        let client_file = Path::new("src/components/MyComponent.js");
        
        // Test legitimate server-side environment variable usage (should NOT be flagged)
        assert!(analyzer.is_legitimate_env_var_usage("const apiKey = process.env.RESEND_API_KEY;", server_file));
        assert!(analyzer.is_legitimate_env_var_usage("const dbUrl = process.env.DATABASE_URL;", server_file));
        assert!(analyzer.is_legitimate_env_var_usage("api_key = os.environ.get('API_KEY')", server_file));
        assert!(analyzer.is_legitimate_env_var_usage("let secret = env::var(\"JWT_SECRET\")?;", server_file));
        
        // Test client-side environment variable usage (legitimate if not exposed)
        assert!(analyzer.is_legitimate_env_var_usage("const apiUrl = process.env.API_URL;", client_file));
        
        // Test client-side exposed variables (these ARE client-side exposed - security issues)
        assert!(analyzer.is_client_side_exposed_env_var("process.env.REACT_APP_SECRET_KEY"));
        assert!(analyzer.is_client_side_exposed_env_var("process.env.NEXT_PUBLIC_API_SECRET"));
        
        // Test hardcoded secrets (should NOT be legitimate)
        assert!(!analyzer.is_legitimate_env_var_usage("const apiKey = 'sk-1234567890abcdef';", server_file));
        assert!(!analyzer.is_legitimate_env_var_usage("password = 'hardcoded123'", server_file));
    }
    
    #[test]
    fn test_server_vs_client_side_detection() {
        let analyzer = SecurityAnalyzer::new().unwrap();
        
        // Server-side files
        assert!(analyzer.is_server_side_file(Path::new("src/server/app.js")));
        assert!(analyzer.is_server_side_file(Path::new("src/api/users.js")));
        assert!(analyzer.is_server_side_file(Path::new("pages/api/auth.js")));
        assert!(analyzer.is_server_side_file(Path::new("src/lib/database.js")));
        assert!(analyzer.is_server_side_file(Path::new(".env")));
        assert!(analyzer.is_server_side_file(Path::new("server.js")));
        
        // Client-side files
        assert!(!analyzer.is_server_side_file(Path::new("src/components/Button.jsx")));
        assert!(!analyzer.is_server_side_file(Path::new("public/index.html")));
        assert!(!analyzer.is_server_side_file(Path::new("src/pages/home.js")));
        assert!(!analyzer.is_server_side_file(Path::new("dist/bundle.js")));
        
        // Ambiguous files (default to server-side for conservative detection)
        assert!(analyzer.is_server_side_file(Path::new("src/utils/helper.js")));
        assert!(analyzer.is_server_side_file(Path::new("config/settings.js")));
    }
    
    #[test]
    fn test_client_side_exposed_env_vars() {
        let analyzer = SecurityAnalyzer::new().unwrap();
        
        // These should be flagged as client-side exposed (security issues)
        assert!(analyzer.is_client_side_exposed_env_var("process.env.REACT_APP_SECRET"));
        assert!(analyzer.is_client_side_exposed_env_var("import.meta.env.VITE_API_KEY"));
        assert!(analyzer.is_client_side_exposed_env_var("process.env.NEXT_PUBLIC_SECRET"));
        assert!(analyzer.is_client_side_exposed_env_var("process.env.VUE_APP_TOKEN"));
        
        // These should NOT be flagged as client-side exposed
        assert!(!analyzer.is_client_side_exposed_env_var("process.env.DATABASE_URL"));
        assert!(!analyzer.is_client_side_exposed_env_var("process.env.JWT_SECRET"));
        assert!(!analyzer.is_client_side_exposed_env_var("process.env.API_KEY"));
    }
    
    #[test]
    fn test_env_var_assignment_context() {
        let analyzer = SecurityAnalyzer::new().unwrap();
        
        // Configuration files where assignments are legitimate
        assert!(analyzer.is_env_var_assignment_context("API_KEY=sk-test123", Path::new(".env")));
        assert!(analyzer.is_env_var_assignment_context("DATABASE_URL=postgres://", Path::new("docker-compose.yml")));
        assert!(analyzer.is_env_var_assignment_context("export SECRET=test", Path::new("setup.sh")));
        
        // Regular source files where assignments might be suspicious
        assert!(!analyzer.is_env_var_assignment_context("const secret = 'hardcoded'", Path::new("src/app.js")));
    }
    
    #[test]
    fn test_enhanced_secret_patterns() {
        let analyzer = SecurityAnalyzer::new().unwrap();
        
        // Test that hardcoded secrets are still detected
        let hardcoded_patterns = [
            "apikey = 'sk-1234567890abcdef1234567890abcdef12345678'",
            "const secret = 'my-super-secret-token-12345678901234567890'",
            "password = 'hardcoded123456'",
        ];
        
        for pattern in &hardcoded_patterns {
            let has_secret = analyzer.secret_patterns.iter().any(|sp| sp.pattern.is_match(pattern));
            assert!(has_secret, "Should detect hardcoded secret in: {}", pattern);
        }
        
        // Test that legitimate env var usage is NOT detected as secret
        let legitimate_patterns = [
            "const apiKey = process.env.API_KEY;",
            "const dbUrl = process.env.DATABASE_URL || 'fallback';",
            "api_key = os.environ.get('API_KEY')",
            "let secret = env::var(\"JWT_SECRET\")?;",
        ];
        
        for pattern in &legitimate_patterns {
            // These should either not match any secret pattern, or be filtered out by context detection
            let matches_old_generic_pattern = pattern.to_lowercase().contains("secret") || 
                                            pattern.to_lowercase().contains("key");
            
            // Our new patterns should be more specific and not match env var access
            let matches_new_patterns = analyzer.secret_patterns.iter()
                .filter(|sp| sp.name.contains("Hardcoded"))
                .any(|sp| sp.pattern.is_match(pattern));
            
            assert!(!matches_new_patterns, "Should NOT detect legitimate env var usage as hardcoded secret: {}", pattern);
        }
    }
    
    #[test]
    fn test_context_aware_false_positive_reduction() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let server_file = temp_dir.path().join("src/server/config.js");
        
        // Create directory structure
        std::fs::create_dir_all(server_file.parent().unwrap()).unwrap();
        
        // Write a file with legitimate environment variable usage
        let content = r#"
const config = {
    apiKey: process.env.RESEND_API_KEY,
    databaseUrl: process.env.DATABASE_URL,
    jwtSecret: process.env.JWT_SECRET,
    port: process.env.PORT || 3000
};
"#;
        
        std::fs::write(&server_file, content).unwrap();
        
        let analyzer = SecurityAnalyzer::new().unwrap();
        let findings = analyzer.analyze_file_for_secrets(&server_file).unwrap();
        
        // Should have zero findings because all are legitimate env var usage
        assert_eq!(findings.len(), 0, "Should not flag legitimate environment variable usage as security issues");
    }
} 
