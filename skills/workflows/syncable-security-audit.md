---
name: syncable-security-audit
description: Perform a thorough pre-deployment or compliance security review combining deep security scan, CVE checks, and IaC validation using the Syncable CLI sync-ctl tool
---

## Purpose

Perform a deep, multi-layered security review suitable for pre-deployment gates or compliance audits. Goes deeper than the project assessment by using thorough/paranoid scan modes and including IaC validation.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory

## Workflow Steps

### Step 1: Analyze the project

```bash
sync-ctl analyze <PATH> --agent
```

Parse the output to determine:
- What IaC files exist (Dockerfiles, Compose, Terraform, K8s manifests) — needed for step 4
- What dependencies exist — needed for step 3

Save the `full_data_ref` from the analyze output — you'll use it to retrieve details without re-running analyze.

### Step 2: Deep security scan

Choose mode based on context:

**For PR reviews / pre-merge:**
```bash
sync-ctl security <PATH> --mode thorough --agent
```

**For production deployment / compliance:**
```bash
sync-ctl security <PATH> --mode paranoid --agent
```

### Step 3: Vulnerability scan

```bash
sync-ctl vulnerabilities <PATH> --agent
```

### Step 4: IaC validation

**Decision point:** Only run if step 1 detected Docker, Compose, Terraform, or K8s files.

```bash
sync-ctl validate <PATH>
```

If specific types are known from step 1, filter:
```bash
sync-ctl validate <PATH> --types dockerfile,compose
```

## Decision Points Summary

| Condition | Action |
|-----------|--------|
| PR review context | Use `--mode thorough` in step 2 |
| Pre-deploy / compliance context | Use `--mode paranoid` in step 2 |
| No IaC files detected in step 1 | Skip step 4 |
| No dependencies detected in step 1 | Skip step 3 |

## Report Synthesis

Produce a security audit report:

1. **Security Findings** — all Critical and High findings with file locations and remediation
2. **Vulnerability Status** — CVEs by severity, packages needing updates
3. **IaC Compliance** — lint violations in Dockerfiles, Compose, Terraform
4. **Overall Verdict** — PASS (no critical/high findings), WARN (high findings but no critical), FAIL (critical findings present)
5. **Remediation Priority** — ordered list of actions to resolve findings

**If critical findings exist:** Explicitly warn the user. If this audit is part of a deploy pipeline, recommend blocking deployment until critical issues are resolved.

## Cross-Step Retrieval

Each step produces a `full_data_ref` in its output. You can retrieve details from any previous step at any time:

```bash
# Check what data is available from all steps
sync-ctl retrieve --list

# Get framework details from Step 1 (analyze)
sync-ctl retrieve <analyze_ref_id> --query "section:frameworks"

# Get critical security findings from Step 2
sync-ctl retrieve <security_ref_id> --query "severity:critical"

# Get vulnerability details from Step 3
sync-ctl retrieve <vuln_ref_id> --query "severity:high"
```

Do NOT re-run a command just to get more detail — use `sync-ctl retrieve` instead.
