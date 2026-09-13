# Frequently Asked Questions

## General

### What is OrbyNode?

OrbyNode is the self-hosted control plane for coding agents. It runs persistent terminal agents (Claude Code, Codex, Gemini, OpenCode, Hermes), owns their PTYs, files, tasks, Git workflows, services and machine state in one daemon, and reconnects your browser without interrupting the work.

The browser and desktop shell are replaceable clients. The daemon is the runtime. Closing a client never kills an agent.

### Why "daemon-centric"?

Traditional terminal multiplexers (tmux, screen) keep the session alive but don't understand what's running inside. OrbyNode understands agent state: waiting for approval, processing a task, blocked on input, or failed. It rolls that state up to tabs, workspaces, and the Attention Center.

Daemon-centric also means single source of truth. The SQLite database owns workspace layout, pane scrollback, task definitions, user accounts, audit logs, and agent sessions. Clients are viewers; the daemon persists.

### How is this different from VS Code Remote, Tmux, or Herdr?

| Feature | OrbyNode | VS Code Remote | tmux | Herdr |
|---------|----------|----------------|------|-------|
| Persistent PTY | ✅ | ❌ (process dies) | ✅ | ✅ |
| Agent state awareness | ✅ | ❌ | ❌ | ✅ |
| Multi-user RBAC | ✅ | ❌ | ❌ | ✅ |
| Web UI | ✅ | ✅ | ❌ | ✅ |
| Self-hosted only | ✅ | ✅ | ✅ | ❌ (SaaS option) |
| Realtime collaboration | ✅ | ✅ | ❌ | ✅ |
| Crash recovery | ✅ (3-tier) | ❌ | Partial | ✅ |
| Remote node control | ✅ | ✅ | ❌ | ✅ |
| Plugin system | ✅ | ✅ | ❌ | ❌ |

### Is OrbyNode a SaaS product?

No. OrbyNode is self-hosted software under Apache-2.0 with Commons Clause. You run it on your infrastructure. No mandatory cloud account, no telemetry, no billing. The only network egress is what you configure (remote nodes, notifications, external tools).

## Security

### What are the security implications?

OrbyNode provides remote terminal access. This is remote-code-execution capability. Treat it accordingly:

- **Network exposure**: The daemon binds to `127.0.0.1:7676` by default. Do not expose it directly to the internet. Use a reverse proxy with TLS, or run it on a private network with VPN access.
- **Authentication**: All routes require session authentication. Mutations require CSRF tokens. API tokens are revocable and shown once at creation.
- **Authorization**: RBAC (Owner, Editor, Viewer) is enforced on every route and WebSocket subscription.
- **Audit**: Durable mutations are logged with user, timestamp, and action.

Read [security-model.md](security-model.md) for the full threat model and defensive posture.

### Can I use OrbyNode over the public internet?

Not recommended without additional hardening:

1. Put OrbyNode behind a reverse proxy (nginx, Caddy, Traefik) with TLS
2. Enable authentication and create user accounts
3. Consider IP allowlisting or VPN-only access
4. Review the audit logs regularly

For production deployments, treat OrbyNode like you would treat SSH access.

### How do I rotate secrets?

- **User passwords**: Users can change their own passwords; Owners can reset any password
- **API tokens**: Revoke old tokens, generate new ones (secrets are shown once)
- **Session cookies**: Users can sign out, which invalidates the session; sessions expire after inactivity
- **Database encryption**: OrbyNode does not encrypt SQLite at rest by default. Use filesystem encryption (LUKS, BitLocker, FileVault) for sensitive deployments.

## Architecture

### Why SQLite instead of PostgreSQL?

SQLite is a single file, zero-config, and sufficient for the workload. OrbyNode is daemon-local; it does not need horizontal database scaling. The entire durable state fits in one file that you can copy for backup.

If you need PostgreSQL, the architecture supports swapping the storage layer (ADR 002 covers the decision).

### How does crash recovery work?

