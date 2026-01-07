//! Core types for Kubernetes resource optimization analysis.
//!
//! These types represent resource usage, recommendations, and analysis results
//! for identifying over-provisioned or under-provisioned Kubernetes workloads.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::path::PathBuf;

// ============================================================================
// Severity
// ============================================================================

/// Severity levels for optimization issues.
///
/// Ordered from most severe to least severe:
/// `Critical > High > Medium > Low > Info`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Critical issues that require immediate attention (e.g., under-provisioned causing OOM)
    Critical,
    /// High impact issues (significant waste or risk)
    High,
    /// Medium impact issues
    #[default]
    Medium,
    /// Low impact issues
    Low,
    /// Informational suggestions
    Info,
}

impl Severity {
    /// Parse a severity from a string (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "critical" => Some(Self::Critical),
            "high" => Some(Self::High),
            "medium" => Some(Self::Medium),
            "low" => Some(Self::Low),
            "info" => Some(Self::Info),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Info => "info",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Ord for Severity {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_val = match self {
            Self::Critical => 0,
            Self::High => 1,
            Self::Medium => 2,
            Self::Low => 3,
            Self::Info => 4,
        };
        let other_val = match other {
            Self::Critical => 0,
            Self::High => 1,
            Self::Medium => 2,
            Self::Low => 3,
            Self::Info => 4,
        };
        // Reverse so Critical > High > Medium > Low > Info
        other_val.cmp(&self_val)
    }
}

impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ============================================================================
// Rule Codes
// ============================================================================

/// A rule/check code identifier for optimization issues.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuleCode(pub String);

impl RuleCode {
    /// Create a new rule code.
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    /// Get the code as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RuleCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for RuleCode {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for RuleCode {
    fn from(s: String) -> Self {
        Self(s)
    }
}

// ============================================================================
// Optimization Issue Type
// ============================================================================

/// Type of optimization issue detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationIssue {
    /// Resources are significantly over-provisioned (wasting money)
    OverProvisioned,
    /// Resources are under-provisioned (risk of OOM or throttling)
    UnderProvisioned,
    /// No resource requests are defined
    NoRequestsDefined,
    /// No resource limits are defined
    NoLimitsDefined,
    /// Excessive ratio between limits and requests
    ExcessiveRatio,
    /// CPU and memory ratio is unusual for workload type
    UnbalancedResources,
    /// Resources are well-configured
    Optimal,
}

impl OptimizationIssue {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OverProvisioned => "over_provisioned",
            Self::UnderProvisioned => "under_provisioned",
            Self::NoRequestsDefined => "no_requests_defined",
            Self::NoLimitsDefined => "no_limits_defined",
            Self::ExcessiveRatio => "excessive_ratio",
            Self::UnbalancedResources => "unbalanced_resources",
            Self::Optimal => "optimal",
        }
    }
}

impl fmt::Display for OptimizationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Workload Type Classification
// ============================================================================

/// Classification of workload type for better recommendations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadType {
    /// Web server / API (typically CPU-light, memory moderate)
    Web,
    /// Background worker / queue consumer
    Worker,
    /// Batch processing job
    Batch,
    /// Database or stateful storage
    Database,
    /// Cache (Redis, Memcached)
    Cache,
    /// Message broker (Kafka, RabbitMQ)
    MessageBroker,
    /// Machine learning / GPU workload
    MachineLearning,
    /// General purpose / unknown
    General,
}

