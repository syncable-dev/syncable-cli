---
name: syncable-analyze
description: Use when the user asks to analyze a project, understand the tech stack, detect frameworks, check what languages are used, identify runtimes or package managers, or as a first step before security/vulnerability scans
allowed-tools:
  - Bash
user-invocable: true
---

## Overview

Detect a project's tech stack — languages, frameworks, runtimes, package managers, dependencies, Docker presence, monorepo structure. Foundation command; most workflows start here.

## Quick Reference

| Flag | Purpose |
|------|---------|
| `--agent` | Compressed output for agent consumption (always use) |
| `--display {matrix\|detailed\|summary}` | Human-readable format |
| `--only <filters>` | Comma-separated: `languages`, `frameworks`, `dependencies` |

## Steps

### 1. Run analysis

```bash
sync-ctl analyze <PATH> --agent
```

**Success criteria:** JSON output with `summary` and `full_data_ref` fields present.

### 2. Report to user

Prioritize: primary language, main framework, runtime version, Docker/K8s presence.

### 3. Drill into details (if needed)

Save the `full_data_ref`. Use `sync-ctl retrieve` — do NOT re-run analyze:

```bash
sync-ctl retrieve <ref_id> --query "section:frameworks"
sync-ctl retrieve <ref_id> --query "section:languages"
sync-ctl retrieve <ref_id> --query "project:<name>"    # monorepos
sync-ctl retrieve <ref_id> --query "language:Go"
sync-ctl retrieve <ref_id> --query "framework:React"
```

**Available queries:** `section:summary`, `section:frameworks`, `section:languages`, `language:<name>`, `framework:<name>`, `project:<name>`, `compact:true`

## Error Handling

| Error | Action |
|-------|--------|
| `No such file or directory` | Ask user to verify path |
| Empty output | No supported project files. Run `sync-ctl support` |
| Timeout on large monorepo | Try `--only languages` for partial scan |
