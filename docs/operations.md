# Operations

Operational guidance for running the OrbyNode daemon.

## Runtime model

The daemon owns agents and durable state. Browser and desktop clients can stop
without stopping work. Restarting a client reconnects to the daemon; restarting
the daemon preserves durable state but not live PTY processes.

## Filesystem layout

By default, OrbyNode uses `~/.orbynode`:

| Path | Purpose |
| --- | --- |
| `~/.orbynode/orbynode.db` | SQLite WAL database. |
| `~/.orbynode/orbynode.db-shm` | SQLite shared memory file. |
| `~/.orbynode/orbynode.db-wal` | SQLite write-ahead log. |
| `~/.orbynode/worktrees` | Task worktrees. |

Set `ORBYNODE_DATA_DIR` to relocate all durable state.

## Environment

| Variable | Default | Purpose |
| --- | --- | --- |
| `ORBYNODE_BIND` | `127.0.0.1:7676` | Listen address. |
| `ORBYNODE_PORT` | `7676` | Port shorthand. |
| `ORBYNODE_DATA_DIR` | `~/.orbynode` | Durable state directory. |
| `ORBYNODE_STATIC_DIR` | embedded | Serve web assets from disk. |
| `ORBYNODE_LOG_FORMAT` | human-readable | `json` enables structured logs. |
| `RUST_LOG` | `info` | Tracing filter. |

## Health and version

```bash
curl -fsS http://127.0.0.1:7676/health
curl -fsS http://127.0.0.1:7676/version
```

## Logs

Human-readable logs are default. Use JSON for collectors:

```bash
ORBYNODE_LOG_FORMAT=json RUST_LOG=info cargo run -p orbynode-daemon
```

Do not log secrets. OrbyNode avoids logging tokens, pairing secrets and
password material.

## Backup and restore

1. Stop writes or use a SQLite-consistent backup tool.
2. Back up `orbynode.db` and, if present, the WAL/SHM files as a set.
3. Restore all files together to the same `ORBYNODE_DATA_DIR`.
4. Start the daemon and verify `/health`, users, projects and audit history.

Do not copy only the main database file while WAL is active unless you use a
SQLite online backup API.

## Upgrades

1. Back up `ORBYNODE_DATA_DIR`.
2. Verify release checksum and Cosign signature.
3. Stop the old daemon gracefully.
4. Replace the daemon binary.
5. Start the replacement and watch migration logs.
6. Verify login, projects, terminals, tasks, audit and `/version`.

SQLite migrations are one-way. Test upgrades on a copied data directory before
production rollout.

## Access hardening

- Keep localhost binding unless you have HTTPS or a trusted overlay.
- Revoke sessions after staff changes or suspected credential compromise.
- Use least-privilege project roles.
- Review `/audit` for authentication and administrative actions.
- Restrict service previews to loopback destinations.

## Remote nodes

Pair remote nodes from an authenticated Owner or Administrator session. Store
the one-time pairing secret outside logs. Revoke the node immediately if its
host is decommissioned or compromised.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Cannot bind | `ORBYNODE_BIND` conflict and permissions. |
| Browser cannot authenticate | Clock skew, expired session, cookie/CSRF handling. |
| Missing UI | `web/dist` absent and no `ORBYNODE_STATIC_DIR`. |
| Database migration fails | Disk space, SQLite integrity and backup state. |
| Terminal ends after daemon restart | Expected: live PTYs are not durable. |
| Remote node offline | Node process, network, heartbeat and revocation state. |
