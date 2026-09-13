# Workflows

Configuration-driven orchestration for coding-agent pipelines.

## Model

A workflow definition contains ordered steps. Each step is one of:

| Kind | Behavior |
| --- | --- |
| `agent` | Runs one configured command and stores output. |
| `parallel` | Marks multiple agent participants as dispatched for this stage. |
| `approval` | Pauses the run until explicit human approval. |

A run records definition, status, current stage and variables. Step output is
stored per run, stage and agent.

## Statuses

| Status | Meaning |
| --- | --- |
| `running` | Non-approval stage is active. |
| `waiting_approval` | Human approval is required. |
| `completed` | Final stage completed. |
| `failed` | Command stage returned failure. |
| `cancelled` | User cancelled an active run. |

## Example pipeline

```json
{
  "name": "delivery",
  "steps": [
    {
      "name": "plan",
      "kind": "agent",
      "agents": ["planner"],
      "command": "true"
    },
    {
      "name": "implement",
      "kind": "parallel",
      "agents": ["backend", "frontend"]
    },
    {
      "name": "test",
      "kind": "agent",
      "agents": ["ci"],
      "command": "cargo test"
    },
    {
      "name": "review",
      "kind": "agent",
      "agents": ["reviewer"],
      "command": "true"
    },
    {
      "name": "approve",
      "kind": "approval"
    }
  ]
}
```

The engine executes stages in order. It advances automatically through
non-approval stages and stops at the first approval. Approval resumes later
stages.

## API

### Create or update definition

```http
POST /workflows
Content-Type: application/json
X-Orbynode-CSRF: <csrf>
```

```json
{
  "name": "delivery",
  "steps": [
    {"name":"test","kind":"agent","agents":["ci"],"command":"true"},
    {"name":"approve","kind":"approval"}
  ]
}
```

Definitions are identified by normalized name and upserted.

### List definitions

```http
GET /workflows
```

### Get definition

```http
GET /workflows/{name}
```

### Start run

```http
POST /workflows/{name}/start
Content-Type: application/json
X-Orbynode-CSRF: <csrf>
```

```json
{"variables":{"project":"app"}}
```

### Latest run

```http
GET /workflow-runs/latest
```

### Get run

```http
GET /workflow-runs/{id}
```

### Advance

```http
POST /workflow-runs/{id}/advance
X-Orbynode-CSRF: <csrf>
```

### Approve

```http
POST /workflow-runs/{id}/approve
X-Orbynode-CSRF: <csrf>
```

Approval works only while status is `waiting_approval`.

### Cancel

```http
POST /workflow-runs/{id}/cancel
X-Orbynode-CSRF: <csrf>
```

Cancellation works only for `running` or `waiting_approval` runs.

## Realtime

Workflow changes publish to the `workflows` stream as critical events. UI state
should derive from snapshots and `workflow.updated` events, not polling.

## Persistence

Tables:

| Table | Purpose |
| --- | --- |
| `workflow_definitions` | Canonical definition and JSON config. |
| `workflow_runs` | Run status, stage and variables. |
| `workflow_steps` | Per-stage agent status and output. |

## Security

All workflow routes require authentication and CSRF. Authorization is
server-side. Treat commands as host execution capability; only trusted roles
should manage definitions containing commands.

## Platform behavior

- Empty definitions are rejected.
- Empty step names are rejected.
- Agent stages without agents are rejected.
- Command failure marks the run failed.
- Approval stages create a pending human step.
- Parallel stages currently record dispatched participants; durable concurrent
  agent process orchestration is a later integration layer.

## Operational rules

- Back up SQLite before changing workflow definitions.
- Use deterministic, unique stage names inside a definition.
- Avoid embedding secrets in commands.
- Prefer short, observable commands and explicit approval gates.
- Treat run IDs as opaque strings.
