//! # Security Analysis Configuration
//! 
//! Configuration options for customizing security analysis behavior.

use serde::{Deserialize, Serialize};

/// Configuration for security analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAnalysisConfig {
    // General settings
    pub include_low_severity: bool,
    pub include_info_level: bool,
    
    // Analysis scope
    pub check_secrets: bool,
    pub check_code_patterns: bool,
    pub check_infrastructure: bool,
    pub check_compliance: bool,
    
    // Language-specific settings
    pub javascript_enabled: bool,
    pub python_enabled: bool,
    pub rust_enabled: bool,
    
    // Framework-specific settings
    pub frameworks_to_check: Vec<String>,
    
    // File filtering
    pub ignore_patterns: Vec<String>,
    pub include_patterns: Vec<String>,
    
    // Git integration
    pub skip_gitignored_files: bool,
    pub downgrade_gitignored_severity: bool,
    pub check_git_history: bool,
    
    // Environment variable handling
    pub check_env_files: bool,
    pub warn_on_public_env_vars: bool,
    pub sensitive_env_keywords: Vec<String>,
    
    // JavaScript/TypeScript specific
    pub check_package_json: bool,
    pub check_node_modules: bool,
    pub framework_env_prefixes: Vec<String>,
    
    // Output customization
    pub max_findings_per_file: Option<usize>,
    pub deduplicate_findings: bool,
    pub group_by_severity: bool,
    
    // Performance settings
    pub max_file_size_mb: Option<usize>,
    pub parallel_analysis: bool,
    pub analysis_timeout_seconds: Option<u64>,
}

