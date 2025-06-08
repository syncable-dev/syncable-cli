//! # Python Security Analyzer
//! 
//! Specialized security analyzer for Python applications.
//! 
//! This analyzer focuses on:
//! - Python web frameworks (Django, Flask, FastAPI, etc.)
//! - AI/ML services and tools (OpenAI, Anthropic, Hugging Face, etc.)
//! - Cloud services commonly used with Python (AWS, GCP, Azure)
//! - Database connections and ORMs (SQLAlchemy, Django ORM, etc.)
//! - Environment variable misuse in Python applications
//! - Common Python anti-patterns and secret exposure patterns
//! - Python package managers and dependency files

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use regex::Regex;
use log::{debug, info, warn};

use super::{SecurityError, SecurityFinding, SecuritySeverity, SecurityCategory, SecurityReport, SecurityAnalysisConfig, GitIgnoreAnalyzer, GitIgnoreRisk};

/// Python-specific security analyzer
pub struct PythonSecurityAnalyzer {
    config: SecurityAnalysisConfig,
    python_patterns: Vec<PythonSecretPattern>,
    framework_patterns: HashMap<String, Vec<FrameworkPattern>>,
    ai_ml_patterns: Vec<AiMlPattern>,
    cloud_patterns: Vec<CloudPattern>,
    database_patterns: Vec<DatabasePattern>,
    env_var_patterns: Vec<EnvVarPattern>,
    gitignore_analyzer: Option<GitIgnoreAnalyzer>,
}

/// Python-specific secret pattern
#[derive(Debug, Clone)]
pub struct PythonSecretPattern {
    pub id: String,
    pub name: String,
    pub pattern: Regex,
    pub severity: SecuritySeverity,
    pub description: String,
    pub context_indicators: Vec<String>,
    pub false_positive_indicators: Vec<String>,
    pub remediation_hints: Vec<String>,
}

/// Framework-specific patterns for Python web frameworks
#[derive(Debug, Clone)]
pub struct FrameworkPattern {
    pub framework: String,
    pub pattern: Regex,
    pub severity: SecuritySeverity,
    pub description: String,
    pub file_extensions: Vec<String>,
}

/// AI/ML service patterns
#[derive(Debug, Clone)]
pub struct AiMlPattern {
    pub service: String,
    pub pattern: Regex,
    pub severity: SecuritySeverity,
    pub description: String,
    pub api_key_format: String,
}

/// Cloud service patterns
#[derive(Debug, Clone)]
pub struct CloudPattern {
    pub provider: String,
    pub service: String,
    pub pattern: Regex,
    pub severity: SecuritySeverity,
    pub description: String,
}

/// Database connection patterns
#[derive(Debug, Clone)]
pub struct DatabasePattern {
    pub database_type: String,
    pub pattern: Regex,
    pub severity: SecuritySeverity,
    pub description: String,
}

/// Environment variable patterns specific to Python
#[derive(Debug, Clone)]
pub struct EnvVarPattern {
    pub pattern: Regex,
    pub severity: SecuritySeverity,
    pub description: String,
    pub sensitive_prefixes: Vec<String>,
}

impl PythonSecurityAnalyzer {
    pub fn new() -> Result<Self, SecurityError> {
        Self::with_config(SecurityAnalysisConfig::default())
    }
    
    pub fn with_config(config: SecurityAnalysisConfig) -> Result<Self, SecurityError> {
        let python_patterns = Self::initialize_python_patterns()?;
        let framework_patterns = Self::initialize_framework_patterns()?;
        let ai_ml_patterns = Self::initialize_ai_ml_patterns()?;
        let cloud_patterns = Self::initialize_cloud_patterns()?;
        let database_patterns = Self::initialize_database_patterns()?;
        let env_var_patterns = Self::initialize_env_var_patterns()?;
        
        Ok(Self {
            config,
            python_patterns,
            framework_patterns,
            ai_ml_patterns,
            cloud_patterns,
            database_patterns,
            env_var_patterns,
            gitignore_analyzer: None,
        })
    }
    
