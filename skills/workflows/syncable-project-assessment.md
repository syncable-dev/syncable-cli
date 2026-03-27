---
name: syncable-project-assessment
description: Use when a user wants a comprehensive project health check - combines stack analysis, security scanning, vulnerability checks, and dependency auditing via Syncable CLI. Trigger on: "assess this project", "full health check", "project overview", "what's the state of this codebase", "onboard me to this repo".
---

## Purpose

Run a comprehensive project health check by chaining multiple Syncable CLI commands. Produces a unified report covering tech stack, security posture, vulnerability status, and dependency health.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory

## Workflow Steps

### Step 1: Analyze the project stack

```bash
sync-ctl analyze <PATH> --json
```

Parse the output to understand:
- What languages and frameworks are present
- Whether dependencies exist (needed for steps 3 and 4)
- Whether secrets-capable files exist (affects step 2 mode)

### Step 2: Security scan

```bash
sync-ctl security <PATH> --mode balanced --format json
```

**Decision point:** If step 1 shows no config files, secrets files, or environment files, use `--mode lightning` instead of `--mode balanced` to save time.

### Step 3: Vulnerability scan

```bash
sync-ctl vulnerabilities <PATH> --format json
```

**Decision point:** If step 1 detected no dependencies (no package.json, requirements.txt, Cargo.toml, go.mod, etc.), **skip this step entirely** and note "No dependencies detected" in the report.

### Step 4: Dependency audit

```bash
sync-ctl dependencies <PATH> --licenses --format json
```

**Decision point:** Same as step 3 — skip if no dependencies detected.

## Decision Points Summary

| Condition | Action |
|-----------|--------|
| No dependencies detected in step 1 | Skip steps 3 and 4 |
| No secrets-capable files in step 1 | Use `--mode lightning` in step 2 |
| Vulnerability scanner missing | Run `sync-ctl tools install --yes`, then retry step 3 |

## Report Synthesis

After all steps complete, synthesize a unified report for the user:

1. **Tech Stack** — primary language, frameworks, runtimes
2. **Security Score** — score from security scan, critical/high finding count
3. **Vulnerabilities** — critical/high CVE count, packages with available fixes
4. **Dependencies** — total count, license concerns (copyleft, unknown)
5. **Recommendations** — top 3-5 actionable items prioritized by severity

## Examples

**Assess current directory:**

The agent runs these commands in sequence, skipping steps based on decision points:

```bash
sync-ctl analyze . --json
sync-ctl security . --mode balanced --format json
sync-ctl vulnerabilities . --format json
sync-ctl dependencies . --licenses --format json
```

Then synthesizes the results into a single report for the user.
