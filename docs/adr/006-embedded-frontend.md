# ADR 006 - Embedded Frontend Delivery

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §4, §5; Milestone 0 (§123)

## Decision

The daemon binary embeds the built web UI. `crates/api/build.rs` walks
`web/dist` at compile time and emits `include_bytes!` entries into `OUT_DIR`;
the Axum fallback route serves them with an SPA fallback to `index.html`.

- If `web/dist` is missing, a placeholder page is embedded so the workspace
  always builds from a fresh clone.
- `ORBYNODE_STATIC_DIR=/path/to/web/dist` serves from disk instead - used in
  frontend development so rebuilds don't require a cargo rebuild.
- Path traversal is rejected fail-closed: any `%` encoding or `..` segment in
  the request path is a 404 (Plan §16).

## Consequences

- Single-binary distribution: install the daemon, get the UI.
- Vite's hashed asset names are the cache story; no cache headers are needed
  in M0 (revisit when assets grow).
- The desktop shell (M5) reuses the same daemon surface, no separate bundling.
