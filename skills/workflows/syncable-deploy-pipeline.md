---
name: syncable-deploy-pipeline
description: Use when deploying a project through Syncable - orchestrates auth, analysis, security gating, and deployment via Syncable CLI. Trigger on: "deploy this", "push to syncable", "set up deployment", "deploy my project".
---

## Purpose

Orchestrate a full deployment pipeline through the Syncable platform: authenticate, analyze the project, run a security audit as a gate, then deploy. Ensures no deployment happens without authentication and security review.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Internet access for Syncable API
- Agent has access to the project directory

## Workflow Steps

### Step 1: Check authentication and platform context

```bash
sync-ctl auth status
```

**Decision point:** If not authenticated:
```bash
sync-ctl auth login
```

Then verify project/environment context:
```bash
sync-ctl project current
```

**Decision point:** If no project selected:
```bash
sync-ctl org list
# Ask user which org
sync-ctl org select <ORG_ID>
sync-ctl project list
# Ask user which project
sync-ctl project select <PROJECT_ID>
sync-ctl env list
# Ask user which environment
sync-ctl env select <ENV_ID>
```

### Step 2: Analyze the project

```bash
sync-ctl analyze <PATH> --agent
```

Save the `full_data_ref` from the analyze output — do not re-run analyze in later steps; use `sync-ctl retrieve` with this ref_id instead.

### Step 3: Pre-deploy security audit

Execute the `syncable-security-audit` workflow inline (all its steps and decision logic). **Note:** Step 2's analyze output is reused here — do not re-run analyze.

1. `sync-ctl security <PATH> --mode paranoid --agent`
2. `sync-ctl vulnerabilities <PATH> --agent`
3. `sync-ctl validate <PATH>` (if IaC files exist per Step 2's analysis)

**CRITICAL GATE:** Check the security output's `status` field:
- If `status` is "CRITICAL_ISSUES_FOUND": present findings to user, warn, require confirmation
- If `status` is "HIGH_ISSUES_FOUND": warn but allow deployment
- If `status` is "CLEAN": proceed to deploy

All critical findings are in the `critical_issues` array of the compressed output — no retrieval needed for the gate decision.

### Step 4: Deploy

```bash
sync-ctl deploy wizard
```

Then monitor:
```bash
sync-ctl deploy status <TASK_ID> --watch
```

## Decision Points Summary

| Condition | Action |
|-----------|--------|
| Not authenticated | Run `sync-ctl auth login` first |
| No project/env selected | Guide user through selection |
| Critical security findings | Warn user, require explicit confirmation to proceed |
| High security findings (no critical) | Warn user but allow deployment |
| Clean security audit | Proceed to deploy |

## Safety

- **Never deploy without the security gate.** Even if the user says "just deploy", run at least a fast security scan.
- **Always confirm with the user before triggering deployment.** Show them what will be deployed, to which environment.
- **Monitor deployment status** after triggering — don't fire-and-forget.

## Cross-Step Retrieval

Each step produces a `full_data_ref` in its output. You can retrieve details from any previous step at any time:

```bash
# Check what data is available from all steps
sync-ctl retrieve --list

# Get framework details from Step 2 (analyze)
sync-ctl retrieve <analyze_ref_id> --query "section:frameworks"

# Get critical security findings from Step 3
sync-ctl retrieve <security_ref_id> --query "severity:critical"

# Get vulnerability details from Step 3
sync-ctl retrieve <vuln_ref_id> --query "severity:high"
```

Do NOT re-run a command just to get more detail — use `sync-ctl retrieve` instead.
