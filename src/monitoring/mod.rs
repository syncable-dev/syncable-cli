//! # Monitoring Module
//! 
//! This module provides anonymized usage telemetry and metrics collection
//! for the Syncable CLI. All data is anonymized and users can opt-out.

pub mod telemetry;

use std::sync::Arc;
use once_cell::sync::Lazy;
use crate::Result;

/// Global telemetry instance
pub static TELEMETRY: Lazy<Arc<telemetry::TelemetryCollector>> = Lazy::new(|| {
    Arc::new(telemetry::TelemetryCollector::new())
});

/// Initialize the monitoring system
pub fn init() -> Result<()> {
    TELEMETRY.initialize()?;
    log::debug!("Telemetry system initialized");
    Ok(())
}

/// Shutdown the monitoring system gracefully
pub fn shutdown() {
    TELEMETRY.shutdown();
    log::debug!("Telemetry system shutdown");
}

/// Record a command execution
pub fn record_command_usage(command: &str, duration_ms: u64, success: bool) {
    TELEMETRY.record_command_usage(command, duration_ms, success);
}

/// Record project analysis metrics
pub fn record_project_analysis(
    project_type: &str,
    file_count: u64,
    language_count: u64,
    framework_count: u64,
    duration_ms: u64,
) {
    TELEMETRY.record_project_analysis(
        project_type,
        file_count,
        language_count,
        framework_count,
        duration_ms,
    );
}

/// Record generation metrics
pub fn record_generation(
    generation_type: &str,
    file_size_bytes: u64,
    duration_ms: u64,
    success: bool,
) {
    TELEMETRY.record_generation(generation_type, file_size_bytes, duration_ms, success);
}

/// Record error occurrence
pub fn record_error(error_type: &str, component: &str) {
    TELEMETRY.record_error(error_type, component);
}

/// Check if telemetry is enabled
pub fn is_enabled() -> bool {
    TELEMETRY.is_enabled()
} 