impl WorkloadType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Worker => "worker",
            Self::Batch => "batch",
            Self::Database => "database",
            Self::Cache => "cache",
            Self::MessageBroker => "message_broker",
            Self::MachineLearning => "machine_learning",
            Self::General => "general",
        }
    }

    /// Get default resource recommendations for this workload type.
    pub fn default_resources(&self) -> ResourceDefaults {
        match self {
            Self::Web => ResourceDefaults {
                cpu_request: "100m",
                cpu_limit: "500m",
                memory_request: "128Mi",
                memory_limit: "512Mi",
                typical_cpu_ratio: 5.0,
                typical_memory_ratio: 4.0,
            },
            Self::Worker => ResourceDefaults {
                cpu_request: "200m",
                cpu_limit: "1000m",
                memory_request: "256Mi",
                memory_limit: "1Gi",
                typical_cpu_ratio: 5.0,
                typical_memory_ratio: 4.0,
            },
            Self::Batch => ResourceDefaults {
                cpu_request: "500m",
                cpu_limit: "2000m",
                memory_request: "512Mi",
                memory_limit: "2Gi",
                typical_cpu_ratio: 4.0,
                typical_memory_ratio: 4.0,
            },
            Self::Database => ResourceDefaults {
                cpu_request: "500m",
                cpu_limit: "2000m",
                memory_request: "1Gi",
                memory_limit: "4Gi",
                typical_cpu_ratio: 4.0,
                typical_memory_ratio: 4.0,
            },
            Self::Cache => ResourceDefaults {
                cpu_request: "100m",
                cpu_limit: "500m",
                memory_request: "256Mi",
                memory_limit: "1Gi",
                typical_cpu_ratio: 5.0,
                typical_memory_ratio: 4.0,
            },
            Self::MessageBroker => ResourceDefaults {
                cpu_request: "250m",
                cpu_limit: "1000m",
                memory_request: "512Mi",
                memory_limit: "2Gi",
                typical_cpu_ratio: 4.0,
                typical_memory_ratio: 4.0,
            },
            Self::MachineLearning => ResourceDefaults {
                cpu_request: "1000m",
                cpu_limit: "4000m",
                memory_request: "2Gi",
                memory_limit: "8Gi",
                typical_cpu_ratio: 4.0,
                typical_memory_ratio: 4.0,
            },
            Self::General => ResourceDefaults {
                cpu_request: "100m",
                cpu_limit: "500m",
                memory_request: "128Mi",
                memory_limit: "512Mi",
                typical_cpu_ratio: 5.0,
                typical_memory_ratio: 4.0,
            },
        }
    }
}

impl fmt::Display for WorkloadType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Default resource recommendations for a workload type.
#[derive(Debug, Clone)]
pub struct ResourceDefaults {
    pub cpu_request: &'static str,
    pub cpu_limit: &'static str,
    pub memory_request: &'static str,
    pub memory_limit: &'static str,
    pub typical_cpu_ratio: f64,
    pub typical_memory_ratio: f64,
}

// ============================================================================
// Resource Specification
// ============================================================================

/// Kubernetes resource specification (CPU/memory requests and limits).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSpec {
    /// CPU request (e.g., "100m", "1")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_request: Option<String>,
    /// CPU limit (e.g., "500m", "2")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_limit: Option<String>,
    /// Memory request (e.g., "128Mi", "1Gi")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_request: Option<String>,
    /// Memory limit (e.g., "512Mi", "4Gi")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<String>,
}

impl ResourceSpec {
    /// Create a new resource spec.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any resources are defined.
    pub fn has_any(&self) -> bool {
        self.cpu_request.is_some()
            || self.cpu_limit.is_some()
            || self.memory_request.is_some()
            || self.memory_limit.is_some()
    }

    /// Check if requests are defined.
    pub fn has_requests(&self) -> bool {
        self.cpu_request.is_some() || self.memory_request.is_some()
    }

    /// Check if limits are defined.
    pub fn has_limits(&self) -> bool {
        self.cpu_limit.is_some() || self.memory_limit.is_some()
    }

    /// Generate YAML snippet for these resources.
    pub fn to_yaml(&self) -> String {
        let mut lines = Vec::new();
        lines.push("resources:".to_string());

        if self.has_requests() {
            lines.push("  requests:".to_string());
            if let Some(cpu) = &self.cpu_request {
                lines.push(format!("    cpu: {}", cpu));
            }
            if let Some(mem) = &self.memory_request {
                lines.push(format!("    memory: {}", mem));
            }
        }

        if self.has_limits() {
            lines.push("  limits:".to_string());
            if let Some(cpu) = &self.cpu_limit {
                lines.push(format!("    cpu: {}", cpu));
            }
            if let Some(mem) = &self.memory_limit {
                lines.push(format!("    memory: {}", mem));
            }
        }

        lines.join("\n")
    }
}

// ============================================================================
// Resource Usage (from metrics)
// ============================================================================

/// Actual resource usage metrics from a live cluster.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage at 50th percentile (millicores)
    pub cpu_p50: f64,
    /// CPU usage at 95th percentile (millicores)
    pub cpu_p95: f64,
    /// CPU usage at 99th percentile (millicores)
    pub cpu_p99: f64,
    /// Maximum CPU usage (millicores)
    pub cpu_max: f64,
    /// Memory usage at 50th percentile (bytes)
    pub memory_p50: u64,
    /// Memory usage at 95th percentile (bytes)
    pub memory_p95: u64,
    /// Memory usage at 99th percentile (bytes)
    pub memory_p99: u64,
    /// Maximum memory usage (bytes)
    pub memory_max: u64,
    /// Number of data samples collected
    pub sample_count: u32,
    /// Period of data collection in hours
    pub period_hours: u32,
}

