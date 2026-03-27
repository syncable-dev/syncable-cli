//! Trend Analyzer for Kubernetes Resource Waste
//!
//! Compares current waste metrics against historical data to identify trends.

use super::live_analyzer::LiveRecommendation;
use super::types::{TrendAnalysis, TrendDirection, WasteMetrics, WorkloadTrend};

/// Analyze trends from live recommendations.
/// Since we don't store historical data, this calculates current state
/// and marks as "unknown" trend direction (no historical comparison available).
pub fn analyze_trends_from_live(current_recs: &[LiveRecommendation]) -> TrendAnalysis {
    // Calculate current waste metrics
    let current = calculate_current_waste(current_recs);

    // Calculate per-workload trends (current state)
    let workload_trends = calculate_workload_trends(current_recs);

    TrendAnalysis {
        period: "current".to_string(),
        current: current.clone(),
        historical: WasteMetrics {
            cpu_waste_millicores: 0,
            memory_waste_bytes: 0,
            waste_percentage: 0.0,
            over_provisioned_count: 0,
        },
        trend: TrendDirection {
            // Without historical data, we report current snapshot
            direction: if current.over_provisioned_count > 5 {
                "needs_attention"
            } else if current.waste_percentage > 50.0 {
                "high_waste"
            } else if current.waste_percentage > 20.0 {
                "moderate_waste"
            } else {
                "acceptable"
            }
            .to_string(),
            change_percent: 0.0,
        },
        workload_trends,
    }
}

/// Calculate waste metrics from current recommendations.
fn calculate_current_waste(recs: &[LiveRecommendation]) -> WasteMetrics {
    let mut total_cpu_waste: u64 = 0;
    let mut total_mem_waste: u64 = 0;
    let mut over_provisioned = 0;
    let mut total_waste_pct = 0.0;

    for rec in recs {
        if rec.cpu_waste_pct > 0.0 || rec.memory_waste_pct > 0.0 {
            over_provisioned += 1;

            let cpu_waste = if rec.cpu_waste_pct > 0.0 {
                let current = rec.current_cpu_millicores.unwrap_or(0);
                current.saturating_sub(rec.recommended_cpu_millicores)
            } else {
                0
            };

            let mem_waste = if rec.memory_waste_pct > 0.0 {
                let current = rec.current_memory_bytes.unwrap_or(0);
                current.saturating_sub(rec.recommended_memory_bytes)
            } else {
                0
            };

            total_cpu_waste += cpu_waste;
            total_mem_waste += mem_waste;
            total_waste_pct += rec.cpu_waste_pct.max(rec.memory_waste_pct);
        }
    }

    let avg_waste_pct = if over_provisioned > 0 {
        total_waste_pct / over_provisioned as f32
    } else {
        0.0
    };

    WasteMetrics {
        cpu_waste_millicores: total_cpu_waste,
        memory_waste_bytes: total_mem_waste,
        waste_percentage: avg_waste_pct,
        over_provisioned_count: over_provisioned,
    }
}

/// Calculate per-workload trends.
fn calculate_workload_trends(recs: &[LiveRecommendation]) -> Vec<WorkloadTrend> {
    recs.iter()
        .filter(|rec| rec.cpu_waste_pct > 10.0 || rec.memory_waste_pct > 10.0)
        .map(|rec| {
            let cpu_change = if rec.cpu_waste_pct > 0.0 {
                let current = rec.current_cpu_millicores.unwrap_or(0) as i64;
                let recommended = rec.recommended_cpu_millicores as i64;
                current - recommended
            } else {
                0
            };

            let mem_change = if rec.memory_waste_pct > 0.0 {
                let current = rec.current_memory_bytes.unwrap_or(0) as i64;
                let recommended = rec.recommended_memory_bytes as i64;
                current - recommended
            } else {
                0
            };

            let direction = if cpu_change > 0 || mem_change > 0 {
                "over-provisioned"
            } else if cpu_change < 0 || mem_change < 0 {
                "under-provisioned"
            } else {
                "optimal"
            };

            WorkloadTrend {
                namespace: rec.namespace.clone(),
                workload_name: rec.workload_name.clone(),
                cpu_change_millicores: cpu_change,
                memory_change_bytes: mem_change,
                direction: direction.to_string(),
            }
        })
        .collect()
}

/// Analyze trends from static recommendations (no Prometheus required).
pub fn analyze_trends_static(
    current_waste_pct: f32,
    over_provisioned_count: usize,
) -> TrendAnalysis {
    // Without historical data, we can only report current state
    TrendAnalysis {
        period: "current".to_string(),
        current: WasteMetrics {
            cpu_waste_millicores: 0,
            memory_waste_bytes: 0,
            waste_percentage: current_waste_pct,
            over_provisioned_count,
        },
        historical: WasteMetrics {
            cpu_waste_millicores: 0,
            memory_waste_bytes: 0,
            waste_percentage: 0.0,
            over_provisioned_count: 0,
        },
        trend: TrendDirection {
            direction: if over_provisioned_count > 5 {
                "needs_attention"
            } else if current_waste_pct > 50.0 {
                "high_waste"
            } else if current_waste_pct > 20.0 {
                "moderate_waste"
            } else {
                "acceptable"
            }
            .to_string(),
            change_percent: 0.0,
        },
        workload_trends: vec![],
    }
}
