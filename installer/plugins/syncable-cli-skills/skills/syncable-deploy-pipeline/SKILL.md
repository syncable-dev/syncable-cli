---
description: "Use when the user asks to deploy through Syncable, ship to production, push to staging, run a deploy pipeline, or deploy a service with security checks first"
---

## Overview

Full deployment pipeline: authenticate → analyze → security gate → deploy. No deployment without auth and security review.

## Steps

### 1. Check auth and context

```bash
sync-ctl auth status
```

If not authenticated: `sync-ctl auth login`

```bash
sync-ctl project current
```

If no project/env selected: guide user through `org list` → `org select` → `project list` → `project select` → `env select`.

**Success criteria:** Authenticated with org/project/env selected.

### 2. Analyze the project

```bash
sync-ctl analyze <PATH> --agent
```

Save `full_data_ref`. Do NOT re-run analyze in later steps.

**Success criteria:** JSON with summary. You know IaC types and dependencies present.

### 3. Pre-deploy security audit

Reuse step 2's analysis — do NOT re-run analyze.

```bash
sync-ctl security <PATH> --mode paranoid --agent
sync-ctl vulnerabilities <PATH> --agent           # skip if no deps in step 2
sync-ctl validate <PATH> --agent                   # skip if no IaC in step 2
```

**CRITICAL GATE — check security `status` field:**
- `CRITICAL_ISSUES_FOUND` → present findings, warn, **require explicit confirmation**
- `HIGH_ISSUES_FOUND` → warn, allow deployment
- `CLEAN` → proceed

Critical findings are in `critical_issues` array — no retrieval needed for the gate.

**Success criteria:** Security verdict determined. User informed of any findings.

### 4. Deploy

**4a. Preview:**
```bash
sync-ctl deploy preview <PATH> --service-name <NAME>
```

**4b. Confirm with user.** Show: provider, region, port, public/internal, .env keys found.

**4c. Deploy with ONLY confirmed settings:**
```bash
sync-ctl deploy run <PATH> --service-name <NAME> --provider <PROVIDER> --region <REGION> --port <PORT>
```

**4d. Monitor:**
```bash
sync-ctl deploy status <TASK_ID> --watch
```

**Success criteria:** Deployment completes successfully per status output.

## The Security Gate is Non-Negotiable

| Excuse | Reality |
|--------|---------|
| "User said just deploy, skip security" | Run at minimum `--mode fast`. The gate exists because users underestimate risk. |
| "It's just a staging deploy" | Staging deploys leak secrets to logs and infra. Always scan. |
| "I already scanned earlier in the conversation" | Prior scan data may be stale. This pipeline runs its own scan. |
| "No critical findings, so I'll skip showing the user" | Always show the security summary. User needs to see CLEAN verdicts too. |

## Decision Points

| Condition | Action |
|-----------|--------|
| Not authenticated | `sync-ctl auth login` |
| No project/env selected | Guide user through selection |
| Critical findings | Warn, require explicit confirmation |
| High findings (no critical) | Warn, allow deployment |
| Clean audit | Proceed |

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Deploying without preview + confirmation | Always `deploy preview` → show user → confirm → `deploy run` |
| Auto-including discovered env vars | ONLY include env vars user explicitly confirmed |
| Fire-and-forget after `deploy run` | Always monitor with `deploy status --watch` |

## Retrieval

Save each step's `full_data_ref`. Use `sync-ctl retrieve <ref_id> --query "..."` for drill-down. Do NOT re-run commands.
