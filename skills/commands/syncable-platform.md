---
name: syncable-platform
description: Use when authenticating with Syncable, managing projects/orgs/environments, or deploying services through the Syncable platform. Trigger on: "syncable login", "select project", "deploy to syncable", "list environments", "switch organization".
---

## Purpose

Manage Syncable platform operations: authenticate, select organizations/projects/environments, and trigger deployments. This skill covers the full platform lifecycle from login to deploy.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Internet access for Syncable API
- For deployment: authenticated session with project and environment selected

## Commands

### Authentication

```bash
# Check if authenticated
sync-ctl auth status

# Log in (opens browser)
sync-ctl auth login

# Log in without auto-opening browser
sync-ctl auth login --no-browser

# Log out
sync-ctl auth logout

# Get access token (for scripting)
sync-ctl auth token --raw
```

### Organization Management

```bash
# List organizations
sync-ctl org list

# Select an organization
sync-ctl org select <ORG_ID>
```

### Project Management

```bash
# List projects in current org
sync-ctl project list

# List projects in a specific org
sync-ctl project list --org-id <ORG_ID>

# Select a project
sync-ctl project select <PROJECT_ID>

# Show current context (org + project)
sync-ctl project current

# Show project details
sync-ctl project info
sync-ctl project info <PROJECT_ID>
```

### Environment Management

```bash
# List environments in current project
sync-ctl env list

# Select an environment
sync-ctl env select <ENV_ID>
```

### Deployment

```bash
# Launch interactive deployment wizard
sync-ctl deploy wizard

# Create a new environment
sync-ctl deploy new-env

# Check deployment status
sync-ctl deploy status <TASK_ID>

# Watch deployment status until complete
sync-ctl deploy status <TASK_ID> --watch
```

## Auth-First Flow

**Always check auth status before any platform operation.** Follow this sequence:

1. Run `sync-ctl auth status` — if not authenticated, guide user through `sync-ctl auth login`
2. Run `sync-ctl project current` — if no project selected, list projects and ask user to select
3. For deployment: ensure environment is selected via `sync-ctl env list` + `sync-ctl env select`

## Output Interpretation

- **auth status** — shows whether user is logged in, token expiry
- **org/project/env list** — shows available items with IDs and names
- **project current** — shows currently selected org, project, and environment
- **deploy status** — shows deployment progress, logs, and final status

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `Not authenticated` | No valid session | Run `sync-ctl auth login` |
| `Token expired` | Session timed out | Run `sync-ctl auth login` to re-authenticate |
| `No project selected` | Project context not set | Run `sync-ctl project list` then `sync-ctl project select <ID>` |
| `No environment selected` | Environment not set | Run `sync-ctl env list` then `sync-ctl env select <ID>` |
| `Deployment failed` | Build or infra error | Check `sync-ctl deploy status <TASK_ID>` for error details |

## Examples

**Full login-to-deploy flow:**
```bash
sync-ctl auth login
sync-ctl org list
sync-ctl org select <ORG_ID>
sync-ctl project list
sync-ctl project select <PROJECT_ID>
sync-ctl env list
sync-ctl env select <ENV_ID>
sync-ctl deploy wizard
```

**Check current context:**
```bash
sync-ctl auth status
sync-ctl project current
```

**Monitor a deployment:**
```bash
sync-ctl deploy status <TASK_ID> --watch
```
