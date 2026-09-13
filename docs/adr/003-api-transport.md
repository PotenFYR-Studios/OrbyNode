# ADR 003 - API Transport

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §46, §56; Milestone 0 (§123)

## Decision

REST over HTTP for commands and CRUD; a single multiplexed WebSocket for all
realtime state (introduced in Milestone 2, ADR 009). No polling endpoints -
the dashboard never derives live state from `GET` loops (Plan §56).

M0 ships only the foundation endpoints:

- `GET /health` - liveness + uptime (JSON)
- `GET /version` - name + version (JSON)
- `/` - embedded web UI (ADR 006)

The HTTP stack is Axum + Tokio + tower. JSON via serde. Structured request
logging via `tower-http` tracing.

## Consequences

- All future state-bearing surfaces (agents, tasks, machines) must come from
  the realtime layer, never from polling; REST stays for mutations.
- Endpoint versioning happens when the first public API surface stabilizes (M17).
