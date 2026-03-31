---
description: "Use when the user asks to optimize Kubernetes resources, right-size pods, estimate cluster costs, detect over-provisioned containers, analyze resource waste, or check K8s configuration drift"
---

## Overview

Analyze K8s manifests and optionally live cluster metrics to recommend resource right-sizing, estimate costs, and detect drift. `--full` adds kubelint + helmlint checks.

## Flags

| Flag | Purpose |
|------|---------|
| `--agent` | Compressed output (always use) |
| `--cluster [CONTEXT]` | Live K8s cluster (current context if omitted) |
| `--prometheus <URL>` | Prometheus for historical metrics |
| `--namespace <NS>` | Target namespace (`*` for all) |
| `--period <DURATION>` | Metrics period (e.g., `7d`, `30d`) |
| `--full` | Include kubelint + helmlint |
| `--cloud-provider {aws\|gcp\|azure\|onprem}` | Cost estimation |
| `--region <REGION>` | Pricing region (default: `us-east-1`) |
| `--fix` | Generate fix suggestions (does NOT modify files) |
| `--apply` | Write fixes to files. Requires `--fix`. **Never use without user confirmation.** |
| `--dry-run` | Preview `--apply` changes |
| `--severity <LEVEL>` | Minimum severity |
| `--threshold <0-100>` | Minimum waste percentage |

## Steps

### 1. Run optimization analysis

```bash
sync-ctl optimize <PATH> --agent
```

**Success criteria:** JSON output with `summary` containing recommendation count and estimated savings.

### 2. Report to user

Priority: right-sizing recommendations with savings > critical security findings (from `--full`) > drift issues > cost breakdown.

### 3. Apply fixes (if requested)

Always follow this sequence — never skip dry-run:

```bash
sync-ctl optimize <PATH> --fix --dry-run --agent   # Preview
# Show user the changes, get explicit confirmation
sync-ctl optimize <PATH> --fix --apply              # Apply only after approval
```

**Success criteria:** User has seen and approved the dry-run output before `--apply`.

### 4. Retrieve details (if needed)

```bash
sync-ctl retrieve <ref_id> --query "severity:high"
sync-ctl retrieve <ref_id> --query "container:my-app"
```

**Available queries:** `severity:<level>`, `container:<name>`

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Using `--apply` without showing dry-run first | Always `--fix --dry-run` first, confirm, then `--fix --apply` |
| Skipping `--cloud-provider` when user asks about costs | Cost estimation requires this flag |
| Running live cluster analysis without checking connectivity | Verify `kubectl cluster-info` first |

## Error Handling

| Error | Action |
|-------|--------|
| `No Kubernetes manifests found` | Run `sync-ctl analyze` to check for K8s presence |
| `Cannot connect to cluster` | Verify `kubectl cluster-info`, check context name |
| `Prometheus unreachable` | Verify URL, fall back to static analysis |
