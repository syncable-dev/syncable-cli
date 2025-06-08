//! # JavaScript/TypeScript Security Analyzer
//! 
//! Specialized security analyzer for JavaScript and TypeScript applications.
//! 
//! This analyzer focuses on:
//! - Framework-specific secret patterns (React, Vue, Angular, etc.)
//! - Environment variable misuse
//! - Hardcoded API keys in configuration objects
//! - Client-side secret exposure patterns
//! - Common JS/TS anti-patterns

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use regex::Regex;
use log::{debug, info};

use super::{SecurityError, SecurityFinding, SecuritySeverity, SecurityCategory, SecurityReport, SecurityAnalysisConfig, GitIgnoreAnalyzer, GitIgnoreRisk};

/// JavaScript/TypeScript specific security analyzer
pub struct JavaScriptSecurityAnalyzer {
    config: SecurityAnalysisConfig,
    js_patterns: Vec<JavaScriptSecretPattern>,
    framework_patterns: HashMap<String, Vec<FrameworkPattern>>,
    env_var_patterns: Vec<EnvVarPattern>,
    gitignore_analyzer: Option<GitIgnoreAnalyzer>,
}

/// JavaScript-specific secret pattern
#[derive(Debug, Clone)]
pub struct JavaScriptSecretPattern {
    pub id: String,
    pub name: String,
    pub pattern: Regex,
    pub severity: SecuritySeverity,
    pub description: String,
    pub context_indicators: Vec<String>, // Code context that increases confidence
    pub false_positive_indicators: Vec<String>, // Context that suggests false positive
}

/// Framework-specific patterns
#[derive(Debug, Clone)]
pub struct FrameworkPattern {
    pub pattern: Regex,
    pub severity: SecuritySeverity,
    pub description: String,
    pub file_extensions: Vec<String>,
}

/// Environment variable patterns
#[derive(Debug, Clone)]
pub struct EnvVarPattern {
    pub pattern: Regex,
    pub severity: SecuritySeverity,
    pub description: String,
    pub public_prefixes: Vec<String>, // Prefixes that indicate public env vars
}

impl JavaScriptSecurityAnalyzer {
    pub fn new() -> Result<Self, SecurityError> {
        Self::with_config(SecurityAnalysisConfig::default())
    }
    
    pub fn with_config(config: SecurityAnalysisConfig) -> Result<Self, SecurityError> {
        let js_patterns = Self::initialize_js_patterns()?;
        let framework_patterns = Self::initialize_framework_patterns()?;
        let env_var_patterns = Self::initialize_env_var_patterns()?;
        
        Ok(Self {
            config,
            js_patterns,
            framework_patterns,
            env_var_patterns,
            gitignore_analyzer: None, // Will be initialized in analyze_project
        })
    }
    
