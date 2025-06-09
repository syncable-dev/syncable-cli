//! # Pattern Engine Module
//! 
//! Ultra-fast multi-pattern matching using Aho-Corasick algorithm and compiled regex sets.

use std::sync::Arc;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use regex::Regex;
use ahash::AHashMap;
use log::debug;

use super::{TurboConfig, SecurityError};
use crate::analyzer::security::{SecuritySeverity, SecurityCategory};

/// A compiled pattern for ultra-fast matching
#[derive(Debug, Clone)]
pub struct CompiledPattern {
    pub id: String,
    pub name: String,
    pub severity: SecuritySeverity,
    pub category: SecurityCategory,
    pub description: String,
    pub remediation: Vec<String>,
    pub references: Vec<String>,
    pub cwe_id: Option<String>,
    pub confidence_boost_keywords: Vec<String>,
    pub false_positive_keywords: Vec<String>,
}

/// Pattern match result
#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub pattern: Arc<CompiledPattern>,
    pub line_number: usize,
    pub column_number: usize,
    pub evidence: String,
    pub confidence: f32,
}

/// High-performance pattern matching engine
pub struct PatternEngine {
    // Multi-pattern matchers
    secret_matcher: AhoCorasick,
    env_var_matcher: AhoCorasick,
    api_key_matcher: AhoCorasick,
    
    // Pattern lookup maps
    secret_patterns: AHashMap<usize, Arc<CompiledPattern>>,
    env_var_patterns: AHashMap<usize, Arc<CompiledPattern>>,
    api_key_patterns: AHashMap<usize, Arc<CompiledPattern>>,
    
    // Specialized matchers for complex patterns
    complex_patterns: Vec<(Regex, Arc<CompiledPattern>)>,
    
    // Performance counters
    total_patterns: usize,
}

impl PatternEngine {
    pub fn new(config: &TurboConfig) -> Result<Self, SecurityError> {
        debug!("Initializing pattern engine with pattern sets: {:?}", config.pattern_sets);
        
        // Load patterns based on configuration
        let (secret_patterns, env_var_patterns, api_key_patterns, complex_patterns) = 
            Self::load_patterns(&config.pattern_sets)?;
        
        // Build Aho-Corasick matchers
        let secret_matcher = Self::build_matcher(&secret_patterns)?;
        let env_var_matcher = Self::build_matcher(&env_var_patterns)?;
        let api_key_matcher = Self::build_matcher(&api_key_patterns)?;
        
        let total_patterns = secret_patterns.len() + env_var_patterns.len() + 
                           api_key_patterns.len() + complex_patterns.len();
        
        debug!("Pattern engine initialized with {} total patterns", total_patterns);
        
        Ok(Self {
            secret_matcher,
            env_var_matcher,
            api_key_matcher,
            secret_patterns: Self::create_pattern_map(secret_patterns),
            env_var_patterns: Self::create_pattern_map(env_var_patterns),
            api_key_patterns: Self::create_pattern_map(api_key_patterns),
            complex_patterns,
            total_patterns,
        })
    }
    
    /// Get total pattern count
    pub fn pattern_count(&self) -> usize {
        self.total_patterns
    }
    
