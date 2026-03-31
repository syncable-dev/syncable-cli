---
name: syncable-security
description: Use when the user asks to scan for secrets, find leaked credentials, check for API keys in code, detect hardcoded passwords, review code security, or run a secrets scan
allowed-tools:
  - Bash
user-invocable: true
---

## Overview

Scan a codebase for leaked secrets (API keys, tokens, passwords, private keys), insecure code patterns, and configuration issues. Returns findings with severity, file locations, and remediation.

## Mode Selection

Always pass `--mode` explicitly:

| Mode | When to use |
|------|------------|
| `lightning` | Quick check, critical files only (.env, configs) |
| `fast` | Smart sampling, large repos during development |
| `balanced` | **Default.** Good coverage with optimizations |
| `thorough` | Pre-deployment, PR security reviews |
| `paranoid` | Compliance audits, production reviews |

## Flags

| Flag | Purpose |
|------|---------|
| `--mode <MODE>` | Scan depth (always specify) |
| `--agent` | Compressed output (always use) |
| `--include-low` | Include low-severity findings |
| `--no-secrets` | Skip secrets detection (code patterns only) |
| `--no-code-patterns` | Skip code patterns (secrets only) |
| `--fail-on-findings` | Exit with error code if findings exist (CI) |
| `--output <FILE>` | Write report to file |

## Steps

### 1. Run scan

```bash
sync-ctl security <PATH> --mode balanced --agent
```

**Success criteria:** JSON output with `summary` containing severity counts.

### 2. Report to user

Priority order: critical findings (leaked secrets) > high (insecure patterns) > summary score > remediation steps.

### 3. Retrieve details (if needed)

Compressed output only includes critical + first 10 high findings. Medium/low are counts only. Use retrieve for details:

```bash
sync-ctl retrieve <ref_id> --query "severity:medium"
sync-ctl retrieve <ref_id> --query "file:src/auth.rs"
sync-ctl retrieve <ref_id> --query "code:hardcoded-secret"
```

Results paginated (default 20). Use `--limit N --offset M` for more.

**Available queries:** `severity:critical|high|medium|low|info`, `file:<path>`, `code:<id>`

## Error Handling

| Error | Action |
|-------|--------|
| `No such file or directory` | Ask user to verify path |
| Very slow scan | Suggest `balanced` or `fast` mode instead |
| No findings with `lightning`/`fast` | Re-run with `balanced` for deeper coverage |
