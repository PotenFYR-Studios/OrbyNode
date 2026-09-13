# Getting Started

This guide covers the fastest path to running OrbyNode: using the installer.

## Quick start

Linux/macOS:

```bash
curl -fsSL https://orbynode.docs.potenfyr.in/install.sh | sh
orbynode-daemon
```

Windows (PowerShell):

```powershell
irm https://orbynode.docs.potenfyr.in/install.ps1 | iex
orbynode-daemon
```

Open <http://127.0.0.1:7676/setup> to create the first Owner account.

## Installation methods

Choose one:

1. **Installer script** (recommended) — [Installation guide](installation.md)
2. **Download binary** — [GitHub Releases](https://github.com/PotenFYR-Studios/OrbyNode/releases/latest)
3. **Build from source** — [Development guide](development.md)

## First-run setup

1. Start the daemon:
   ```bash
   orbynode-daemon
   ```
2. Open <http://127.0.0.1:7676/setup>
3. Read the security warning (OrbyNode provides terminal access = remote code execution capability)
4. Create the first Owner with username, display name, and password
5. Sign in with the returned session cookie

The first Owner can be created only while the users table is empty. This prevents anonymous account creation after deployment.

## Configuration

OrbyNode reads environment variables for runtime configuration:

| Variable | Default | Purpose |
| --- | --- | --- |
| `ORBYNODE_BIND` | `127.0.0.1:7676` | Bind address |
| `ORBYNODE_PORT` | `7676` | Port shorthand (overrides BIND port) |
| `ORBYNODE_DATA_DIR` | `~/.orbynode` | SQLite database and durable state |
| `ORBYNODE_LOG_FORMAT` | `human` | Set to `json` for structured logs |
| `RUST_LOG` | `info` | Log level (error, warn, info, debug, trace) |

Example with custom port and debug logging:

```bash
ORBYNODE_PORT=8080 RUST_LOG=debug orbynode-daemon
```

Configuration is environment-only; OrbyNode does not read config files. This keeps the deployment surface explicit and auditable.

## Using OrbyNode

### Start a project

1. Navigate to **Projects** in the web UI
2. Click **New Project**
3. Provide a name and path (absolute or relative to `ORBYNODE_DATA_DIR`)
4. OrbyNode creates the directory if it doesn't exist

### Launch an agent

1. Open a project
2. Click **Start Agent** or open a terminal pane
3. Choose the agent type (Claude Code, Codex, Gemini, etc.)
4. OrbyNode spawns the agent in a real PTY and tracks its state

### Persist across reconnects

Close your browser. Reopen <http://127.0.0.1:7676>. The agent is still running, its terminal output intact. OrbyNode owns the PTY; clients are replaceable viewers.

### Multi-user access

1. Navigate to **Users**
2. Create accounts with roles: Owner, Editor, Viewer
3. Share the instance URL; each user signs in with their credentials

RBAC is enforced on every route and WebSocket subscription. Audit logs track all mutations.

## Next steps

- [Architecture](../ARCHITECTURE.md) — daemon-centric design and realtime model
- [Security](security-model.md) — authentication, RBAC, audit, threat model
- [REST API](rest-api.md) — public `/api/v1` surface for external integrations
- [Development](development.md) — build from source, run tests, frontend workflow
- [Operations](operations.md) — deployment, monitoring, backup, upgrade
- [Roadmap](../ROADMAP.md) — milestone status and v1.0 readiness