    /// Scan content for all patterns
    pub fn scan_content(&self, content: &str, quick_reject: bool) -> Vec<PatternMatch> {
        // Quick reject using Boyer-Moore substring search
        if quick_reject && !self.quick_contains_secrets(content) {
            return Vec::new();
        }
        
        let mut matches = Vec::new();
        
        // Split content into lines for line number tracking
        let lines: Vec<&str> = content.lines().collect();
        let mut line_offsets = vec![0];
        let mut offset = 0;
        
        for line in &lines {
            offset += line.len() + 1; // +1 for newline
            line_offsets.push(offset);
        }
        
        // Run multi-pattern matchers
        matches.extend(self.run_matcher(&self.secret_matcher, content, &self.secret_patterns, &lines, &line_offsets));
        matches.extend(self.run_matcher(&self.env_var_matcher, content, &self.env_var_patterns, &lines, &line_offsets));
        matches.extend(self.run_matcher(&self.api_key_matcher, content, &self.api_key_patterns, &lines, &line_offsets));
        
        // Run complex patterns (regex-based)
        for (line_num, line) in lines.iter().enumerate() {
            for (regex, pattern) in &self.complex_patterns {
                if let Some(mat) = regex.find(line) {
                    let confidence = self.calculate_confidence(line, content, &pattern);
                    
                    matches.push(PatternMatch {
                        pattern: Arc::clone(pattern),
                        line_number: line_num + 1,
                        column_number: mat.start() + 1,
                        evidence: self.extract_evidence(line, mat.start(), mat.end()),
                        confidence,
                    });
                }
            }
        }
        
        // Intelligent confidence filtering - adaptive threshold based on pattern type
        matches.retain(|m| {
            let threshold = match m.pattern.id.as_str() {
                id if id.contains("aws-access-key") || id.contains("openai-api-key") => 0.3, // High-confidence patterns
                id if id.contains("jwt-token") || id.contains("database-url") => 0.5, // Medium confidence patterns
                id if id.contains("generic") => 0.7, // Generic patterns need higher confidence
                _ => 0.6, // Default threshold
            };
            m.confidence > threshold
        });
        
        matches
    }
    
    /// Quick check if content might contain secrets
    fn quick_contains_secrets(&self, content: &str) -> bool {
        // Common secret indicators (optimized for speed)
        const QUICK_PATTERNS: &[&str] = &[
            "api", "key", "secret", "token", "password", "credential",
            "auth", "private", "-----BEGIN", "sk_", "pk_", "eyJ",
        ];
        
        let content_lower = content.to_lowercase();
        QUICK_PATTERNS.iter().any(|&pattern| content_lower.contains(pattern))
    }
    
    /// Run Aho-Corasick matcher and collect results
    fn run_matcher(
        &self,
        matcher: &AhoCorasick,
        content: &str,
        patterns: &AHashMap<usize, Arc<CompiledPattern>>,
        lines: &[&str],
        line_offsets: &[usize],
    ) -> Vec<PatternMatch> {
        let mut matches = Vec::new();
        
        for mat in matcher.find_iter(content) {
            let pattern_id = mat.pattern().as_usize();
            if let Some(pattern) = patterns.get(&pattern_id) {
                // Find line and column
                let (line_num, col_num) = self.offset_to_line_col(mat.start(), line_offsets);
                let line = lines.get(line_num.saturating_sub(1)).unwrap_or(&"");
                
                let confidence = self.calculate_confidence(line, content, pattern);
                
                matches.push(PatternMatch {
                    pattern: Arc::clone(pattern),
                    line_number: line_num,
                    column_number: col_num,
                    evidence: self.extract_evidence(line, mat.start(), mat.end()),
                    confidence,
                });
            }
        }
        
        matches
    }
    
    /// Convert byte offset to line and column numbers
    fn offset_to_line_col(&self, offset: usize, line_offsets: &[usize]) -> (usize, usize) {
        let line_num = line_offsets.binary_search(&offset)
            .unwrap_or_else(|i| i.saturating_sub(1));
        
        let line_start = line_offsets.get(line_num).copied().unwrap_or(0);
        let col_num = offset - line_start + 1;
        
        (line_num + 1, col_num)
    }
    
    /// Calculate confidence score for a match
    fn calculate_confidence(&self, line: &str, content: &str, pattern: &CompiledPattern) -> f32 {
        let mut confidence: f32 = 0.6;
        
        let line_lower = line.to_lowercase();
        let content_lower = content.to_lowercase();
        
        // Basic false positive detection
        if line_lower.starts_with("//") || line_lower.starts_with("#") || line_lower.contains("example") ||
           line_lower.contains("placeholder") || line_lower.contains("your_") || line_lower.contains("todo") {
            return 0.0; // Skip obvious examples/docs
        }
        
        // Boost confidence for actual assignments
        if line.contains("=") || line.contains(":") {
            confidence += 0.2;
        }
        
        // Check pattern-specific keywords
        for keyword in &pattern.confidence_boost_keywords {
            if content_lower.contains(&keyword.to_lowercase()) {
                confidence += 0.1;
            }
        }
        
        for keyword in &pattern.false_positive_keywords {
            if line_lower.contains(&keyword.to_lowercase()) {
                confidence -= 0.4;
            }
        }
        
        confidence.clamp(0.0, 1.0)
    }
    

    
    /// Extract evidence with context
    fn extract_evidence(&self, line: &str, start: usize, end: usize) -> String {
        // Mask the actual secret value
        let prefix = &line[..start.min(line.len())];
        let suffix = &line[end.min(line.len())..];
        let masked = "*".repeat((end - start).min(20));
        
        format!("{}{}{}", prefix, masked, suffix).trim().to_string()
    }
    