impl Default for SecurityAnalysisConfig {
    fn default() -> Self {
        Self {
            // General settings
            include_low_severity: false,
            include_info_level: false,
            
            // Analysis scope
            check_secrets: true,
            check_code_patterns: true,
            check_infrastructure: true,
            check_compliance: false, // Disabled by default as it requires more setup
            
            // Language-specific settings
            javascript_enabled: true,
            python_enabled: true,
            rust_enabled: true,
            
            // Framework-specific settings
            frameworks_to_check: vec![
                "React".to_string(),
                "Vue".to_string(),
                "Angular".to_string(),
                "Next.js".to_string(),
                "Vite".to_string(),
                "Express".to_string(),
                "Django".to_string(),
                "Spring Boot".to_string(),
            ],
            
            // File filtering - Enhanced patterns to reduce false positives
            ignore_patterns: vec![
                // Dependencies and build artifacts
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "build".to_string(),
                ".next".to_string(),
                "coverage".to_string(),
                "dist".to_string(),
                ".nuxt".to_string(),
                ".output".to_string(),
                ".vercel".to_string(),
                ".netlify".to_string(),
                
                // Minified and bundled files
                "*.min.js".to_string(),
                "*.min.css".to_string(),
                "*.bundle.js".to_string(),
                "*.bundle.css".to_string(),
                "*.chunk.js".to_string(),
                "*.vendor.js".to_string(),
                "*.map".to_string(),
                
                // Lock files and package managers
                "*.lock".to_string(),
                "*.lockb".to_string(),
                "yarn.lock".to_string(),
                "package-lock.json".to_string(),
                "pnpm-lock.yaml".to_string(),
                "bun.lockb".to_string(),
                "cargo.lock".to_string(),
                "go.sum".to_string(),
                "poetry.lock".to_string(),
                "composer.lock".to_string(),
                "gemfile.lock".to_string(),
                
                // Asset files
                "*.jpg".to_string(),
                "*.jpeg".to_string(),
                "*.png".to_string(),
                "*.gif".to_string(),
                "*.bmp".to_string(),
                "*.svg".to_string(),
                "*.ico".to_string(),
                "*.webp".to_string(),
                "*.tiff".to_string(),
                "*.mp3".to_string(),
                "*.mp4".to_string(),
                "*.avi".to_string(),
                "*.mov".to_string(),
                "*.pdf".to_string(),
                "*.ttf".to_string(),
                "*.otf".to_string(),
                "*.woff".to_string(),
                "*.woff2".to_string(),
                "*.eot".to_string(),
                
                // Test and example files
                "*_sample.*".to_string(),
                "*example*".to_string(),
                "*test*".to_string(),
                "*spec*".to_string(),
                "*mock*".to_string(),
                "*fixture*".to_string(),
                "test/*".to_string(),
                "tests/*".to_string(),
                "__test__/*".to_string(),
                "__tests__/*".to_string(),
                "spec/*".to_string(),
                "specs/*".to_string(),
                
                // Documentation
                "*.md".to_string(),
                "*.txt".to_string(),
                "*.rst".to_string(),
                "docs/*".to_string(),
                "documentation/*".to_string(),
                
                // IDE and editor files
                ".vscode/*".to_string(),
                ".idea/*".to_string(),
                ".vs/*".to_string(),
                "*.swp".to_string(),
                "*.swo".to_string(),
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
                
                // TypeScript and generated files
                "*.d.ts".to_string(),
                "*.generated.*".to_string(),
                "*.auto.*".to_string(),
                
                // Framework-specific
                ".angular/*".to_string(),
                ".svelte-kit/*".to_string(),
                "storybook-static/*".to_string(),
            ],
            include_patterns: vec![], // Empty means include all (subject to ignore patterns)
            
            // Git integration
            skip_gitignored_files: true,
            downgrade_gitignored_severity: false,
            check_git_history: false, // Disabled by default for performance
            
            // Environment variable handling
            check_env_files: true,
            warn_on_public_env_vars: true,
            sensitive_env_keywords: vec![
                "SECRET".to_string(),
                "KEY".to_string(),
                "TOKEN".to_string(),
                "PASSWORD".to_string(),
                "PASS".to_string(),
                "AUTH".to_string(),
                "API".to_string(),
                "PRIVATE".to_string(),
                "CREDENTIAL".to_string(),
                "CERT".to_string(),
                "SSL".to_string(),
                "TLS".to_string(),
                "OAUTH".to_string(),
                "CLIENT_SECRET".to_string(),
                "ACCESS_TOKEN".to_string(),
                "REFRESH_TOKEN".to_string(),
                "DATABASE_URL".to_string(),
                "DB_PASS".to_string(),
                "STRIPE_SECRET".to_string(),
                "AWS_SECRET".to_string(),
                "FIREBASE_PRIVATE".to_string(),
            ],
            
            // JavaScript/TypeScript specific
            check_package_json: true,
            check_node_modules: false, // Usually don't want to scan dependencies
            framework_env_prefixes: vec![
                "REACT_APP_".to_string(),
                "NEXT_PUBLIC_".to_string(),
                "VITE_".to_string(),
                "VUE_APP_".to_string(),
                "EXPO_PUBLIC_".to_string(),
                "NUXT_PUBLIC_".to_string(),
                "GATSBY_".to_string(),
                "STORYBOOK_".to_string(),
            ],
            
            // Output customization
            max_findings_per_file: Some(50), // Prevent overwhelming output
            deduplicate_findings: true,
            group_by_severity: true,
            
            // Performance settings
            max_file_size_mb: Some(10), // Skip very large files
            parallel_analysis: true,
            analysis_timeout_seconds: Some(300), // 5 minutes max
        }
    }
}

impl SecurityAnalysisConfig {
    /// Create a configuration optimized for JavaScript/TypeScript projects
    pub fn for_javascript() -> Self {
        let mut config = Self::default();
        config.javascript_enabled = true;
        config.python_enabled = false;
        config.rust_enabled = false;
        config.check_package_json = true;
        config.frameworks_to_check = vec![
            "React".to_string(),
            "Vue".to_string(),
            "Angular".to_string(),
            "Next.js".to_string(),
            "Vite".to_string(),
            "Express".to_string(),
            "Svelte".to_string(),
            "Nuxt".to_string(),
        ];
        config
    }
    
