# Development Guide

Local development workflow for OrbyNode.

## Stack

- Rust 2024 workspace
- Axum, Tokio, SQLx and SQLite
- Vite + React + TypeScript + Bun + Magic UI
- Optional Tauri desktop shell

Do not use Next.js or another React meta-framework.

## Repository layout

```text
crates/core          shared config and paths
crates/daemon        binary entry point
crates/api           Axum routes, auth, gateway, embedded UI
crates/terminal      real PTY runtime
crates/realtime      event bus, replay, bounded sessions
crates/database      SQLite migrations and repositories
crates/auth          Argon2id, sessions, throttling, permissions
crates/agents        agent detection and Attention Center
crates/files         files, Git and worktrees
crates/services      ports, services and process snapshots
crates/nodes         remote-node identity and pairing
crates/notifications browser events and webhooks
crates/workflows     configuration-driven orchestration
web                  Vite + React + TypeScript client
docs                 project documentation and ADRs
```

## Setup

```bash
git clone https://github.com/PotenFYR-Studios/OrbyNode.git
cd OrbyNode
(cd web && bun install)
cargo build
```

## Run backend

```bash
cargo run -p orbynode-daemon
```

Defaults:

- bind `127.0.0.1:7676`
- data directory `~/.orbynode`
- embedded static UI

## Run frontend

### Build once and embed

```bash
(cd web && bun run build)
cargo run -p orbynode-daemon
```

### Iterate against disk assets

```bash
(cd web && bun run build)
ORBYNODE_STATIC_DIR="$PWD/web/dist" cargo run -p orbynode-daemon
```

### Vite dev server

```bash
(cd web && bun run dev)
```

Use the Vite proxy for local daemon requests.

## Checks

Required before every PR:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
(cd web && bun run check)
(cd web && bun run build)
```

Equivalent:

```bash
scripts/check.sh
```

## Testing patterns

- Put unit tests beside the code they cover.
- Use SQLite `sqlite::memory:` for database tests.
- Use API route tests for auth and routing behavior.
- Keep filesystem tests in temporary directories.
- Keep realtime tests deterministic and bounded.
- Cover security-relevant behavior explicitly.

## Database migrations

Migrations live in `crates/database/src/lib.rs`. Add a new numbered migration;
do not edit an applied migration.

Local test database:

```bash
rm -rf /tmp/orbynode-dev
ORBYNODE_DATA_DIR=/tmp/orbynode-dev cargo run -p orbynode-daemon
```

## Debugging

Use structured logs:

```bash
ORBYNODE_LOG_FORMAT=json RUST_LOG=debug cargo run -p orbynode-daemon
```

Inspect:

```bash
curl -fsS http://127.0.0.1:7676/health
curl -fsS http://127.0.0.1:7676/version
```

## Release build

```bash
(cd web && bun install)
(cd web && bun run build)
cargo build --release --locked -p orbynode-daemon
```

Local release archive, checksum and SBOM:

```bash
scripts/release.sh <rust-target-triple>
```

## Code standards

- Prefer correctness, security and bounded resources before latency.
- Keep `crates/core` small.
- Avoid polling, unbounded queues and global locks.
- Justify every new dependency.
- Use server-side authorization for every new route or subscription.
- Keep comments for non-obvious invariants only.

## Frontend standards

- Use Vite, React, TypeScript, Bun and Magic UI.
- Keep strict TypeScript.
- Derive live state from snapshots and realtime events.
- Do not poll.
- Respect `prefers-reduced-motion`.
- Do not render terminal bytes as HTML.

## Commit discipline

- One focused change per commit.
- Imperative subject, for example `Fix session revocation`.
- Explain behavior, tests and risks in the body.
- Never add AI attribution.
- Do not change Git identity.
