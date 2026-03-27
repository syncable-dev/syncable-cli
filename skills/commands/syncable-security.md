---
name: syncable-security
description: Use when scanning code for secrets, credentials, API keys, or insecure code patterns using Syncable CLI. Trigger on: "scan for secrets", "find leaked credentials", "security scan", "is this code secure", "check for hardcoded passwords".
---

## Purpose

Perform security analysis on a codebase: detect leaked secrets (API keys, tokens, passwords, private keys), identify insecure code patterns, and analyze configuration security. Returns findings with severity levels, file locations, and remediation guidance.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory

## Commands

### Standard security scan

```bash
sync-ctl security <PATH> --mode balanced --format json
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
| `--format json` | Machine-readable output (always use when processing results) |
| `--include-low` | Include low-severity findings (off by default) |
| `--no-secrets` | Skip secrets detection (only code patterns) |
| `--no-code-patterns` | Skip code pattern analysis (only secrets) |
| `--fail-on-findings` | Exit with error code if findings exist (for CI) |
| `--output <FILE>` | Write report to file |

## Output Interpretation

The JSON output contains:

- **findings** — array of security issues, each with:
  - `severity` — Critical, High, Medium, Low, Info
  - `category` — secrets, code_pattern, configuration, infrastructure
  - `file` — exact file path
  - `line` — line number
  - `description` — what was found
  - `remediation` — how to fix it
- **summary** — total counts by severity
- **score** — overall security score (0-100)

**Priority for reporting to user:**
1. Critical findings first (leaked secrets, hardcoded credentials)
2. High findings (insecure patterns)
3. Summary with score
4. Remediation steps for top findings

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `No such file or directory` | Invalid path | Ask user to verify the project path |
| Very slow scan | Large repo with `thorough`/`paranoid` mode | Suggest trying `balanced` or `fast` mode first |
| No findings | Clean project or scan mode too light | If `lightning`/`fast`, suggest re-running with `balanced` for deeper coverage |

## Examples

**Quick secrets check on current directory:**
```bash
sync-ctl security . --mode balanced --format json
```

**Deep pre-deploy audit:**
```bash
sync-ctl security . --mode paranoid --format json
```

**Secrets-only scan (skip code patterns):**
```bash
sync-ctl security . --mode thorough --no-code-patterns --format json
```

**Save report to file:**
```bash
sync-ctl security . --mode thorough --format json --output security-report.json
```
