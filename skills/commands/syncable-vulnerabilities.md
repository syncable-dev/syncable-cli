---
name: syncable-vulnerabilities
description: Use when checking project dependencies for known CVEs or security vulnerabilities using Syncable CLI. Trigger on: "check for CVEs", "vulnerable dependencies", "dependency security", "are my packages safe", "npm audit", "cargo audit".
---

## Purpose

Scan project dependencies for known CVEs (Common Vulnerabilities and Exposures) across npm, pip, cargo, go, and java ecosystems. Returns vulnerable packages with severity, affected versions, and available fixes.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory
- Language-specific scanning tools should be installed. If a scan fails with "tool not found", run `sync-ctl tools install` to install missing scanners.

## Commands

### Scan for vulnerabilities

```bash
sync-ctl vulnerabilities <PATH> --agent
```

### Filter by severity

```bash
sync-ctl vulnerabilities <PATH> --severity high --agent
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--agent` | Compressed output for agent consumption (always use) |
| `--severity {low\|medium\|high\|critical}` | Only show findings at or above this severity |
| `--output <FILE>` | Write report to file |

## Output Interpretation

**Priority for reporting to user:**
1. Critical/High CVEs with available fixes — actionable immediately
2. Critical/High CVEs without fixes — flag as risk
3. Medium/Low — mention count, don't enumerate unless asked

## Reading Results

When you use `--agent`, the output is a compressed summary. All **critical** and **high** findings are included in full detail. Medium/low findings are deduplicated into patterns.

The output JSON includes:
- `status` — e.g., "CRITICAL_ISSUES_FOUND", "HIGH_ISSUES_FOUND", "CLEAN"
- `summary` — counts by severity (total, critical, high, medium, low)
- `critical_issues` — full details for every critical finding
- `high_issues` — first 10 high-severity findings
- `patterns` — deduplicated medium/low findings with counts
- `full_data_ref` — reference ID for retrieving full data
- `retrieval_hint` — exact command for drill-down

To drill into specifics:
```bash
# Get all critical findings
sync-ctl retrieve <ref_id> --query "severity:critical"

# Get findings for a specific file
sync-ctl retrieve <ref_id> --query "file:package.json"
```

**Available query filters:** `severity:<level>`, `file:<path>`

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `tool not found` or scanner missing | Language-specific audit tool not installed | Run `sync-ctl tools install` to install missing scanners, then retry |
| `No dependencies found` | No package manager files detected | Run `sync-ctl analyze <PATH> --agent` first to verify the project has dependencies |
| Timeout | Very large dependency tree | Try scanning specific subdirectories in a monorepo |

## Examples

**Scan current project:**
```bash
sync-ctl vulnerabilities . --agent
```

**Only critical and high severity:**
```bash
sync-ctl vulnerabilities . --severity high --agent
```

**Save report:**
```bash
sync-ctl vulnerabilities . --agent --output vuln-report.json
```

**Install missing scanners first:**
```bash
sync-ctl tools install --yes
sync-ctl vulnerabilities . --agent
```
