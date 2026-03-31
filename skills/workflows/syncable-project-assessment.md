---
name: syncable-project-assessment
description: Use when the user asks to assess a project, run a health check, get an overview of project status, evaluate project health, or wants a comprehensive report covering stack, security, vulnerabilities, and dependencies
allowed-tools:
  - Bash
user-invocable: true
---

## Overview

Chain analyze + security + vulnerabilities + dependencies into a unified health report. Each step informs the next via decision points.

## Steps

### 1. Analyze the project stack

```bash
sync-ctl analyze <PATH> --agent
```

Parse output for: languages, frameworks, whether dependencies exist (gates steps 3-4), whether secrets-capable files exist (gates step 2 mode). Save `full_data_ref`.

**Success criteria:** JSON with `summary` field. You know what languages, frameworks, and dependency files are present.

### 2. Security scan

```bash
sync-ctl security <PATH> --mode balanced --agent
```

**Decision:** No config/secrets/env files in step 1 → use `--mode lightning` instead.

**Success criteria:** JSON with severity counts. All critical/high findings captured.

### 3. Vulnerability scan

```bash
sync-ctl vulnerabilities <PATH> --agent
```

**Decision:** No dependencies detected in step 1 → **skip entirely**, note "No dependencies detected" in report.

If scanner missing: `sync-ctl tools install --yes`, then retry.

**Success criteria:** JSON with CVE counts by severity, or step skipped with documented reason.

### 4. Dependency audit

```bash
sync-ctl dependencies <PATH> --licenses --agent
```

**Decision:** Same as step 3 — skip if no dependencies.

**Success criteria:** JSON with total count, prod/dev split, license distribution, or step skipped.

## Decision Points

| Condition | Action |
|-----------|--------|
| No dependencies in step 1 | Skip steps 3 and 4 |
| No secrets-capable files in step 1 | Use `--mode lightning` in step 2 |
| Vulnerability scanner missing | `sync-ctl tools install --yes`, retry |

## Report Synthesis

After all steps, synthesize ONE report:

1. **Tech Stack** — primary language, frameworks, runtimes
2. **Security Score** — score, critical/high count
3. **Vulnerabilities** — critical/high CVE count, packages with fixes
4. **Dependencies** — total count, license concerns
5. **Recommendations** — top 3-5 actions by severity

## Retrieval

Save each step's `full_data_ref`. Use `sync-ctl retrieve <ref_id> --query "..."` for drill-down. Do NOT re-run commands for more detail.
