---
description: "Use when the user asks to check for CVEs, scan dependencies for vulnerabilities, find known security issues in packages, or audit npm/pip/cargo/go/java dependency security"
---

## Overview

Scan project dependencies for known CVEs across npm, pip, cargo, go, and java ecosystems. Returns vulnerable packages with severity, affected versions, and available fixes.

## Flags

| Flag | Purpose |
|------|---------|
| `--agent` | Compressed output (always use) |
| `--severity {low\|medium\|high\|critical}` | Minimum severity threshold |
| `--output <FILE>` | Write report to file |

## Steps

### 1. Run vulnerability scan

```bash
sync-ctl vulnerabilities <PATH> --agent
```

**Success criteria:** JSON output with `summary` containing severity counts.

### 2. Report to user

Priority: critical/high CVEs with fixes (actionable) > critical/high without fixes (risk flag) > medium/low (mention count only, don't enumerate unless asked).

### 3. Retrieve details (if needed)

Compressed output includes critical + first 10 high findings. Medium/low are counts only:

```bash
sync-ctl retrieve <ref_id> --query "severity:medium"
sync-ctl retrieve <ref_id> --query "severity:low"
sync-ctl retrieve <ref_id> --query "file:services/api"
```

Results paginated (default 20). Use `--limit N --offset M` for more.

**Available queries:** `severity:<level>`, `file:<path>`

## Error Handling

| Error | Action |
|-------|--------|
| `tool not found` / scanner missing | Run `sync-ctl tools install --yes`, then retry |
| `No dependencies found` | Run `sync-ctl analyze <PATH> --agent` first to verify dependencies exist |
| Timeout on large dep tree | Try scanning specific subdirectories |