// ============================================================================
// Cost Savings
// ============================================================================

/// Estimated cost savings from optimization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostSavings {
    /// CPU cores freed
    pub cpu_cores_freed: f64,
    /// Memory GB freed
    pub memory_gb_freed: f64,
    /// Estimated monthly savings in USD
    pub monthly_usd: f64,
    /// Estimated yearly savings in USD
    pub yearly_usd: f64,
}

// ============================================================================
// Resource Recommendation
// ============================================================================

/// A resource optimization recommendation for a single container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRecommendation {
    /// The Kubernetes resource kind (e.g., "Deployment", "StatefulSet")
    pub resource_kind: String,
    /// The resource name
    pub resource_name: String,
    /// The namespace (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// The container name
    pub container: String,
    /// The file path where this resource is defined
    pub file_path: PathBuf,
    /// Line number in the file (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// The type of optimization issue
    pub issue: OptimizationIssue,
    /// Severity of the issue
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// Detected workload type
    pub workload_type: WorkloadType,
    /// Current resource specification
    pub current: ResourceSpec,
    /// Actual usage metrics (if available from live cluster)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_usage: Option<ResourceUsage>,
    /// Recommended resource specification
    pub recommended: ResourceSpec,
    /// Estimated cost savings (if calculable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub savings: Option<CostSavings>,
    /// YAML snippet for the fix
    pub fix_yaml: String,
    /// Rule code that triggered this recommendation
    pub rule_code: RuleCode,
}

impl ResourceRecommendation {
    /// Get a full identifier for the resource.
    pub fn resource_identifier(&self) -> String {
        match &self.namespace {
            Some(ns) => format!("{}/{}", ns, self.resource_name),
            None => self.resource_name.clone(),
        }
    }
}

impl Ord for ResourceRecommendation {
    fn cmp(&self, other: &Self) -> Ordering {
        // Sort by severity first, then by file path, then by line
        match self.severity.cmp(&other.severity) {
            Ordering::Equal => match self.file_path.cmp(&other.file_path) {
                Ordering::Equal => self.line.cmp(&other.line),
                other => other,
            },
            other => other,
        }
    }
}

impl PartialOrd for ResourceRecommendation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ResourceRecommendation {
    fn eq(&self, other: &Self) -> bool {
        self.resource_kind == other.resource_kind
            && self.resource_name == other.resource_name
            && self.container == other.container
            && self.namespace == other.namespace
    }
}

impl Eq for ResourceRecommendation {}

// ============================================================================
// Resource Warning
// ============================================================================

/// A warning about a resource that isn't a recommendation but needs attention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceWarning {
    /// The resource identifier
    pub resource: String,
    /// The type of issue
    pub issue: OptimizationIssue,
    /// Severity
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
}

// ============================================================================
// Optimization Summary
// ============================================================================

/// Summary statistics for an optimization analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptimizationSummary {
    /// Number of resources analyzed
    pub resources_analyzed: u32,
    /// Number of containers analyzed
    pub containers_analyzed: u32,
    /// Number of over-provisioned containers
    pub over_provisioned: u32,
    /// Number of under-provisioned containers
    pub under_provisioned: u32,
    /// Number of containers missing requests
    pub missing_requests: u32,
    /// Number of containers missing limits
    pub missing_limits: u32,
    /// Number of optimal containers
    pub optimal: u32,
    /// Total waste percentage (weighted average)
    pub total_waste_percentage: f32,
    /// Estimated monthly savings in USD (if calculable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_monthly_savings_usd: Option<f32>,
}

// ============================================================================
// Analysis Metadata
// ============================================================================

/// Metadata about the analysis run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    /// Analysis mode (static or live)
    pub mode: AnalysisMode,
    /// Analysis duration in milliseconds
    pub duration_ms: u64,
    /// Syncable CLI version
    pub version: String,
    /// Timestamp of the analysis
    pub timestamp: String,
    /// Path analyzed
    pub path: PathBuf,
}

/// Analysis mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisMode {
    /// Static analysis of manifests (no cluster access)
    Static,
    /// Live analysis with cluster metrics
    Live,
}

