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
- `422` validation error (malformed JSON, missing required fields).
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

| Method | Route | Permission | Purpose |
| --- | --- | --- | --- |
| `POST` | `/terminals` | TerminalCreate | Create a PTY. |
| `GET` | `/terminals` | TerminalView | List live terminal metadata. |
| `GET` | `/terminals/{id}` | TerminalView | Get terminal metadata. |
| `DELETE` | `/terminals/{id}` | TerminalTerminate | Terminate the PTY. |
| `POST` | `/terminals/{id}/input` | TerminalWrite | Write bounded input. |
| `POST` | `/terminals/{id}/resize` | TerminalWrite | Resize the PTY. |
| `GET` | `/terminals/{id}/ws` | TerminalView | WebSocket upgrade for live output. |

**Input body** (`POST /terminals/{id}/input`):
```json
{"data":"ls\n"}
```
Empty input and payloads larger than 64 KiB are rejected.

**Resize body** (`POST /terminals/{id}/resize`):
```json
{"cols":120,"rows":40}
```

Terminal output uses WebSocket streams (`/ws`). Do not poll.

## Workspaces, tabs and panes

Workspace routes power the web workspace surface described by
[ADR 021](adr/021-herdr-parity-web-workspaces.md). Every route requires an
authenticated session. Reads require `TerminalView`; mutations require
`TerminalCreate` (or `TerminalWrite` for pane input/output) and a valid CSRF
header.

| Method | Route | Permission | Purpose |
| --- | --- | --- | --- |
| `GET` | `/workspaces` | TerminalView | List authorized workspaces with their tabs. |
| `POST` | `/workspaces` | TerminalCreate | Create a workspace. |
| `PATCH` | `/workspaces/{id}` | TerminalCreate | Rename a workspace. |
| `DELETE` | `/workspaces/{id}` | TerminalCreate | Delete a workspace and its durable layout metadata. |
| `GET` | `/workspaces/{id}/tabs` | TerminalView | List tabs in workspace order. |
| `POST` | `/workspaces/{id}/tabs` | TerminalCreate | Create a tab in a workspace. |
| `PATCH` | `/tabs/{id}` | TerminalCreate | Rename a tab. |
| `DELETE` | `/tabs/{id}` | TerminalCreate | Delete a tab and its pane metadata. |
| `GET` | `/tabs/{id}/panes` | TerminalView | List panes in tab order, including terminal bindings and persisted pane metadata. |
| `POST` | `/tabs/{id}/panes` | TerminalCreate | Create a pane and live PTY. |
| `PATCH` | `/panes/{id}` | TerminalCreate | Update pane metadata (title). |
| `DELETE` | `/panes/{id}` | TerminalCreate | Close the live PTY when present, then delete the pane metadata. |
| `POST` | `/panes/{id}/input` | TerminalWrite | Write bounded input to the pane PTY. |
| `GET` | `/panes/{id}/output` | TerminalView | Read journaled output after a sequence number. |

**Workspace create body** (`POST /workspaces`):
```json
{"name":"Primary"}
```
`name` must not be empty.

**Tab create body** (`POST /workspaces/{id}/tabs`):
```json
{"name":"API"}
```

**Pane create body** (`POST /tabs/{id}/panes`):
```json
{"title":"agent","cwd":"/home/user/src/app"}
```
`title` may be empty; `cwd` is optional and defaults to the daemon's normal
terminal directory.

**Pane update body** (`PATCH /panes/{id}`):
```json
{"title":"tests"}
```

**Pane input body** (`POST /panes/{id}/input`):
```json
{"data":"cargo test\n"}
```
Empty input and payloads larger than 64 KiB are rejected.

**Pane output query** (`GET /panes/{id}/output?after_seq=42&limit=256`):
`after_seq` defaults to `0`; `limit` defaults to `256` and is capped at `2000`.
Use WebSocket streams for live output rather than polling this recovery endpoint.

## Projects

| Method | Route | Permission | Purpose |
| --- | --- | --- | --- |
| `GET` | `/projects` | ProjectView | List authorized projects. |
| `POST` | `/projects` | ProjectManage | Create a project. |
| `GET` | `/projects/{id}` | ProjectView | Return one authorized project. |
| `DELETE` | `/projects/{id}` | ProjectManage | Delete project metadata. |
| `GET` | `/projects/{id}/sessions` | ProjectView | List project sessions. |
| `POST` | `/projects/{id}/sessions` | ProjectManage | Create a project session. |

**Project create body** (`POST /projects`):
```json
{"name":"app","path":"/home/user/src/app"}
```

## Tasks and worktrees

