//! # Security Pattern Management
//! 
//! Centralized management of security patterns for different tools and services.

use std::collections::HashMap;
use regex::Regex;

use super::{SecuritySeverity, SecurityCategory};

/// Manager for organizing security patterns by tool/service
pub struct SecretPatternManager {
    patterns_by_tool: HashMap<String, Vec<ToolPattern>>,
    generic_patterns: Vec<GenericPattern>,
}

/// Tool-specific pattern (e.g., Firebase, Stripe, etc.)
#[derive(Debug, Clone)]
pub struct ToolPattern {
    pub tool_name: String,
    pub pattern_type: String, // e.g., "api_key", "config_object", "token"
    pub pattern: Regex,
    pub severity: SecuritySeverity,
    pub description: String,
    pub public_safe: bool, // Whether this type of key is safe to expose publicly
    pub context_keywords: Vec<String>, // Keywords that increase confidence
    pub false_positive_keywords: Vec<String>, // Keywords that suggest false positive
}

/// Generic patterns that apply across tools
#[derive(Debug, Clone)]
pub struct GenericPattern {
    pub id: String,
    pub name: String,
    pub pattern: Regex,
    pub severity: SecuritySeverity,
    pub category: SecurityCategory,
    pub description: String,
}

impl SecretPatternManager {
    pub fn new() -> Result<Self, regex::Error> {
        let patterns_by_tool = Self::initialize_tool_patterns()?;
        let generic_patterns = Self::initialize_generic_patterns()?;
        
        Ok(Self {
            patterns_by_tool,
            generic_patterns,
        })
    }
    
