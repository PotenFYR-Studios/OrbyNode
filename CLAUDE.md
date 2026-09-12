# OrbyNode — Working Rules for AI Assistants

Self-hosted, daemon-centric control plane for coding agents. Rust workspace
(`crates/`) + TypeScript/React web UI (`web/`). Authoritative long-form plan:
`Plan.md` (untracked, by design). Read `ARCHITECTURE.md` and `docs/adr/`
before non-trivial changes.

## Scope discipline

- Work milestone by milestone (`ROADMAP.md` = status, `Plan.md` §123+ = detail).
- Do not start a later milestone's features early; do not make the whole
  project in one pass.
- Substantial architectural changes get an ADR in `docs/adr/` first.

## Non-negotiables (Plan §151 — full list there)

- Daemon is the runtime; clients (browser, tray) are replaceable and must
  never own agent processes.
- Localhost binding is the default; `0.0.0.0` is opt-in.
- Realtime state comes from WebSocket snapshot + events, never polling.
- PTY output captured once, fanned out; buffers and queues bounded.
- Durable state → database; high-frequency transient state → memory.
- No secret logging, no silent config overwrite, no mandatory cloud/telemetry.
- Dependency additions need justification; prefer stdlib and existing deps.

## Code conventions

- Rust: edition 2024, workspace deps in the root `Cargo.toml`, `clippy`
  clean, `rustfmt` formatted. Keep `crates/core` small — no dumping ground.
- TypeScript: strict, no unused locals/params, `npm run check` must pass.
- Tests live next to the code they cover; every non-trivial change ships a
  check (`cargo test`, web: typecheck + build as the M0 gate).

## Verification before claiming done

```sh
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check --prefix web && npm run build --prefix web
```

All four must pass. Do not mark work complete merely because it compiles.
