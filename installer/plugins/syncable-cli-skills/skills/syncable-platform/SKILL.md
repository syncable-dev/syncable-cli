---
description: "Authenticate, login, sign in to Syncable platform. Switch organizations, projects, and environments. Deploy services to cloud providers. Check current context and manage platform settings using sync-ctl"
---

## Purpose

Manage Syncable platform operations: authenticate, switch organizations/projects/environments by name, check current context, and trigger deployments. This skill handles the full platform lifecycle.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Internet access for Syncable API

## Auth-First Rule

**Always check auth before any platform operation:**

```bash
sync-ctl auth status
```

If the response says "Not logged in" or "Session expired":

```bash
sync-ctl auth login
```

This opens a browser for OAuth device flow. Tell the user: "A browser window will open for you to authorize. If it doesn't open automatically, visit the URL shown and enter the code." Then wait — the command blocks until the user authorizes.

## Switching Context

### Switch project by name

When user says "change project to my-app" or "use project my-app":

1. List projects to find the ID:
```bash
sync-ctl project list
```

2. Parse the table output to find the row where NAME matches the user's request (case-insensitive, partial match OK).

3. Select by ID:
```bash
sync-ctl project select <PROJECT_ID>
```

**If multiple matches:** show the user the matching projects and ask which one.
**If no matches:** tell the user no project with that name was found, and show available projects.

### Switch organization by name

When user says "switch org to acme" or "use organization acme":

1. List organizations:
```bash
sync-ctl org list
```

2. Parse output to find matching org name → ID.

3. Select:
```bash
sync-ctl org select <ORG_ID>
```

**Note:** Switching org clears the current project and environment selection. After switching org, you'll need to select a project too.

### Switch environment by name

When user says "use staging" or "switch to production":

```bash
sync-ctl env select <ENV_NAME>
```

Environment select accepts **names directly** (not just IDs). No need to list first.

### Combined switches

When user says "switch to my-app on staging":

1. Find and select the project (list → match → select)
2. Then select the environment by name:
```bash
sync-ctl env select staging
```

When user says "switch to acme org, project my-app, staging env":

1. `sync-ctl org list` → find acme → `sync-ctl org select <ID>`
2. `sync-ctl project list` → find my-app → `sync-ctl project select <ID>`
3. `sync-ctl env select staging`

## Checking Current Context

When user says "what project am I on" or "show current context":

```bash
sync-ctl project current
```

Shows: organization, project, environment, and last updated timestamp.

## Commands Reference

### Authentication

```bash
sync-ctl auth status              # Check if authenticated
sync-ctl auth login               # Log in (opens browser)
sync-ctl auth login --no-browser  # Log in without auto-opening browser
sync-ctl auth logout              # Log out and clear credentials
sync-ctl auth token --raw         # Print raw access token
```

### Organizations

```bash
sync-ctl org list                 # List all organizations
sync-ctl org select <ORG_ID>     # Select organization (requires UUID)
```

### Projects

```bash
sync-ctl project list             # List projects in current org
sync-ctl project list --org-id <ID>  # List projects in specific org
sync-ctl project select <ID>     # Select project (requires UUID)
sync-ctl project current          # Show current context
sync-ctl project info             # Show current project details
sync-ctl project info <ID>       # Show specific project details
```

### Environments

```bash
sync-ctl env list                 # List environments in current project
sync-ctl env select <NAME_OR_ID>  # Select environment (accepts name or ID)
```

### Deployment (non-interactive — use these as an agent)

```bash
# Preview deployment recommendation (returns JSON)
sync-ctl deploy preview <PATH>
sync-ctl deploy preview <PATH> --provider hetzner --region nbg1

# Deploy with settings (triggers actual deployment)
sync-ctl deploy run <PATH> --provider hetzner --region nbg1 --port 8080 --public
sync-ctl deploy run <PATH> --env "NODE_ENV=production" --secret "DATABASE_URL" --env-file .env

# Monitor deployment
sync-ctl deploy status <TASK_ID>
sync-ctl deploy status <TASK_ID> --watch

# Interactive wizard (for humans, NOT usable by agents)
sync-ctl deploy wizard

# Create a new environment
sync-ctl deploy new-env
```