| Method | Route | Permission | Purpose |
| --- | --- | --- | --- |
| `GET` | `/projects/{id}/tasks` | TasksView | List tasks for a project. |
| `POST` | `/projects/{id}/tasks` | TasksWrite | Create a task. |
| `GET` | `/tasks/{id}` | TasksView | Get a task. |
| `DELETE` | `/tasks/{id}` | TasksWrite | Delete the task. |
| `POST` | `/tasks/{id}/move` | TasksWrite | Move task between columns with optimistic concurrency. |
| `POST` | `/tasks/{id}/worktree` | TasksWrite | Create or reuse a task worktree and `agent/*` branch. |
| `DELETE` | `/tasks/{id}/worktree` | TasksWrite | Remove the task worktree. |

**Task create body** (`POST /projects/{id}/tasks`):
```json
{
  "title": "Add retry",
  "description": "Retry transient job failures",
  "priority": 2
}
```
`description` optional; `priority` defaults to `0`.

**Move body** (`POST /tasks/{id}/move`):
```json
{"state":"review","version":7}
```
`version` is the optimistic concurrency token (the version the client last saw).
Mismatch returns `409`.

## Files and Git

| Method | Route | Permission | Purpose |
| --- | --- | --- | --- |
| `GET` | `/projects/{id}/files` | FilesView | List a rooted directory. |
| `POST` | `/projects/{id}/files/write` | FilesWrite | Write a rooted file. |
| `DELETE` | `/projects/{id}/files` | FilesWrite | Delete a rooted file. |
| `GET` | `/projects/{id}/files/read` | FilesView | Read a rooted file. |
| `GET` | `/projects/{id}/git/status` | GitView | Git status. |
| `GET` | `/projects/{id}/git/diff` | GitView | Diff. |
| `GET` | `/projects/{id}/git/log` | GitView | History. |
| `GET` | `/projects/{id}/git/branches` | GitView | Branch list. |
| `POST` | `/projects/{id}/git/stage` | GitWrite | Stage paths. |
| `POST` | `/projects/{id}/git/commit` | GitWrite | Commit staged changes. |
| `POST` | `/projects/{id}/git/branch` | GitWrite | Create branch. |
| `POST` | `/projects/{id}/git/switch` | GitWrite | Switch branch. |

All paths are canonicalized inside the authorized project root.

**List files query** (`GET /projects/{id}/files?path=src`):
`path` defaults to empty (project root).

**Write body** (`POST /projects/{id}/files/write`):
```json
{"path":"src/main.rs","contents":"fn main() {}"}
```

**Delete body** (`DELETE /projects/{id}/files`):
```json
{"path":"src/old.rs"}
```

**Stage body** (`POST /projects/{id}/git/stage`):
```json
{"paths":["src/main.rs","src/lib.rs"]}
```

**Commit body** (`POST /projects/{id}/git/commit`):
```json
{"message":"feat: add main"}
```

**Branch body** (`POST /projects/{id}/git/branch`):
```json
{"name":"feature/xyz"}
```

## RBAC and audit

| Method | Route | Permission | Purpose |
| --- | --- | --- | --- |
| `GET` | `/users` | UsersView | List users. |
| `POST` | `/users` | UsersManage | Create a user. |
| `GET` | `/projects/{id}/members` | ProjectView | List project members. |
| `POST` | `/projects/{id}/members` | ProjectManage | Set or update a member role. |
| `DELETE` | `/projects/{id}/members/{user_id}` | ProjectManage | Remove a member. |
| `GET` | `/audit` | AuditView | Read the audit tail. |

**User create body** (`POST /users`):
```json
{"username":"dev","display_name":"Dev","password":"secret","role":"Developer"}
```
`role` is one of `Owner`, `Developer`, `Viewer`.

**Member set body** (`POST /projects/{id}/members`):
```json
{"user_id":42,"role":"Developer"}
```

**Audit query** (`GET /audit?limit=100`):
`limit` defaults to `100`.

## Agents and attention

| Method | Route | Permission | Purpose |
| --- | --- | --- | --- |
| `GET` | `/agents` | AgentView | Return detected agents. |
| `GET` | `/attention` | AgentView | Return Attention Center items. |
| `POST` | `/attention/{id}/resolve` | AgentControl | Resolve one item. |

## Nodes

| Method | Route | Permission | Purpose |
| --- | --- | --- | --- |
| `GET` | `/nodes` | AgentView | List remote nodes. |
| `POST` | `/nodes` | SettingsManage | Register a node identity. |
| `POST` | `/nodes/{id}/pairing` | SettingsManage | Create a short-lived pairing challenge. |
| `POST` | `/nodes/{id}/revoke` | SettingsManage | Revoke the node. |
| `GET` | `/nodes/{id}/agents` | AgentView | Return aggregated node agents. |

**Node register body** (`POST /nodes`):
```json
{"name":"remote","fingerprint":"sha256:...","public_key":"ssh-ed25519 ..."}
```

## Services, observability and previews

