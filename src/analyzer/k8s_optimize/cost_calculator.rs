//! Cost Calculator for Kubernetes Resource Waste
//!
//! Estimates the cost of wasted resources based on cloud provider pricing.
//! Supports AWS, GCP, Azure, and on-prem estimates.

use super::live_analyzer::LiveRecommendation;
use super::types::{
    CloudProvider, CostBreakdown, CostEstimation, ResourceRecommendation, WorkloadCost,
};

/// Default pricing per vCPU-hour (in USD)
const AWS_CPU_HOURLY: f64 = 0.0416; // ~$30/month per vCPU (on-demand m5.large)
const GCP_CPU_HOURLY: f64 = 0.0335; // ~$24/month per vCPU (n1-standard)
const AZURE_CPU_HOURLY: f64 = 0.0400; // ~$29/month per vCPU (D2s v3)
const ONPREM_CPU_HOURLY: f64 = 0.0250; // ~$18/month per vCPU (rough estimate)

/// Default pricing per GB-hour (in USD)
const AWS_MEM_HOURLY: f64 = 0.0052; // ~$3.75/month per GB
const GCP_MEM_HOURLY: f64 = 0.0045; // ~$3.24/month per GB
const AZURE_MEM_HOURLY: f64 = 0.0050; // ~$3.60/month per GB
const ONPREM_MEM_HOURLY: f64 = 0.0030; // ~$2.16/month per GB (rough estimate)

/// Hours in a month (for cost calculations)
const HOURS_PER_MONTH: f64 = 730.0;

