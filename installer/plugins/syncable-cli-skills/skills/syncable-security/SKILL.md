---
description: "Scan code for leaked secrets, credentials, API keys, hardcoded passwords, and insecure code patterns using the Syncable CLI sync-ctl security scanner"
---

## Purpose

Perform security analysis on a codebase: detect leaked secrets (API keys, tokens, passwords, private keys), identify insecure code patterns, and analyze configuration security. Returns findings with severity levels, file locations, and remediation guidance.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory

## Commands

### Standard security scan

```bash
sync-ctl security <PATH> --mode balanced --agent
```

### Mode Selection Guide

Always pass `--mode` explicitly. Choose based on context:

| Mode | When to use | Speed |
|------|------------|-------|
| `lightning` | Quick check, only critical files (.env, configs) | Fastest |
| `fast` | Smart sampling, good for large repos during development | Fast |
| `balanced` | **Default choice.** Good coverage with optimizations | Medium |
| `thorough` | Pre-deployment reviews, PR security checks | Slow |
| `paranoid` | Compliance audits, production security reviews | Slowest |

### Key Flags

| Flag | Purpose |
|------|---------|
| `--mode {lightning\|fast\|balanced\|thorough\|paranoid}` | Scan depth (always specify) |
| `--agent` | Compressed output for agent consumption (always use when processing results) |
| `--include-low` | Include low-severity findings (off by default) |
| `--no-secrets` | Skip secrets detection (only code patterns) |
| `--no-code-patterns` | Skip code pattern analysis (only secrets) |
| `--fail-on-findings` | Exit with error code if findings exist (for CI) |
| `--output <FILE>` | Write report to file |

## Output Interpretation

**Priority for reporting to user:**
1. Critical findings first (leaked secrets, hardcoded credentials)
2. High findings (insecure patterns)
3. Summary with score
4. Remediation steps for top findings

## Reading Results

When you use `--agent`, the output is a **compressed summary** — NOT the full data. Only critical and high findings are inline. To get medium/low details, you MUST use `sync-ctl retrieve`.

**What's in the compressed output:**
- `summary` — counts by severity
- `critical_issues` — full details for every critical finding
- `high_issues` — first 10 high-severity findings
- `patterns` — deduplicated medium/low findings as counts only (NO individual details)
- `full_data_ref` — reference ID for the full stored data

**IMPORTANT: Do NOT try to extract medium/low details from the compressed output. Use retrieve instead.**

```bash
# Get medium-severity findings
sync-ctl retrieve <ref_id> --query "severity:medium"

# Get findings for a specific file
sync-ctl retrieve <ref_id> --query "file:src/auth.rs"

# Get findings by rule code
sync-ctl retrieve <ref_id> --query "code:hardcoded-secret"
```

Results are paginated (default 20). Use `--limit N --offset M` for more.

**Available query filters:** `severity:critical|high|medium|low|info`, `file:<path>`, `code:<id>`

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `No such file or directory` | Invalid path | Ask user to verify the project path |
| Very slow scan | Large repo with `thorough`/`paranoid` mode | Suggest trying `balanced` or `fast` mode first |
| No findings | Clean project or scan mode too light | If `lightning`/`fast`, suggest re-running with `balanced` for deeper coverage |

## Examples

**Quick secrets check on current directory:**
```bash
sync-ctl security . --mode balanced --agent
```

**Deep pre-deploy audit:**
```bash
sync-ctl security . --mode paranoid --agent
```

**Secrets-only scan (skip code patterns):**
```bash
sync-ctl security . --mode thorough --no-code-patterns --agent
```

**Save report to file:**
```bash
sync-ctl security . --mode thorough --agent --output security-report.json
```
