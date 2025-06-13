//! Telemetry collection and reporting
//! 
//! This module handles the collection and export of anonymized usage metrics.
//! All data is anonymized and privacy-focused.

use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use crate::Result;

/// Configuration for telemetry
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub export_interval_seconds: u64,
    pub timeout_seconds: u64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "https://telemetry.syncable.dev".to_string(),
            export_interval_seconds: 60,
            timeout_seconds: 10,
        }
    }
}

/// Metric data structure
#[derive(Debug, Clone)]
pub struct MetricEvent {
    pub timestamp: u64,
    pub session_id: String,
    pub install_id: String,
    pub metric_type: String,
    pub value: f64,
    pub labels: std::collections::HashMap<String, String>,
}

/// Main telemetry collector
pub struct TelemetryCollector {
    config: TelemetryConfig,
    session_id: String,
    install_id: String,
    metrics: Mutex<Vec<MetricEvent>>,
}

impl TelemetryCollector {
    /// Create a new telemetry collector
    pub fn new() -> Self {
        let config = Self::load_config();
        let session_id = Uuid::new_v4().to_string();
        let install_id = Self::get_or_create_install_id();
        
        Self {
            config,
            session_id,
            install_id,
            metrics: Mutex::new(Vec::new()),
        }
    }
    
    /// Initialize the telemetry system
    pub fn initialize(&self) -> Result<()> {
        if !self.config.enabled {
            log::debug!("Telemetry disabled, skipping initialization");
            return Ok(());
        }
        
        log::info!("Telemetry initialized with session ID: {}", &self.session_id[..8]);
        Ok(())
    }
    
    /// Record command usage
    pub fn record_command_usage(&self, command: &str, duration_ms: u64, success: bool) {
        if !self.config.enabled {
            return;
        }
        
        log::debug!("Recording command usage: {} ({}ms, success: {})", command, duration_ms, success);
        
        let mut labels = std::collections::HashMap::new();
        labels.insert("command".to_string(), command.to_string());
        labels.insert("success".to_string(), success.to_string());
        
        self.record_metric("command_executed", 1.0, labels);
        
        let mut duration_labels = std::collections::HashMap::new();
        duration_labels.insert("command".to_string(), command.to_string());
        self.record_metric("command_duration_ms", duration_ms as f64, duration_labels);
    }
    
    /// Record project analysis metrics
    pub fn record_project_analysis(
        &self,
        project_type: &str,
        file_count: u64,
        language_count: u64,
        framework_count: u64,
        duration_ms: u64,
    ) {
        if !self.config.enabled {
            return;
        }
        
        log::debug!(
            "Recording analysis: {} ({} files, {} langs, {} frameworks, {}ms)", 
            project_type, file_count, language_count, framework_count, duration_ms
        );
        
        let mut labels = std::collections::HashMap::new();
        labels.insert("project_type".to_string(), project_type.to_string());
        labels.insert("file_count_bucket".to_string(), Self::bucket_file_count(file_count));
        
        self.record_metric("project_analyzed", 1.0, labels);
        self.record_metric("analysis_duration_ms", duration_ms as f64, 
            std::collections::HashMap::from([("project_type".to_string(), project_type.to_string())]));
        self.record_metric("file_count", file_count as f64, 
            std::collections::HashMap::from([("project_type".to_string(), project_type.to_string())]));
        self.record_metric("language_count", language_count as f64, 
            std::collections::HashMap::from([("project_type".to_string(), project_type.to_string())]));
        self.record_metric("framework_count", framework_count as f64, 
            std::collections::HashMap::from([("project_type".to_string(), project_type.to_string())]));
    }
    
    /// Record generation metrics
    pub fn record_generation(
        &self,
        generation_type: &str,
        file_size_bytes: u64,
        duration_ms: u64,
        success: bool,
    ) {
        if !self.config.enabled {
            return;
        }
        
        log::debug!(
            "Recording generation: {} ({} bytes, {}ms, success: {})", 
            generation_type, file_size_bytes, duration_ms, success
        );
        
        let mut labels = std::collections::HashMap::new();
        labels.insert("generation_type".to_string(), generation_type.to_string());
        labels.insert("success".to_string(), success.to_string());
        
        self.record_metric("generation_completed", 1.0, labels);
        self.record_metric("generation_duration_ms", duration_ms as f64, 
            std::collections::HashMap::from([("generation_type".to_string(), generation_type.to_string())]));
        self.record_metric("generated_file_size_bytes", file_size_bytes as f64, 
            std::collections::HashMap::from([("generation_type".to_string(), generation_type.to_string())]));
    }
    
