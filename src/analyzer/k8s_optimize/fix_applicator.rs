//! Precise Fix Locator and Applicator
//!
//! Locates exact positions of resource definitions in YAML files and applies
//! targeted fixes with safety measures (backups, dry-run, validation).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::live_analyzer::LiveRecommendation;
use super::types::{
    FixApplicationResult, FixImpact, FixResourceValues, FixRisk, FixSource, FixStatus, PreciseFix,
    ResourceRecommendation, Severity,
};

/// YAML location info for a resource.
#[derive(Debug, Clone)]
pub struct YamlLocation {
    /// Line number where the resource starts (1-indexed)
    pub start_line: u32,
    /// Line number where the resources section starts
    pub resources_line: Option<u32>,
    /// Column where resources starts
    pub resources_column: Option<u32>,
    /// Full path within the YAML (for nested resources like Helm)
    pub yaml_path: String,
}

/// Locate resources in a YAML file and return precise fix locations.
pub fn locate_resources_in_file(
    file_path: &Path,
    recommendations: &[LiveRecommendation],
) -> Vec<PreciseFix> {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut fixes = Vec::new();

    // Parse YAML documents
    for doc in yaml_rust2::YamlLoader::load_from_str(&content).unwrap_or_default() {
        // Find workloads in the document
        let locations = find_workload_locations(&content, &doc);

        for rec in recommendations {
            if let Some(loc) =
                locations.get(&(rec.workload_name.clone(), rec.container_name.clone()))
            {
                let fix = create_precise_fix(file_path, rec, loc);
                fixes.push(fix);
            }
        }
    }

    fixes
}

/// Locate resources from static analysis recommendations.
pub fn locate_resources_from_static(recommendations: &[ResourceRecommendation]) -> Vec<PreciseFix> {
    let mut fixes = Vec::new();

    for rec in recommendations {
        // Static recommendations include file path
        let fix = PreciseFix {
            id: generate_fix_id(&rec.resource_name, &rec.container),
            file_path: rec.file_path.clone(),
            line_number: rec.line.unwrap_or(0),
            column: None,
            resource_kind: rec.resource_kind.clone(),
            resource_name: rec.resource_name.clone(),
            container_name: rec.container.clone(),
            namespace: rec.namespace.clone(),
            current: FixResourceValues {
                cpu_request: rec.current.cpu_request.clone(),
                cpu_limit: rec.current.cpu_limit.clone(),
                memory_request: rec.current.memory_request.clone(),
                memory_limit: rec.current.memory_limit.clone(),
            },
            recommended: FixResourceValues {
                cpu_request: rec.recommended.cpu_request.clone(),
                cpu_limit: rec.recommended.cpu_limit.clone(),
                memory_request: rec.recommended.memory_request.clone(),
                memory_limit: rec.recommended.memory_limit.clone(),
            },
            confidence: severity_to_confidence(&rec.severity),
            source: FixSource::StaticAnalysis,
            impact: assess_impact(rec),
            status: FixStatus::Pending,
        };
        fixes.push(fix);
    }

    fixes
}