Three-tier recovery after hard OS crash:

1. **Shape** — workspaces, tabs, panes, layout, cwd, env, titles restored from SQLite
2. **Scrollback** — pane output journaled durably; replayed before client attaches
3. **Agent sessions** — panes that ran agents with native integration can be resumed (opt-in via `restore.resume_agents`)

The daemon restarts and replays state before serving requests. Processes that died during crash cannot be restored; the product never claims otherwise. Tier-2 replay and tier-3 agent resume are the documented guarantees.

### How does the plugin system work?

OrbyNode exposes a REST/WebSocket API with JSON responses. Plugins are external processes that call this API. The daemon does not load arbitrary native code; it owns the runtime surface and plugins consume it.

Native integrations (Claude Code, Codex, etc.) are special: OrbyNode detects their state machine and rolls it up to the UI. Third-party plugins cannot inject code into the daemon.

## Operations

### How do I back up OrbyNode?

Copy the data directory:

```bash
cp -r ~/.orbynode ~/.orbynode-backup
```

Or use SQLite's backup API:

```bash
sqlite3 ~/.orbynode/orbynode.db ".backup ~/.orbynode/orbynode-backup.db"
```

Restore by replacing the data directory or database file.

### How do I upgrade?

1. Install the new version (same install method; it replaces binaries)
2. Restart the daemon
3. OrbyNode runs schema migrations on startup

Rollback: reinstall the old version. The database schema is backward-compatible within a major version.

### How do I monitor OrbyNode?

- **Health endpoint**: `GET /health` returns `{"status":"ok","uptime_secs":...}`
- **Logs**: `RUST_LOG=debug` for verbose output; `ORBYNODE_LOG_FORMAT=json` for structured logs
- **Metrics**: OrbyNode does not expose Prometheus metrics by default; use process monitoring (systemd, supervisord) for resource tracking

### Can I run multiple OrbyNode instances?

Yes. Each instance has its own data directory (`ORBYNODE_DATA_DIR`). Use cases:

- **Multi-tenant**: separate instances per team or project
- **Canary testing**: run a staging instance alongside production
- **Geographic distribution**: instances on different machines

Remote nodes let one OrbyNode instance control daemons on other machines, which is usually simpler than running multiple independent instances.

## Development

### How do I contribute?

Read [CONTRIBUTING.md](../CONTRIBUTING.md). Keep changes milestone-scoped, add ADRs for substantial architecture, and run the full check suite before opening a pull request.

### What's the frontend stack?

Vite + React + TypeScript + Bun + Magic UI. Next.js is not used. The frontend is built into static files and embedded in the daemon binary; no separate frontend server is required.

### How do I add a new agent integration?

1. Study `docs/agents.md` for the agent detection model
2. Add a detector in `crates/daemon/src/agent_detection.rs`
3. Map the agent's state machine to OrbyNode's attention states
4. Update the UI to show agent-specific icons and commands
5. Add tests in `crates/daemon/src/agent_detection_test.rs`

Native integrations require understanding the agent's output format and state transitions. Start with a simple detector and refine based on real usage.

## Licensing

### What license does OrbyNode use?

Apache-2.0 with Commons Clause. You can use, modify, and build around OrbyNode for free, including commercial use and building products or services around it. The Commons Clause withholds selling the software itself, or charging for a product or service whose value derives entirely or substantially from its functionality.

PotenFYR names and trademarks stay unlicensed. Keep the required license notices, including the Commons Clause notice.

### Can I fork OrbyNode?

Yes. The Apache-2.0 license permits forking and modification. The Commons Clause means you cannot sell the fork as a paid product. You can charge for services around it (hosting, support, custom development), just not the software itself as the primary paid offering.

### Can I use OrbyNode in my commercial product?

Yes, as long as you're not selling OrbyNode itself as the product. You can embed it, extend it, and build commercial services on top of it. See the [LICENSE](../LICENSE) file for the full terms.