impl fmt::Display for AnalysisMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static => write!(f, "static"),
            Self::Live => write!(f, "live"),
        }
    }
}

// ============================================================================
// Optimization Result
// ============================================================================

/// Complete result of an optimization analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    /// Summary statistics
    pub summary: OptimizationSummary,
    /// Resource recommendations
    pub recommendations: Vec<ResourceRecommendation>,
    /// Warnings (issues that need attention but aren't recommendations)
    pub warnings: Vec<ResourceWarning>,
    /// Analysis metadata
    pub metadata: AnalysisMetadata,
}

impl OptimizationResult {
    /// Create a new empty result.
    pub fn new(path: PathBuf, mode: AnalysisMode) -> Self {
        Self {
            summary: OptimizationSummary::default(),
            recommendations: Vec::new(),
            warnings: Vec::new(),
            metadata: AnalysisMetadata {
                mode,
                duration_ms: 0,
                version: env!("CARGO_PKG_VERSION").to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                path,
            },
        }
    }

    /// Check if there are any recommendations.
    pub fn has_recommendations(&self) -> bool {
        !self.recommendations.is_empty()
    }

    /// Check if there are any warnings.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get the maximum severity in recommendations.
    pub fn max_severity(&self) -> Option<Severity> {
        self.recommendations.iter().map(|r| r.severity).max()
    }

    /// Sort recommendations by severity (most severe first).
    pub fn sort(&mut self) {
        self.recommendations.sort();
    }

    /// Filter recommendations by minimum severity.
    pub fn filter_by_severity(&mut self, min_severity: Severity) {
        self.recommendations.retain(|r| r.severity >= min_severity);
    }

    /// Filter recommendations by minimum waste threshold.
    pub fn filter_by_threshold(&mut self, _threshold_percent: u8) {
        // TODO: Implement when we have waste percentage per recommendation
    }
}

// ============================================================================
// Tests
// ============================================================================

// ============================================================================
// Unified Report (for --full flag with JSON output)
// ============================================================================

/// A comprehensive analysis report combining all analysis types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedReport {
    /// Overall summary
    pub summary: UnifiedSummary,
    /// Live cluster analysis (if connected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_analysis: Option<LiveClusterSummary>,
    /// Static resource optimization findings
    pub resource_optimization: ResourceOptimizationReport,
    /// Security and best practices findings (kubelint)
    pub security: SecurityReport,
    /// Helm chart validation findings
    pub helm_validation: HelmValidationReport,
    /// Suggested fixes from live data (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_fixes: Option<Vec<LiveFix>>,
    /// Trend analysis (if historical data available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend_analysis: Option<TrendAnalysis>,
    /// Cost estimation (if provider configured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_estimation: Option<CostEstimation>,
    /// Precise fixes ready for application
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precise_fixes: Option<Vec<PreciseFix>>,
    /// Analysis metadata
    pub metadata: UnifiedMetadata,
}

/// A fix suggestion based on live cluster data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveFix {
    /// Namespace of the workload
    pub namespace: String,
    /// Workload name
    pub workload_name: String,
    /// Container name
    pub container_name: String,
    /// Confidence level (0-100)
    pub confidence: u8,
    /// Data source (e.g., "Prometheus", "Combined")
    pub source: String,
    /// YAML fix snippet
    pub fix_yaml: String,
}

/// Overall summary across all analysis types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSummary {
    /// Total resources analyzed
    pub total_resources: usize,
    /// Total issues found
    pub total_issues: usize,
    /// Critical issues count
    pub critical_issues: usize,
    /// High priority issues
    pub high_issues: usize,
    /// Medium priority issues
    pub medium_issues: usize,
    /// Overall confidence (0-100)
    pub confidence: u8,
    /// Overall health score (0-100)
    pub health_score: u8,
}

/// Live cluster analysis summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveClusterSummary {
    /// Data source used
    pub source: String,
    /// Resources analyzed
    pub resources_analyzed: usize,
    /// Over-provisioned count
    pub over_provisioned: usize,
    /// Under-provisioned count
    pub under_provisioned: usize,
    /// Optimal count
    pub optimal: usize,
    /// Confidence percentage
    pub confidence: u8,
    /// Whether P95 data from Prometheus was used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses_p95: Option<bool>,
    /// Time range of historical data (e.g., "7d")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_period: Option<String>,
}

