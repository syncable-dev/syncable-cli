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
sync-ctl vulnerabilities <PATH> --format json
```

### Filter by severity

```bash
sync-ctl vulnerabilities <PATH> --severity high --format json
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--format json` | Machine-readable output (always use) |
| `--severity {low\|medium\|high\|critical}` | Only show findings at or above this severity |
| `--output <FILE>` | Write report to file |

## Output Interpretation

The JSON output contains an array of vulnerability findings, each with:

- **package** — affected dependency name
- **version** — installed version
- **severity** — Critical, High, Medium, Low
- **cve** — CVE identifier (e.g., CVE-2024-1234)
- **description** — what the vulnerability is
- **fix_version** — version that resolves it (if available)
- **ecosystem** — npm, pip, cargo, go, java

**Priority for reporting to user:**
1. Critical/High CVEs with available fixes — actionable immediately
2. Critical/High CVEs without fixes — flag as risk
3. Medium/Low — mention count, don't enumerate unless asked

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `tool not found` or scanner missing | Language-specific audit tool not installed | Run `sync-ctl tools install` to install missing scanners, then retry |
| `No dependencies found` | No package manager files detected | Run `sync-ctl analyze <PATH> --json` first to verify the project has dependencies |
| Timeout | Very large dependency tree | Try scanning specific subdirectories in a monorepo |

## Examples

**Scan current project:**
```bash
sync-ctl vulnerabilities . --format json
```

**Only critical and high severity:**
```bash
sync-ctl vulnerabilities . --severity high --format json
```

**Save report:**
```bash
sync-ctl vulnerabilities . --format json --output vuln-report.json
```

**Install missing scanners first:**
```bash
sync-ctl tools install --yes
sync-ctl vulnerabilities . --format json
```