/// Find workload locations in YAML content.
fn find_workload_locations(
    content: &str,
    _doc: &yaml_rust2::Yaml,
) -> HashMap<(String, String), YamlLocation> {
    let mut locations = HashMap::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut current_kind = String::new();
    let mut current_name = String::new();
    let mut current_container = String::new();
    let mut workload_start_line: u32 = 0;
    let mut in_containers = false;
    let mut resources_line: Option<u32> = None;

    for (idx, line) in lines.iter().enumerate() {
        let line_num = (idx + 1) as u32;
        let trimmed = line.trim();

        // Detect kind
        if trimmed.starts_with("kind:") {
            current_kind = trimmed.trim_start_matches("kind:").trim().to_string();
            workload_start_line = line_num;
            current_name.clear();
            current_container.clear();
            in_containers = false;
            resources_line = None;
        }

        // Detect metadata name
        if trimmed.starts_with("name:") && !in_containers {
            current_name = trimmed.trim_start_matches("name:").trim().to_string();
        }

        // Detect containers section
        if trimmed == "containers:" {
            in_containers = true;
        }

        // Detect container name
        if in_containers && trimmed.starts_with("- name:") {
            current_container = trimmed.trim_start_matches("- name:").trim().to_string();
        }

        // Detect resources section
        if in_containers && trimmed == "resources:" {
            resources_line = Some(line_num);

            // Only add if we have all the info
            if !current_name.is_empty() && !current_container.is_empty() {
                let key = (current_name.clone(), current_container.clone());
                locations.insert(
                    key,
                    YamlLocation {
                        start_line: workload_start_line,
                        resources_line,
                        resources_column: Some(line.len() as u32 - trimmed.len() as u32),
                        yaml_path: format!(
                            "{}/{}/containers/{}/resources",
                            current_kind, current_name, current_container
                        ),
                    },
                );
            }
        }
    }

    locations
}

/// Create a precise fix from a live recommendation.
fn create_precise_fix(
    file_path: &Path,
    rec: &LiveRecommendation,
    loc: &YamlLocation,
) -> PreciseFix {
    let cpu_str = format_millicores(rec.recommended_cpu_millicores);
    let mem_str = format_bytes(rec.recommended_memory_bytes);

    // Current values
    let current_cpu = rec.current_cpu_millicores.map(format_millicores);
    let current_mem = rec.current_memory_bytes.map(format_bytes);

    PreciseFix {
        id: generate_fix_id(&rec.workload_name, &rec.container_name),
        file_path: file_path.to_path_buf(),
        line_number: loc.resources_line.unwrap_or(loc.start_line),
        column: loc.resources_column,
        resource_kind: rec.workload_kind.clone(),
        resource_name: rec.workload_name.clone(),
        container_name: rec.container_name.clone(),
        namespace: Some(rec.namespace.clone()),
        current: FixResourceValues {
            cpu_request: current_cpu.clone(),
            cpu_limit: current_cpu.map(|c| double_millicores(&c)),
            memory_request: current_mem.clone(),
            memory_limit: current_mem.clone(),
        },
        recommended: FixResourceValues {
            cpu_request: Some(cpu_str.clone()),
            cpu_limit: Some(double_millicores(&cpu_str)),
            memory_request: Some(mem_str.clone()),
            memory_limit: Some(mem_str),
        },
        confidence: rec.confidence,
        source: match rec.data_source {
            super::live_analyzer::DataSource::Prometheus => FixSource::PrometheusP95,
            super::live_analyzer::DataSource::MetricsServer => FixSource::MetricsServer,
            super::live_analyzer::DataSource::Combined => FixSource::Combined,
            super::live_analyzer::DataSource::Static => FixSource::StaticAnalysis,
        },
        impact: FixImpact {
            risk: if rec.confidence >= 80 {
                FixRisk::Low
            } else if rec.confidence >= 60 {
                FixRisk::Medium
            } else {
                FixRisk::High
            },
            monthly_savings: 0.0, // Will be calculated by cost estimator
            oom_risk: rec.memory_waste_pct < -10.0, // Reducing memory below current usage
            throttle_risk: rec.cpu_waste_pct < -10.0, // Reducing CPU below current usage
            recommendation: if rec.confidence >= 80 {
                "Safe to apply - high confidence based on observed usage".to_string()
            } else if rec.confidence >= 60 {
                "Review before applying - moderate confidence".to_string()
            } else {
                "Manual review required - limited data available".to_string()
            },
        },
        status: FixStatus::Pending,
    }
}

