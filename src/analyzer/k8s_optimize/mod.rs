//! Kubernetes Resource Optimization Analyzer
//!
//! A native Rust analyzer for detecting over-provisioned and under-provisioned
//! Kubernetes workloads. Helps reduce cloud costs by right-sizing resource
//! requests and limits.
//!
//! # Features
//!
//! ## Phase 1: Static Analysis
//! - Static analysis of Kubernetes manifests (no cluster access required)
//! - **Terraform HCL support** - Parse `kubernetes_*` provider resources
//! - Pattern-based detection of over/under-provisioning
//! - Workload type classification for smarter recommendations
//! - Support for Deployments, StatefulSets, DaemonSets, Jobs, CronJobs
//! - Helm chart and Kustomize directory support
//! - Multiple output formats (table, JSON)
//!
//! ## Phase 2: Live Cluster Analysis
//! - **Kubernetes API integration** - Connect to real clusters via kubeconfig
//! - **metrics-server support** - Real-time CPU/memory usage data
//! - **Prometheus integration** - Historical metrics (P50, P95, P99, max)
//! - Data-driven recommendations based on actual usage
//! - Waste percentage calculations with confidence levels
//!
//! # Example
//!
//! ```rust,ignore
//! use syncable_cli::analyzer::k8s_optimize::{lint, K8sOptimizeConfig, OptimizationResult};
//! use std::path::Path;
//!
//! // Static analysis (no cluster needed)
//! let config = K8sOptimizeConfig::default();
//! let result = lint(Path::new("./k8s/"), &config);
//!
//! // Or using the backward-compatible analyze() function:
//! let result = analyze(Path::new("./k8s/"), &config);
//!
//! // Live cluster analysis (requires kubeconfig)
//! use syncable_cli::analyzer::k8s_optimize::live_analyzer::{LiveAnalyzer, LiveAnalyzerConfig};
//! let live_config = LiveAnalyzerConfig::default();
//! let analyzer = LiveAnalyzer::new(live_config).await?;
//! let live_result = analyzer.analyze().await?;
//! ```
//!
//! # Optimization Rules
//!
//! The analyzer checks for these common issues (K8S-OPT-001 through K8S-OPT-010):
//!
//! ## Over-Provisioning Detection
//! - K8S-OPT-005: CPU request > 1 core for non-batch workload
//! - K8S-OPT-006: Memory request > 2Gi for non-database workload
//! - K8S-OPT-007: Excessive CPU limit-to-request ratio (> 10x)
//! - K8S-OPT-008: Excessive memory limit-to-request ratio (> 4x)
//!
//! ## Under-Provisioning Detection
//! - K8S-OPT-001: No CPU request defined
//! - K8S-OPT-002: No memory request defined
//! - K8S-OPT-003: No CPU limit defined
//! - K8S-OPT-004: No memory limit defined
//!
//! ## Best Practices
//! - K8S-OPT-009: Requests equal to limits (no bursting allowed)
//! - K8S-OPT-010: Unbalanced resource allocation for workload type

// ============================================================================
// Core modules (new structure)
// ============================================================================

/// Configuration for the optimizer.
pub mod config;

/// Core data types.
pub mod types;

/// Parsing utilities (YAML, Terraform, Helm).
pub mod parser;

/// Output formatting (table, JSON, YAML).
pub mod formatter;

/// Individual optimization rules (K8S-OPT-001 through K8S-OPT-010).
pub mod rules;

/// Annotation-based rule ignoring (pragma).
pub mod pragma;

// ============================================================================
// Analysis modules
// ============================================================================

/// Static analysis of Kubernetes manifests.
pub mod static_analyzer;

/// Recommendation generation (now in rules/).
pub mod recommender;

/// Terraform parser (now in parser/terraform.rs, re-exported for compatibility).
pub mod terraform_parser;

// ============================================================================
// Live cluster analysis modules
// ============================================================================

/// Live cluster analyzer.
pub mod live_analyzer;

/// Kubernetes metrics-server client.
pub mod metrics_client;

/// Prometheus client for historical metrics.
pub mod prometheus_client;

// ============================================================================
// Cost and fix modules
// ============================================================================

/// Cost calculation and estimation.
pub mod cost_calculator;

/// Trend analysis.
pub mod trend_analyzer;

