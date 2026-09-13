# OrbyNode Architecture

OrbyNode is a self-hosted control plane for coding agents. One primary daemon
owns terminals, agents, durable state, authorization, realtime fan-out and
remote-node aggregation. Browser and desktop clients are replaceable views.

## Core principles

- **Daemon is the runtime.** Closing clients never terminates agents.
- **Real PTYs.** Terminal bytes are captured once and fanned out.
- **Realtime, not polling.** WebSocket snapshots and event replay drive state.
- **Bounded resources.** Queues and replay rings have explicit caps.
- **Durable state in SQLite.** Transient high-frequency state stays in memory.
- **Secure by default.** Localhost binding, authentication and server-side RBAC.
- **Cross-platform.** Rust daemon, static Vite client, optional Tauri shell.

## Components

| Component | Responsibility |
| --- | --- |
| `orbynode-core` | Version, configuration, data paths. Intentionally small. |
| `orbynode-terminal` | PTY creation, resize/input/output, capture and termination. |
| `orbynode-realtime` | Stream event bus, sequences, replay rings, subscriptions. |
| `orbynode-database` | SQLite WAL, migrations, projects, sessions, tasks and audit. |
| `orbynode-auth` | Argon2id, sessions, throttling, roles and permissions. |
| `orbynode-agents` | Output/process detection and Attention Center. |
| `orbynode-files` | Root-contained files, Git operations and worktrees. |
| `orbynode-services` | Port discovery, service registry and process snapshots. |
| `orbynode-nodes` | Remote-node identity, pairing, heartbeat and revocation. |
| `orbynode-notifications` | Browser realtime events and generic webhook delivery. |
| `orbynode-workflows` | Sequential, parallel and approval workflow stages. |
| `orbynode-api` | Axum REST, middleware, gateway, previews and embedded UI. |
| `orbynode-daemon` | Configuration, tracing, binding and graceful lifecycle. |

## Request and realtime flow

```text
Browser / Tauri
    |
    | authenticated REST mutation
    v
Axum middleware --> role/project authorization --> SQLite / PTY / service
    |
    | domain event
    v
Realtime event bus --> sequence + bounded replay
    |
    | authenticated subscriptions
    v
Authorized clients receive snapshots, replay and live events
```

Terminal PTY output is published once. Slow clients receive bounded queues and
explicit overflow or resync signals; critical attention events are not dropped
in favor of droppable terminal or metric traffic.

## Persistence

SQLite uses WAL and foreign keys. Durable repositories cover:

- projects, sessions and terminal layouts,
- users, sessions, memberships and audit logs,
- tasks and worktree metadata,
- remote-node identity and pairing state,
- workflow definitions, runs and steps,
- settings, API tokens, webhooks, MCP and plugin manifests.

Terminal bytes, live metrics and event replay remain transient and bounded.

## Web and desktop

The web client uses **Vite + React + TypeScript + Bun + Magic UI** and is
built to static assets. Production assets are embedded in the daemon; disk
mode supports frontend development. Next.js is not used.

The optional Tauri desktop shell is separate from the daemon workspace. It
reuses the daemon's local HTTP and WebSocket surface and must never become the
agent runtime.

## Security boundaries

- Browser users are authenticated with server-side sessions and CSRF checks.
- Project and global roles are evaluated per request and subscription.
- File access is canonicalized inside authorized project roots.
- Service previews target loopback only.
- Remote nodes use expiring pairing secrets, hashed credentials and revocation.
- API tokens are hashed; creation-time secrets are shown once.
- Plugins remain inert manifests until capability isolation is implemented.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `ORBYNODE_BIND` | `127.0.0.1:7676` | Socket address. |
| `ORBYNODE_PORT` | `7676` | Port shorthand. |
| `ORBYNODE_DATA_DIR` | `~/.orbynode` | Durable data directory. |
| `ORBYNODE_STATIC_DIR` | embedded | Serve web assets from disk for development. |
| `ORBYNODE_LOG_FORMAT` | human-readable | `json` for structured logs. |
| `RUST_LOG` | `info` | Tracing filter. |

## Testing

Unit tests live next to code. Integration-style API tests exercise
authentication, routing and workflows. The realtime load test checks bounded
fan-out. CI runs Rust checks on Linux, macOS and Windows plus Bun web checks.

## Key decisions

Architecture decisions are recorded in [docs/adr](docs/adr/). The most
load-bearing decisions cover daemon ownership, PTY abstraction, realtime
protocol, persistence, authentication, embedded frontend, remote-node identity
and platform interfaces.
