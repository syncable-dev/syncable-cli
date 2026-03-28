---
name: syncable-optimize
description: Use when optimizing Kubernetes resource requests/limits, analyzing costs, or detecting configuration drift using Syncable CLI. Trigger on: "optimize k8s", "right-size pods", "k8s cost analysis", "resource recommendations", "over-provisioned containers".
---

## Purpose

Analyze Kubernetes manifests and optionally live cluster metrics to recommend resource right-sizing, estimate costs, and detect configuration drift. Can also run kubelint security checks and helmlint validation with `--full` flag.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- For static analysis: K8s manifest files (YAML) in the project
- For live cluster analysis: valid kubeconfig with cluster access
- For cost estimation: `--cloud-provider` flag

## Commands

### Static manifest analysis

```bash
sync-ctl optimize <PATH> --agent
```

### Live cluster analysis

```bash
sync-ctl optimize <PATH> --cluster --agent
sync-ctl optimize <PATH> --cluster my-context --namespace default --agent
```

### With Prometheus metrics

```bash
sync-ctl optimize <PATH> --cluster --prometheus http://localhost:9090 --period 30d --agent
```

### Full analysis (includes kubelint + helmlint)

```bash
sync-ctl optimize <PATH> --full --agent
```

### Cost estimation

```bash
sync-ctl optimize <PATH> --cluster --cloud-provider aws --region us-east-1 --agent
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--agent` | Compressed output for agent consumption (always use) |
| `--cluster [CONTEXT]` | Connect to live K8s cluster (uses current context if no name given) |
| `--prometheus <URL>` | Prometheus URL for historical metrics |
| `--namespace <NS>` | Target namespace (or `*` for all) |
| `--period <DURATION>` | Analysis period for metrics (e.g., `7d`, `30d`) |
| `--full` | Include kubelint security checks + helmlint validation |
| `--cloud-provider {aws\|gcp\|azure\|onprem}` | Cloud provider for cost estimation |
| `--region <REGION>` | Region for pricing (default: `us-east-1`) |
| `--fix` | Generate fix suggestions |
| `--apply` | **DANGEROUS:** Apply fixes to manifest files. Requires `--fix`. Never use without explicit user confirmation. |
| `--dry-run` | Preview changes without applying |
| `--severity <LEVEL>` | Minimum severity to report |
| `--threshold <0-100>` | Minimum waste percentage threshold |

## Output Interpretation

**Priority for reporting to user:**
1. High-confidence right-sizing recommendations with cost savings
2. Critical security findings (from `--full`)
3. Drift detection issues
4. Cost breakdown summary

## Reading Results

When you use `--agent`, the output is a compressed summary. High-confidence right-sizing recommendations are included in full. Cost summary and drift findings are always present when applicable.

The output JSON includes:
- `summary` — total recommendations, estimated savings, containers analyzed
- `top_recommendations` — highest-impact right-sizing suggestions
- `costs` — cost attribution summary (if `--cloud-provider` set)
- `drift` — configuration drift issues (if `--cluster` set)
- `full_data_ref` — reference ID for retrieving full data
- `retrieval_hint` — exact command for drill-down

To drill into specifics:
```bash
# Get high-severity findings
sync-ctl retrieve <ref_id> --query "severity:high"

# Get recommendations for a specific container
sync-ctl retrieve <ref_id> --query "container:my-app"
```

**Available query filters:** `severity:<level>`, `container:<name>`

## Safety

- `--fix` only generates suggestions — it does NOT modify files
- `--apply` (requires `--fix`) writes changes to files — always confirm with user first
- `--dry-run` previews what `--apply` would do — use this to show the user before applying
- Recommend: always run `--fix --dry-run` first, show output, then `--fix --apply` only after user approval

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `No Kubernetes manifests found` | No YAML with K8s resources | Run `sync-ctl analyze <PATH> --agent` to check for K8s presence |
| `Cannot connect to cluster` | Invalid kubeconfig or cluster unreachable | Check `kubectl cluster-info` works, verify context name |
| `Prometheus unreachable` | Wrong URL or Prometheus not running | Verify URL, fall back to static analysis without `--prometheus` |

## Examples

**Quick static analysis:**
```bash
sync-ctl optimize . --agent
```

**Full analysis with live cluster and cost estimation:**
```bash
sync-ctl optimize . --cluster --cloud-provider aws --full --agent
```

**Preview fixes before applying:**
```bash
sync-ctl optimize . --fix --dry-run --agent
```
