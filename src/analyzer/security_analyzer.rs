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
        }
    }
}

pub struct SecurityAnalyzer {
    config: SecurityAnalysisConfig,
    secret_patterns: Vec<SecretPattern>,
    security_rules: HashMap<Language, Vec<SecurityRule>>,
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
        })
    }
    
    /// Perform comprehensive security analysis with appropriate progress for verbosity level
    pub fn analyze_security(&self, analysis: &ProjectAnalysis) -> Result<SecurityReport, SecurityError> {
        let start_time = Instant::now();
        info!("Starting comprehensive security analysis");
        
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
        
        // 3. Infrastructure Security
        if self.config.check_infrastructure {
            if let Some(ref pb) = main_pb {
                pb.set_message("Analyzing infrastructure security...");
            }
            
            if is_verbose {
                findings.extend(self.analyze_infrastructure_security(&analysis.project_root, &analysis.technologies)?);
            } else {
                findings.extend(self.analyze_infrastructure_security_with_progress(&analysis.project_root, &analysis.technologies, &multi_progress)?);
            }
            
            if let Some(ref pb) = main_pb {
                current_progress += phase_weight;
                pb.set_position(current_progress);
            }
        }
        
        // 4. Environment Variables Security
        if let Some(ref pb) = main_pb {
            pb.set_message("Analyzing environment variables...");
        }
        
        findings.extend(self.analyze_environment_security(&analysis.environment_variables));
        if let Some(ref pb) = main_pb {
            current_progress += phase_weight;
            pb.set_position(current_progress);
        }
        
        // 5. Framework-specific Security
        if let Some(ref pb) = main_pb {
            pb.set_message("Analyzing framework security...");
        }
        
        if is_verbose {
            findings.extend(self.analyze_framework_security(&analysis.project_root, &analysis.technologies)?);
        } else {
            findings.extend(self.analyze_framework_security_with_progress(&analysis.project_root, &analysis.technologies, &multi_progress)?);
        }
        
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
        
        // Generate compliance status
        let compliance_status = if self.config.check_compliance {
            self.assess_compliance(&findings, &analysis.technologies)
        } else {
            HashMap::new()
        };
        
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
            
            // Database URLs and Passwords
            ("Database URL", r#"(?i)(database_url|db_url)["']?\s*[:=]\s*["']?[^"'\s]+"#, SecuritySeverity::High),
            ("Password", r#"(?i)(password|passwd|pwd)["']?\s*[:=]\s*["']?[^"']{6,}"#, SecuritySeverity::Medium),
            ("JWT Secret", r#"(?i)(jwt[_-]?secret)["']?\s*[:=]\s*["']?[A-Za-z0-9_\-+/=]{20,}"#, SecuritySeverity::High),
            
            // Private Keys
            ("RSA Private Key", r"-----BEGIN RSA PRIVATE KEY-----", SecuritySeverity::Critical),
            ("SSH Private Key", r"-----BEGIN OPENSSH PRIVATE KEY-----", SecuritySeverity::Critical),
            ("PGP Private Key", r"-----BEGIN PGP PRIVATE KEY BLOCK-----", SecuritySeverity::Critical),
            
            // Cloud Provider Keys
            ("Google Cloud Service Account", r#""type":\s*"service_account""#, SecuritySeverity::High),
            ("Azure Storage Key", r"DefaultEndpointsProtocol=https;AccountName=", SecuritySeverity::High),
            
            // Generic patterns last (lowest priority)
            ("Generic API Key", r#"(?i)(api[_-]?key|apikey)["']?\s*[:=]\s*["']?[A-Za-z0-9_\-]{20,}"#, SecuritySeverity::High),
            ("Generic Secret", r#"(?i)(secret|token|key)["']?\s*[:=]\s*["']?[A-Za-z0-9_\-+/=]{24,}"#, SecuritySeverity::Medium),
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
            if let Some(_rules) = self.security_rules.get(&Language::from_string(&language.name)) {
                let source_files = self.collect_source_files(project_root, &language.name)?;
                total_files += source_files.len();
                language_files.push((language, source_files));
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
            if let Some(rules) = self.security_rules.get(&Language::from_string(&language.name)) {
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
            if let Some(_rules) = self.security_rules.get(&Language::from_string(&language.name)) {
                let source_files = self.collect_source_files(project_root, &language.name)?;
                total_files += source_files.len();
                language_files.push((language, source_files));
            }
        }
        
        if total_files == 0 {
            info!("No source files found for code pattern analysis");
            return Ok(findings);
        }
        
        info!("📄 Found {} source files across {} languages", total_files, language_files.len());
        
        // Process all languages without progress tracking
        for (language, source_files) in language_files {
            if let Some(rules) = self.security_rules.get(&Language::from_string(&language.name)) {
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
                    evidence: Some(format!("Variable: {} = {:?}", env_var.name, env_var.default_value)),
                    remediation: vec![
                        "Remove default value for sensitive environment variables".to_string(),
                        "Use a secure secret management system".to_string(),
                        "Document required environment variables separately".to_string(),
                    ],
                    references: vec![
                        "https://owasp.org/www-project-top-ten/2017/A3_2017-Sensitive_Data_Exposure".to_string(),
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
                if let Some(captures) = pattern.pattern.find(line) {
                    // Skip if it looks like a placeholder or example
                    if self.is_likely_placeholder(line) {
                        continue;
                    }
                    
                    findings.push(SecurityFinding {
                        id: format!("secret-{}-{}", pattern.name.to_lowercase().replace(' ', "-"), line_num),
                        title: format!("Potential {} Exposure", pattern.name),
                        description: pattern.description.clone(),
                        severity: pattern.severity.clone(),
                        category: SecurityCategory::SecretsExposure,
                        file_path: Some(file_path.to_path_buf()),
                        line_number: Some(line_num + 1),
                        evidence: Some(format!("Line: {}", line.trim())),
                        remediation: vec![
                            "Remove sensitive data from source code".to_string(),
                            "Use environment variables for secrets".to_string(),
                            "Consider using a secure secret management service".to_string(),
                            "Add this file to .gitignore if it contains secrets".to_string(),
                        ],
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
        
        // Add more generic recommendations...
        
        recommendations
    }
}

impl Language {
    fn from_string(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "rust" => Language::Rust,
            "javascript" | "js" => Language::JavaScript,
            "typescript" | "ts" => Language::TypeScript,
            "python" | "py" => Language::Python,
            "go" | "golang" => Language::Go,
            "java" => Language::Java,
            "kotlin" => Language::Kotlin,
            _ => Language::Unknown,
        }
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
} 