---
description: "Use when the user asks to log in to Syncable, authenticate, switch projects or organizations, check current context, deploy a service, check deployment status, or manage platform settings"
---

## Overview

Manage Syncable platform: authenticate, switch org/project/environment, check context, deploy services.

## Auth-First Rule

**Always check auth before any platform operation:**

```bash
sync-ctl auth status
```

If not authenticated: `sync-ctl auth login` (opens browser for OAuth). Tell user a browser will open. Command blocks until authorized.

**Success criteria:** `sync-ctl auth status` shows authenticated.

## Context Switching

### By name (org/project require list → match → select)

```bash
# Organization (switching clears project + env)
sync-ctl org list → find ID → sync-ctl org select <ORG_ID>

# Project (requires UUID)
sync-ctl project list → find ID → sync-ctl project select <PROJECT_ID>

# Environment (accepts name directly)
sync-ctl env select <ENV_NAME>
```

Multiple matches: show user and ask. No matches: show available options.

### Check current context

```bash
sync-ctl project current
```

## Commands Reference

```bash
# Auth
sync-ctl auth status | login | login --no-browser | logout | token --raw

# Org
sync-ctl org list | org select <ID>

# Project
sync-ctl project list | project select <ID> | project current | project info [ID]

# Environment
sync-ctl env list | env select <NAME_OR_ID>
```

## Deploy Flow (Agents)

### 1. Preview

```bash
sync-ctl deploy preview <PATH> --service-name my-service
```

**Success criteria:** JSON with `recommendation`, `connected_providers`, `alternatives`.

### 2. Confirm with user

Show: service name, provider, region, port, public/internal. If .env files found, list keys and ask which to include.

**CRITICAL:** NEVER deploy without preview + user confirmation. ONLY include env vars the user explicitly confirmed.

### 3. Deploy

```bash
sync-ctl deploy run <PATH> --service-name <NAME> --provider <PROVIDER> --region <REGION> --port <PORT>
```

**Success criteria:** JSON with `config_id` and `task_id`.

### 4. Monitor

```bash
sync-ctl deploy status <TASK_ID> --watch
```

### Deploy Run Flags

| Flag | Purpose |
|------|---------|
| `--service-name <NAME>` | Service name (always set) |
| `--provider <gcp\|hetzner\|azure>` | Cloud provider |
| `--region <REGION>` | Deployment region |
| `--port <PORT>` | Port to expose |
| `--public` | Publicly accessible (default: internal) |
| `--env <KEY=VALUE>` | Non-secret env var (repeatable) |
| `--secret <KEY>` | Secret (user prompted for value, repeatable) |
| `--env-file <PATH>` | Load from .env file |
| `--min-instances <N>` / `--max-instances <N>` | Replica range |

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Auto-including discovered env vars without asking | ONLY set env vars the user explicitly requested |
| Deploying without preview | Always `deploy preview` first, show user, get confirmation |
| Using directory name as service name | Always set `--service-name` explicitly |

## Error Handling

| Error | Action |
|-------|--------|
| `Not authenticated` / `Token expired` | `sync-ctl auth login` |
| `No organization selected` | `sync-ctl org list` then `org select <ID>` |
| `No project selected` | `sync-ctl project list` then `project select <ID>` |
| `Deployment failed` | `sync-ctl deploy status <TASK_ID>` for details |