    /// Build Aho-Corasick matcher from patterns
    fn build_matcher(patterns: &[(String, Arc<CompiledPattern>)]) -> Result<AhoCorasick, SecurityError> {
        let strings: Vec<&str> = patterns.iter().map(|(s, _)| s.as_str()).collect();
        
        let matcher = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostFirst)
            .ascii_case_insensitive(true)
            .build(&strings)
            .map_err(|e| SecurityError::PatternEngine(format!("Failed to build matcher: {}", e)))?;
        
        Ok(matcher)
    }
    
    /// Create pattern lookup map
    fn create_pattern_map(patterns: Vec<(String, Arc<CompiledPattern>)>) -> AHashMap<usize, Arc<CompiledPattern>> {
        patterns.into_iter()
            .enumerate()
            .map(|(id, (_, pattern))| (id, pattern))
            .collect()
    }
    
    /// Load patterns based on pattern sets
    fn load_patterns(pattern_sets: &[String]) -> Result<(
        Vec<(String, Arc<CompiledPattern>)>,
        Vec<(String, Arc<CompiledPattern>)>,
        Vec<(String, Arc<CompiledPattern>)>,
        Vec<(Regex, Arc<CompiledPattern>)>,
    ), SecurityError> {
        let mut secret_patterns = Vec::new();
        let mut env_var_patterns = Vec::new();
        let mut api_key_patterns = Vec::new();
        let mut complex_patterns = Vec::new();
        
        // Load default patterns
        if pattern_sets.contains(&"default".to_string()) {
            Self::load_default_patterns(&mut secret_patterns, &mut env_var_patterns, 
                                      &mut api_key_patterns, &mut complex_patterns)?;
        }
        
        // Load additional pattern sets
        for set in pattern_sets {
            match set.as_str() {
                "aws" => Self::load_aws_patterns(&mut api_key_patterns)?,
                "gcp" => Self::load_gcp_patterns(&mut api_key_patterns)?,
                "azure" => Self::load_azure_patterns(&mut api_key_patterns)?,
                "crypto" => Self::load_crypto_patterns(&mut secret_patterns)?,
                _ => {}
            }
        }
        
        Ok((secret_patterns, env_var_patterns, api_key_patterns, complex_patterns))
    }
    
    /// Load default security patterns - focused on ACTUAL secrets, not references
    fn load_default_patterns(
        secret_patterns: &mut Vec<(String, Arc<CompiledPattern>)>,
        env_var_patterns: &mut Vec<(String, Arc<CompiledPattern>)>,
        api_key_patterns: &mut Vec<(String, Arc<CompiledPattern>)>,
        complex_patterns: &mut Vec<(Regex, Arc<CompiledPattern>)>,
    ) -> Result<(), SecurityError> {
        // ONLY detect actual API key values, not variable names
        
        // OpenAI API Keys - actual key format
        api_key_patterns.push((
            "sk-".to_string(),
            Arc::new(CompiledPattern {
                id: "openai-api-key".to_string(),
                name: "OpenAI API Key".to_string(),
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::SecretsExposure,
                description: "OpenAI API key detected".to_string(),
                remediation: vec![
                    "Remove API key from source code".to_string(),
                    "Use environment variables".to_string(),
                ],
                references: vec!["https://platform.openai.com/docs/api-reference".to_string()],
                cwe_id: Some("CWE-798".to_string()),
                confidence_boost_keywords: vec!["openai".to_string(), "gpt".to_string()],
                false_positive_keywords: vec![
                    "sk-xxxxxxxx".to_string(), "sk-...".to_string(), "sk_test".to_string(),
                    "example".to_string(), "placeholder".to_string(), "your_".to_string(),
                    "TODO".to_string(), "FIXME".to_string(), "XXX".to_string(),
                ],
            }),
        ));
        
        // Complex regex patterns for ACTUAL secret assignments with values
        complex_patterns.push((
            // Only match when there's an actual long value, not just variable names
            Regex::new(r#"(?i)(?:api[_-]?key|secret[_-]?key|access[_-]?token)\s*[:=]\s*['"]([a-zA-Z0-9+/=]{32,})['"]"#)
                .map_err(|e| SecurityError::PatternEngine(format!("Regex error: {}", e)))?,
            Arc::new(CompiledPattern {
                id: "long-secret-value".to_string(),
                name: "Hardcoded Secret Value".to_string(),
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::SecretsExposure,
                description: "Long secret value hardcoded in source code".to_string(),
                remediation: vec![
                    "Use environment variables for secrets".to_string(),
                    "Implement proper secret management".to_string(),
                ],
                references: vec![],
                cwe_id: Some("CWE-798".to_string()),
                confidence_boost_keywords: vec!["bearer".to_string(), "auth".to_string()],
                false_positive_keywords: vec![
                    "process.env".to_string(), "getenv".to_string(), "example".to_string(),
                    "placeholder".to_string(), "your_".to_string(), "TODO".to_string(),
                    "test".to_string(), "demo".to_string(), "fake".to_string(),
                ],
            }),
        ));
        
        // JWT tokens (actual token format)
        complex_patterns.push((
            Regex::new(r#"\beyJ[a-zA-Z0-9+/=]{100,}\b"#)
                .map_err(|e| SecurityError::PatternEngine(format!("Regex error: {}", e)))?,
            Arc::new(CompiledPattern {
                id: "jwt-token".to_string(),
                name: "JWT Token".to_string(),
                severity: SecuritySeverity::High,
                category: SecurityCategory::SecretsExposure,
                description: "JWT token detected in source code".to_string(),
                remediation: vec![
                    "Never hardcode JWT tokens".to_string(),
                    "Use secure token storage".to_string(),
                ],
                references: vec![],
                cwe_id: Some("CWE-798".to_string()),
                confidence_boost_keywords: vec!["bearer".to_string(), "authorization".to_string()],
                false_positive_keywords: vec!["example".to_string(), "demo".to_string()],
            }),
        ));
        
        // Database connection strings with embedded credentials
        complex_patterns.push((
            Regex::new(r#"(?i)(?:postgres|mysql|mongodb)://[^:\s]+:[^@\s]+@[^/\s]+/[^\s]*"#)
                .map_err(|e| SecurityError::PatternEngine(format!("Regex error: {}", e)))?,
            Arc::new(CompiledPattern {
                id: "database-url-with-creds".to_string(),
                name: "Database URL with Credentials".to_string(),
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::SecretsExposure,
                description: "Database connection string with embedded credentials".to_string(),
                remediation: vec![
                    "Use environment variables for database credentials".to_string(),
                    "Use connection string without embedded passwords".to_string(),
                ],
                references: vec![],
                cwe_id: Some("CWE-798".to_string()),
                confidence_boost_keywords: vec!["connection".to_string(), "database".to_string()],
                false_positive_keywords: vec![
                    "example.com".to_string(), "localhost".to_string(), "placeholder".to_string(),
                    "your_".to_string(), "user:pass".to_string(),
                ],
            }),
        ));
        
        // Private SSH/SSL keys
        secret_patterns.push((
            "-----BEGIN".to_string(),
            Arc::new(CompiledPattern {
                id: "private-key-header".to_string(),
                name: "Private Key".to_string(),
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::SecretsExposure,
                description: "Private key detected".to_string(),
                remediation: vec![
                    "Never commit private keys to version control".to_string(),
                    "Use secure key storage solutions".to_string(),
                ],
                references: vec![],
                cwe_id: Some("CWE-321".to_string()),
                confidence_boost_keywords: vec!["PRIVATE".to_string(), "RSA".to_string(), "DSA".to_string()],
                false_positive_keywords: vec!["PUBLIC".to_string(), "CERTIFICATE".to_string()],
            }),
        ));
        
        Ok(())
    }
    
    /// Load AWS-specific patterns
    fn load_aws_patterns(api_key_patterns: &mut Vec<(String, Arc<CompiledPattern>)>) -> Result<(), SecurityError> {
        api_key_patterns.push((
            "AKIA".to_string(),
            Arc::new(CompiledPattern {
                id: "aws-access-key".to_string(),
                name: "AWS Access Key".to_string(),
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::SecretsExposure,
                description: "AWS Access Key ID detected".to_string(),
                remediation: vec![
                    "Remove AWS credentials from source code".to_string(),
                    "Use IAM roles or environment variables".to_string(),
                    "Rotate the exposed key immediately".to_string(),
                ],
                references: vec!["https://docs.aws.amazon.com/security/".to_string()],
                cwe_id: Some("CWE-798".to_string()),
                confidence_boost_keywords: vec!["aws".to_string(), "s3".to_string(), "ec2".to_string()],
                false_positive_keywords: vec!["AKIA00000000".to_string()],
            }),
        ));
        
        Ok(())
    }
    
    /// Load GCP-specific patterns
    fn load_gcp_patterns(api_key_patterns: &mut Vec<(String, Arc<CompiledPattern>)>) -> Result<(), SecurityError> {
        api_key_patterns.push((
            "AIza".to_string(),
            Arc::new(CompiledPattern {
                id: "gcp-api-key".to_string(),
                name: "Google Cloud API Key".to_string(),
                severity: SecuritySeverity::High,
                category: SecurityCategory::SecretsExposure,
                description: "Google Cloud API key detected".to_string(),
                remediation: vec![
                    "Use service accounts instead of API keys".to_string(),
                    "Restrict API key usage by IP/referrer".to_string(),
                ],
                references: vec!["https://cloud.google.com/security/".to_string()],
                cwe_id: Some("CWE-798".to_string()),
                confidence_boost_keywords: vec!["google".to_string(), "gcp".to_string(), "firebase".to_string()],
                false_positive_keywords: vec![],
            }),
        ));
        
        Ok(())
    }
    
    /// Load Azure-specific patterns
    fn load_azure_patterns(_api_key_patterns: &mut Vec<(String, Arc<CompiledPattern>)>) -> Result<(), SecurityError> {
        // Azure patterns would go here
        Ok(())
    }
    
    /// Load cryptocurrency-related patterns
    fn load_crypto_patterns(secret_patterns: &mut Vec<(String, Arc<CompiledPattern>)>) -> Result<(), SecurityError> {
        secret_patterns.push((
            "-----BEGIN".to_string(),
            Arc::new(CompiledPattern {
                id: "private-key".to_string(),
                name: "Private Key".to_string(),
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::SecretsExposure,
                description: "Private key detected".to_string(),
                remediation: vec![
                    "Never commit private keys to version control".to_string(),
                    "Use secure key storage solutions".to_string(),
                ],
                references: vec![],
                cwe_id: Some("CWE-321".to_string()),
                confidence_boost_keywords: vec!["RSA".to_string(), "PRIVATE".to_string()],
                false_positive_keywords: vec!["PUBLIC".to_string()],
            }),
        ));
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pattern_engine_creation() {
        let config = TurboConfig::default();
        let engine = PatternEngine::new(&config);
        assert!(engine.is_ok());
        
        let engine = engine.unwrap();
        assert!(engine.pattern_count() > 0);
    }
    
    #[test]
    fn test_pattern_matching() {
        let config = TurboConfig::default();
        let engine = PatternEngine::new(&config).unwrap();
        
        let content = r#"
            const apiKey = "sk-1234567890abcdef";
            password = "super_secret_password";
            process.env.DATABASE_URL
        "#;
        
        let matches = engine.scan_content(content, false);
        assert!(!matches.is_empty());
        
        // Should find API key and password
        assert!(matches.iter().any(|m| m.pattern.id == "openai-api-key"));
        assert!(matches.iter().any(|m| m.pattern.id == "generic-password"));
    }
    
    #[test]
    fn test_quick_reject() {
        let config = TurboConfig::default();
        let engine = PatternEngine::new(&config).unwrap();
        
        let safe_content = "fn main() { println!(\"Hello, world!\"); }";
        let matches = engine.scan_content(safe_content, true);
        assert!(matches.is_empty());
    }
} 