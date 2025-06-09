//! # Results Module
//! 
//! Aggregation and processing of security scan results.

use std::collections::HashMap;
use std::time::Duration;

use ahash::AHashMap;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use crate::analyzer::security::{SecurityFinding, SecuritySeverity, SecurityCategory};
use super::SecurityError;

/// Security analysis report with comprehensive metrics
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityReport {
    pub analyzed_at: DateTime<Utc>,
    pub scan_duration: Duration,
    pub overall_score: f32,
    pub risk_level: SecuritySeverity,
    pub total_findings: usize,
    pub files_scanned: usize,
    pub findings_by_severity: HashMap<SecuritySeverity, usize>,
    pub findings_by_category: HashMap<SecurityCategory, usize>,
    pub findings: Vec<SecurityFinding>,
    pub recommendations: Vec<String>,
    pub performance_metrics: PerformanceMetrics,
}

/// Performance metrics for the scan
#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_duration: Duration,
    pub file_discovery_time: Duration,
    pub pattern_matching_time: Duration,
    pub files_per_second: f64,
    pub cache_hit_rate: f64,
    pub memory_usage_mb: f64,
}

/// Result aggregator for combining and processing findings
pub struct ResultAggregator;

impl ResultAggregator {
    /// Aggregate findings into a comprehensive report
    pub fn aggregate(mut findings: Vec<SecurityFinding>, scan_duration: Duration) -> SecurityReport {
        // Deduplicate findings
        findings = Self::deduplicate_findings(findings);
        
        // Sort by severity (critical first)
        findings.sort_by_key(|f| std::cmp::Reverse(severity_to_number(&f.severity)));
        
        // Calculate metrics
        let total_findings = findings.len();
        let findings_by_severity = Self::count_by_severity(&findings);
        let findings_by_category = Self::count_by_category(&findings);
        let overall_score = Self::calculate_security_score(&findings);
        let risk_level = Self::determine_risk_level(&findings);
        
        // Generate recommendations
        let recommendations = Self::generate_recommendations(&findings);
        
        // Create performance metrics (placeholder values for now)
        let performance_metrics = PerformanceMetrics {
            total_duration: scan_duration,
            file_discovery_time: Duration::from_millis(0), // TODO: Track actual time
            pattern_matching_time: Duration::from_millis(0), // TODO: Track actual time
            files_per_second: 0.0, // TODO: Calculate actual rate
            cache_hit_rate: 0.0, // TODO: Get from cache stats
            memory_usage_mb: 0.0, // TODO: Track memory usage
        };
        
        SecurityReport {
            analyzed_at: Utc::now(),
            scan_duration,
            overall_score,
            risk_level,
            total_findings,
            files_scanned: 0, // TODO: Track actual count
            findings_by_severity,
            findings_by_category,
            findings,
            recommendations,
            performance_metrics,
        }
    }
    
    /// Create an empty report
    pub fn empty() -> SecurityReport {
        SecurityReport {
            analyzed_at: Utc::now(),
            scan_duration: Duration::from_secs(0),
            overall_score: 100.0,
            risk_level: SecuritySeverity::Info,
            total_findings: 0,
            files_scanned: 0,
            findings_by_severity: HashMap::new(),
            findings_by_category: HashMap::new(),
            findings: Vec::new(),
            recommendations: vec!["No security issues detected.".to_string()],
            performance_metrics: PerformanceMetrics {
                total_duration: Duration::from_secs(0),
                file_discovery_time: Duration::from_secs(0),
                pattern_matching_time: Duration::from_secs(0),
                files_per_second: 0.0,
                cache_hit_rate: 0.0,
                memory_usage_mb: 0.0,
            },
        }
    }
    
    /// Deduplicate findings based on content similarity
    fn deduplicate_findings(findings: Vec<SecurityFinding>) -> Vec<SecurityFinding> {
        let mut seen: AHashMap<String, SecurityFinding> = AHashMap::new();
        
        for finding in findings {
            // Create a deduplication key
            let key = format!(
                "{}-{}-{}-{}",
                finding.id,
                finding.file_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
                finding.line_number.unwrap_or(0),
                finding.title
            );
            
            // Keep the finding with the highest severity
            match seen.get(&key) {
                Some(existing) if severity_to_number(&existing.severity) >= severity_to_number(&finding.severity) => {
                    // Keep existing
                }
                _ => {
                    seen.insert(key, finding);
                }
            }
        }
        
        seen.into_values().collect()
    }
    