/// Apply fixes to files.
pub fn apply_fixes(
    fixes: &mut [PreciseFix],
    backup_dir: Option<&Path>,
    dry_run: bool,
    min_confidence: u8,
) -> FixApplicationResult {
    let mut applied = 0;
    let mut skipped = 0;
    let mut failed = 0;
    let mut errors = Vec::new();

    // Create backup directory if requested
    let backup_path = if !dry_run {
        if let Some(dir) = backup_dir {
            match fs::create_dir_all(dir) {
                Ok(_) => Some(dir.to_path_buf()),
                Err(e) => {
                    errors.push(format!("Failed to create backup dir: {}", e));
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Group fixes by file
    let mut fixes_by_file: HashMap<PathBuf, Vec<&mut PreciseFix>> = HashMap::new();
    for fix in fixes.iter_mut() {
        fixes_by_file
            .entry(fix.file_path.clone())
            .or_default()
            .push(fix);
    }

    // Process each file
    for (file_path, file_fixes) in fixes_by_file.iter_mut() {
        // Read file content
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("Failed to read {}: {}", file_path.display(), e));
                for fix in file_fixes.iter_mut() {
                    fix.status = FixStatus::Failed;
                    failed += 1;
                }
                continue;
            }
        };

        // Create backup if not dry run
        if !dry_run {
            if let Some(ref backup) = backup_path {
                let backup_file = backup.join(file_path.file_name().unwrap_or_default());
                if let Err(e) = fs::write(&backup_file, &content) {
                    errors.push(format!("Failed to backup {}: {}", file_path.display(), e));
                }
            }
        }

        let mut modified_content = content.clone();
        let mut line_offset: i32 = 0;

        // Sort fixes by line number (descending) to avoid offset issues
        file_fixes.sort_by(|a, b| b.line_number.cmp(&a.line_number));

        for fix in file_fixes.iter_mut() {
            // Check confidence threshold
            if fix.confidence < min_confidence {
                fix.status = FixStatus::Skipped;
                skipped += 1;
                continue;
            }

            // Check risk level
            if fix.impact.risk == FixRisk::Critical {
                fix.status = FixStatus::Skipped;
                skipped += 1;
                continue;
            }

            // Apply the fix
            match apply_single_fix(&mut modified_content, fix, &mut line_offset) {
                Ok(_) => {
                    fix.status = if dry_run {
                        FixStatus::Pending
                    } else {
                        FixStatus::Applied
                    };
                    applied += 1;
                }
                Err(e) => {
                    fix.status = FixStatus::Failed;
                    errors.push(format!("Fix {} failed: {}", fix.id, e));
                    failed += 1;
                }
            }
        }

        // Write modified content if not dry run
        if !dry_run && applied > 0 {
            if let Err(e) = fs::write(file_path, &modified_content) {
                errors.push(format!("Failed to write {}: {}", file_path.display(), e));
            }
        }
    }

    FixApplicationResult {
        total_fixes: fixes.len(),
        applied,
        skipped,
        failed,
        backup_path,
        fixes: fixes.to_vec(),
        errors,
    }
}

/// Apply a single fix to the content.
fn apply_single_fix(
    content: &mut String,
    fix: &PreciseFix,
    _line_offset: &mut i32,
) -> Result<(), String> {
    let lines: Vec<&str> = content.lines().collect();

    // Find the resources section for this container
    let target_line = fix.line_number as usize;

    if target_line == 0 || target_line > lines.len() {
        return Err(format!("Invalid line number: {}", target_line));
    }

    // Build the new resources YAML
    let indent = detect_indent(&lines, target_line - 1);
    let new_resources = generate_resources_yaml(fix, &indent);

    // Find end of current resources section
    let (start_idx, end_idx) = find_resources_section(&lines, target_line - 1)?;

    // Replace the section
    let mut new_lines: Vec<String> = Vec::new();
    new_lines.extend(lines[..start_idx].iter().map(|s| s.to_string()));
    new_lines.push(new_resources);
    new_lines.extend(lines[end_idx..].iter().map(|s| s.to_string()));

    *content = new_lines.join("\n");

    Ok(())
}

/// Find the resources section boundaries.
fn find_resources_section(lines: &[&str], start: usize) -> Result<(usize, usize), String> {
    let base_indent = lines
        .get(start)
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);

    // Find the end of resources section
    let mut end = start + 1;
    while end < lines.len() {
        let line = lines[end];
        let trimmed = line.trim_start();

        // Empty lines are part of the section
        if trimmed.is_empty() {
            end += 1;
            continue;
        }

        let current_indent = line.len() - trimmed.len();

        // If we're back to base indent or less, we've exited the section
        if current_indent <= base_indent && !trimmed.starts_with('-') {
            break;
        }

        end += 1;
    }

    Ok((start, end))
}