    /// Analyze a Python project for security vulnerabilities
    pub fn analyze_project(&mut self, project_root: &Path) -> Result<SecurityReport, SecurityError> {
        let mut findings = Vec::new();
        
        // Initialize gitignore analyzer for comprehensive file protection assessment
        let mut gitignore_analyzer = GitIgnoreAnalyzer::new(project_root)
            .map_err(|e| SecurityError::AnalysisFailed(format!("Failed to initialize gitignore analyzer: {}", e)))?;
        
        info!("🔍 Using gitignore-aware security analysis for Python project at {}", project_root.display());
        
        // Get Python files using gitignore-aware collection
        let python_extensions = ["py", "pyx", "pyi", "pyw"];
        let python_files = gitignore_analyzer.get_files_to_analyze(&python_extensions)
            .map_err(|e| SecurityError::Io(e))?
            .into_iter()
            .filter(|file| {
                if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
                    python_extensions.contains(&ext)
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();
        
        info!("Found {} Python files to analyze (gitignore-filtered)", python_files.len());
        
        // Analyze each Python file with gitignore context
        for file_path in &python_files {
            let gitignore_status = gitignore_analyzer.analyze_file(file_path);
            let mut file_findings = self.analyze_python_file(file_path)?;
            
            // Enhance findings with gitignore risk assessment
            for finding in &mut file_findings {
                self.enhance_finding_with_gitignore_status(finding, &gitignore_status);
            }
            
            findings.extend(file_findings);
        }
        
        // Analyze Python configuration files with gitignore awareness
        findings.extend(self.analyze_config_files_with_gitignore(project_root, &mut gitignore_analyzer)?);
        
        // Comprehensive environment file analysis with gitignore risk assessment
        findings.extend(self.analyze_env_files_with_gitignore(project_root, &mut gitignore_analyzer)?);
        
        // Analyze Python-specific dependency files
        findings.extend(self.analyze_dependency_files_with_gitignore(project_root, &mut gitignore_analyzer)?);
        
        // Generate gitignore recommendations for any secret files found
        let secret_files: Vec<PathBuf> = findings.iter()
            .filter_map(|f| f.file_path.as_ref())
            .cloned()
            .collect();
        
        let gitignore_recommendations = gitignore_analyzer.generate_gitignore_recommendations(&secret_files);
        
        // Create report with enhanced recommendations
        let mut report = SecurityReport::from_findings(findings);
        report.recommendations.extend(gitignore_recommendations);
        
        // Add Python-specific security recommendations
        report.recommendations.extend(self.generate_python_security_recommendations());
        
        Ok(report)
    }
    
    /// Analyze a single Python file for security vulnerabilities
    fn analyze_python_file(&self, file_path: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        let content = fs::read_to_string(file_path)?;
        let mut findings = Vec::new();
        
        // Check against Python-specific patterns
        for pattern in &self.python_patterns {
            findings.extend(self.check_python_pattern_in_content(&content, pattern, file_path)?);
        }
        
        // Check against AI/ML service patterns
        for pattern in &self.ai_ml_patterns {
            findings.extend(self.check_ai_ml_pattern_in_content(&content, pattern, file_path)?);
        }
        
        // Check against cloud service patterns
        for pattern in &self.cloud_patterns {
            findings.extend(self.check_cloud_pattern_in_content(&content, pattern, file_path)?);
        }
        
        // Check against database patterns
        for pattern in &self.database_patterns {
            findings.extend(self.check_database_pattern_in_content(&content, pattern, file_path)?);
        }
        
        // Check framework-specific patterns based on file content
        let detected_framework = self.detect_python_framework(&content);
        if let Some(framework) = detected_framework {
            if let Some(framework_patterns) = self.framework_patterns.get(&framework) {
                for pattern in framework_patterns {
                    findings.extend(self.check_framework_pattern_in_content(&content, pattern, file_path)?);
                }
            }
        }
        
        // Check environment variable usage
        findings.extend(self.check_env_var_usage(&content, file_path)?);
        
        // Check for insecure Python practices
        findings.extend(self.check_insecure_python_practices(&content, file_path)?);
        
        Ok(findings)
    }
    
    /// Check a Python-specific pattern in file content
    fn check_python_pattern_in_content(
        &self,
        content: &str,
        pattern: &PythonSecretPattern,
        file_path: &Path,
    ) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            if let Some(captures) = pattern.pattern.captures(line) {
                // Check for false positive indicators
                if pattern.false_positive_indicators.iter().any(|indicator| {
                    line.to_lowercase().contains(&indicator.to_lowercase())
                }) {
                    debug!("Skipping potential false positive in {}: {}", file_path.display(), line.trim());
                    continue;
                }
                
                // Extract the secret value and position if captured
                let (evidence, column_number) = if captures.len() > 1 {
                    if let Some(match_) = captures.get(1) {
                        (Some(self.mask_secret(match_.as_str())), Some(match_.start() + 1))
                    } else {
                        (Some(line.trim().to_string()), None)
                    }
                } else {
                    if let Some(match_) = captures.get(0) {
                        (Some(line.trim().to_string()), Some(match_.start() + 1))
                    } else {
                        (Some(line.trim().to_string()), None)
                    }
                };
                
                // Check context for confidence scoring
                let context_score = self.calculate_context_confidence(content, &pattern.context_indicators);
                let adjusted_severity = self.adjust_severity_by_context(pattern.severity.clone(), context_score);
                
                findings.push(SecurityFinding {
                    id: format!("{}-{}", pattern.id, line_num),
                    title: format!("{} Detected", pattern.name),
                    description: format!("{} (Context confidence: {:.1})", pattern.description, context_score),
                    severity: adjusted_severity,
                    category: SecurityCategory::SecretsExposure,
                    file_path: Some(file_path.to_path_buf()),
                    line_number: Some(line_num + 1),
                    column_number,
                    evidence,
                    remediation: pattern.remediation_hints.clone(),
                    references: vec![
                        "https://owasp.org/www-project-top-ten/2021/A05_2021-Security_Misconfiguration/".to_string(),
                        "https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html".to_string(),
                        "https://docs.python.org/3/library/os.html#os.environ".to_string(),
                    ],
                    cwe_id: Some("CWE-200".to_string()),
                    compliance_frameworks: vec!["SOC2".to_string(), "GDPR".to_string()],
                });
            }
        }
        
        Ok(findings)
    }
    