/// Resource optimization findings summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceOptimizationReport {
    /// Summary stats
    pub summary: ResourceOptimizationSummary,
    /// Detailed recommendations
    pub recommendations: Vec<ResourceRecommendation>,
}

/// Resource optimization summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceOptimizationSummary {
    pub resources: usize,
    pub containers: usize,
    pub over_provisioned: usize,
    pub missing_requests: usize,
    pub optimal: usize,
    pub estimated_waste_percent: f32,
}

/// Security analysis report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    /// Summary stats
    pub summary: SecuritySummary,
    /// Detailed findings
    pub findings: Vec<SecurityFinding>,
}

/// Security summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySummary {
    pub objects_analyzed: usize,
    pub checks_run: usize,
    pub critical: usize,
    pub warnings: usize,
}

/// A security finding from kubelint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub code: String,
    pub severity: String,
    pub object_kind: String,
    pub object_name: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

/// Helm validation report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmValidationReport {
    /// Summary stats
    pub summary: HelmValidationSummary,
    /// Per-chart findings
    pub charts: Vec<ChartValidation>,
}

/// Helm validation summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmValidationSummary {
    pub charts_analyzed: usize,
    pub charts_with_issues: usize,
    pub total_issues: usize,
}

/// Validation results for a single Helm chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartValidation {
    pub chart_name: String,
    pub issues: Vec<HelmIssue>,
}

/// A Helm chart validation issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmIssue {
    pub code: String,
    pub severity: String,
    pub message: String,
}

/// Metadata about the unified analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMetadata {
    pub path: String,
    pub analysis_time_ms: u64,
    pub timestamp: String,
    pub version: String,
}

// ============================================================================
// Trend Analysis (Phase 1)
// ============================================================================

/// Trend analysis comparing current state to historical data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Comparison period (e.g., "7d", "30d")
    pub period: String,
    /// Current waste metrics
    pub current: WasteMetrics,
    /// Historical waste metrics (from start of period)
    pub historical: WasteMetrics,
    /// Change direction and percentage
    pub trend: TrendDirection,
    /// Per-workload trends
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub workload_trends: Vec<WorkloadTrend>,
}

/// Waste metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasteMetrics {
    /// Total CPU waste in millicores
    pub cpu_waste_millicores: u64,
    /// Total memory waste in bytes
    pub memory_waste_bytes: u64,
    /// Average waste percentage
    pub waste_percentage: f32,
    /// Number of over-provisioned workloads
    pub over_provisioned_count: usize,
}

/// Trend direction with magnitude.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDirection {
    /// "improving", "worsening", or "stable"
    pub direction: String,
    /// Percentage change (positive = more waste, negative = less waste)
    pub change_percent: f32,
}

/// Trend for a single workload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadTrend {
    pub namespace: String,
    pub workload_name: String,
    /// Waste change in millicores (positive = more waste)
    pub cpu_change_millicores: i64,
    /// Waste change in bytes (positive = more waste)
    pub memory_change_bytes: i64,
    pub direction: String,
}

// ============================================================================
// Cost Estimation (Phase 2)
// ============================================================================

/// Cost estimation for resource waste.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimation {
    /// Cloud provider used for pricing
    pub provider: CloudProvider,
    /// Region for pricing (affects costs)
    pub region: String,
    /// Monthly cost of wasted resources
    pub monthly_waste_cost: f64,
    /// Annual projected waste cost
    pub annual_waste_cost: f64,
    /// Monthly savings if recommendations applied
    pub monthly_savings: f64,
    /// Annual projected savings
    pub annual_savings: f64,
    /// Currency code (e.g., "USD")
    pub currency: String,
    /// Breakdown by resource type
    pub breakdown: CostBreakdown,
    /// Per-workload costs
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub workload_costs: Vec<WorkloadCost>,
}

/// Cloud provider for pricing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CloudProvider {
    Aws,
    Gcp,
    Azure,
    OnPrem,
    Unknown,
}

impl Default for CloudProvider {
    fn default() -> Self {
        CloudProvider::Unknown
    }
}

/// Cost breakdown by resource type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// CPU waste cost per month
    pub cpu_cost: f64,
    /// Memory waste cost per month
    pub memory_cost: f64,
}

/// Cost for a single workload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadCost {
    pub namespace: String,
    pub workload_name: String,
    /// Monthly waste cost for this workload
    pub monthly_cost: f64,
    /// Potential monthly savings
    pub monthly_savings: f64,
}