/// Detect indentation at a line.
fn detect_indent(lines: &[&str], line_idx: usize) -> String {
    lines
        .get(line_idx)
        .map(|l| {
            let trimmed = l.trim_start();
            let indent_len = l.len() - trimmed.len();
            " ".repeat(indent_len)
        })
        .unwrap_or_else(|| "        ".to_string()) // Default 8 spaces
}

/// Generate YAML for resources section.
fn generate_resources_yaml(fix: &PreciseFix, indent: &str) -> String {
    let child_indent = format!("{}  ", indent);

    let mut yaml = format!("{}resources:\n", indent);
    yaml.push_str(&format!("{}requests:\n", child_indent));

    if let Some(ref cpu) = fix.recommended.cpu_request {
        yaml.push_str(&format!("{}  cpu: \"{}\"\n", child_indent, cpu));
    }
    if let Some(ref mem) = fix.recommended.memory_request {
        yaml.push_str(&format!("{}  memory: \"{}\"\n", child_indent, mem));
    }

    yaml.push_str(&format!("{}limits:\n", child_indent));

    if let Some(ref cpu) = fix.recommended.cpu_limit {
        yaml.push_str(&format!("{}  cpu: \"{}\"\n", child_indent, cpu));
    }
    if let Some(ref mem) = fix.recommended.memory_limit {
        yaml.push_str(&format!("{}  memory: \"{}\"", child_indent, mem));
    }

    yaml
}

/// Generate a unique fix ID.
fn generate_fix_id(workload: &str, container: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("fix-{}-{}-{}", workload, container, ts % 10000)
}

/// Convert severity to confidence score.
fn severity_to_confidence(severity: &Severity) -> u8 {
    match severity {
        Severity::Critical => 95,
        Severity::High => 80,
        Severity::Medium => 60,
        Severity::Low => 40,
        Severity::Info => 20,
    }
}

/// Assess impact of a static recommendation.
fn assess_impact(rec: &ResourceRecommendation) -> FixImpact {
    let risk = match rec.severity {
        Severity::Critical | Severity::High => FixRisk::High,
        Severity::Medium => FixRisk::Medium,
        _ => FixRisk::Low,
    };

    FixImpact {
        risk,
        monthly_savings: 0.0,
        oom_risk: false,
        throttle_risk: false,
        recommendation: rec.message.clone(),
    }
}

/// Format millicores to K8s CPU string.
fn format_millicores(millicores: u64) -> String {
    if millicores >= 1000 && millicores % 1000 == 0 {
        format!("{}", millicores / 1000)
    } else {
        format!("{}m", millicores)
    }
}

/// Double the millicores value for limits.
fn double_millicores(value: &str) -> String {
    if value.ends_with('m') {
        let m: u64 = value.trim_end_matches('m').parse().unwrap_or(100);
        format!("{}m", m * 2)
    } else {
        let cores: f64 = value.parse().unwrap_or(0.5);
        format!("{}", cores * 2.0)
    }
}

/// Format bytes to K8s memory string.
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 && bytes % (1024 * 1024 * 1024) == 0 {
        format!("{}Gi", bytes / (1024 * 1024 * 1024))
    } else if bytes >= 1024 * 1024 {
        format!("{}Mi", bytes / (1024 * 1024))
    } else if bytes >= 1024 {
        format!("{}Ki", bytes / 1024)
    } else {
        format!("{}", bytes)
    }
}
