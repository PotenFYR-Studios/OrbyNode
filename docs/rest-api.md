# REST API

Versioned public REST surface: `/api/v1`.

Product routes remain available for the embedded UI. New external integrations
should use `/api/v1` where available.

## Authentication

Browser routes use the daemon session cookie. Mutations also require:

```http
X-Orbynode-CSRF: <session csrf>
```

Public `/api/v1` routes are protected by authenticated session middleware.
Bearer API tokens are stored and revocable; token secrets are shown once at
creation.

## Conventions

- JSON request and response bodies.
- `snake_case` fields.
- `401` missing/invalid authentication.
- `403` authenticated but forbidden or missing CSRF.
- `404` unknown resource.
- `409` conflict or optimistic-concurrency failure.
- `429` throttled.
- Durable mutations are audited when applicable.

## Health and version

### `GET /health`

```json
{"status":"ok","uptime_secs":42}
```

### `GET /version`

```json
{"name":"OrbyNode","version":"0.0.1"}
```

## Setup and authentication

### `GET /setup`

```json
{"setup_pending":true}
```

### `POST /setup`

```json
{
  "username": "owner",
  "display_name": "Owner",
  "password": "correct horse"
}
```

Creates the first Owner only. Returns `201` and a session cookie.

### `POST /login`

```json
{"username":"owner","password":"correct horse"}
```

Returns session metadata and sets the cookie. Mutations then require the CSRF
header.

### `POST /logout`

Deletes the server session and clears the cookie.

### `GET /me`

Returns the current user.

## Terminals

### `POST /terminals`

Create a PTY.

### `GET /terminals`

List live terminal metadata.

### `POST /terminals/{id}/input`

```json
{"data":"ls\n"}
```

Requires terminal write.

### `POST /terminals/{id}/resize`

```json
{"cols":120,"rows":40}
```

### `POST /terminals/{id}/terminate`

Terminate the PTY.

Terminal output uses WebSocket streams. Do not poll.

## Projects

### `GET /projects`

List authorized projects.

### `POST /projects`

```json
{"name":"app","path":"/home/user/src/app"}
```

### `GET /projects/{id}`

Return one authorized project.

### `DELETE /projects/{id}`

Delete project metadata.

## Tasks and worktrees

### `GET /projects/{id}/tasks`

List tasks for a project.

### `POST /projects/{id}/tasks`

```json
{
  "title": "Add retry",
  "description": "Retry transient job failures",
  "priority": 2
}
```

### `PATCH /tasks/{id}`

Update mutable fields.

### `POST /tasks/{id}/move`

```json
{"state":"review","version":7}
```

Version mismatch returns `409`.

### `DELETE /tasks/{id}`

Delete the task.

### `POST /tasks/{id}/worktree`

Create or reuse a task worktree and `agent/*` branch.

## Files and Git

| Method | Route | Purpose |
| --- | --- | --- |
| `GET` | `/projects/{id}/files` | List a rooted directory. |
| `POST` | `/projects/{id}/files/write` | Write a rooted file. |
| `DELETE` | `/projects/{id}/files` | Delete a rooted file. |
| `GET` | `/projects/{id}/files/read` | Read a rooted file. |
| `GET` | `/projects/{id}/git/status` | Git status. |
| `GET` | `/projects/{id}/git/diff` | Diff. |
| `GET` | `/projects/{id}/git/log` | History. |
| `GET` | `/projects/{id}/git/branches` | Branch list. |
| `POST` | `/projects/{id}/git/stage` | Stage paths. |
| `POST` | `/projects/{id}/git/commit` | Commit staged changes. |
| `POST` | `/projects/{id}/git/branch` | Create branch. |
| `POST` | `/projects/{id}/git/switch` | Switch branch. |

All paths are canonicalized inside the authorized project root.

## RBAC and audit

### `GET /users`

List users.

### `POST /users`

Create a user.

### `GET /projects/{id}/members`

List project members.

### `POST /projects/{id}/members`

Set or update a member role.

### `GET /audit`

Read the audit tail.

## Agents and attention

### `GET /agents`

Return detected agents.

### `GET /attention`

Return Attention Center items.

### `POST /attention/{id}/resolve`

Resolve one item.

## Nodes

### `GET /nodes`

List remote nodes.

### `POST /nodes`

Register a node identity.

### `POST /nodes/{id}/pairing`

Create a short-lived pairing challenge.

### `POST /nodes/{id}/revoke`

Revoke the node.

### `GET /nodes/{id}/agents`

Return aggregated node agents.

## Services, observability and previews

| Method | Route | Purpose |
| --- | --- | --- |
| `GET` | `/services/ports` | Discover listening ports. |
| `GET` | `/services/host` | Host snapshot. |
| `GET` | `/services` | Registered services. |
| `POST` | `/services` | Register service. |
| `DELETE` | `/services/{name}` | Remove service. |
| `GET` | `/preview/{port}/{*path}` | Loopback-only preview. |
| `GET` | `/observability/host` | Host metrics. |
| `GET` | `/observability/sessions` | Terminal process snapshots. |

## Notifications

### `GET /notifications/rules`

List notification rules.

### `POST /notifications/rules`

Upsert a rule.

### `POST /notifications/test`

Send a test notification.

## Workflows

### `GET /workflows`

List definitions.

### `POST /workflows`

```json
{
  "name": "delivery",
  "steps": [
    {"name":"plan","kind":"agent","agents":["planner"],"command":"true"},
    {"name":"approve","kind":"approval","agents":[]}
  ]
}
```

### `POST /workflows/{name}/start`

```json
{"variables":{}}
```

### `GET /workflow-runs/latest`

Return the newest run.

### `GET /workflow-runs/{id}`

Return a run.

### `POST /workflow-runs/{id}/advance`

Advance a running stage.

### `POST /workflow-runs/{id}/approve`

Approve the current approval stage.

### `POST /workflow-runs/{id}/cancel`

Cancel an active run.

## Platform `/api/v1`

### Tokens

| Method | Route | Purpose |
| --- | --- | --- |
| `POST` | `/api/v1/tokens` | Create token; secret shown once. |
| `GET` | `/api/v1/tokens` | List token metadata. |
| `POST` | `/api/v1/tokens/{id}/revoke` | Revoke token. |

### Webhooks

| Method | Route | Purpose |
| --- | --- | --- |
| `GET` | `/api/v1/webhooks` | List webhooks. |
| `POST` | `/api/v1/webhooks` | Create webhook. |
| `PUT` | `/api/v1/webhooks/{name}` | Update webhook. |
| `DELETE` | `/api/v1/webhooks/{name}` | Delete webhook. |

### Events

### `POST /api/v1/events`

```json
{
  "stream": "project:1",
  "type": "custom.updated",
  "data": {"ok": true},
  "critical": false
}
```

### MCP

#### `GET /api/v1/mcp`

Return versioned MCP-style tool manifest.

#### `POST /api/v1/mcp/tools/{tool}/invoke`

Invoke a manifest tool.

Built-ins include:

- `dashboard.snapshot`,
- `terminal.create`,
- `task.move`.

### Plugins

| Method | Route | Purpose |
| --- | --- | --- |
| `GET` | `/api/v1/plugins` | List plugin manifests. |
| `POST` | `/api/v1/plugins` | Register plugin. |
| `GET` | `/api/v1/plugins/{id}` | Get plugin. |
| `PUT` | `/api/v1/plugins/{id}` | Update plugin. |

Plugin manifests are disabled by default. Arbitrary plugin execution is not
enabled.

## Versioning

Breaking changes require `/api/v2` and migration guidance. Additive fields must
be documented and independently ignorable.
