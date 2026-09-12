# ADR 001 — Daemon Architecture

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §5, §109; Milestone 0 (§123)

## Decision

OrbyNode is a single primary daemon process (`orbynode`) — the authoritative
runtime. Browsers and the later desktop shell are clients of the daemon; none
of them host agent processes. Closing any client never terminates agents.

Structure: Rust workspace with `crates/daemon` (binary), `crates/api` (HTTP
router + embedded web UI), `crates/core` (shared types: config, version).
New subsystems become new crates; `core` stays small.

Configuration comes from `ORBYNODE_*` env vars over secure defaults:
`127.0.0.1:7676`, data dir `~/.orbynode`. `0.0.0.0` requires explicit opt-in
(Plan §17). Structured logging via `tracing`, JSON when `ORBYNODE_LOG_FORMAT=json`.

## Consequences

- Agent lifetime is bound to the daemon process, not to any client connection.
- Configuration surface stays minimal until the setup wizard (M4) moves it to DB-backed settings.
- A future Tauri shell (M5) only needs to talk to the daemon's HTTP surface.