    /// Record error occurrence
    pub fn record_error(&self, error_type: &str, component: &str) {
        if !self.config.enabled {
            return;
        }
        
        log::debug!("Recording error: {} in {}", error_type, component);
        
        let mut labels = std::collections::HashMap::new();
        labels.insert("error_type".to_string(), error_type.to_string());
        labels.insert("component".to_string(), component.to_string());
        
        self.record_metric("error_occurred", 1.0, labels);
    }
    
    /// Record a metric event
    fn record_metric(&self, metric_type: &str, value: f64, labels: std::collections::HashMap<String, String>) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        
        let event = MetricEvent {
            timestamp,
            session_id: self.session_id.clone(),
            install_id: self.install_id.clone(),
            metric_type: metric_type.to_string(),
            value,
            labels,
        };
        
                if let Ok(mut metrics) = self.metrics.lock() {
            // Log the metric for debugging before moving the event
            log::trace!("Metric recorded: {} = {} (labels: {:?})", metric_type, value, event.labels);
            
            metrics.push(event);
        }
    }
    
    /// Check if telemetry is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
    
    /// Shutdown telemetry gracefully
    pub fn shutdown(&self) {
        if !self.config.enabled {
            return;
        }
        
        // Export any remaining metrics
        if let Ok(metrics) = self.metrics.lock() {
            log::debug!("Shutting down telemetry. Collected {} metrics this session.", metrics.len());
            
            // In a full implementation, this would send the metrics to the backend
            // For now, we just log a summary
            if !metrics.is_empty() && log::log_enabled!(log::Level::Debug) {
                let mut metric_summary = std::collections::HashMap::new();
                for metric in metrics.iter() {
                    *metric_summary.entry(&metric.metric_type).or_insert(0) += 1;
                }
                log::debug!("Telemetry summary: {:?}", metric_summary);
            }
        }
    }
    
    /// Load configuration from environment and config files
    fn load_config() -> TelemetryConfig {
        let mut config = TelemetryConfig::default();
        
        // Check environment variables
        if let Ok(value) = std::env::var("SYNCABLE_TELEMETRY_ENABLED") {
            config.enabled = value.parse().unwrap_or(true);
        }
        
        if let Ok(value) = std::env::var("SYNCABLE_TELEMETRY_ENDPOINT") {
            config.endpoint = value;
        }
        
        // Check for opt-out file
        if let Some(home_dir) = dirs::home_dir() {
            let opt_out_file = home_dir.join(".syncable-cli").join("telemetry-opt-out");
            if opt_out_file.exists() {
                config.enabled = false;
                log::info!("Telemetry disabled via opt-out file");
            }
        }
        
        config
    }
    
    /// Get or create a persistent install ID for this installation
    fn get_or_create_install_id() -> String {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("syncable-cli");
        
        let id_file = config_dir.join("install-id");
        
        // Try to read existing ID
        if let Ok(id) = std::fs::read_to_string(&id_file) {
            let id = id.trim();
            if !id.is_empty() {
                return id.to_string();
            }
        }
        
        // Create new ID
        let new_id = Uuid::new_v4().to_string();
        
        // Try to save it
        if let Err(e) = std::fs::create_dir_all(&config_dir) {
            log::warn!("Failed to create config directory: {}", e);
        } else if let Err(e) = std::fs::write(&id_file, &new_id) {
            log::warn!("Failed to save install ID: {}", e);
        }
        
        new_id
    }
    
    /// Bucket file counts for privacy (avoid exact counts)
    fn bucket_file_count(count: u64) -> String {
        match count {
            0..=10 => "small".to_string(),
            11..=100 => "medium".to_string(),
            101..=1000 => "large".to_string(),
            _ => "xlarge".to_string(),
        }
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

// Implement Send + Sync so it can be used in static context
unsafe impl Send for TelemetryCollector {}
unsafe impl Sync for TelemetryCollector {} 