    /// Count findings by severity
    fn count_by_severity(findings: &[SecurityFinding]) -> HashMap<SecuritySeverity, usize> {
        let mut counts = HashMap::new();
        for finding in findings {
            *counts.entry(finding.severity.clone()).or_insert(0) += 1;
        }
        counts
    }
    
    /// Count findings by category
    fn count_by_category(findings: &[SecurityFinding]) -> HashMap<SecurityCategory, usize> {
        let mut counts = HashMap::new();
        for finding in findings {
            *counts.entry(finding.category.clone()).or_insert(0) += 1;
        }
        counts
    }
    
    /// Calculate overall security score (0-100)
    fn calculate_security_score(findings: &[SecurityFinding]) -> f32 {
        if findings.is_empty() {
            return 100.0;
        }
        
        let total_penalty: f32 = findings.iter().map(|f| match f.severity {
            SecuritySeverity::Critical => 25.0,
            SecuritySeverity::High => 15.0,
            SecuritySeverity::Medium => 8.0,
            SecuritySeverity::Low => 3.0,
            SecuritySeverity::Info => 1.0,
        }).sum();
        
        (100.0 - total_penalty).max(0.0)
    }
    
    /// Determine overall risk level
    fn determine_risk_level(findings: &[SecurityFinding]) -> SecuritySeverity {
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
    
    /// Generate recommendations based on findings
    fn generate_recommendations(findings: &[SecurityFinding]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Check for unprotected secrets
        if findings.iter().any(|f| f.category == SecurityCategory::SecretsExposure && !f.file_path.as_ref().map(|p| p.to_string_lossy().contains(".gitignore")).unwrap_or(false)) {
            recommendations.push("🔐 Implement comprehensive secret management:".to_string());
            recommendations.push("   • Add sensitive files to .gitignore immediately".to_string());
            recommendations.push("   • Use environment variables for all secrets".to_string());
            recommendations.push("   • Consider using a secure vault service (e.g., HashiCorp Vault)".to_string());
        }
        
        // Check for critical findings
        let critical_count = findings.iter().filter(|f| f.severity == SecuritySeverity::Critical).count();
        if critical_count > 0 {
            recommendations.push(format!("🚨 Address {} CRITICAL security issues immediately", critical_count));
            recommendations.push("   • Review and rotate any exposed credentials".to_string());
            recommendations.push("   • Check git history for committed secrets".to_string());
        }
        
        // Framework-specific recommendations
        if findings.iter().any(|f| f.description.contains("React") || f.description.contains("Next.js")) {
            recommendations.push("⚛️ React/Next.js Security:".to_string());
            recommendations.push("   • Use NEXT_PUBLIC_ prefix only for truly public values".to_string());
            recommendations.push("   • Keep sensitive API keys server-side only".to_string());
        }
        
        // Database security
        if findings.iter().any(|f| f.title.contains("Database") || f.title.contains("SQL")) {
            recommendations.push("🗄️ Database Security:".to_string());
            recommendations.push("   • Use connection pooling with encrypted credentials".to_string());
            recommendations.push("   • Implement least-privilege database access".to_string());
            recommendations.push("   • Enable SSL/TLS for database connections".to_string());
        }
        
        // General best practices
        recommendations.push("\n📋 General Security Best Practices:".to_string());
        recommendations.push("   • Enable automated security scanning in CI/CD".to_string());
        recommendations.push("   • Regularly update dependencies".to_string());
        recommendations.push("   • Implement security headers".to_string());
        recommendations.push("   • Use HTTPS everywhere".to_string());
        
        recommendations
    }
}

/// Convert severity to numeric value for sorting
fn severity_to_number(severity: &SecuritySeverity) -> u8 {
    match severity {
        SecuritySeverity::Critical => 5,
        SecuritySeverity::High => 4,
        SecuritySeverity::Medium => 3,
        SecuritySeverity::Low => 2,
        SecuritySeverity::Info => 1,
    }
}

impl SecurityReport {
    /// Create an empty report
    pub fn empty() -> Self {
        ResultAggregator::empty()
    }
    
