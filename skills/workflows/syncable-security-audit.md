---
name: syncable-security-audit
description: Use when the user asks for a security audit, pre-deployment security review, compliance check, thorough security assessment before shipping, or deep security scan for production readiness
allowed-tools:
  - Bash
user-invocable: true
---

## Overview

Deep multi-layered security review for pre-deployment gates or compliance. Uses thorough/paranoid scan modes and includes IaC validation. Stricter than project-assessment.

## Steps

### 1. Analyze the project

```bash
sync-ctl analyze <PATH> --agent
```

Determine: IaC files present (gates step 4), dependencies present (gates step 3). Save `full_data_ref`.

**Success criteria:** You know which IaC types and dependency files exist.

### 2. Deep security scan

**PR review / pre-merge:**
```bash
sync-ctl security <PATH> --mode thorough --agent
```

**Production / compliance:**
```bash
sync-ctl security <PATH> --mode paranoid --agent
```

**Success criteria:** JSON with severity counts. All critical/high findings captured with file locations.

### 3. Vulnerability scan

```bash
sync-ctl vulnerabilities <PATH> --agent
```

**Decision:** No dependencies in step 1 → skip, note in report.

**Success criteria:** CVE counts by severity captured.

### 4. IaC validation

```bash
sync-ctl validate <PATH> --agent
```

**Decision:** No IaC files in step 1 → skip.

Filter if types known: `--types dockerfile,compose`

**Success criteria:** Lint violations captured with severity and file locations.

## Decision Points

| Condition | Action |
|-----------|--------|
| PR review context | `--mode thorough` in step 2 |
| Pre-deploy / compliance | `--mode paranoid` in step 2 |
| No IaC files in step 1 | Skip step 4 |
| No dependencies in step 1 | Skip step 3 |

## Report: Verdict Format

1. **Security Findings** — critical/high with locations and remediation
2. **Vulnerability Status** — CVEs by severity, packages needing updates
3. **IaC Compliance** — lint violations
4. **Verdict** — PASS (no critical/high) | WARN (high but no critical) | FAIL (critical present)
5. **Remediation Priority** — ordered action list

**If critical findings exist:** Explicitly warn user. If part of deploy pipeline, recommend blocking deployment.

## Common Mistakes

| Mistake | Reality |
|---------|---------|
| "Project looks simple, lightning mode is fine" | Security audits require thorough or paranoid. That's what distinguishes this from project-assessment. |
| Skipping IaC validation because "it's just Dockerfiles" | Dockerfile misconfigurations are a top attack vector. Always validate if IaC exists. |
| Reporting verdict without running all applicable steps | Every applicable step must complete before issuing a PASS/WARN/FAIL verdict. |

## Retrieval

Save each step's `full_data_ref`. Use `sync-ctl retrieve <ref_id> --query "..."` for drill-down. Do NOT re-run commands.