    /// Analyze a JavaScript/TypeScript project
    pub fn analyze_project(&mut self, project_root: &Path) -> Result<SecurityReport, SecurityError> {
        let mut findings = Vec::new();
        
        // Initialize gitignore analyzer for comprehensive file protection assessment
        let mut gitignore_analyzer = GitIgnoreAnalyzer::new(project_root)
            .map_err(|e| SecurityError::AnalysisFailed(format!("Failed to initialize gitignore analyzer: {}", e)))?;
        
        info!("🔍 Using gitignore-aware security analysis for {}", project_root.display());
        
        // Get JS/TS files using gitignore-aware collection
        let js_extensions = ["js", "jsx", "ts", "tsx", "vue", "svelte"];
        let js_files = gitignore_analyzer.get_files_to_analyze(&js_extensions)
            .map_err(|e| SecurityError::Io(e))?
            .into_iter()
            .filter(|file| {
                if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
                    js_extensions.contains(&ext)
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();
        
        info!("Found {} JavaScript/TypeScript files to analyze (gitignore-filtered)", js_files.len());
        
        // Analyze each file with gitignore context
        for file_path in &js_files {
            let gitignore_status = gitignore_analyzer.analyze_file(file_path);
            let mut file_findings = self.analyze_js_file(file_path)?;
            
            // Enhance findings with gitignore risk assessment
            for finding in &mut file_findings {
                self.enhance_finding_with_gitignore_status(finding, &gitignore_status);
            }
            
            findings.extend(file_findings);
        }
        
        // Analyze package.json and other config files with gitignore awareness
        findings.extend(self.analyze_config_files_with_gitignore(project_root, &mut gitignore_analyzer)?);
        
        // Comprehensive environment file analysis with gitignore risk assessment
        findings.extend(self.analyze_env_files_with_gitignore(project_root, &mut gitignore_analyzer)?);
        
        // Generate gitignore recommendations for any secret files found
        let secret_files: Vec<PathBuf> = findings.iter()
            .filter_map(|f| f.file_path.as_ref())
            .cloned()
            .collect();
        
        let gitignore_recommendations = gitignore_analyzer.generate_gitignore_recommendations(&secret_files);
        
        // Create report with enhanced recommendations
        let mut report = SecurityReport::from_findings(findings);
        report.recommendations.extend(gitignore_recommendations);
        
        Ok(report)
    }
    
    /// Initialize JavaScript-specific secret patterns
    fn initialize_js_patterns() -> Result<Vec<JavaScriptSecretPattern>, SecurityError> {
        let patterns = vec![
            // Firebase config object
            JavaScriptSecretPattern {
                id: "js-firebase-config".to_string(),
                name: "Firebase Configuration Object".to_string(),
                pattern: Regex::new(r#"(?i)(?:const\s+|let\s+|var\s+)?firebaseConfig\s*[=:]\s*\{[^}]*apiKey\s*:\s*["']([^"']+)["'][^}]*\}"#)?,
                severity: SecuritySeverity::Medium,
                description: "Firebase configuration object with API key detected".to_string(),
                context_indicators: vec!["initializeApp".to_string(), "firebase".to_string()],
                false_positive_indicators: vec!["example".to_string(), "placeholder".to_string(), "your-api-key".to_string()],
            },
            
            // Stripe publishable key (less sensitive but should be noted)
            JavaScriptSecretPattern {
                id: "js-stripe-public-key".to_string(),
                name: "Stripe Publishable Key".to_string(),
                pattern: Regex::new(r#"(?i)pk_(?:test_|live_)[a-zA-Z0-9]{24,}"#)?,
                severity: SecuritySeverity::Low,
                description: "Stripe publishable key detected (public but should be environment variable)".to_string(),
                context_indicators: vec!["stripe".to_string(), "payment".to_string()],
                false_positive_indicators: vec![],
            },
            
            // Supabase anon key
            JavaScriptSecretPattern {
                id: "js-supabase-anon-key".to_string(),
                name: "Supabase Anonymous Key".to_string(),
                pattern: Regex::new(r#"(?i)(?:supabase|anon).*?["\']eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+["\']"#)?,
                severity: SecuritySeverity::Medium,
                description: "Supabase anonymous key detected".to_string(),
                context_indicators: vec!["supabase".to_string(), "createClient".to_string()],
                false_positive_indicators: vec!["example".to_string(), "placeholder".to_string()],
            },
            
            // Auth0 configuration
            JavaScriptSecretPattern {
                id: "js-auth0-config".to_string(),
                name: "Auth0 Configuration".to_string(),
                pattern: Regex::new(r#"(?i)(?:domain|clientId)\s*:\s*["']([a-zA-Z0-9.-]+\.auth0\.com|[a-zA-Z0-9]{32})["']"#)?,
                severity: SecuritySeverity::Medium,
                description: "Auth0 configuration detected".to_string(),
                context_indicators: vec!["auth0".to_string(), "webAuth".to_string()],
                false_positive_indicators: vec!["example".to_string(), "your-domain".to_string()],
            },
            
            // Process.env hardcoded values
            JavaScriptSecretPattern {
                id: "js-hardcoded-env".to_string(),
                name: "Hardcoded process.env Assignment".to_string(),
                pattern: Regex::new(r#"process\.env\.[A-Z_]+\s*=\s*["']([^"']+)["']"#)?,
                severity: SecuritySeverity::High,
                description: "Hardcoded assignment to process.env detected".to_string(),
                context_indicators: vec![],
                false_positive_indicators: vec!["development".to_string(), "test".to_string()],
            },
            
            // Clerk keys
            JavaScriptSecretPattern {
                id: "js-clerk-key".to_string(),
                name: "Clerk API Key".to_string(),
                pattern: Regex::new(r#"(?i)(?:clerk|pk_test_|pk_live_)[a-zA-Z0-9_-]{20,}"#)?,
                severity: SecuritySeverity::Medium,
                description: "Clerk API key detected".to_string(),
                context_indicators: vec!["clerk".to_string(), "ClerkProvider".to_string()],
                false_positive_indicators: vec![],
            },
            
            // Generic API key in object assignment
            JavaScriptSecretPattern {
                id: "js-api-key-object".to_string(),
                name: "API Key in Object Assignment".to_string(),
                pattern: Regex::new(r#"(?i)(?:apiKey|api_key|clientSecret|client_secret|accessToken|access_token|secretKey|secret_key)\s*:\s*["']([A-Za-z0-9_-]{20,})["']"#)?,
                severity: SecuritySeverity::High,
                description: "API key or secret assigned in object literal".to_string(),
                context_indicators: vec!["fetch".to_string(), "axios".to_string(), "headers".to_string()],
                false_positive_indicators: vec!["process.env".to_string(), "import.meta.env".to_string(), "placeholder".to_string()],
            },
            
            // Bearer tokens in fetch headers
            JavaScriptSecretPattern {
                id: "js-bearer-token".to_string(),
                name: "Bearer Token in Code".to_string(),
                pattern: Regex::new(r#"(?i)(?:authorization|bearer)\s*:\s*["'](?:bearer\s+)?([A-Za-z0-9_-]{20,})["']"#)?,
                severity: SecuritySeverity::Critical,
                description: "Bearer token hardcoded in authorization header".to_string(),
                context_indicators: vec!["fetch".to_string(), "axios".to_string(), "headers".to_string()],
                false_positive_indicators: vec!["${".to_string(), "process.env".to_string(), "import.meta.env".to_string()],
            },
            
            // Database connection strings
            JavaScriptSecretPattern {
                id: "js-database-url".to_string(),
                name: "Database Connection URL".to_string(),
                pattern: Regex::new(r#"(?i)(?:mongodb|postgres|mysql)://[^"'\s]+:[^"'\s]+@[^"'\s]+"#)?,
                severity: SecuritySeverity::Critical,
                description: "Database connection string with credentials detected".to_string(),
                context_indicators: vec!["connect".to_string(), "mongoose".to_string(), "client".to_string()],
                false_positive_indicators: vec!["localhost".to_string(), "example.com".to_string()],
            },
        ];
        
        Ok(patterns)
    }
    
    /// Initialize framework-specific patterns
    fn initialize_framework_patterns() -> Result<HashMap<String, Vec<FrameworkPattern>>, SecurityError> {
        let mut frameworks = HashMap::new();
        
        // React patterns
        frameworks.insert("react".to_string(), vec![
            FrameworkPattern {
                pattern: Regex::new(r#"(?i)react_app_[a-z_]+\s*=\s*["']([^"']+)["']"#)?,
                severity: SecuritySeverity::Medium,
                description: "React environment variable potentially exposed in build".to_string(),
                file_extensions: vec!["js".to_string(), "jsx".to_string(), "ts".to_string(), "tsx".to_string()],
            },
        ]);
        
        // Next.js patterns
        frameworks.insert("nextjs".to_string(), vec![
            FrameworkPattern {
                pattern: Regex::new(r#"(?i)next_public_[a-z_]+\s*=\s*["']([^"']+)["']"#)?,
                severity: SecuritySeverity::Low,
                description: "Next.js public environment variable (ensure it should be public)".to_string(),
                file_extensions: vec!["js".to_string(), "jsx".to_string(), "ts".to_string(), "tsx".to_string()],
            },
        ]);
        
        // Vite patterns
        frameworks.insert("vite".to_string(), vec![
            FrameworkPattern {
                pattern: Regex::new(r#"(?i)vite_[a-z_]+\s*=\s*["']([^"']+)["']"#)?,
                severity: SecuritySeverity::Medium,
                description: "Vite environment variable potentially exposed in build".to_string(),
                file_extensions: vec!["js".to_string(), "jsx".to_string(), "ts".to_string(), "tsx".to_string(), "vue".to_string()],
            },
        ]);
        
        Ok(frameworks)
    }
    
    /// Initialize environment variable patterns
    fn initialize_env_var_patterns() -> Result<Vec<EnvVarPattern>, SecurityError> {
        let patterns = vec![
            EnvVarPattern {
                pattern: Regex::new(r#"process\.env\.([A-Z_]+)"#)?,
                severity: SecuritySeverity::Info,
                description: "Environment variable usage detected".to_string(),
                public_prefixes: vec![
                    "REACT_APP_".to_string(),
                    "NEXT_PUBLIC_".to_string(),
                    "VITE_".to_string(),
                    "VUE_APP_".to_string(),
                    "EXPO_PUBLIC_".to_string(),
                    "NUXT_PUBLIC_".to_string(),
                ],
            },
            EnvVarPattern {
                pattern: Regex::new(r#"import\.meta\.env\.([A-Z_]+)"#)?,
                severity: SecuritySeverity::Info,
                description: "Vite environment variable usage detected".to_string(),
                public_prefixes: vec!["VITE_".to_string()],
            },
        ];
        
        Ok(patterns)
    }
    
    /// Collect all JavaScript/TypeScript files
    fn collect_js_files(&self, project_root: &Path) -> Result<Vec<PathBuf>, SecurityError> {
        let extensions = ["js", "jsx", "ts", "tsx", "vue", "svelte"];
        let mut files = Vec::new();
        
        fn collect_recursive(dir: &Path, extensions: &[&str], files: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    // Skip common build/dependency directories
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if matches!(dir_name, "node_modules" | ".git" | "build" | "dist" | ".next" | "coverage") {
                            continue;
                        }
                    }
                    collect_recursive(&path, extensions, files)?;
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext) {
                        files.push(path);
                    }
                }
            }
            Ok(())
        }
        
        collect_recursive(project_root, &extensions, &mut files)?;
        Ok(files)
    }
    
    /// Analyze a single JavaScript/TypeScript file
    fn analyze_js_file(&self, file_path: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        let content = fs::read_to_string(file_path)?;
        let mut findings = Vec::new();
        
        // Check against JavaScript-specific patterns
        for pattern in &self.js_patterns {
            findings.extend(self.check_pattern_in_content(&content, pattern, file_path)?);
        }
        
        // Check environment variable usage
        findings.extend(self.check_env_var_usage(&content, file_path)?);
        
        Ok(findings)
    }
    
    /// Check a specific pattern in file content
    fn check_pattern_in_content(
        &self,
        content: &str,
        pattern: &JavaScriptSecretPattern,
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
                        (Some(match_.as_str().to_string()), Some(match_.start() + 1))
                    } else {
                        (Some(line.trim().to_string()), None)
                    }
                } else {
                    // For patterns without capture groups, use the full match
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
                    remediation: self.generate_js_remediation(&pattern.id),
                    references: vec![
                        "https://owasp.org/www-project-top-ten/2021/A05_2021-Security_Misconfiguration/".to_string(),
                        "https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html".to_string(),
                    ],
                    cwe_id: Some("CWE-200".to_string()),
                    compliance_frameworks: vec!["SOC2".to_string(), "GDPR".to_string()],
                });
            }
        }
        
        Ok(findings)
    }
    
    /// Check environment variable usage patterns with context-aware detection
    fn check_env_var_usage(&self, content: &str, file_path: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        // Determine if this is likely server-side or client-side code
        let is_server_side = self.is_server_side_file(file_path, content);
        
        for pattern in &self.env_var_patterns {
            for (line_num, line) in content.lines().enumerate() {
                if let Some(captures) = pattern.pattern.captures(line) {
                    if let Some(var_name) = captures.get(1) {
                        let var_name = var_name.as_str();
                        
                        // Check if this is a public environment variable
                        let is_public = pattern.public_prefixes.iter().any(|prefix| var_name.starts_with(prefix));
                        
                        // Context-aware detection: Only flag as problematic if:
                        // 1. It's a sensitive variable AND
                        // 2. It's in client-side code AND 
                        // 3. It doesn't have a public prefix
                        if !is_public && self.is_sensitive_var_name(var_name) && !is_server_side {
                            // Extract column position from the pattern match
                            let column_number = captures.get(0)
                                .map(|m| m.start() + 1);
                            
                            findings.push(SecurityFinding {
                                id: format!("js-env-sensitive-{}", line_num),
                                title: "Sensitive Environment Variable in Client Code".to_string(),
                                description: format!("Environment variable '{}' appears sensitive and may be exposed to client in browser code", var_name),
                                severity: SecuritySeverity::High,
                                category: SecurityCategory::SecretsExposure,
                                file_path: Some(file_path.to_path_buf()),
                                line_number: Some(line_num + 1),
                                column_number,
                                evidence: Some(line.trim().to_string()),
                                remediation: vec![
                                    "Move sensitive environment variables to server-side code".to_string(),
                                    "Use public environment variable prefixes only for non-sensitive data".to_string(),
                                    "Consider using a backend API endpoint to handle sensitive operations".to_string(),
                                ],
                                references: vec![
                                    "https://nextjs.org/docs/basic-features/environment-variables".to_string(),
                                    "https://vitejs.dev/guide/env-and-mode.html".to_string(),
                                ],
                                cwe_id: Some("CWE-200".to_string()),
                                compliance_frameworks: vec!["SOC2".to_string()],
                            });
                        }
                        // For server-side code using environment variables, this is GOOD practice - don't flag it
                    }
                }
            }
        }
        
        Ok(findings)
    }
    
    /// Analyze configuration files (package.json, etc.)
    fn analyze_config_files(&self, project_root: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        // Check package.json for exposed scripts or configs
        let package_json = project_root.join("package.json");
        if package_json.exists() {
            findings.extend(self.analyze_package_json(&package_json)?);
        }
        
        Ok(findings)
    }
    
    /// Analyze package.json for security issues
    fn analyze_package_json(&self, package_json: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        let content = fs::read_to_string(package_json)?;
        
        // Look for hardcoded secrets in scripts or config
        if content.contains("REACT_APP_") || content.contains("NEXT_PUBLIC_") || content.contains("VITE_") {
            for (line_num, line) in content.lines().enumerate() {
                if line.contains("sk_") || line.contains("pk_live_") || line.contains("eyJ") {
                    findings.push(SecurityFinding {
                        id: format!("package-json-secret-{}", line_num),
                        title: "Potential Secret in package.json".to_string(),
                        description: "Potential API key or token found in package.json".to_string(),
                        severity: SecuritySeverity::High,
                        category: SecurityCategory::SecretsExposure,
                        file_path: Some(package_json.to_path_buf()),
                        line_number: Some(line_num + 1),
                        column_number: None,
                        evidence: Some(line.trim().to_string()),
                        remediation: vec![
                            "Remove secrets from package.json".to_string(),
                            "Use environment variables instead".to_string(),
                            "Add package.json to .gitignore if it contains secrets (not recommended)".to_string(),
                        ],
                        references: vec![
                            "https://docs.npmjs.com/cli/v8/configuring-npm/package-json".to_string(),
                        ],
                        cwe_id: Some("CWE-200".to_string()),
                        compliance_frameworks: vec!["SOC2".to_string()],
                    });
                }
            }
        }
        
        Ok(findings)
    }
    
    /// Analyze environment files
    fn analyze_env_files(&self, project_root: &Path) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        // Check for .env files that might be accidentally committed
        let env_files = [".env", ".env.local", ".env.production", ".env.development"];
        
        for env_file in &env_files {
            // Skip template/example files
            if self.is_template_file(env_file) {
                debug!("Skipping template env file: {}", env_file);
                continue;
            }
            
            let env_path = project_root.join(env_file);
            if env_path.exists() {
                // Check if this file should be tracked by git
                findings.push(SecurityFinding {
                    id: format!("env-file-{}", env_file.replace('.', "-")),
                    title: "Environment File Detected".to_string(),
                    description: format!("Environment file '{}' found - ensure it's properly protected", env_file),
                    severity: SecuritySeverity::Medium,
                    category: SecurityCategory::SecretsExposure,
                    file_path: Some(env_path),
                    line_number: None,
                    column_number: None,
                    evidence: None,
                    remediation: vec![
                        "Ensure environment files are in .gitignore".to_string(),
                        "Use .env.example files for documentation".to_string(),
                        "Never commit actual environment files to version control".to_string(),
                    ],
                    references: vec![
                        "https://github.com/motdotla/dotenv#should-i-commit-my-env-file".to_string(),
                    ],
                    cwe_id: Some("CWE-200".to_string()),
                    compliance_frameworks: vec!["SOC2".to_string()],
                });
            }
        }
        
        Ok(findings)
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
    
    /// Check if a variable name appears sensitive
    fn is_sensitive_var_name(&self, var_name: &str) -> bool {
        let sensitive_keywords = [
            "SECRET", "KEY", "TOKEN", "PASSWORD", "PASS", "AUTH", "API",
            "PRIVATE", "CREDENTIAL", "CERT", "SSL", "TLS", "OAUTH",
            "CLIENT_SECRET", "ACCESS_TOKEN", "REFRESH_TOKEN",
        ];
        
        let var_upper = var_name.to_uppercase();
        sensitive_keywords.iter().any(|keyword| var_upper.contains(keyword))
    }
    
    /// Determine if a JavaScript file is likely server-side or client-side
    fn is_server_side_file(&self, file_path: &Path, content: &str) -> bool {
        // Check file path indicators
        let path_str = file_path.to_string_lossy().to_lowercase();
        let server_path_indicators = [
            "/server/", "/backend/", "/api/", "/routes/", "/controllers/",
            "/middleware/", "/models/", "/services/", "/utils/", "/lib/",
            "server.js", "server.ts", "index.js", "index.ts", "app.js", "app.ts",
            "/pages/api/", "/app/api/", // Next.js API routes
            "server-side", "backend", "node_modules", // Clear server indicators
        ];
        
        let client_path_indicators = [
            "/client/", "/frontend/", "/public/", "/static/", "/assets/",
            "/components/", "/views/", "/pages/", "/src/components/",
            "client.js", "client.ts", "main.js", "main.ts", "app.tsx", "index.html",
        ];
        
        // Strong server-side path indicators
        if server_path_indicators.iter().any(|indicator| path_str.contains(indicator)) {
            return true;
        }
        
        // Strong client-side path indicators
        if client_path_indicators.iter().any(|indicator| path_str.contains(indicator)) {
            return false;
        }
        
        // Check content for server-side indicators
        let server_content_indicators = [
            "require(", "module.exports", "exports.", "__dirname", "__filename",
            "process.env", "process.exit", "process.argv", "fs.readFile", "fs.writeFile",
            "http.createServer", "express(", "app.listen", "app.use", "app.get", "app.post",
            "import express", "import fs", "import path", "import http", "import https",
            "cors(", "bodyParser", "middleware", "mongoose.connect", "sequelize",
            "jwt.sign", "bcrypt", "crypto.createHash", "nodemailer", "socket.io",
            "console.log", // While not exclusive, very common in server code
        ];
        
        let client_content_indicators = [
            "document.", "window.", "navigator.", "localStorage", "sessionStorage",
            "addEventListener", "querySelector", "getElementById", "fetch(",
            "XMLHttpRequest", "React.", "ReactDOM", "useState", "useEffect",
            "Vue.", "Angular", "svelte", "alert(", "confirm(", "prompt(",
            "location.href", "history.push", "router.push", "browser",
        ];
        
        let server_matches = server_content_indicators.iter()
            .filter(|&indicator| content.contains(indicator))
            .count();
            
        let client_matches = client_content_indicators.iter()
            .filter(|&indicator| content.contains(indicator))
            .count();
        
        // If we have server indicators and no clear client indicators, assume server-side
        if server_matches > 0 && client_matches == 0 {
            return true;
        }
        
        // If we have client indicators and no server indicators, assume client-side
        if client_matches > 0 && server_matches == 0 {
            return false;
        }
        
        // If mixed or unclear, use a heuristic
        if server_matches > client_matches {
            return true;
        }
        
        // Default to client-side for mixed/unclear files (safer for security)
        false
    }
    
    /// Generate JavaScript-specific remediation advice
    fn generate_js_remediation(&self, pattern_id: &str) -> Vec<String> {
        match pattern_id {
            id if id.contains("firebase") => vec![
                "Move Firebase configuration to environment variables".to_string(),
                "Use Firebase App Check for additional security".to_string(),
                "Implement proper Firebase security rules".to_string(),
            ],
            id if id.contains("stripe") => vec![
                "Use environment variables for Stripe keys".to_string(),
                "Ensure you're using publishable keys in client-side code".to_string(),
                "Keep secret keys on the server side only".to_string(),
            ],
            id if id.contains("bearer") => vec![
                "Never hardcode bearer tokens in client-side code".to_string(),
                "Use secure token storage mechanisms".to_string(),
                "Implement token refresh flows".to_string(),
            ],
            _ => vec![
                "Move secrets to environment variables".to_string(),
                "Use server-side API routes for sensitive operations".to_string(),
                "Implement proper secret management practices".to_string(),
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
        
        // Add gitignore-specific remediation
        let gitignore_action = gitignore_status.recommended_action();
        if gitignore_action != "No action needed" {
            finding.remediation.insert(0, format!("🔒 GitIgnore: {}", gitignore_action));
        }
        
        // Add git history warning for tracked files
        if gitignore_status.risk_level == GitIgnoreRisk::Tracked {
            finding.remediation.insert(1, "⚠️ CRITICAL: Remove this file from git history using git-filter-branch or BFG Repo-Cleaner".to_string());
            finding.remediation.insert(2, "🔑 Rotate any exposed secrets immediately".to_string());
        }
    }
    
    /// Analyze configuration files with gitignore awareness
    fn analyze_config_files_with_gitignore(
        &self,
        project_root: &Path,
        gitignore_analyzer: &mut GitIgnoreAnalyzer,
    ) -> Result<Vec<SecurityFinding>, SecurityError> {
        let mut findings = Vec::new();
        
        // Check package.json with gitignore assessment
        let package_json = project_root.join("package.json");
        if package_json.exists() {
            let gitignore_status = gitignore_analyzer.analyze_file(&package_json);
            let mut package_findings = self.analyze_package_json(&package_json)?;
            
            // Enhance findings with gitignore context
            for finding in &mut package_findings {
                self.enhance_finding_with_gitignore_status(finding, &gitignore_status);
            }
            
            findings.extend(package_findings);
        }
        
        // Check other common config files
        let config_files = [
            "tsconfig.json",
            "vite.config.js",
            "vite.config.ts", 
            "next.config.js",
            "next.config.ts",
            "nuxt.config.js",
            "nuxt.config.ts",
            // Note: .env.example is now excluded as it's a template file
        ];
        
        for config_file in &config_files {
            // Skip template/example files
            if self.is_template_file(config_file) {
                debug!("Skipping template config file: {}", config_file);
                continue;
            }
            
            let config_path = project_root.join(config_file);
            if config_path.exists() {
                let gitignore_status = gitignore_analyzer.analyze_file(&config_path);
                
                // Only analyze if file contains potential secrets or is not properly protected
                if gitignore_status.should_be_ignored || !gitignore_status.is_ignored {
                    if let Ok(content) = fs::read_to_string(&config_path) {
                        // Basic secret pattern check for config files
                        if self.contains_potential_secrets(&content) {
                            let mut finding = SecurityFinding {
                                id: format!("config-file-{}", config_file.replace('.', "-")),
                                title: "Potential Secrets in Configuration File".to_string(),
                                description: format!("Configuration file '{}' may contain secrets", config_file),
                                severity: SecuritySeverity::Medium,
                                category: SecurityCategory::SecretsExposure,
                                file_path: Some(config_path.clone()),
                                line_number: None,
                                column_number: None,
                                evidence: None,
                                remediation: vec![
                                    "Review configuration file for hardcoded secrets".to_string(),
                                    "Use environment variables for sensitive configuration".to_string(),
                                ],
                                references: vec![],
                                cwe_id: Some("CWE-200".to_string()),
                                compliance_frameworks: vec!["SOC2".to_string()],
                            };
                            
                            self.enhance_finding_with_gitignore_status(&mut finding, &gitignore_status);
                            findings.push(finding);
                        }
                    }
                }
            }
        }
        
        Ok(findings)
    }
    
    /// Check if a file is a template/example file that should be excluded from security alerts
    fn is_template_file(&self, file_name: &str) -> bool {
        let template_indicators = [
            "sample", "example", "template", "template.env", "env.template",
            "sample.env", "env.sample", "example.env", "env.example",
            "examples", "samples", "templates", "demo", "test", 
            ".env.sample", ".env.example", ".env.template", ".env.demo", ".env.test"
        ];
        
        let file_name_lower = file_name.to_lowercase();
        
        // Check for exact matches or contains patterns
        template_indicators.iter().any(|indicator| {
            file_name_lower == *indicator || 
            file_name_lower.contains(indicator) ||
            file_name_lower.ends_with(indicator)
        })
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
                    file_name.contains("config") ||
                    file_name.ends_with(".key") ||
                    file_name.ends_with(".pem")
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
                    "Secret File Tracked by Git".to_string(),
                    format!("Secret file '{}' is tracked by git and may expose credentials in version history", relative_path.display()),
                ),
                GitIgnoreRisk::Exposed => (
                    SecuritySeverity::High,
                    "Secret File Not in GitIgnore".to_string(),
                    format!("Secret file '{}' exists but is not protected by .gitignore", relative_path.display()),
                ),
                GitIgnoreRisk::Protected => (
                    SecuritySeverity::Info,
                    "Secret File Properly Protected".to_string(),
                    format!("Secret file '{}' is properly ignored but detected for verification", relative_path.display()),
                ),
                GitIgnoreRisk::Safe => continue, // Skip files that appear safe
            };
            
            let mut finding = SecurityFinding {
                id: format!("env-file-{}", relative_path.to_string_lossy().replace('/', "-").replace('.', "-")),
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
                ],
                references: vec![
                    "https://github.com/motdotla/dotenv#should-i-commit-my-env-file".to_string(),
                ],
                cwe_id: Some("CWE-200".to_string()),
                compliance_frameworks: vec!["SOC2".to_string()],
            };
            
            self.enhance_finding_with_gitignore_status(&mut finding, &gitignore_status);
            findings.push(finding);
        }
        
        Ok(findings)
    }
    
    /// Check if content contains potential secrets (basic patterns)
    fn contains_potential_secrets(&self, content: &str) -> bool {
        let secret_indicators = [
            "sk_", "pk_live_", "eyJ", "AKIA", "-----BEGIN",
            "client_secret", "api_key", "access_token",
            "private_key", "secret_key", "bearer",
        ];
        
        let content_lower = content.to_lowercase();
        secret_indicators.iter().any(|indicator| content_lower.contains(&indicator.to_lowercase()))
    }
}

impl SecurityReport {
    /// Create a security report from a list of findings
    pub fn from_findings(findings: Vec<SecurityFinding>) -> Self {
        let total_findings = findings.len();
        let mut findings_by_severity = HashMap::new();
        let mut findings_by_category = HashMap::new();
        
        for finding in &findings {
            *findings_by_severity.entry(finding.severity.clone()).or_insert(0) += 1;
            *findings_by_category.entry(finding.category.clone()).or_insert(0) += 1;
        }
        
        // Calculate overall score (simple implementation)
        let score_penalty = findings.iter().map(|f| match f.severity {
            SecuritySeverity::Critical => 25.0,
            SecuritySeverity::High => 15.0,
            SecuritySeverity::Medium => 8.0,
            SecuritySeverity::Low => 3.0,
            SecuritySeverity::Info => 1.0,
        }).sum::<f32>();
        
        let overall_score = (100.0 - score_penalty).max(0.0);
        
        // Determine risk level
        let risk_level = if findings.iter().any(|f| f.severity == SecuritySeverity::Critical) {
            SecuritySeverity::Critical
        } else if findings.iter().any(|f| f.severity == SecuritySeverity::High) {
            SecuritySeverity::High
        } else if findings.iter().any(|f| f.severity == SecuritySeverity::Medium) {
            SecuritySeverity::Medium
        } else if !findings.is_empty() {
            SecuritySeverity::Low
        } else {
            SecuritySeverity::Info
        };
        
        Self {
            analyzed_at: chrono::Utc::now(),
            overall_score,
            risk_level,
            total_findings,
            findings_by_severity,
            findings_by_category,
            findings,
            recommendations: vec![
                "Review all detected secrets and move them to environment variables".to_string(),
                "Implement proper secret management practices".to_string(),
                "Use framework-specific environment variable patterns correctly".to_string(),
            ],
            compliance_status: HashMap::new(),
        }
    }
} 