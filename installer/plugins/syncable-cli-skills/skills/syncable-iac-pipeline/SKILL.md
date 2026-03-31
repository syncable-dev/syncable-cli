---
description: "Validate all infrastructure-as-code files by combining Dockerfile linting, Docker Compose validation, Kubernetes manifest checking, and Helm chart analysis using the Syncable CLI sync-ctl tool"
---

## Purpose

Validate all infrastructure-as-code files in a project by chaining IaC linting with Kubernetes optimization and security checks. Covers Dockerfiles, Docker Compose, Terraform, K8s manifests, and Helm charts.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory
- Project must contain IaC files

## Workflow Steps

### Step 1: Analyze the project

```bash
sync-ctl analyze <PATH> --agent
```

Parse the output to determine:
- Which IaC types exist (Dockerfile, Compose, Terraform, K8s manifests)
- Whether K8s manifests are present — needed for step 3

Save the `full_data_ref` from the analyze output — the ref_id from this step can be reused in later steps to retrieve IaC file details without re-running analyze.

### Step 2: Validate IaC files

```bash
sync-ctl validate <PATH>
```

If step 1 revealed specific types, you can filter:
```bash
sync-ctl validate <PATH> --types dockerfile,compose,terraform
```

### Step 3: Kubernetes optimization (conditional)

**Decision point:** Only run if step 1 detected Kubernetes manifests or Helm charts.

```bash
sync-ctl optimize <PATH> --full --agent
```

The `--full` flag includes kubelint security checks and helmlint validation on top of resource optimization.

## Decision Points Summary

| Condition | Action |
|-----------|--------|
| No K8s manifests or Helm charts in step 1 | Skip step 3 |
| No IaC files at all in step 1 | Abort workflow, tell user no IaC files found |

## Report Synthesis

Produce an IaC validation report:

1. **Dockerfile Issues** — hadolint violations by severity
2. **Docker Compose Issues** — dclint violations
3. **Terraform Issues** — validation errors
4. **Kubernetes Issues** — kubelint security findings and resource optimization recommendations (if step 3 ran)
5. **Actionable Fixes** — which issues can be auto-fixed with `--fix`

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
