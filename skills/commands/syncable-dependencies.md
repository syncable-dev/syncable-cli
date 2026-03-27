---
name: syncable-dependencies
description: Use when auditing project dependencies for licenses, production/dev split, or detailed dependency analysis using Syncable CLI. Trigger on: "license audit", "list dependencies", "dependency analysis", "what licenses am I using", "show me all packages".
---

## Purpose

Analyze project dependencies in detail: list all packages, check license types, separate production from development dependencies, and optionally flag vulnerabilities inline. Use this for license compliance and dependency inventory.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory

## Commands

### Full dependency analysis with licenses

```bash
sync-ctl dependencies <PATH> --licenses --format json
```

### Production dependencies only

```bash
sync-ctl dependencies <PATH> --licenses --prod-only --format json
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--format json` | Machine-readable output (always use) |
| `--licenses` | Include license information for each dependency |
| `--vulnerabilities` | Quick inline vulnerability check (for thorough CVE scanning, use the standalone `sync-ctl vulnerabilities` command instead) |
| `--prod-only` | Show only production dependencies |
| `--dev-only` | Show only development dependencies |

## Output Interpretation

The JSON output contains:

- **dependencies** — array of packages with name, version, license, and prod/dev classification
- **summary** — total counts, license distribution

**Priority for reporting to user:**
1. License concerns (copyleft in commercial projects, unknown licenses)
2. Dependency counts (prod vs dev)
3. Specific packages only if asked

**When to use `--vulnerabilities` vs standalone `vulnerabilities` command:**
- Use `--vulnerabilities` here for a quick inline check alongside license info
- Use `sync-ctl vulnerabilities` for a dedicated, thorough CVE scan

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `No dependencies found` | No package manager files | Verify project path, run `sync-ctl analyze` to check for supported package managers |
| Incomplete results | Some package managers not fully parsed | Note which ecosystems were scanned and which may be missing |

## Examples

**Full audit with licenses:**
```bash
sync-ctl dependencies . --licenses --format json
```

**Production-only for license compliance:**
```bash
sync-ctl dependencies . --licenses --prod-only --format json
```

**Quick vulnerability check alongside deps:**
```bash
sync-ctl dependencies . --licenses --vulnerabilities --format json
```
