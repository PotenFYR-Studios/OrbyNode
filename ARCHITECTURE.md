# OrbyNode — Architecture

> The self-hosted control plane for coding agents.

OrbyNode is a daemon-centric system. One process — the **daemon** — is the
authoritative runtime that owns PTYs, agents, and all persistent state.
Browsers and the (future) desktop shell are clients. Closing a client never
stops agents.

```text
Browser / PWA ──HTTP──► OrbyNode daemon
                          ├── REST API (commands, CRUD)
                          ├── embedded web UI
                          └── realtime (M2+): one WebSocket per tab,
                              snapshot + incremental events
```

## Crates

| Crate             | Role                                                |
| ----------------- | --------------------------------------------------- |
| `crates/core`     | Shared types: version, config, paths. Deliberately small. |
| `crates/api`      | Axum router, REST endpoints, embedded web UI.        |
| `crates/daemon`   | `orbynode` binary: config, tracing, server loop.     |

New subsystems (terminal, realtime, database, auth, …) become new crates when
a milestone introduces them — see `Plan.md` §120 for the target layout.

## Key decisions

Architecture decisions are recorded as ADRs in `docs/adr/`. The load-bearing
ones so far:

- **[ADR 001](docs/adr/001-daemon-architecture.md)** — daemon is the runtime; clients are replaceable.
- **[ADR 003](docs/adr/003-api-transport.md)** — REST for commands, WebSocket for state; no polling.
- **[ADR 006](docs/adr/006-embedded-frontend.md)** — web UI compiled into the binary; `ORBYNODE_STATIC_DIR` for dev.

## Configuration

Environment variables over secure defaults:

| Variable               | Default             | Meaning                          |
| ---------------------- | ------------------- | -------------------------------- |
| `ORBYNODE_BIND`        | `127.0.0.1:7676`    | Listen address. `0.0.0.0` is opt-in only. |
| `ORBYNODE_PORT`        | `7676`              | Port shorthand for the default host. |
| `ORBYNODE_DATA_DIR`    | `~/.orbynode`       | Durable data directory.          |
| `ORBYNODE_STATIC_DIR`  | embedded assets     | Serve the web UI from disk (dev). |
| `ORBYNODE_LOG_FORMAT`  | human-readable      | `json` for structured JSON logs. |
| `RUST_LOG`             | `info`              | Tracing filter.                  |

## Build and run

```sh
npm ci --prefix web && npm run build --prefix web   # build web UI
cargo run --release -p orbynode-daemon              # run daemon
curl -s localhost:7676/health                       # verify
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development workflow and
[SECURITY.md](SECURITY.md) for the security model.