// ============================================================================
// Precise Fix Application (Phase 3)
// ============================================================================

/// A precise fix target with exact file location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreciseFix {
    /// Unique identifier for this fix
    pub id: String,
    /// Target file path
    pub file_path: PathBuf,
    /// Line number where the resource is defined
    pub line_number: u32,
    /// Column number (for precise positioning)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    /// Resource kind (Deployment, StatefulSet, etc.)
    pub resource_kind: String,
    /// Resource name
    pub resource_name: String,
    /// Container name being fixed
    pub container_name: String,
    /// Namespace (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Current values being replaced
    pub current: FixResourceValues,
    /// Recommended new values
    pub recommended: FixResourceValues,
    /// Confidence level (0-100)
    pub confidence: u8,
    /// Data source for recommendation
    pub source: FixSource,
    /// Impact assessment
    pub impact: FixImpact,
    /// Fix status
    #[serde(default)]
    pub status: FixStatus,
}

/// Resource values for a fix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResourceValues {
    /// CPU request (e.g., "100m")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_request: Option<String>,
    /// CPU limit (e.g., "500m")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_limit: Option<String>,
    /// Memory request (e.g., "128Mi")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_request: Option<String>,
    /// Memory limit (e.g., "512Mi")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<String>,
}

/// Source of the fix recommendation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FixSource {
    /// Based on P95 Prometheus metrics
    PrometheusP95,
    /// Based on metrics-server real-time data
    MetricsServer,
    /// Combined sources (highest confidence)
    Combined,
    /// Static analysis heuristics
    StaticAnalysis,
}

impl Default for FixSource {
    fn default() -> Self {
        FixSource::StaticAnalysis
    }
}

/// Impact assessment for applying a fix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixImpact {
    /// Risk level of applying this fix
    pub risk: FixRisk,
    /// Estimated monthly savings from this fix
    pub monthly_savings: f64,
    /// Whether this could cause OOM issues
    pub oom_risk: bool,
    /// Whether this could cause CPU throttling
    pub throttle_risk: bool,
    /// Recommended action
    pub recommendation: String,
}

/// Risk level for a fix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FixRisk {
    /// Safe to apply automatically
    Low,
    /// Review recommended before applying
    Medium,
    /// Manual review required
    High,
    /// Do not auto-apply
    Critical,
}

impl Default for FixRisk {
    fn default() -> Self {
        FixRisk::Medium
    }
}

/// Status of a fix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FixStatus {
    #[default]
    Pending,
    Applied,
    Skipped,
    Failed,
    Backed,
}

/// Result of applying fixes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixApplicationResult {
    /// Total fixes attempted
    pub total_fixes: usize,
    /// Successfully applied
    pub applied: usize,
    /// Skipped (low confidence, high risk, etc.)
    pub skipped: usize,
    /// Failed to apply
    pub failed: usize,
    /// Backup directory path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<PathBuf>,
    /// Individual fix results
    pub fixes: Vec<PreciseFix>,
    /// Errors encountered
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);
    }

    #[test]
    fn test_severity_parse() {
        assert_eq!(Severity::parse("critical"), Some(Severity::Critical));
        assert_eq!(Severity::parse("HIGH"), Some(Severity::High));
        assert_eq!(Severity::parse("invalid"), None);
    }

    #[test]
    fn test_resource_spec_yaml() {
        let spec = ResourceSpec {
            cpu_request: Some("100m".to_string()),
            cpu_limit: Some("500m".to_string()),
            memory_request: Some("128Mi".to_string()),
            memory_limit: Some("512Mi".to_string()),
        };

        let yaml = spec.to_yaml();
        assert!(yaml.contains("cpu: 100m"));
        assert!(yaml.contains("memory: 512Mi"));
    }

    #[test]
    fn test_workload_type_defaults() {
        let web_defaults = WorkloadType::Web.default_resources();
        assert_eq!(web_defaults.cpu_request, "100m");
        assert_eq!(web_defaults.memory_request, "128Mi");

        let db_defaults = WorkloadType::Database.default_resources();
        assert_eq!(db_defaults.memory_request, "1Gi");
    }

    #[test]
    fn test_optimization_result_new() {
        let result = OptimizationResult::new(PathBuf::from("."), AnalysisMode::Static);
        assert!(result.recommendations.is_empty());
        assert!(!result.has_recommendations());
        assert_eq!(result.metadata.mode, AnalysisMode::Static);
    }
}