| Method | Route | Permission | Purpose |
| --- | --- | --- | --- |
| `GET` | `/services/ports` | ServicesView | Discover listening ports. |
| `GET` | `/services/host` | ServicesView | Host snapshot. |
| `GET` | `/services` | ServicesView | Registered services. |
| `POST` | `/services` | ServicesControl | Register service. |
| `DELETE` | `/services/{name}` | ServicesControl | Remove service. |
| `GET` | `/preview/{port}/{*path}` | ServicesView | Loopback-only preview. |
| `GET` | `/preview/{port}` | ServicesView | Loopback-only preview root. |
| `GET` | `/observability/host` | ServicesView | Host metrics. |
| `GET` | `/observability/sessions` | ServicesView | Terminal process snapshots. |

## Notifications

| Method | Route | Permission | Purpose |
| --- | --- | --- | --- |
| `GET` | `/notifications/rules` | SettingsView | List notification rules. |
| `POST` | `/notifications/rules` | SettingsManage | Upsert a rule. |
| `POST` | `/notifications/test` | SettingsManage | Send a test notification. |

**Rule upsert body** (`POST /notifications/rules`):
```json
{
  "event": "task.moved",
  "project_id": 1,
  "channel": "webhook",
  "target": "my-webhook",
  "enabled": true
}
```
`project_id` optional; `enabled` defaults to `false`.

## Workflows

| Method | Route | Permission | Purpose |
| --- | --- | --- | --- |
| `GET` | `/workflows` | AgentView | List definitions. |
| `POST` | `/workflows` | AgentStart | Create a definition. |
| `GET` | `/workflows/{name}` | AgentView | Get a definition. |
| `POST` | `/workflows/{name}/start` | AgentStart | Start a run. |
| `GET` | `/workflow-runs/latest` | AgentView | Return the newest run. |
| `GET` | `/workflow-runs/{id}` | AgentView | Return a run. |
| `POST` | `/workflow-runs/{id}/advance` | AgentControl | Advance a running stage. |
| `POST` | `/workflow-runs/{id}/approve` | AgentApprove | Approve the current approval stage. |
| `POST` | `/workflow-runs/{id}/cancel` | AgentControl | Cancel an active run. |

**Definition create body** (`POST /workflows`):
```json
{
  "name": "delivery",
  "steps": [
    {"name":"plan","kind":"agent","agents":["planner"],"command":"true"},
    {"name":"approve","kind":"approval","agents":[]}
  ]
}
```

**Start body** (`POST /workflows/{name}/start`):
```json
{"variables":{}}
```

## Platform `/api/v1`

### Tokens

| Method | Route | Purpose |
| --- | --- | --- |
| `POST` | `/api/v1/tokens` | Create token; secret shown once. |
| `GET` | `/api/v1/tokens` | List token metadata. |
| `POST` | `/api/v1/tokens/{id}/revoke` | Revoke token. |

**Token create body** (`POST /api/v1/tokens`):
```json
{"name":"ci","scopes":["projects:read","tasks:write"],"expires_at":1735689600}
```
`scopes` optional; `expires_at` optional Unix epoch seconds.

### Webhooks

| Method | Route | Purpose |
| --- | --- | --- |
| `GET` | `/api/v1/webhooks` | List webhooks. |
| `POST` | `/api/v1/webhooks` | Create webhook. |
| `PUT` | `/api/v1/webhooks/{name}` | Update webhook. |
| `DELETE` | `/api/v1/webhooks/{name}` | Delete webhook. |

**Webhook create/update body** (`POST/PUT /api/v1/webhooks`):
```json
{"name":"deploy","url":"https://ci.example.com/hook","secret":"shared","events":"task.*,project.*","enabled":true}
```
`secret` is required on create; on update it can be omitted to keep the existing
one.

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
Publish an event to the realtime bus. `critical` marks the event as high
priority.

### MCP

#### `GET /api/v1/mcp`

Return versioned MCP-style tool manifest.

#### `POST /api/v1/mcp/tools/{tool}/invoke`

Invoke a manifest tool.

Built-ins include:
- `dashboard.snapshot`
- `terminal.create`
- `task.move`

### Plugins

| Method | Route | Permission | Purpose |
| --- | --- | --- | --- |
| `GET` | `/api/v1/plugins` | SettingsView | List plugin manifests. |
| `POST` | `/api/v1/plugins` | SettingsManage | Register plugin. |
| `GET` | `/api/v1/plugins/{id}` | SettingsView | Get plugin. |
| `PUT` | `/api/v1/plugins/{id}` | UsersManage | Update plugin (full replace). |

Plugin manifests are disabled by default. Arbitrary plugin execution is not
enabled.

## Versioning

Breaking changes require `/api/v2` and migration guidance. Additive fields must
be documented and independently ignorable.

## Realtime

For live terminal output, workflow progress, and file-system events, use the
WebSocket gateway at `/ws` documented in [realtime.md](realtime.md). Do not
poll REST endpoints.