    /// Create a configuration optimized for Python projects
    pub fn for_python() -> Self {
        let mut config = Self::default();
        config.javascript_enabled = false;
        config.python_enabled = true;
        config.rust_enabled = false;
        config.check_package_json = false;
        config.frameworks_to_check = vec![
            "Django".to_string(),
            "Flask".to_string(),
            "FastAPI".to_string(),
            "Tornado".to_string(),
        ];
        config
    }
    
    /// Create a high-security configuration with strict settings
    pub fn high_security() -> Self {
        let mut config = Self::default();
        config.include_low_severity = true;
        config.include_info_level = true;
        config.skip_gitignored_files = false; // Check everything
        config.check_git_history = true;
        config.warn_on_public_env_vars = true;
        config.max_findings_per_file = None; // No limit
        config
    }
    
    /// Create a fast configuration for CI/CD pipelines
    pub fn fast_ci() -> Self {
        let mut config = Self::default();
        config.include_low_severity = false;
        config.include_info_level = false;
        config.check_compliance = false;
        config.check_git_history = false;
        config.parallel_analysis = true;
        config.max_findings_per_file = Some(20); // Limit output
        config.analysis_timeout_seconds = Some(120); // 2 minutes max
        config
    }
    
    /// Check if a file should be analyzed based on patterns
    pub fn should_analyze_file(&self, file_path: &std::path::Path) -> bool {
        let file_path_str = file_path.to_string_lossy();
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        // Check ignore patterns first
        for pattern in &self.ignore_patterns {
            if self.matches_pattern(pattern, &file_path_str, file_name) {
                return false;
            }
        }
        
        // If include patterns are specified, file must match at least one
        if !self.include_patterns.is_empty() {
            return self.include_patterns.iter().any(|pattern| {
                self.matches_pattern(pattern, &file_path_str, file_name)
            });
        }
        
        true
    }
    
    /// Check if a pattern matches a file
    fn matches_pattern(&self, pattern: &str, file_path: &str, file_name: &str) -> bool {
        if pattern.contains('*') {
            // Use glob matching for wildcard patterns
            glob::Pattern::new(pattern)
                .map(|p| p.matches(file_path) || p.matches(file_name))
                .unwrap_or(false)
        } else {
            // Simple string matching
            file_path.contains(pattern) || file_name.contains(pattern)
        }
    }
    
    /// Check if an environment variable name appears sensitive
    pub fn is_sensitive_env_var(&self, var_name: &str) -> bool {
        let var_upper = var_name.to_uppercase();
        self.sensitive_env_keywords.iter()
            .any(|keyword| var_upper.contains(keyword))
    }
    
    /// Check if an environment variable should be public (safe for client-side)
    pub fn is_public_env_var(&self, var_name: &str) -> bool {
        self.framework_env_prefixes.iter()
            .any(|prefix| var_name.starts_with(prefix))
    }
    
    /// Get the maximum file size to analyze in bytes
    pub fn max_file_size_bytes(&self) -> Option<usize> {
        self.max_file_size_mb.map(|mb| mb * 1024 * 1024)
    }
}

/// Preset configurations for common use cases
#[derive(Debug, Clone, Copy)]
pub enum SecurityConfigPreset {
    /// Default balanced configuration
    Default,
    /// Optimized for JavaScript/TypeScript projects
    JavaScript,
    /// Optimized for Python projects
    Python,
    /// High-security configuration with strict settings
    HighSecurity,
    /// Fast configuration for CI/CD pipelines
    FastCI,
}

impl SecurityConfigPreset {
    pub fn to_config(self) -> SecurityAnalysisConfig {
        match self {
            Self::Default => SecurityAnalysisConfig::default(),
            Self::JavaScript => SecurityAnalysisConfig::for_javascript(),
            Self::Python => SecurityAnalysisConfig::for_python(),
            Self::HighSecurity => SecurityAnalysisConfig::high_security(),
            Self::FastCI => SecurityAnalysisConfig::fast_ci(),
        }
    }
}

impl From<SecurityConfigPreset> for SecurityAnalysisConfig {
    fn from(preset: SecurityConfigPreset) -> Self {
        preset.to_config()
    }
} 