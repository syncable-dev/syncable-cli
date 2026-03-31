---
name: syncable-validate
description: Use when the user asks to validate Dockerfiles, lint Docker Compose files, check Kubernetes manifests, validate Terraform configs, lint IaC files, or review infrastructure-as-code
allowed-tools:
  - Bash
user-invocable: true
---

## Overview

Validate IaC files against best practices. Covers Dockerfiles (hadolint), Docker Compose (dclint), and Terraform. Reports violations with severity, locations, and auto-fix suggestions.

## Flags

| Flag | Purpose |
|------|---------|
| `--agent` | Compressed output (always use) |
| `--types <list>` | Filter: `dockerfile`, `compose`, `terraform` (comma-separated) |
| `--fix` | Auto-fix issues where possible |

## What Gets Checked

| Type | Linter | Examples |
|------|--------|----------|
| Dockerfile | hadolint (Rust) | Pin versions, avoid `latest`, COPY not ADD |
| Docker Compose | dclint (Rust) | Service naming, volumes, networks (15 rules) |
| Terraform | Terraform validator | Syntax, providers, resource definitions |

## Steps

### 1. Run validation

```bash
sync-ctl validate <PATH> --agent
```

**Success criteria:** JSON output with `status` field (`ERRORS_FOUND`, `WARNINGS_ONLY`, or `CLEAN`).

### 2. Report to user

Priority: errors (build/deploy failures) > warnings (best practice violations) > info (suggestions).

### 3. Retrieve details (if needed)

Compressed output includes all errors in full. Warnings are deduplicated counts:

```bash
sync-ctl retrieve <ref_id> --query "severity:high"
sync-ctl retrieve <ref_id> --query "file:Dockerfile"
sync-ctl retrieve <ref_id> --query "code:DL3006"
```

**Available queries:** `severity:<level>`, `file:<path>`, `code:<id>`

## Error Handling

| Error | Action |
|-------|--------|
| `No IaC files found` | Run `sync-ctl analyze <PATH> --agent` to verify what IaC exists |
| `Unknown type` | Valid types: `dockerfile`, `compose`, `terraform` |