    /// Check AI/ML service patterns
    fn check_ai_ml_pattern_in_content(
        &self,
        content: &str,
        pattern: &AiMlPattern,
        file_path: &Path,
    ) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            if let Some(captures) = pattern.pattern.captures(line) {
                let evidence = if captures.len() > 1 {
                    captures.get(1).map(|m| self.mask_secret(m.as_str()))
                } else {
                    Some(line.trim().to_string())
                };
                
                let column_number = captures.get(0).map(|m| m.start() + 1);
                
                findings.push(SecurityFinding {
                    id: format!("ai-ml-{}-{}", pattern.service.to_lowercase().replace(" ", "-"), line_num),
                    title: format!("{} API Key Detected", pattern.service),
                    description: format!("{} (Expected format: {})", pattern.description, pattern.api_key_format),
                    severity: pattern.severity.clone(),
                    category: SecurityCategory::SecretsExposure,
                    file_path: Some(file_path.to_path_buf()),
                    line_number: Some(line_num + 1),
                    column_number,
                    evidence,
                    remediation: vec![
                        format!("Store {} API key in environment variables", pattern.service),
                        "Use a secrets management service for production".to_string(),
                        "Implement API key rotation policies".to_string(),
                        "Monitor API key usage for anomalies".to_string(),
                    ],
                    references: vec![
                        "https://owasp.org/www-project-api-security/".to_string(),
                        format!("https://platform.openai.com/docs/quickstart/account-setup"),
                    ],
                    cwe_id: Some("CWE-798".to_string()),
                    compliance_frameworks: vec!["SOC2".to_string(), "GDPR".to_string()],
                });
            }
        }
        
        Ok(findings)
    }
    
    /// Check cloud service patterns
    fn check_cloud_pattern_in_content(
        &self,
        content: &str,
        pattern: &CloudPattern,
        file_path: &Path,
    ) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            if let Some(captures) = pattern.pattern.captures(line) {
                let evidence = if captures.len() > 1 {
                    captures.get(1).map(|m| self.mask_secret(m.as_str()))
                } else {
                    Some(line.trim().to_string())
                };
                
                let column_number = captures.get(0).map(|m| m.start() + 1);
                
                findings.push(SecurityFinding {
                    id: format!("cloud-{}-{}-{}", 
                              pattern.provider.to_lowercase(),
                              pattern.service.to_lowercase().replace(" ", "-"),
                              line_num),
                    title: format!("{} {} Detected", pattern.provider, pattern.service),
                    description: pattern.description.clone(),
                    severity: pattern.severity.clone(),
                    category: SecurityCategory::SecretsExposure,
                    file_path: Some(file_path.to_path_buf()),
                    line_number: Some(line_num + 1),
                    column_number,
                    evidence,
                    remediation: vec![
                        format!("Use {} managed identity or role-based access", pattern.provider),
                        "Store credentials in secure key management service".to_string(),
                        "Implement credential rotation policies".to_string(),
                        "Use least-privilege access principles".to_string(),
                    ],
                    references: vec![
                        "https://owasp.org/www-project-top-ten/2021/A07_2021-Identification_and_Authentication_Failures/".to_string(),
                        format!("https://docs.aws.amazon.com/security/"),
                    ],
                    cwe_id: Some("CWE-522".to_string()),
                    compliance_frameworks: vec!["SOC2".to_string(), "PCI-DSS".to_string()],
                });
            }
        }
        
        Ok(findings)
    }
    
    /// Check database patterns
    fn check_database_pattern_in_content(
        &self,
        content: &str,
        pattern: &DatabasePattern,
        file_path: &Path,
    ) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            if pattern.pattern.is_match(line) {
                // Mask the connection string for evidence
                let masked_line = self.mask_database_connection(line);
                
                findings.push(SecurityFinding {
                    id: format!("database-{}-{}", pattern.database_type.to_lowercase(), line_num),
                    title: format!("{} Connection String with Credentials", pattern.database_type),
                    description: pattern.description.clone(),
                    severity: pattern.severity.clone(),
                    category: SecurityCategory::SecretsExposure,
                    file_path: Some(file_path.to_path_buf()),
                    line_number: Some(line_num + 1),
                    column_number: None,
                    evidence: Some(masked_line),
                    remediation: vec![
                        "Use environment variables for database credentials".to_string(),
                        "Implement connection pooling with credential management".to_string(),
                        "Use database authentication mechanisms like IAM roles".to_string(),
                        "Consider using encrypted connection strings".to_string(),
                    ],
                    references: vec![
                        "https://owasp.org/www-project-top-ten/2021/A07_2021-Identification_and_Authentication_Failures/".to_string(),
                        "https://cheatsheetseries.owasp.org/cheatsheets/Database_Security_Cheat_Sheet.html".to_string(),
                    ],
                    cwe_id: Some("CWE-798".to_string()),
                    compliance_frameworks: vec!["SOC2".to_string(), "GDPR".to_string(), "PCI-DSS".to_string()],
                });
            }
        }
        
        Ok(findings)
    }
    
    /// Check framework-specific patterns
    fn check_framework_pattern_in_content(
        &self,
        content: &str,
        pattern: &FrameworkPattern,
        file_path: &Path,
    ) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            if let Some(captures) = pattern.pattern.captures(line) {
                let evidence = if captures.len() > 1 {
                    captures.get(1).map(|m| self.mask_secret(m.as_str()))
                } else {
                    Some(line.trim().to_string())
                };
                
                findings.push(SecurityFinding {
                    id: format!("framework-{}-{}", pattern.framework.to_lowercase(), line_num),
                    title: format!("{} Security Issue", pattern.framework),
                    description: pattern.description.clone(),
                    severity: pattern.severity.clone(),
                    category: SecurityCategory::SecretsExposure,
                    file_path: Some(file_path.to_path_buf()),
                    line_number: Some(line_num + 1),
                    column_number: None,
                    evidence,
                    remediation: self.generate_framework_remediation(&pattern.framework),
                    references: vec![
                        format!("https://docs.djangoproject.com/en/stable/topics/security/"),
                        "https://owasp.org/www-project-top-ten/".to_string(),
                    ],
                    cwe_id: Some("CWE-200".to_string()),
                    compliance_frameworks: vec!["SOC2".to_string()],
                });
            }
        }
        
        Ok(findings)
    }
    
    /// Initialize Python-specific secret patterns
    fn initialize_python_patterns() -> Result<Vec<PythonSecretPattern>, SecurityError> {
        let patterns = vec![
            // Django SECRET_KEY pattern
            PythonSecretPattern {
                id: "python-django-secret-key".to_string(),
                name: "Django SECRET_KEY".to_string(),
                pattern: Regex::new(r#"(?i)SECRET_KEY\s*=\s*["']([A-Za-z0-9!@#$%^&*()_+\-=\[\]{}|;:,.<>?/~`]{40,})["']"#)?,
                severity: SecuritySeverity::Critical,
                description: "Django SECRET_KEY found in source code".to_string(),
                context_indicators: vec!["django".to_string(), "settings".to_string(), "SECRET_KEY".to_string()],
                false_positive_indicators: vec!["example".to_string(), "your-secret-key".to_string(), "fake".to_string()],
                remediation_hints: vec![
                    "Move SECRET_KEY to environment variables".to_string(),
                    "Use python-decouple or similar library".to_string(),
                    "Never commit SECRET_KEY to version control".to_string(),
                ],
            },
            
            // Flask SECRET_KEY pattern
            PythonSecretPattern {
                id: "python-flask-secret-key".to_string(),
                name: "Flask SECRET_KEY".to_string(),
                pattern: Regex::new(r#"(?i)app\.secret_key\s*=\s*["']([A-Za-z0-9!@#$%^&*()_+\-=\[\]{}|;:,.<>?/~`]{20,})["']"#)?,
                severity: SecuritySeverity::High,
                description: "Flask SECRET_KEY hardcoded in application".to_string(),
                context_indicators: vec!["flask".to_string(), "app".to_string(), "secret_key".to_string()],
                false_positive_indicators: vec!["example".to_string(), "your-secret".to_string()],
                remediation_hints: vec![
                    "Use os.environ.get('SECRET_KEY')".to_string(),
                    "Store in environment variables".to_string(),
                ],
            },
            
            // FastAPI JWT secret
            PythonSecretPattern {
                id: "python-fastapi-jwt-secret".to_string(),
                name: "FastAPI JWT Secret".to_string(),
                pattern: Regex::new(r#"(?i)(?:jwt_secret|jwt_key|secret_key)\s*=\s*["']([A-Za-z0-9!@#$%^&*()_+\-=\[\]{}|;:,.<>?/~`]{20,})["']"#)?,
                severity: SecuritySeverity::High,
                description: "FastAPI JWT secret hardcoded in source".to_string(),
                context_indicators: vec!["fastapi".to_string(), "jwt".to_string(), "token".to_string()],
                false_positive_indicators: vec!["example".to_string(), "test".to_string()],
                remediation_hints: vec![
                    "Use Pydantic Settings for configuration".to_string(),
                    "Store JWT secrets in environment variables".to_string(),
                ],
            },
            
            // Database connection strings
            PythonSecretPattern {
                id: "python-database-url".to_string(),
                name: "Database Connection String".to_string(),
                pattern: Regex::new(r#"(?i)(?:database_url|db_url|sqlalchemy_database_uri)\s*=\s*["'](?:postgresql|mysql|sqlite|mongodb)://[^"']*:[^"']*@[^"']+["']"#)?,
                severity: SecuritySeverity::Critical,
                description: "Database connection string with credentials detected".to_string(),
                context_indicators: vec!["database".to_string(), "sqlalchemy".to_string(), "connect".to_string()],
                false_positive_indicators: vec!["localhost".to_string(), "example.com".to_string(), "user:pass".to_string()],
                remediation_hints: vec![
                    "Use environment variables for database credentials".to_string(),
                    "Consider using connection pooling and secrets management".to_string(),
                ],
            },
            
            // Generic API key pattern
            PythonSecretPattern {
                id: "python-api-key-assignment".to_string(),
                name: "API Key Assignment".to_string(),
                pattern: Regex::new(r#"(?i)(?:api_key|apikey|access_key|secret_key|private_key|auth_token|bearer_token)\s*=\s*["']([A-Za-z0-9_-]{20,})["']"#)?,
                severity: SecuritySeverity::High,
                description: "API key hardcoded in variable assignment".to_string(),
                context_indicators: vec!["requests".to_string(), "api".to_string(), "client".to_string()],
                false_positive_indicators: vec!["os.environ".to_string(), "config".to_string(), "settings".to_string()],
                remediation_hints: vec![
                    "Use environment variables or config files".to_string(),
                    "Consider using secrets management services".to_string(),
                ],
            },
        ];
        
        Ok(patterns)
    }
    
    /// Initialize AI/ML service patterns
    fn initialize_ai_ml_patterns() -> Result<Vec<AiMlPattern>, SecurityError> {
        let patterns = vec![
            // OpenAI API keys
            AiMlPattern {
                service: "OpenAI".to_string(),
                pattern: Regex::new(r#"(?i)(?:openai[_-]?api[_-]?key|openai[_-]?key)\s*[=:]\s*["']?(sk-[A-Za-z0-9]{32,})["']?"#)?,
                severity: SecuritySeverity::Critical,
                description: "OpenAI API key detected".to_string(),
                api_key_format: "sk-[32+ alphanumeric characters]".to_string(),
            },
            
            // OpenAI Organization ID
            AiMlPattern {
                service: "OpenAI Organization".to_string(),
                pattern: Regex::new(r#"(?i)(?:openai[_-]?org[_-]?id|openai[_-]?organization)\s*[=:]\s*["']?(org-[A-Za-z0-9]{20,})["']?"#)?,
                severity: SecuritySeverity::Medium,
                description: "OpenAI organization ID detected".to_string(),
                api_key_format: "org-[20+ alphanumeric characters]".to_string(),
            },
            
            // Anthropic Claude API keys
            AiMlPattern {
                service: "Anthropic Claude".to_string(),
                pattern: Regex::new(r#"(?i)(?:anthropic[_-]?api[_-]?key|claude[_-]?api[_-]?key)\s*[=:]\s*["']?(sk-ant-[A-Za-z0-9]{40,})["']?"#)?,
                severity: SecuritySeverity::Critical,
                description: "Anthropic Claude API key detected".to_string(),
                api_key_format: "sk-ant-[40+ alphanumeric characters]".to_string(),
            },
            
            // Hugging Face API tokens
            AiMlPattern {
                service: "Hugging Face".to_string(),
                pattern: Regex::new(r#"(?i)(?:huggingface[_-]?api[_-]?key|huggingface[_-]?token|hf[_-]?token)\s*[=:]\s*["']?(hf_[A-Za-z0-9]{30,})["']?"#)?,
                severity: SecuritySeverity::High,
                description: "Hugging Face API token detected".to_string(),
                api_key_format: "hf_[30+ alphanumeric characters]".to_string(),
            },
            
            // Google AI/Gemini API keys
            AiMlPattern {
                service: "Google AI/Gemini".to_string(),
                pattern: Regex::new(r#"(?i)(?:google[_-]?ai[_-]?api[_-]?key|gemini[_-]?api[_-]?key)\s*[=:]\s*["']?(AIza[A-Za-z0-9_-]{35,})["']?"#)?,
                severity: SecuritySeverity::Critical,
                description: "Google AI/Gemini API key detected".to_string(),
                api_key_format: "AIza[35+ alphanumeric characters with underscores/dashes]".to_string(),
            },
            
            // Cohere API keys
            AiMlPattern {
                service: "Cohere".to_string(),
                pattern: Regex::new(r#"(?i)(?:cohere[_-]?api[_-]?key)\s*[=:]\s*["']?([A-Za-z0-9]{40,})["']?"#)?,
                severity: SecuritySeverity::High,
                description: "Cohere API key detected".to_string(),
                api_key_format: "[40+ alphanumeric characters]".to_string(),
            },
            
            // Replicate API tokens
            AiMlPattern {
                service: "Replicate".to_string(),
                pattern: Regex::new(r#"(?i)(?:replicate[_-]?api[_-]?token|replicate[_-]?token)\s*[=:]\s*["']?(r8_[A-Za-z0-9]{30,})["']?"#)?,
                severity: SecuritySeverity::High,
                description: "Replicate API token detected".to_string(),
                api_key_format: "r8_[30+ alphanumeric characters]".to_string(),
            },
            
            // Stability AI API keys
            AiMlPattern {
                service: "Stability AI".to_string(),
                pattern: Regex::new(r#"(?i)(?:stability[_-]?ai[_-]?api[_-]?key|stable[_-]?diffusion[_-]?api[_-]?key)\s*[=:]\s*["']?(sk-[A-Za-z0-9]{40,})["']?"#)?,
                severity: SecuritySeverity::High,
                description: "Stability AI API key detected".to_string(),
                api_key_format: "sk-[40+ alphanumeric characters]".to_string(),
            },
            
            // DeepSeek API keys
            AiMlPattern {
                service: "DeepSeek".to_string(),
                pattern: Regex::new(r#"(?i)(?:deepseek[_-]?api[_-]?key)\s*[=:]\s*["']?(sk-[A-Za-z0-9]{32,})["']?"#)?,
                severity: SecuritySeverity::High,
                description: "DeepSeek API key detected".to_string(),
                api_key_format: "sk-[32+ alphanumeric characters]".to_string(),
            },
            
            // Mistral AI API keys
            AiMlPattern {
                service: "Mistral AI".to_string(),
                pattern: Regex::new(r#"(?i)(?:mistral[_-]?api[_-]?key)\s*[=:]\s*["']?([A-Za-z0-9]{32,})["']?"#)?,
                severity: SecuritySeverity::High,
                description: "Mistral AI API key detected".to_string(),
                api_key_format: "[32+ alphanumeric characters]".to_string(),
            },
            
            // Together AI API keys
            AiMlPattern {
                service: "Together AI".to_string(),
                pattern: Regex::new(r#"(?i)(?:together[_-]?ai[_-]?api[_-]?key|together[_-]?api[_-]?key)\s*[=:]\s*["']?([A-Za-z0-9]{40,})["']?"#)?,
                severity: SecuritySeverity::High,
                description: "Together AI API key detected".to_string(),
                api_key_format: "[40+ alphanumeric characters]".to_string(),
            },
            
            // Weights & Biases API keys
            AiMlPattern {
                service: "Weights & Biases".to_string(),
                pattern: Regex::new(r#"(?i)(?:wandb[_-]?api[_-]?key|wandb[_-]?key)\s*[=:]\s*["']?([A-Za-z0-9]{40,})["']?"#)?,
                severity: SecuritySeverity::Medium,
                description: "Weights & Biases API key detected".to_string(),
                api_key_format: "[40+ alphanumeric characters]".to_string(),
            },
            
            // MLflow tracking server credentials
            AiMlPattern {
                service: "MLflow".to_string(),
                pattern: Regex::new(r#"(?i)(?:mlflow[_-]?tracking[_-]?username|mlflow[_-]?tracking[_-]?password)\s*[=:]\s*["']?([A-Za-z0-9]{8,})["']?"#)?,
                severity: SecuritySeverity::Medium,
                description: "MLflow tracking credentials detected".to_string(),
                api_key_format: "[8+ alphanumeric characters]".to_string(),
            },
        ];
        
        Ok(patterns)
    }
    
    /// Initialize cloud service patterns
    fn initialize_cloud_patterns() -> Result<Vec<CloudPattern>, SecurityError> {
        let patterns = vec![
            // AWS Access Keys
            CloudPattern {
                provider: "AWS".to_string(),
                service: "IAM Access Key".to_string(),
                pattern: Regex::new(r#"(?i)(?:aws[_-]?access[_-]?key[_-]?id)\s*[=:]\s*["']?(AKIA[A-Z0-9]{16})["']?"#)?,
                severity: SecuritySeverity::Critical,
                description: "AWS Access Key ID detected".to_string(),
            },
            
            // AWS Secret Access Keys
            CloudPattern {
                provider: "AWS".to_string(),
                service: "IAM Secret Key".to_string(),
                pattern: Regex::new(r#"(?i)(?:aws[_-]?secret[_-]?access[_-]?key)\s*[=:]\s*["']?([A-Za-z0-9/+=]{40})["']?"#)?,
                severity: SecuritySeverity::Critical,
                description: "AWS Secret Access Key detected".to_string(),
            },
            
            // AWS Session Tokens
            CloudPattern {
                provider: "AWS".to_string(),
                service: "Session Token".to_string(),
                pattern: Regex::new(r#"(?i)(?:aws[_-]?session[_-]?token)\s*[=:]\s*["']?([A-Za-z0-9/+=]{100,})["']?"#)?,
                severity: SecuritySeverity::High,
                description: "AWS Session Token detected".to_string(),
            },
            
            // Google Cloud Service Account Keys
            CloudPattern {
                provider: "GCP".to_string(),
                service: "Service Account Key".to_string(),
                pattern: Regex::new(r#"(?i)(?:google[_-]?application[_-]?credentials|gcp[_-]?service[_-]?account)\s*[=:]\s*["']?([A-Za-z0-9/+=]{50,})["']?"#)?,
                severity: SecuritySeverity::Critical,
                description: "Google Cloud Service Account key detected".to_string(),
            },
            
            // Azure Storage Account Keys
            CloudPattern {
                provider: "Azure".to_string(),
                service: "Storage Account Key".to_string(),
                pattern: Regex::new(r#"(?i)(?:azure[_-]?storage[_-]?account[_-]?key|azure[_-]?storage[_-]?key)\s*[=:]\s*["']?([A-Za-z0-9/+=]{88})["']?"#)?,
                severity: SecuritySeverity::Critical,
                description: "Azure Storage Account key detected".to_string(),
            },
            
            // Azure Service Principal
            CloudPattern {
                provider: "Azure".to_string(),
                service: "Service Principal".to_string(),
                pattern: Regex::new(r#"(?i)(?:azure[_-]?client[_-]?secret|azure[_-]?tenant[_-]?id)\s*[=:]\s*["']?([A-Za-z0-9-]{32,})["']?"#)?,
                severity: SecuritySeverity::Critical,
                description: "Azure Service Principal credentials detected".to_string(),
            },
            
            // DigitalOcean API tokens
            CloudPattern {
                provider: "DigitalOcean".to_string(),
                service: "API Token".to_string(),
                pattern: Regex::new(r#"(?i)(?:digitalocean[_-]?api[_-]?token|do[_-]?api[_-]?token)\s*[=:]\s*["']?(dop_v1_[A-Za-z0-9]{64})["']?"#)?,
                severity: SecuritySeverity::High,
                description: "DigitalOcean API token detected".to_string(),
            },
            
            // Heroku API keys
            CloudPattern {
                provider: "Heroku".to_string(),
                service: "API Key".to_string(),
                pattern: Regex::new(r#"(?i)(?:heroku[_-]?api[_-]?key)\s*[=:]\s*["']?([A-Za-z0-9-]{36})["']?"#)?,
                severity: SecuritySeverity::High,
                description: "Heroku API key detected".to_string(),
            },
            
            // Stripe API keys
            CloudPattern {
                provider: "Stripe".to_string(),
                service: "API Key".to_string(),
                pattern: Regex::new(r#"(?i)(?:stripe[_-]?api[_-]?key|stripe[_-]?secret[_-]?key)\s*[=:]\s*["']?(sk_live_[A-Za-z0-9]{24}|sk_test_[A-Za-z0-9]{24})["']?"#)?,
                severity: SecuritySeverity::Critical,
                description: "Stripe API key detected".to_string(),
            },
            
            // Twilio credentials
            CloudPattern {
                provider: "Twilio".to_string(),
                service: "Auth Token".to_string(),
                pattern: Regex::new(r#"(?i)(?:twilio[_-]?auth[_-]?token|twilio[_-]?account[_-]?sid)\s*[=:]\s*["']?([A-Za-z0-9]{32,34})["']?"#)?,
                severity: SecuritySeverity::High,
                description: "Twilio credentials detected".to_string(),
            },
        ];
        
        Ok(patterns)
    }
    
    /// Initialize framework-specific patterns
    fn initialize_framework_patterns() -> Result<HashMap<String, Vec<FrameworkPattern>>, SecurityError> {
        let mut frameworks = HashMap::new();
        
        // Django patterns
        frameworks.insert("django".to_string(), vec![
            FrameworkPattern {
                framework: "Django".to_string(),
                pattern: Regex::new(r#"(?i)(?:database|databases)\s*=\s*\{[^}]*['"']password['"']\s*:\s*['"']([^'"']+)['"'][^}]*\}"#)?,
                severity: SecuritySeverity::Critical,
                description: "Django database password in settings".to_string(),
                file_extensions: vec!["py".to_string()],
            },
            FrameworkPattern {
                framework: "Django".to_string(),
                pattern: Regex::new(r#"(?i)email[_-]?host[_-]?password\s*=\s*["']([^"']+)["']"#)?,
                severity: SecuritySeverity::High,
                description: "Django email password in settings".to_string(),
                file_extensions: vec!["py".to_string()],
            },
        ]);
        
        // Flask patterns
        frameworks.insert("flask".to_string(), vec![
            FrameworkPattern {
                framework: "Flask".to_string(),
                pattern: Regex::new(r#"(?i)app\.config\[['"']([A-Z_]*(?:SECRET|KEY|PASSWORD|TOKEN)[A-Z_]*)['"']\]\s*=\s*["']([^"']+)["']"#)?,
                severity: SecuritySeverity::High,
                description: "Flask configuration with potential secret".to_string(),
                file_extensions: vec!["py".to_string()],
            },
        ]);
        
        // FastAPI patterns
        frameworks.insert("fastapi".to_string(), vec![
            FrameworkPattern {
                framework: "FastAPI".to_string(),
                pattern: Regex::new(r#"(?i)class\s+Settings\([^)]*\):[^}]*([A-Z_]*(?:SECRET|KEY|PASSWORD|TOKEN)[A-Z_]*)\s*:\s*str\s*=\s*["']([^"']+)["']"#)?,
                severity: SecuritySeverity::High,
                description: "FastAPI Settings class with hardcoded secret".to_string(),
                file_extensions: vec!["py".to_string()],
            },
        ]);
        
        Ok(frameworks)
    }
    
    /// Initialize database patterns
    fn initialize_database_patterns() -> Result<Vec<DatabasePattern>, SecurityError> {
        let patterns = vec![
            // PostgreSQL connection strings
            DatabasePattern {
                database_type: "PostgreSQL".to_string(),
                pattern: Regex::new(r#"(?i)postgresql://[^:]+:[^@]+@[^/]+/[^"'\s]+"#)?,
                severity: SecuritySeverity::Critical,
                description: "PostgreSQL connection string with credentials".to_string(),
            },
            
            // MySQL connection strings
            DatabasePattern {
                database_type: "MySQL".to_string(),
                pattern: Regex::new(r#"(?i)mysql://[^:]+:[^@]+@[^/]+/[^"'\s]+"#)?,
                severity: SecuritySeverity::Critical,
                description: "MySQL connection string with credentials".to_string(),
            },
            
            // MongoDB connection strings
            DatabasePattern {
                database_type: "MongoDB".to_string(),
                pattern: Regex::new(r#"(?i)mongodb://[^:]+:[^@]+@[^/]+/[^"'\s]+"#)?,
                severity: SecuritySeverity::Critical,
                description: "MongoDB connection string with credentials".to_string(),
            },
            
            // Redis connection strings
            DatabasePattern {
                database_type: "Redis".to_string(),
                pattern: Regex::new(r#"(?i)redis://[^:]*:[^@]+@[^/]+/[^"'\s]*"#)?,
                severity: SecuritySeverity::High,
                description: "Redis connection string with password".to_string(),
            },
            
            // SQLAlchemy database URLs
            DatabasePattern {
                database_type: "SQLAlchemy".to_string(),
                pattern: Regex::new(r#"(?i)sqlalchemy_database_uri\s*=\s*["'][^"']*://[^:]+:[^@]+@[^"']+"#)?,
                severity: SecuritySeverity::Critical,
                description: "SQLAlchemy database URI with credentials".to_string(),
            },
        ];
        
        Ok(patterns)
    }
    
    /// Initialize environment variable patterns specific to Python
    fn initialize_env_var_patterns() -> Result<Vec<EnvVarPattern>, SecurityError> {
        let patterns = vec![
            EnvVarPattern {
                pattern: Regex::new(r#"os\.environ(?:\.get)?\(['"']([A-Z_]+)['"']\)"#)?,
                severity: SecuritySeverity::Info,
                description: "Environment variable usage detected".to_string(),
                sensitive_prefixes: vec![
                    "SECRET".to_string(),
                    "KEY".to_string(),
                    "PASSWORD".to_string(),
                    "TOKEN".to_string(),
                    "API".to_string(),
                    "AUTH".to_string(),
                    "PRIVATE".to_string(),
                    "CREDENTIAL".to_string(),
                ],
            },
            EnvVarPattern {
                pattern: Regex::new(r#"getenv\(['"']([A-Z_]+)['"']\)"#)?,
                severity: SecuritySeverity::Info,
                description: "Environment variable access via getenv".to_string(),
                sensitive_prefixes: vec![
                    "SECRET".to_string(),
                    "KEY".to_string(),
                    "PASSWORD".to_string(),
                    "TOKEN".to_string(),
                ],
            },
        ];
        
        Ok(patterns)
    }
    
    /// Check environment variable usage patterns
    fn check_env_var_usage(&self, content: &str, file_path: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        for pattern in &self.env_var_patterns {
            for (line_num, line) in content.lines().enumerate() {
                if let Some(captures) = pattern.pattern.captures(line) {
                    if let Some(var_name) = captures.get(1) {
                        let var_name = var_name.as_str();
                        
                        // Check if this appears to be a sensitive variable
                        let is_sensitive = pattern.sensitive_prefixes.iter().any(|prefix| {
                            var_name.to_uppercase().contains(prefix)
                        });
                        
                        if is_sensitive {
                            // Check if this is properly protected (not hardcoded)
                            if !line.contains("=") || line.contains("os.environ") || line.contains("getenv") {
                                // This is good practice - environment variable usage
                                continue;
                            }
                            
                            let column_number = captures.get(0).map(|m| m.start() + 1);
                            
                            findings.push(SecurityFinding {
                                id: format!("env-var-misuse-{}", line_num),
                                title: "Potential Environment Variable Misuse".to_string(),
                                description: format!("Sensitive environment variable '{}' usage detected", var_name),
                                severity: SecuritySeverity::Medium,
                                category: SecurityCategory::SecretsExposure,
                                file_path: Some(file_path.to_path_buf()),
                                line_number: Some(line_num + 1),
                                column_number,
                                evidence: Some(line.trim().to_string()),
                                remediation: vec![
                                    "Ensure sensitive environment variables are properly protected".to_string(),
                                    "Use python-decouple or similar libraries for configuration".to_string(),
                                    "Document required environment variables".to_string(),
                                ],
                                references: vec![
                                    "https://12factor.net/config".to_string(),
                                    "https://docs.python.org/3/library/os.html#os.environ".to_string(),
                                ],
                                cwe_id: Some("CWE-200".to_string()),
                                compliance_frameworks: vec!["SOC2".to_string()],
                            });
                        }
                    }
                }
            }
        }
        
        Ok(findings)
    }
    
    /// Check for insecure Python practices
    fn check_insecure_python_practices(&self, content: &str, file_path: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        // Check for eval() usage
        if let Ok(eval_pattern) = Regex::new(r#"eval\s*\("#) {
            for (line_num, line) in content.lines().enumerate() {
                if eval_pattern.is_match(line) {
                    findings.push(SecurityFinding {
                        id: format!("insecure-eval-{}", line_num),
                        title: "Dangerous eval() Usage".to_string(),
                        description: "Use of eval() function detected - potential code injection risk".to_string(),
                        severity: SecuritySeverity::High,
                        category: SecurityCategory::CodeInjection,
                        file_path: Some(file_path.to_path_buf()),
                        line_number: Some(line_num + 1),
                        column_number: None,
                        evidence: Some(line.trim().to_string()),
                        remediation: vec![
                            "Avoid using eval() with user input".to_string(),
                            "Use ast.literal_eval() for safe evaluation of literals".to_string(),
                            "Consider using json.loads() for JSON data".to_string(),
                        ],
                        references: vec![
                            "https://owasp.org/www-project-top-ten/2021/A03_2021-Injection/".to_string(),
                        ],
                        cwe_id: Some("CWE-95".to_string()),
                        compliance_frameworks: vec!["SOC2".to_string()],
                    });
                }
            }
        }
        
        // Check for shell injection via subprocess
        if let Ok(subprocess_pattern) = Regex::new(r#"subprocess\.(call|run|Popen)\([^)]*shell\s*=\s*True"#) {
            for (line_num, line) in content.lines().enumerate() {
                if subprocess_pattern.is_match(line) {
                    findings.push(SecurityFinding {
                        id: format!("shell-injection-{}", line_num),
                        title: "Potential Shell Injection".to_string(),
                        description: "subprocess call with shell=True detected - potential command injection risk".to_string(),
                        severity: SecuritySeverity::High,
                        category: SecurityCategory::CommandInjection,
                        file_path: Some(file_path.to_path_buf()),
                        line_number: Some(line_num + 1),
                        column_number: None,
                        evidence: Some(line.trim().to_string()),
                        remediation: vec![
                            "Avoid using shell=True with user input".to_string(),
                            "Use subprocess with list arguments instead".to_string(),
                            "Validate and sanitize all user inputs".to_string(),
                        ],
                        references: vec![
                            "https://owasp.org/www-project-top-ten/2021/A03_2021-Injection/".to_string(),
                        ],
                        cwe_id: Some("CWE-78".to_string()),
                        compliance_frameworks: vec!["SOC2".to_string()],
                    });
                }
            }
        }
        
        Ok(findings)
    }
    
    /// Detect Python framework based on content
    fn detect_python_framework(&self, content: &str) -> Option<String> {
        if content.contains("django") || content.contains("Django") {
            Some("django".to_string())
        } else if content.contains("flask") || content.contains("Flask") {
            Some("flask".to_string())
        } else if content.contains("fastapi") || content.contains("FastAPI") {
            Some("fastapi".to_string())
        } else {
            None
        }
    }
    
    /// Mask sensitive information in evidence
    fn mask_secret(&self, secret: &str) -> String {
        if secret.len() <= 8 {
            "*".repeat(secret.len())
        } else {
            format!("{}***{}", &secret[..4], &secret[secret.len()-4..])
        }
    }
    
    /// Mask database connection string
    fn mask_database_connection(&self, connection_str: &str) -> String {
        // Replace password in connection string with asterisks
        if let Ok(re) = Regex::new(r"://([^:]+):([^@]+)@") {
            re.replace(connection_str, "://$1:***@").to_string()
        } else {
            connection_str.to_string()
        }
    }
    
    /// Calculate confidence score based on context indicators
    fn calculate_context_confidence(&self, content: &str, indicators: &[String]) -> f32 {
        let total_indicators = indicators.len() as f32;
        if total_indicators == 0.0 {
            return 0.5; // Neutral confidence
        }
        
        let found_indicators = indicators.iter()
            .filter(|indicator| content.to_lowercase().contains(&indicator.to_lowercase()))
            .count() as f32;
        
        found_indicators / total_indicators
    }
    
    /// Adjust severity based on context confidence
    fn adjust_severity_by_context(&self, base_severity: SecuritySeverity, confidence: f32) -> SecuritySeverity {
        match base_severity {
            SecuritySeverity::Critical => base_severity, // Keep critical as-is
            SecuritySeverity::High => {
                if confidence < 0.3 {
                    SecuritySeverity::Medium
                } else {
                    base_severity
                }
            }
            SecuritySeverity::Medium => {
                if confidence > 0.7 {
                    SecuritySeverity::High
                } else if confidence < 0.3 {
                    SecuritySeverity::Low
                } else {
                    base_severity
                }
            }
            _ => base_severity,
        }
    }
    
    /// Generate framework-specific remediation advice
    fn generate_framework_remediation(&self, framework: &str) -> Vec<String> {
        match framework.to_lowercase().as_str() {
            "django" => vec![
                "Use Django's built-in security features".to_string(),
                "Store SECRET_KEY in environment variables".to_string(),
                "Use django-environ for configuration management".to_string(),
                "Enable Django's security middleware".to_string(),
            ],
            "flask" => vec![
                "Use Flask-Security for authentication".to_string(),
                "Store secrets in environment variables".to_string(),
                "Use Flask-Talisman for security headers".to_string(),
                "Implement proper session management".to_string(),
            ],
            "fastapi" => vec![
                "Use Pydantic Settings for configuration".to_string(),
                "Implement proper JWT token management".to_string(),
                "Use dependency injection for secrets".to_string(),
                "Enable HTTPS and security headers".to_string(),
            ],
            _ => vec![
                "Follow framework-specific security best practices".to_string(),
                "Use environment variables for sensitive data".to_string(),
            ],
        }
    }
    
    /// Enhance a security finding with gitignore risk assessment
    fn enhance_finding_with_gitignore_status(
        &self,
        finding: &mut SecurityFinding,
        gitignore_status: &super::gitignore::GitIgnoreStatus,
    ) {
        // Adjust severity based on gitignore risk
        finding.severity = match gitignore_status.risk_level {
            GitIgnoreRisk::Tracked => SecuritySeverity::Critical, // Always critical if tracked
            GitIgnoreRisk::Exposed => {
                // Upgrade severity if exposed
                match &finding.severity {
                    SecuritySeverity::Medium => SecuritySeverity::High,
                    SecuritySeverity::Low => SecuritySeverity::Medium,
                    other => other.clone(),
                }
            }
            GitIgnoreRisk::Protected => {
                // Downgrade slightly if protected
                match &finding.severity {
                    SecuritySeverity::Critical => SecuritySeverity::High,
                    SecuritySeverity::High => SecuritySeverity::Medium,
                    other => other.clone(),
                }
            }
            GitIgnoreRisk::Safe => finding.severity.clone(),
        };
        
        // Add gitignore context to description
        finding.description.push_str(&format!(" (GitIgnore: {})", gitignore_status.description()));
        
        // Add git history warning for tracked files
        if gitignore_status.risk_level == GitIgnoreRisk::Tracked {
            finding.remediation.insert(0, "⚠️ CRITICAL: Remove this file from git history using git-filter-branch or BFG Repo-Cleaner".to_string());
            finding.remediation.insert(1, "🔑 Rotate any exposed secrets immediately".to_string());
        }
    }
    
    /// Analyze Python configuration files with gitignore awareness
    fn analyze_config_files_with_gitignore(
        &self,
        project_root: &Path,
        gitignore_analyzer: &mut GitIgnoreAnalyzer,
    ) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        // Python configuration files to check
        let config_files = [
            "settings.py",      // Django settings
            "config.py",        // Flask/general config
            "main.py",          // FastAPI main
            "app.py",           // Flask app
            "manage.py",        // Django management
            "wsgi.py",          // WSGI config
            "asgi.py",          // ASGI config
        ];
        
        for config_file in &config_files {
            let config_path = project_root.join(config_file);
            if config_path.exists() {
                let gitignore_status = gitignore_analyzer.analyze_file(&config_path);
                
                if let Ok(content) = fs::read_to_string(&config_path) {
                    // Basic secret pattern check for config files
                    if self.contains_potential_python_secrets(&content) {
                        let mut finding = SecurityFinding {
                            id: format!("config-file-{}", config_file.replace('.', "-")),
                            title: "Potential Secrets in Python Configuration File".to_string(),
                            description: format!("Python configuration file '{}' may contain secrets", config_file),
                            severity: SecuritySeverity::Medium,
                            category: SecurityCategory::SecretsExposure,
                            file_path: Some(config_path.clone()),
                            line_number: None,
                            column_number: None,
                            evidence: None,
                            remediation: vec![
                                "Review configuration file for hardcoded secrets".to_string(),
                                "Use environment variables for sensitive configuration".to_string(),
                                "Consider using python-decouple or similar libraries".to_string(),
                            ],
                            references: vec![
                                "https://12factor.net/config".to_string(),
                            ],
                            cwe_id: Some("CWE-200".to_string()),
                            compliance_frameworks: vec!["SOC2".to_string()],
                        };
                        
                        self.enhance_finding_with_gitignore_status(&mut finding, &gitignore_status);
                        findings.push(finding);
                    }
                }
            }
        }
        
        Ok(findings)
    }
    
    /// Analyze Python dependency files with gitignore awareness
    fn analyze_dependency_files_with_gitignore(
        &self,
        project_root: &Path,
        gitignore_analyzer: &mut GitIgnoreAnalyzer,
    ) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        // Python dependency files to check
        let dependency_files = [
            "requirements.txt",
            "requirements-dev.txt",
            "requirements-prod.txt",
            "Pipfile",
            "Pipfile.lock",
            "pyproject.toml",
            "poetry.lock",
            "conda-requirements.txt",
            "environment.yml",
        ];
        
        for dep_file in &dependency_files {
            let dep_path = project_root.join(dep_file);
            if dep_path.exists() {
                let gitignore_status = gitignore_analyzer.analyze_file(&dep_path);
                
                // Generally, dependency files should be tracked, but check for any embedded secrets
                if let Ok(content) = fs::read_to_string(&dep_path) {
                    if self.contains_potential_python_secrets(&content) {
                        let mut finding = SecurityFinding {
                            id: format!("dependency-file-{}", dep_file.replace('.', "-").replace('-', "_")),
                            title: "Potential Secrets in Python Dependency File".to_string(),
                            description: format!("Python dependency file '{}' may contain secrets", dep_file),
                            severity: SecuritySeverity::High,
                            category: SecurityCategory::SecretsExposure,
                            file_path: Some(dep_path.clone()),
                            line_number: None,
                            column_number: None,
                            evidence: None,
                            remediation: vec![
                                "Remove any secrets from dependency files".to_string(),
                                "Use environment variables for configuration".to_string(),
                                "Review dependency sources for security".to_string(),
                            ],
                            references: vec![
                                "https://pip.pypa.io/en/stable/topics/secure-installs/".to_string(),
                            ],
                            cwe_id: Some("CWE-200".to_string()),
                            compliance_frameworks: vec!["SOC2".to_string()],
                        };
                        
                        self.enhance_finding_with_gitignore_status(&mut finding, &gitignore_status);
                        findings.push(finding);
                    }
                }
            }
        }
        
        Ok(findings)
    }
    
    /// Analyze environment files with comprehensive gitignore risk assessment
    fn analyze_env_files_with_gitignore(
        &self,
        project_root: &Path,
        gitignore_analyzer: &mut GitIgnoreAnalyzer,
    ) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        // Get all potential environment files using gitignore analyzer
        let env_files = gitignore_analyzer.get_files_to_analyze(&[])
            .map_err(|e| SecurityError::Io(e))?
            .into_iter()
            .filter(|file| {
                if let Some(file_name) = file.file_name().and_then(|n| n.to_str()) {
                    // Exclude template/example files from security alerts
                    if self.is_template_file(file_name) {
                        debug!("Skipping template file: {}", file_name);
                        return false;
                    }
                    
                    file_name.starts_with(".env") || 
                    file_name.contains("credentials") || 
                    file_name.contains("secrets") ||
                    file_name.ends_with(".key") ||
                    file_name.ends_with(".pem") ||
                    file_name == "secret.json" ||
                    file_name == "service-account.json"
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();
        
        for env_file in env_files {
            let gitignore_status = gitignore_analyzer.analyze_file(&env_file);
            let relative_path = env_file.strip_prefix(project_root)
                .unwrap_or(&env_file);
            
            // Create finding based on gitignore risk assessment
            let (severity, title, description) = match gitignore_status.risk_level {
                GitIgnoreRisk::Tracked => (
                    SecuritySeverity::Critical,
                    "Python Secret File Tracked by Git".to_string(),
                    format!("Python secret file '{}' is tracked by git and may expose credentials in version history", relative_path.display()),
                ),
                GitIgnoreRisk::Exposed => (
                    SecuritySeverity::High,
                    "Python Secret File Not in GitIgnore".to_string(),
                    format!("Python secret file '{}' exists but is not protected by .gitignore", relative_path.display()),
                ),
                GitIgnoreRisk::Protected => (
                    SecuritySeverity::Info,
                    "Python Secret File Properly Protected".to_string(),
                    format!("Python secret file '{}' is properly ignored but detected for verification", relative_path.display()),
                ),
                GitIgnoreRisk::Safe => continue, // Skip files that appear safe
            };
            
            let mut finding = SecurityFinding {
                id: format!("python-env-file-{}", relative_path.to_string_lossy().replace('/', "-").replace('.', "-")),
                title,
                description,
                severity,
                category: SecurityCategory::SecretsExposure,
                file_path: Some(env_file.clone()),
                line_number: None,
                column_number: None,
                evidence: None,
                remediation: vec![
                    "Ensure sensitive files are in .gitignore".to_string(),
                    "Use .env.example files for documentation".to_string(),
                    "Never commit actual environment files to version control".to_string(),
                    "Use python-decouple for environment variable management".to_string(),
                ],
                references: vec![
                    "https://github.com/motdotla/dotenv#should-i-commit-my-env-file".to_string(),
                    "https://pypi.org/project/python-decouple/".to_string(),
                ],
                cwe_id: Some("CWE-200".to_string()),
                compliance_frameworks: vec!["SOC2".to_string()],
            };
            
            self.enhance_finding_with_gitignore_status(&mut finding, &gitignore_status);
            findings.push(finding);
        }
        
        Ok(findings)
    }
    
    /// Check if a file is a template/example file that should be excluded from security alerts
    fn is_template_file(&self, file_name: &str) -> bool {
        let template_indicators = [
            "sample", "example", "template", "template.env", "env.template",
            "sample.env", "env.sample", "example.env", "env.example",
            "examples", "samples", "templates", "demo", "test", 
            ".env.sample", ".env.example", ".env.template", ".env.demo", ".env.test",
            "example.json", "sample.json", "template.json"
        ];
        
        let file_name_lower = file_name.to_lowercase();
        
        // Check for exact matches or contains patterns
        template_indicators.iter().any(|indicator| {
            file_name_lower == *indicator || 
            file_name_lower.contains(indicator) ||
            file_name_lower.ends_with(indicator)
        })
    }
    
    /// Check if content contains potential Python secrets (basic patterns)
    fn contains_potential_python_secrets(&self, content: &str) -> bool {
        let secret_indicators = [
            "sk_", "pk_live_", "eyJ", "AKIA", "-----BEGIN",
            "client_secret", "api_key", "access_token", "SECRET_KEY",
            "private_key", "secret_key", "bearer", "password",
            "token", "credentials", "auth"
        ];
        
        let content_lower = content.to_lowercase();
        secret_indicators.iter().any(|indicator| content_lower.contains(&indicator.to_lowercase()))
    }
    
    /// Generate Python-specific security recommendations
    fn generate_python_security_recommendations(&self) -> Vec<String> {
        vec![
            "🐍 Python Security Best Practices:".to_string(),
            "   • Use environment variables for all secrets and configuration".to_string(),
            "   • Install python-decouple or python-dotenv for configuration management".to_string(),
            "   • Keep requirements.txt and poetry.lock files up to date".to_string(),
            "   • Use virtual environments to isolate dependencies".to_string(),
            "   • Run 'pip-audit' or 'safety check' to scan for vulnerable packages".to_string(),
            "   • Enable Django's security middleware if using Django".to_string(),
            "   • Use parameterized queries to prevent SQL injection".to_string(),
            "   • Validate and sanitize all user inputs".to_string(),
            "   • Use HTTPS in production environments".to_string(),
            "   • Implement proper error handling and logging".to_string(),
            "   • Consider using tools like bandit for static security analysis".to_string(),
        ]
    }
} 