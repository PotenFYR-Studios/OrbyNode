# ADR 004 — Database / Storage

- Status: Proposed
- Date: Milestone 3 (Projects and Persistence)
- Context: Plan §9, §74; Milestone 3 (§126)

## Decision

To be finalized at the start of Milestone 3. Expected shape:

- SQLite with WAL, accessed through SQLx with compile-time-checked queries.
- Migrations checked into the repo, forward-only.
- Durable state only (Plan §74): projects, sessions, tasks, settings, users.
  Terminal bytes, presence, metrics, mouse/animation state never hit the DB —
  those stay in memory or bounded ring buffers.
- Short transactions, prepared statements, batched writes.

## Consequences

- No external DB service required for standard installs (Plan §152).
- A PostgreSQL backend remains a future option (§92) but is not designed in.
