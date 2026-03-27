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
sync-ctl analyze <PATH> --json
```

### Step 3: Pre-deploy security audit

Execute the `syncable-security-audit` workflow inline (all its steps and decision logic). **Note:** Step 2's analyze output is reused here — do not re-run analyze.

1. `sync-ctl security <PATH> --mode paranoid --format json`
2. `sync-ctl vulnerabilities <PATH> --format json`
3. `sync-ctl validate <PATH>` (if IaC files exist per Step 2's analysis)

**CRITICAL GATE:** If the security audit finds **critical** findings:
- Present all critical findings to the user
- Explicitly warn: "Critical security findings detected. Deploying with these issues is not recommended."
- Ask the user whether to proceed or abort
- **Never deploy silently when critical findings exist**

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
