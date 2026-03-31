---
description: "Audit project dependencies for licenses, production vs development split, and detailed package analysis using the Syncable CLI sync-ctl tool"
---

## Purpose

Analyze project dependencies in detail: list all packages, check license types, separate production from development dependencies, and optionally flag vulnerabilities inline. Use this for license compliance and dependency inventory.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory

## Commands

### Full dependency analysis with licenses

```bash
sync-ctl dependencies <PATH> --licenses --agent
```

### Production dependencies only

```bash
sync-ctl dependencies <PATH> --licenses --prod-only --agent
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--agent` | Compressed output for agent consumption (always use) |
| `--licenses` | Include license information for each dependency |
| `--vulnerabilities` | Quick inline vulnerability check (for thorough CVE scanning, use the standalone `sync-ctl vulnerabilities` command instead) |
| `--prod-only` | Show only production dependencies |
| `--dev-only` | Show only development dependencies |

## Output Interpretation

**Priority for reporting to user:**
1. License concerns (copyleft in commercial projects, unknown licenses)
2. Dependency counts (prod vs dev)
3. Specific packages only if asked

**When to use `--vulnerabilities` vs standalone `vulnerabilities` command:**
- Use `--vulnerabilities` here for a quick inline check alongside license info
- Use `sync-ctl vulnerabilities` for a dedicated, thorough CVE scan

## Reading Results

When you use `--agent`, the output is a **compressed summary** with counts, license distribution, and source breakdown. Individual package details are NOT in the compressed output — use `sync-ctl retrieve` to get them.

**What's in the compressed output:**
- `total` — total dependency count
- `production` / `development` — prod vs dev split
- `by_source` — counts per ecosystem (npm, crates.io, pypi, etc.)
- `by_license` — license distribution
- `full_data_ref` — reference ID for the full data

**To get individual package details, use retrieve:**
```bash
# Get the full dependency list
sync-ctl retrieve <ref_id>

# Search for a specific package
sync-ctl retrieve <ref_id> --query "file:package.json"
```

Results are paginated (default 20). Use `--limit N --offset M` for more.

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `No dependencies found` | No package manager files | Verify project path, run `sync-ctl analyze` to check for supported package managers |
| Incomplete results | Some package managers not fully parsed | Note which ecosystems were scanned and which may be missing |

## Examples

**Full audit with licenses:**
```bash
sync-ctl dependencies . --licenses --agent
```

**Production-only for license compliance:**
```bash
sync-ctl dependencies . --licenses --prod-only --agent
```

**Quick vulnerability check alongside deps:**
```bash
sync-ctl dependencies . --licenses --vulnerabilities --agent
```
