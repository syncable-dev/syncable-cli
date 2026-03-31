---
description: "Use when the user asks to validate all infrastructure files, run an IaC review, check Docker/Compose/K8s/Terraform/Helm files together, or lint infrastructure-as-code"
---

## Overview

Chain analyze + validate + K8s optimize for a complete IaC review. Covers Dockerfiles, Compose, Terraform, K8s manifests, and Helm charts.

## Steps

### 1. Analyze the project

```bash
sync-ctl analyze <PATH> --agent
```

Determine: which IaC types exist, whether K8s manifests/Helm charts present (gates step 3). Save `full_data_ref`.

**Success criteria:** You know which IaC types are present. If NO IaC files at all → abort workflow, tell user.

### 2. Validate IaC files

```bash
sync-ctl validate <PATH> --agent
```

Filter if types known from step 1: `--types dockerfile,compose,terraform`

**Success criteria:** JSON with `status` field and violations by severity.

### 3. Kubernetes optimization (conditional)

**Decision:** Only run if step 1 detected K8s manifests or Helm charts.

```bash
sync-ctl optimize <PATH> --full --agent
```

`--full` includes kubelint security + helmlint validation + resource optimization.

**Success criteria:** JSON with recommendations, or step skipped with reason.

## Decision Points

| Condition | Action |
|-----------|--------|
| No IaC files at all | Abort, tell user |
| No K8s/Helm in step 1 | Skip step 3 |

## Report Synthesis

1. **Dockerfile Issues** — hadolint violations by severity
2. **Docker Compose Issues** — dclint violations
3. **Terraform Issues** — validation errors
4. **Kubernetes Issues** — kubelint findings + resource optimization (if step 3 ran)
5. **Actionable Fixes** — which issues can be auto-fixed with `--fix`

## Retrieval

Save each step's `full_data_ref`. Use `sync-ctl retrieve <ref_id> --query "..."` for drill-down:

```bash
sync-ctl retrieve <validate_ref_id> --query "severity:high"
sync-ctl retrieve <validate_ref_id> --query "file:Dockerfile"
sync-ctl retrieve <optimize_ref_id> --query "container:my-app"
```

Do NOT re-run commands for more detail.
