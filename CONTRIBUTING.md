# Contributing to OrbyNode

Thanks for helping build OrbyNode. Keep changes milestone-scoped (see
[ROADMAP.md](ROADMAP.md)); substantial architectural decisions go through an
[ADR](docs/adr/) first.

## Development setup

Requirements: Rust 1.85+, Bun 1.1+, Node 20+ for CI compatibility.

```sh
(cd web && bun install)          # frontend deps
cargo build                  # backend
```

## Workflow

1. Build the web UI (`(cd web && bun run build)`) so the daemon embeds real
   assets. Without it, the daemon still builds and serves a placeholder page.
2. Run the daemon: `cargo run -p orbynode-daemon` (or with
   `ORBYNODE_STATIC_DIR=web/dist` to iterate on the UI without rebuilding Rust).
3. Frontend dev server with proxy: `bun run dev --cwd web`.

## Checks (all required before a PR)

```sh
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
(cd web && bun run check)
(cd web && bun run build)
```

## Ground rules

- Terminal access equals remote code execution: never weaken auth, CORS, or
  path checks to make a change easier. See [SECURITY.md](SECURITY.md).
- No polling-based realtime; no unbounded buffers.
- New dependencies need a stated reason. Prefer stdlib and existing deps.
- Keep `crates/core` small.

## Commit messages

Focus on the change. No AI attribution in commits or PRs.

Use Vite + React + TypeScript + Bun + Magic UI. Do not use Next.js.