/// Fix application to manifest files.
pub mod fix_applicator;

// ============================================================================
// Placeholder subfolders (for future organization)
// ============================================================================

/// Live analysis subfolder (future home for live_analyzer, metrics_client, prometheus_client).
mod live;

/// Cost analysis subfolder (future home for cost_calculator, trend_analyzer).
mod cost;

/// Fix application subfolder (future home for fix_applicator).
mod fix;

// ============================================================================
// Re-exports: Configuration
// ============================================================================

pub use config::K8sOptimizeConfig;

// ============================================================================
// Re-exports: Core types
// ============================================================================

pub use types::{
    // Core types
    AnalysisMetadata,
    AnalysisMode,
    ChartValidation,
    CloudProvider,
    CostBreakdown,
    // Cost estimation types
    CostEstimation,
    CostSavings,
    FixApplicationResult,
    FixImpact,
    FixResourceValues,
    FixRisk,
    FixSource,
    FixStatus,
    HelmIssue,
    HelmValidationReport,
    HelmValidationSummary,
    LiveClusterSummary,
    LiveFix,
    OptimizationIssue,
    OptimizationResult,
    OptimizationSummary,
    // Precise fix types
    PreciseFix,
    ResourceOptimizationReport,
    ResourceOptimizationSummary,
    ResourceRecommendation,
    ResourceSpec,
    ResourceUsage,
    ResourceWarning,
    RuleCode,
    SecurityFinding,
    SecurityReport,
    SecuritySummary,
    Severity,
    // Trend analysis types
    TrendAnalysis,
    TrendDirection,
    UnifiedMetadata,
    // Unified report types (for --full JSON output)
    UnifiedReport,
    UnifiedSummary,
    WasteMetrics,
    WorkloadCost,
    WorkloadTrend,
    WorkloadType,
};

// ============================================================================
// Re-exports: Formatting
// ============================================================================

pub use formatter::{OutputFormat, format_result, format_result_to_string};

// ============================================================================
// Re-exports: Static analysis (primary API)
// ============================================================================

// Primary API - new lint() functions
pub use static_analyzer::{
    analyze as lint, analyze_content as lint_content, analyze_file as lint_file,
};

// Backward compatibility - keep analyze() functions
pub use static_analyzer::{analyze, analyze_content, analyze_file};

// ============================================================================
// Re-exports: Parser utilities
// ============================================================================

pub use parser::{
    TerraformContainer,
    TerraformK8sResource,
    TfResourceSpec,
    bytes_to_memory_string,
    cpu_limit_to_request_ratio,
    detect_workload_type,
    extract_container_image,
    extract_container_name,
    extract_resources,
    memory_limit_to_request_ratio,
    millicores_to_cpu_string,
    // YAML parsing
    parse_cpu_to_millicores,
    parse_memory_to_bytes,
    // Terraform parsing
    parse_terraform_k8s_resources,
};

// ============================================================================
// Re-exports: Rules
// ============================================================================

pub use rules::{
    ContainerContext,
    // Rule trait and context
    OptimizationRule,
    RuleContext,
    // Rule registry
    all_rules,
    // Rule codes
    codes as rule_codes,
    generate_recommendations,
    rule_description,
};

// ============================================================================
// Re-exports: Pragma (annotation-based ignores)
// ============================================================================

pub use pragma::{
    IGNORE_ANNOTATION_PREFIX, extract_annotations, get_ignore_reason, get_ignored_rules,
    should_ignore_rule,
};

// ============================================================================
// Re-exports: Live cluster analysis
// ============================================================================

pub use live_analyzer::{
    DataSource, LiveAnalysisResult, LiveAnalyzer, LiveAnalyzerConfig, LiveRecommendation,
};
pub use metrics_client::{MetricsClient, PodMetrics, PodResources, ResourceComparison};
pub use prometheus_client::{
    ContainerHistory, HistoricalRecommendation, PrometheusAuth, PrometheusClient,
};

// ============================================================================
// Re-exports: Cost estimation and trends
// ============================================================================

pub use cost_calculator::{calculate_from_live, calculate_from_static};
pub use fix_applicator::{apply_fixes, locate_resources_from_static, locate_resources_in_file};
pub use trend_analyzer::{analyze_trends_from_live, analyze_trends_static};
