---
name: syncable-dependencies
description: Use when the user asks to audit dependencies, check licenses, list packages, review dependency health, check for copyleft issues, or see prod vs dev dependency split
allowed-tools:
  - Bash
user-invocable: true
---

## Overview

List all project dependencies with license types, prod/dev split, and ecosystem breakdown. Use for license compliance and dependency inventory.

## Flags

| Flag | Purpose |
|------|---------|
| `--agent` | Compressed output (always use) |
| `--licenses` | Include license info per dependency |
| `--vulnerabilities` | Quick inline vuln check (for thorough CVE scan, use `sync-ctl vulnerabilities` instead) |
| `--prod-only` | Production dependencies only |
| `--dev-only` | Development dependencies only |

## Steps

### 1. Run dependency audit

```bash
sync-ctl dependencies <PATH> --licenses --agent
```

**Success criteria:** JSON output with `total`, `production`/`development` counts, and `by_license` distribution.

### 2. Report to user

Priority: license concerns (copyleft, unknown) > dependency counts (prod vs dev) > specific packages (only if asked).

### 3. Retrieve package details (if needed)

Compressed output has counts and distributions only. Individual packages require retrieve:

```bash
sync-ctl retrieve <ref_id>
sync-ctl retrieve <ref_id> --query "file:package.json"
```

Results paginated (default 20). Use `--limit N --offset M` for more.

## Error Handling

| Error | Action |
|-------|--------|
| `No dependencies found` | Run `sync-ctl analyze` to verify supported package managers exist |
| Incomplete results | Note which ecosystems were scanned vs missing |