### Deploy Flow for Agents

**Step 1: Preview** — get a recommendation as JSON:
```bash
sync-ctl deploy preview . --service-name my-service
```

The output includes:
- `recommendation` — provider, region, machine type, port, health check, with reasoning
- `connected_providers` — all available providers
- `alternatives` — other options the user could choose
- `parsed_env_files` — discovered .env files and non-secret keys
- `deployed_service_endpoints` — URLs of already-deployed services (public and private)

**Step 2: Show user the preview and get explicit confirmation.** Present:
- Service name, provider, region, machine type
- Port and whether public or internal
- If .env files found, list the keys and ask user which ones to include
- If deployed service endpoints exist, list them and ask which to wire up

**CRITICAL RULES:**
- NEVER deploy without showing the preview first and getting user confirmation
- ONLY include env vars the user explicitly asked for or confirmed — do NOT auto-include discovered env vars or service endpoints
- If the user says "deploy with API_BASE=..." then ONLY set API_BASE, nothing else
- If the user wants additional env vars, they need to say so explicitly
- Always use `--service-name` to set the name the user wants, not the directory name

**Step 3: Deploy** — with ONLY the settings the user confirmed:
```bash
sync-ctl deploy run . --service-name frontend-v2 --provider hetzner --region nbg1 --port 8080 --env "API_BASE=https://..."
```

Returns JSON with `config_id` and `task_id`.

**Step 4: Monitor** — watch until complete:
```bash
sync-ctl deploy status <TASK_ID> --watch
```

### Deploy Run Flags

| Flag | Purpose |
|------|---------|
| `--service-name <NAME>` | Service name (ALWAYS set this — defaults to directory name otherwise) |
| `--provider <gcp\|hetzner\|azure>` | Cloud provider |
| `--region <REGION>` | Deployment region (e.g., `nbg1`, `us-central1`) |
| `--machine-type <TYPE>` | Machine type (e.g., `cx22`, `e2-small`) |
| `--port <PORT>` | Port to expose |
| `--public` | Make service publicly accessible (default: internal-only) |
| `--cpu <CPU>` | CPU allocation for GCP/Azure (e.g., `1000m`) |
| `--memory <MEM>` | Memory allocation for GCP/Azure (e.g., `512Mi`) |
| `--env <KEY=VALUE>` | Non-secret env var (repeatable) |
| `--secret <KEY>` | Secret key — user prompted in terminal for value (repeatable) |
| `--env-file <PATH>` | Load env vars from .env file (secrets auto-detected by key name) |
| `--min-instances <N>` | Minimum replicas |
| `--max-instances <N>` | Maximum replicas |

## Output Format

All platform commands output human-readable text (no `--agent` mode). Parse the table output to extract IDs and names. Table format is typically:

```
ID                                       NAME                           DESCRIPTION
------...
abc123-def456-...                        my-project                     My project description
```

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `Not authenticated` | No valid session | Run `sync-ctl auth login` |
| `Token expired` | Session timed out | Run `sync-ctl auth login` |
| `No organization selected` | Org not set | Run `sync-ctl org list` then `sync-ctl org select <ID>` |
| `No project selected` | Project not set | Run `sync-ctl project list` then `sync-ctl project select <ID>` |
| `No environment selected` | Env not set | Run `sync-ctl env list` then `sync-ctl env select <NAME>` |
| `Project not found` | Wrong ID | Run `sync-ctl project list` to see available projects |
| `Deployment failed` | Build or infra error | Check `sync-ctl deploy status <TASK_ID>` for details |

## Examples

**User: "change project to my-app"**
```bash
sync-ctl auth status          # Check auth first
sync-ctl project list         # Find my-app's ID
sync-ctl project select <ID>  # Select it
```

**User: "switch to staging"**
```bash
sync-ctl env select staging
```

**User: "switch to acme org and select the api-gateway project on production"**
```bash
sync-ctl auth status
sync-ctl org list              # Find acme's ID
sync-ctl org select <ACME_ID>
sync-ctl project list          # Find api-gateway's ID
sync-ctl project select <ID>
sync-ctl env select production
```

**User: "what am I connected to?"**
```bash
sync-ctl project current
```