    /// Initialize patterns for specific tools/services
    fn initialize_tool_patterns() -> Result<HashMap<String, Vec<ToolPattern>>, regex::Error> {
        let mut patterns = HashMap::new();
        
        // Firebase patterns
        patterns.insert("firebase".to_string(), vec![
            ToolPattern {
                tool_name: "Firebase".to_string(),
                pattern_type: "api_key".to_string(),
                pattern: Regex::new(r#"(?i)(?:firebase.*)?apiKey\s*[:=]\s*["']([A-Za-z0-9_-]{39})["']"#)?,
                severity: SecuritySeverity::Medium, // Firebase API keys are safe to expose
                description: "Firebase API key (safe to expose publicly)".to_string(),
                public_safe: true,
                context_keywords: vec!["firebase".to_string(), "initializeApp".to_string(), "getApps".to_string()],
                false_positive_keywords: vec!["example".to_string(), "placeholder".to_string(), "your-api-key".to_string()],
            },
            ToolPattern {
                tool_name: "Firebase".to_string(),
                pattern_type: "service_account".to_string(),
                pattern: Regex::new(r#"(?i)(?:type|client_email|private_key).*firebase.*service_account"#)?,
                severity: SecuritySeverity::Critical,
                description: "Firebase service account credentials (CRITICAL - never expose)".to_string(),
                public_safe: false,
                context_keywords: vec!["service_account".to_string(), "private_key".to_string(), "client_email".to_string()],
                false_positive_keywords: vec![],
            },
        ]);
        
        // Stripe patterns
        patterns.insert("stripe".to_string(), vec![
            ToolPattern {
                tool_name: "Stripe".to_string(),
                pattern_type: "publishable_key".to_string(),
                pattern: Regex::new(r#"pk_(?:test_|live_)[a-zA-Z0-9]{24,}"#)?,
                severity: SecuritySeverity::Low, // Publishable keys are meant to be public
                description: "Stripe publishable key (safe for client-side use)".to_string(),
                public_safe: true,
                context_keywords: vec!["stripe".to_string(), "publishable".to_string()],
                false_positive_keywords: vec![],
            },
            ToolPattern {
                tool_name: "Stripe".to_string(),
                pattern_type: "secret_key".to_string(),
                pattern: Regex::new(r#"sk_(?:test_|live_)[a-zA-Z0-9]{24,}"#)?,
                severity: SecuritySeverity::Critical,
                description: "Stripe secret key (CRITICAL - server-side only)".to_string(),
                public_safe: false,
                context_keywords: vec!["stripe".to_string(), "secret".to_string()],
                false_positive_keywords: vec![],
            },
            ToolPattern {
                tool_name: "Stripe".to_string(),
                pattern_type: "webhook_secret".to_string(),
                pattern: Regex::new(r#"whsec_[a-zA-Z0-9]{32,}"#)?,
                severity: SecuritySeverity::High,
                description: "Stripe webhook endpoint secret".to_string(),
                public_safe: false,
                context_keywords: vec!["webhook".to_string(), "endpoint".to_string()],
                false_positive_keywords: vec![],
            },
        ]);
        
        // Supabase patterns
        patterns.insert("supabase".to_string(), vec![
            ToolPattern {
                tool_name: "Supabase".to_string(),
                pattern_type: "anon_key".to_string(),
                pattern: Regex::new(r#"(?i)supabase.*anon.*["\']eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+["\']"#)?,
                severity: SecuritySeverity::Medium, // Anon keys are meant for client-side
                description: "Supabase anonymous key (safe for client-side use with RLS)".to_string(),
                public_safe: true,
                context_keywords: vec!["supabase".to_string(), "anon".to_string(), "createClient".to_string()],
                false_positive_keywords: vec!["example".to_string(), "placeholder".to_string()],
            },
            ToolPattern {
                tool_name: "Supabase".to_string(),
                pattern_type: "service_role_key".to_string(),
                pattern: Regex::new(r#"(?i)supabase.*service.*role.*["\']eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+["\']"#)?,
                severity: SecuritySeverity::Critical,
                description: "Supabase service role key (CRITICAL - server-side only)".to_string(),
                public_safe: false,
                context_keywords: vec!["service".to_string(), "role".to_string(), "bypass".to_string()],
                false_positive_keywords: vec![],
            },
        ]);
        
        // Clerk patterns
        patterns.insert("clerk".to_string(), vec![
            ToolPattern {
                tool_name: "Clerk".to_string(),
                pattern_type: "publishable_key".to_string(),
                pattern: Regex::new(r#"pk_test_[a-zA-Z0-9_-]{60,}|pk_live_[a-zA-Z0-9_-]{60,}"#)?,
                severity: SecuritySeverity::Low,
                description: "Clerk publishable key (safe for client-side use)".to_string(),
                public_safe: true,
                context_keywords: vec!["clerk".to_string(), "publishable".to_string()],
                false_positive_keywords: vec![],
            },
            ToolPattern {
                tool_name: "Clerk".to_string(),
                pattern_type: "secret_key".to_string(),
                pattern: Regex::new(r#"sk_test_[a-zA-Z0-9_-]{60,}|sk_live_[a-zA-Z0-9_-]{60,}"#)?,
                severity: SecuritySeverity::Critical,
                description: "Clerk secret key (CRITICAL - server-side only)".to_string(),
                public_safe: false,
                context_keywords: vec!["clerk".to_string(), "secret".to_string()],
                false_positive_keywords: vec![],
            },
        ]);
        
        // Auth0 patterns
        patterns.insert("auth0".to_string(), vec![
            ToolPattern {
                tool_name: "Auth0".to_string(),
                pattern_type: "domain".to_string(),
                pattern: Regex::new(r#"[a-zA-Z0-9-]+\.auth0\.com"#)?,
                severity: SecuritySeverity::Low,
                description: "Auth0 domain (safe to expose)".to_string(),
                public_safe: true,
                context_keywords: vec!["auth0".to_string(), "domain".to_string()],
                false_positive_keywords: vec!["example".to_string(), "your-domain".to_string()],
            },
            ToolPattern {
                tool_name: "Auth0".to_string(),
                pattern_type: "client_id".to_string(),
                pattern: Regex::new(r#"(?i)(?:client_?id|clientId)\s*[:=]\s*["']([a-zA-Z0-9]{32})["']"#)?,
                severity: SecuritySeverity::Low,
                description: "Auth0 client ID (safe for client-side use)".to_string(),
                public_safe: true,
                context_keywords: vec!["auth0".to_string(), "client".to_string()],
                false_positive_keywords: vec![],
            },
            ToolPattern {
                tool_name: "Auth0".to_string(),
                pattern_type: "client_secret".to_string(),
                pattern: Regex::new(r#"(?i)(?:client_?secret|clientSecret)\s*[:=]\s*["']([a-zA-Z0-9_-]{64})["']"#)?,
                severity: SecuritySeverity::Critical,
                description: "Auth0 client secret (CRITICAL - server-side only)".to_string(),
                public_safe: false,
                context_keywords: vec!["auth0".to_string(), "secret".to_string()],
                false_positive_keywords: vec![],
            },
        ]);
        
        // AWS patterns
        patterns.insert("aws".to_string(), vec![
            ToolPattern {
                tool_name: "AWS".to_string(),
                pattern_type: "access_key".to_string(),
                // More specific - must be in assignment context
                pattern: Regex::new(r#"(?i)(?:aws[_-]?access[_-]?key|access[_-]?key[_-]?id)\s*[:=]\s*["']?(AKIA[0-9A-Z]{16})["']?"#)?,
                severity: SecuritySeverity::Critical,
                description: "AWS access key ID in assignment (CRITICAL)".to_string(),
                public_safe: false,
                context_keywords: vec!["aws".to_string(), "access".to_string(), "key".to_string()],
                false_positive_keywords: vec!["example".to_string(), "AKIAEXAMPLE".to_string()],
            },
            ToolPattern {
                tool_name: "AWS".to_string(),
                pattern_type: "secret_key".to_string(),
                pattern: Regex::new(r#"(?i)(?:aws[_-]?secret|secret[_-]?access[_-]?key)\s*[:=]\s*["']([A-Za-z0-9/+=]{40})["']"#)?,
                severity: SecuritySeverity::Critical,
                description: "AWS secret access key (CRITICAL)".to_string(),
                public_safe: false,
                context_keywords: vec!["aws".to_string(), "secret".to_string()],
                false_positive_keywords: vec!["example".to_string(), "your_secret".to_string(), "placeholder".to_string()],
            },
        ]);
        
        // OpenAI patterns
        patterns.insert("openai".to_string(), vec![
            ToolPattern {
                tool_name: "OpenAI".to_string(),
                pattern_type: "api_key".to_string(),
                pattern: Regex::new(r#"sk-[A-Za-z0-9]{48}"#)?,
                severity: SecuritySeverity::High,
                description: "OpenAI API key".to_string(),
                public_safe: false,
                context_keywords: vec!["openai".to_string(), "gpt".to_string(), "api".to_string()],
                false_positive_keywords: vec![],
            },
        ]);
        
        // Vercel patterns
        patterns.insert("vercel".to_string(), vec![
            ToolPattern {
                tool_name: "Vercel".to_string(),
                pattern_type: "token".to_string(),
                pattern: Regex::new(r#"(?i)vercel.*token.*["\'][a-zA-Z0-9]{24,}["\']"#)?,
                severity: SecuritySeverity::High,
                description: "Vercel deployment token".to_string(),
                public_safe: false,
                context_keywords: vec!["vercel".to_string(), "deploy".to_string()],
                false_positive_keywords: vec![],
            },
        ]);
        
        // Netlify patterns
        patterns.insert("netlify".to_string(), vec![
            ToolPattern {
                tool_name: "Netlify".to_string(),
                pattern_type: "access_token".to_string(),
                pattern: Regex::new(r#"(?i)netlify.*token.*["\'][a-zA-Z0-9_-]{40,}["\']"#)?,
                severity: SecuritySeverity::High,
                description: "Netlify access token".to_string(),
                public_safe: false,
                context_keywords: vec!["netlify".to_string(), "deploy".to_string()],
                false_positive_keywords: vec![],
            },
        ]);
        
        Ok(patterns)
    }
    
    /// Initialize generic patterns that apply across tools
    fn initialize_generic_patterns() -> Result<Vec<GenericPattern>, regex::Error> {
        let patterns = vec![
            GenericPattern {
                id: "bearer-token".to_string(),
                name: "Bearer Token".to_string(),
                // More specific - exclude template literals and ensure it's a real assignment
                pattern: Regex::new(r#"(?i)(?:authorization|bearer)\s*[:=]\s*["'](?:bearer\s+)?([A-Za-z0-9_-]{32,})["'](?!\s*\$\{)"#)?,
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::SecretsExposure,
                description: "Bearer token in authorization header (excluding templates)".to_string(),
            },
            GenericPattern {
                id: "jwt-token".to_string(),
                name: "JWT Token".to_string(),
                // More specific JWT pattern - must be properly formatted and in assignment context
                pattern: Regex::new(r#"(?i)(?:token|jwt|authorization|bearer)\s*[:=]\s*["']?eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}["']?"#)?,
                severity: SecuritySeverity::Medium,
                category: SecurityCategory::SecretsExposure,
                description: "JSON Web Token detected in assignment".to_string(),
            },
            GenericPattern {
                id: "database-url".to_string(),
                name: "Database Connection URL".to_string(),
                pattern: Regex::new(r#"(?i)(?:mongodb|postgres|mysql)://[^"'\s]+:[^"'\s]+@[^"'\s]+"#)?,
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::SecretsExposure,
                description: "Database connection string with credentials".to_string(),
            },
            GenericPattern {
                id: "private-key".to_string(),
                name: "Private Key".to_string(),
                pattern: Regex::new(r#"-----BEGIN (?:RSA |OPENSSH |PGP )?PRIVATE KEY-----"#)?,
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::SecretsExposure,
                description: "Private key detected".to_string(),
            },
            GenericPattern {
                id: "generic-api-key".to_string(),
                name: "Generic API Key".to_string(),
                // More specific - require longer keys and exclude common false positives
                pattern: Regex::new(r#"(?i)(?:api[_-]?key|apikey)\s*[:=]\s*["']([A-Za-z0-9_-]{32,})["']"#)?,
                severity: SecuritySeverity::High,
                category: SecurityCategory::SecretsExposure,
                description: "Generic API key pattern (32+ characters)".to_string(),
            },
        ];
        
        Ok(patterns)
    }
    
    /// Get patterns for a specific tool
    pub fn get_tool_patterns(&self, tool: &str) -> Option<&Vec<ToolPattern>> {
        self.patterns_by_tool.get(tool)
    }
    
    /// Get all generic patterns
    pub fn get_generic_patterns(&self) -> &Vec<GenericPattern> {
        &self.generic_patterns
    }
    
    /// Get all supported tools
    pub fn get_supported_tools(&self) -> Vec<String> {
        self.patterns_by_tool.keys().cloned().collect()
    }
    
    /// Get patterns for JavaScript/TypeScript frameworks
    pub fn get_js_framework_patterns(&self) -> Vec<&ToolPattern> {
        let js_tools = ["firebase", "stripe", "supabase", "clerk", "auth0", "vercel", "netlify"];
        js_tools.iter()
            .filter_map(|tool| self.patterns_by_tool.get(*tool))
            .flat_map(|patterns| patterns.iter())
            .collect()
    }
}

impl Default for SecretPatternManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize security patterns")
    }
}

impl ToolPattern {
    /// Check if this pattern should be treated as a high-confidence match given the context
    pub fn assess_confidence(&self, file_content: &str, line_content: &str) -> f32 {
        let mut confidence: f32 = 0.5; // Base confidence
        
        // Increase confidence for context keywords
        for keyword in &self.context_keywords {
            if file_content.to_lowercase().contains(&keyword.to_lowercase()) {
                confidence += 0.2;
            }
        }
        
        // Decrease confidence for false positive indicators
        for indicator in &self.false_positive_keywords {
            if line_content.to_lowercase().contains(&indicator.to_lowercase()) {
                confidence -= 0.3;
            }
        }
        
        confidence.clamp(0.0, 1.0)
    }
    
    /// Get severity adjusted for public safety
    pub fn effective_severity(&self) -> SecuritySeverity {
        if self.public_safe {
            match &self.severity {
                SecuritySeverity::Critical => SecuritySeverity::Medium,
                SecuritySeverity::High => SecuritySeverity::Low,
                other => other.clone(),
            }
        } else {
            self.severity.clone()
        }
    }
} 