    /// Get a summary of the report
    pub fn summary(&self) -> String {
        format!(
            "Security Score: {:.0}/100 | Risk: {:?} | Findings: {} | Duration: {:.1}s",
            self.overall_score,
            self.risk_level,
            self.total_findings,
            self.scan_duration.as_secs_f64()
        )
    }
    
    /// Check if the scan found any critical issues
    pub fn has_critical_issues(&self) -> bool {
        self.findings_by_severity.get(&SecuritySeverity::Critical)
            .map(|&count| count > 0)
            .unwrap_or(false)
    }
    
    /// Get findings filtered by severity
    pub fn findings_by_severity_level(&self, severity: SecuritySeverity) -> Vec<&SecurityFinding> {
        self.findings.iter()
            .filter(|f| f.severity == severity)
            .collect()
    }
    
    /// Export report as JSON
    pub fn to_json(&self) -> Result<String, SecurityError> {
        serde_json::to_string_pretty(&self)
            .map_err(|e| SecurityError::Cache(format!("Failed to serialize report: {}", e)))
    }
    
    /// Export report as SARIF (Static Analysis Results Interchange Format)
    pub fn to_sarif(&self) -> Result<String, SecurityError> {
        // TODO: Implement SARIF export for GitHub integration
        Err(SecurityError::Cache("SARIF export not yet implemented".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_result_aggregation() {
        let findings = vec![
            SecurityFinding {
                id: "test-1".to_string(),
                title: "Critical Finding".to_string(),
                description: "Test critical".to_string(),
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::SecretsExposure,
                file_path: Some(PathBuf::from("test.js")),
                line_number: Some(10),
                column_number: Some(5),
                evidence: None,
                remediation: vec![],
                references: vec![],
                cwe_id: None,
                compliance_frameworks: vec![],
            },
            SecurityFinding {
                id: "test-2".to_string(),
                title: "Medium Finding".to_string(),
                description: "Test medium".to_string(),
                severity: SecuritySeverity::Medium,
                category: SecurityCategory::InsecureConfiguration,
                file_path: Some(PathBuf::from("config.json")),
                line_number: Some(20),
                column_number: Some(1),
                evidence: None,
                remediation: vec![],
                references: vec![],
                cwe_id: None,
                compliance_frameworks: vec![],
            },
        ];
        
        let report = ResultAggregator::aggregate(findings, Duration::from_secs(5));
        
        assert_eq!(report.total_findings, 2);
        assert_eq!(report.risk_level, SecuritySeverity::Critical);
        assert!(report.overall_score < 100.0);
        assert!(!report.recommendations.is_empty());
    }
    
    #[test]
    fn test_deduplication() {
        let findings = vec![
            SecurityFinding {
                id: "dup-1".to_string(),
                title: "Duplicate Finding".to_string(),
                description: "Test".to_string(),
                severity: SecuritySeverity::High,
                category: SecurityCategory::SecretsExposure,
                file_path: Some(PathBuf::from("test.js")),
                line_number: Some(10),
                column_number: Some(5),
                evidence: None,
                remediation: vec![],
                references: vec![],
                cwe_id: None,
                compliance_frameworks: vec![],
            },
            SecurityFinding {
                id: "dup-1".to_string(),
                title: "Duplicate Finding".to_string(),
                description: "Test".to_string(),
                severity: SecuritySeverity::Medium, // Lower severity
                category: SecurityCategory::SecretsExposure,
                file_path: Some(PathBuf::from("test.js")),
                line_number: Some(10),
                column_number: Some(5),
                evidence: None,
                remediation: vec![],
                references: vec![],
                cwe_id: None,
                compliance_frameworks: vec![],
            },
        ];
        
        let deduplicated = ResultAggregator::deduplicate_findings(findings);
        assert_eq!(deduplicated.len(), 1);
        assert_eq!(deduplicated[0].severity, SecuritySeverity::High); // Should keep higher severity
    }
    
    #[test]
    fn test_security_score_calculation() {
        let findings = vec![
            SecurityFinding {
                id: "test".to_string(),
                title: "Test".to_string(),
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
            },
        ];
        
        let score = ResultAggregator::calculate_security_score(&findings);
        assert_eq!(score, 75.0); // 100 - 25 (critical penalty)
    }
} 