/// Calculate cost estimation from live analysis results.
pub fn calculate_from_live(
    recommendations: &[LiveRecommendation],
    provider: CloudProvider,
    region: &str,
) -> CostEstimation {
    let (cpu_hourly, mem_hourly) = get_pricing(&provider);

    let mut total_cpu_waste_millicores: u64 = 0;
    let mut total_memory_waste_bytes: u64 = 0;
    let mut workload_costs: Vec<WorkloadCost> = Vec::new();

    for rec in recommendations {
        // Calculate waste (only for over-provisioned resources)
        let cpu_waste = if rec.cpu_waste_pct > 0.0 {
            // Current CPU minus recommended = waste
            let current = rec.current_cpu_millicores.unwrap_or(0);
            current.saturating_sub(rec.recommended_cpu_millicores)
        } else {
            0
        };

        let memory_waste = if rec.memory_waste_pct > 0.0 {
            let current = rec.current_memory_bytes.unwrap_or(0);
            current.saturating_sub(rec.recommended_memory_bytes)
        } else {
            0
        };

        total_cpu_waste_millicores += cpu_waste;
        total_memory_waste_bytes += memory_waste;

        // Calculate per-workload cost
        let cpu_cores = cpu_waste as f64 / 1000.0;
        let memory_gb = memory_waste as f64 / (1024.0 * 1024.0 * 1024.0);

        let monthly_cost =
            (cpu_cores * cpu_hourly * HOURS_PER_MONTH) + (memory_gb * mem_hourly * HOURS_PER_MONTH);

        if monthly_cost > 0.01 {
            workload_costs.push(WorkloadCost {
                namespace: rec.namespace.clone(),
                workload_name: rec.workload_name.clone(),
                monthly_cost: round_cost(monthly_cost),
                monthly_savings: round_cost(monthly_cost),
            });
        }
    }

    // Calculate totals
    let cpu_cores = total_cpu_waste_millicores as f64 / 1000.0;
    let memory_gb = total_memory_waste_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

    let cpu_monthly = cpu_cores * cpu_hourly * HOURS_PER_MONTH;
    let mem_monthly = memory_gb * mem_hourly * HOURS_PER_MONTH;
    let monthly_waste = cpu_monthly + mem_monthly;

    // Sort workloads by cost (highest first)
    workload_costs.sort_by(|a, b| {
        b.monthly_cost
            .partial_cmp(&a.monthly_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    CostEstimation {
        provider,
        region: region.to_string(),
        monthly_waste_cost: round_cost(monthly_waste),
        annual_waste_cost: round_cost(monthly_waste * 12.0),
        monthly_savings: round_cost(monthly_waste),
        annual_savings: round_cost(monthly_waste * 12.0),
        currency: "USD".to_string(),
        breakdown: CostBreakdown {
            cpu_cost: round_cost(cpu_monthly),
            memory_cost: round_cost(mem_monthly),
        },
        workload_costs,
    }
}

/// Calculate cost estimation from static analysis results.
pub fn calculate_from_static(
    recommendations: &[ResourceRecommendation],
    provider: CloudProvider,
    region: &str,
) -> CostEstimation {
    let (cpu_hourly, mem_hourly) = get_pricing(&provider);

    let mut total_cpu_waste_millicores: u64 = 0;
    let mut total_memory_waste_bytes: u64 = 0;
    let mut workload_costs: Vec<WorkloadCost> = Vec::new();

    for rec in recommendations {
        // For static analysis, estimate waste from current vs recommended
        let cpu_waste = parse_cpu_to_millicores(&rec.current.cpu_request)
            .saturating_sub(parse_cpu_to_millicores(&rec.recommended.cpu_request));

        let memory_waste = parse_memory_to_bytes(&rec.current.memory_request)
            .saturating_sub(parse_memory_to_bytes(&rec.recommended.memory_request));

        total_cpu_waste_millicores += cpu_waste;
        total_memory_waste_bytes += memory_waste;

        let cpu_cores = cpu_waste as f64 / 1000.0;
        let memory_gb = memory_waste as f64 / (1024.0 * 1024.0 * 1024.0);

        let monthly_cost =
            (cpu_cores * cpu_hourly * HOURS_PER_MONTH) + (memory_gb * mem_hourly * HOURS_PER_MONTH);

        if monthly_cost > 0.01 {
            workload_costs.push(WorkloadCost {
                namespace: rec
                    .namespace
                    .clone()
                    .unwrap_or_else(|| "default".to_string()),
                workload_name: rec.resource_name.clone(),
                monthly_cost: round_cost(monthly_cost),
                monthly_savings: round_cost(monthly_cost),
            });
        }
    }

    let cpu_cores = total_cpu_waste_millicores as f64 / 1000.0;
    let memory_gb = total_memory_waste_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

    let cpu_monthly = cpu_cores * cpu_hourly * HOURS_PER_MONTH;
    let mem_monthly = memory_gb * mem_hourly * HOURS_PER_MONTH;
    let monthly_waste = cpu_monthly + mem_monthly;

    workload_costs.sort_by(|a, b| {
        b.monthly_cost
            .partial_cmp(&a.monthly_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    CostEstimation {
        provider,
        region: region.to_string(),
        monthly_waste_cost: round_cost(monthly_waste),
        annual_waste_cost: round_cost(monthly_waste * 12.0),
        monthly_savings: round_cost(monthly_waste),
        annual_savings: round_cost(monthly_waste * 12.0),
        currency: "USD".to_string(),
        breakdown: CostBreakdown {
            cpu_cost: round_cost(cpu_monthly),
            memory_cost: round_cost(mem_monthly),
        },
        workload_costs,
    }
}

/// Get pricing for a cloud provider.
fn get_pricing(provider: &CloudProvider) -> (f64, f64) {
    match provider {
        CloudProvider::Aws => (AWS_CPU_HOURLY, AWS_MEM_HOURLY),
        CloudProvider::Gcp => (GCP_CPU_HOURLY, GCP_MEM_HOURLY),
        CloudProvider::Azure => (AZURE_CPU_HOURLY, AZURE_MEM_HOURLY),
        CloudProvider::OnPrem => (ONPREM_CPU_HOURLY, ONPREM_MEM_HOURLY),
        CloudProvider::Unknown => (AWS_CPU_HOURLY, AWS_MEM_HOURLY), // Default to AWS
    }
}

/// Parse CPU string (e.g., "100m", "1.5") to millicores.
fn parse_cpu_to_millicores(cpu: &Option<String>) -> u64 {
    let cpu_str = match cpu {
        Some(s) => s,
        None => return 0,
    };

    if cpu_str.ends_with('m') {
        cpu_str.trim_end_matches('m').parse().unwrap_or(0)
    } else {
        // Full cores
        let cores: f64 = cpu_str.parse().unwrap_or(0.0);
        (cores * 1000.0) as u64
    }
}

/// Parse memory string (e.g., "128Mi", "1Gi") to bytes.
fn parse_memory_to_bytes(memory: &Option<String>) -> u64 {
    let mem_str = match memory {
        Some(s) => s,
        None => return 0,
    };

    let mem_str = mem_str.trim();

    if mem_str.ends_with("Gi") {
        let val: f64 = mem_str.trim_end_matches("Gi").parse().unwrap_or(0.0);
        (val * 1024.0 * 1024.0 * 1024.0) as u64
    } else if mem_str.ends_with("Mi") {
        let val: f64 = mem_str.trim_end_matches("Mi").parse().unwrap_or(0.0);
        (val * 1024.0 * 1024.0) as u64
    } else if mem_str.ends_with("Ki") {
        let val: f64 = mem_str.trim_end_matches("Ki").parse().unwrap_or(0.0);
        (val * 1024.0) as u64
    } else {
        // Assume bytes
        mem_str.parse().unwrap_or(0)
    }
}

/// Round cost to 2 decimal places.
fn round_cost(cost: f64) -> f64 {
    (cost * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu() {
        assert_eq!(parse_cpu_to_millicores(&Some("100m".to_string())), 100);
        assert_eq!(parse_cpu_to_millicores(&Some("1".to_string())), 1000);
        assert_eq!(parse_cpu_to_millicores(&Some("1.5".to_string())), 1500);
        assert_eq!(parse_cpu_to_millicores(&None), 0);
    }

    #[test]
    fn test_parse_memory() {
        assert_eq!(
            parse_memory_to_bytes(&Some("128Mi".to_string())),
            128 * 1024 * 1024
        );
        assert_eq!(
            parse_memory_to_bytes(&Some("1Gi".to_string())),
            1024 * 1024 * 1024
        );
        assert_eq!(parse_memory_to_bytes(&None), 0);
    }

    #[test]
    fn test_round_cost() {
        assert_eq!(round_cost(10.1234), 10.12);
        assert_eq!(round_cost(10.125), 10.13);
    }
}
