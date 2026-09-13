# Getting Started

This guide builds OrbyNode from source and starts the local daemon.

## Requirements

- Rust 1.85+
- Bun 1.1+
- Node 20+ only for CI compatibility
- A Unix shell; Windows uses the same commands from Git Bash or WSL

## Build and run

```bash
git clone https://github.com/PotenFYR-Studios/OrbyNode.git
cd OrbyNode
(cd web && bun install)
(cd web && bun run build)
cargo run --release -p orbynode-daemon
```

Open <http://127.0.0.1:7676>.

## First-run setup

1. Open `/setup`.
2. Read the warning about terminal access.
3. Create the first Owner with a username, display name and password.
4. Sign in from the returned session cookie.

The first Owner can be created only while the users table is empty.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `ORBYNODE_BIND` | `127.0.0.1:7676` | Bind address. |
| `ORBYNODE_PORT` | `7676` | Port shorthand. |
| `ORBYNODE_DATA_DIR` | `~/.orbynode` | SQLite and durable state. |
| `ORBYNODE_STATIC_DIR` | embedded | Serve `web/dist` from disk for development. |
| `ORBYNODE_LOG_FORMAT` | human | Set to `json` for structured logs. |
| `RUST_LOG` | `info` | Tracing filter. |

Example development daemon:

```bash
(cd web && bun run build)
ORBYNODE_STATIC_DIR="$PWD/web/dist" cargo run -p orbynode-daemon
```

## Frontend workflow

The frontend is Vite + React + TypeScript + Bun + Magic UI. Next.js is not
used.

```bash
(cd web && bun install)
(cd web && bun run dev)
```

The Vite dev server proxies daemon traffic for local development.

## Full checks

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
(cd web && bun run check)
(cd web && bun run build)
```

Or run:

```bash
scripts/check.sh
```

## Release build

```bash
(cd web && bun install)
(cd web && bun run build)
cargo build --release --locked -p orbynode-daemon
```

For an archive, checksum and SBOM:

```bash
scripts/release.sh <rust-target-triple>
```

## Next steps

- Read [Architecture](../ARCHITECTURE.md).
- Review [Security](../SECURITY.md).
- Track status in [Roadmap](../ROADMAP.md).
