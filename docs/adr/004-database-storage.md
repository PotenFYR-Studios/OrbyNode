# ADR 004 — Database / Storage

- Status: Accepted
- Date: 2026-09-13 (Milestone 3)
- Context: Plan §9, §74; Milestone 3 (§126)

## Decision

`crates/database` owns persistence: SQLite via SQLx, WAL mode, compile-time
checked queries (`query!` macros need a build-time DB; this codebase uses the
unchecked `query` API at M3 and migrates to macros once the sqlx CLI lands in
CI — tracked as follow-up, not a design change).

- **Schema v1:** `meta` (key/value), `projects`, `sessions`,
  `session_terminals` (layout + terminal metadata), `settings` (namespaced
  key/value). Foreign keys ON. Migrations via `_sqlx_migrations`-style
  user table `schema_migrations` with versioned SQL files in
  `crates/database/migrations/`.
- **What is stored:** durable state only (§74) — projects, sessions, terminal
  layout/metadata, settings. Terminal bytes, presence, bus events, metrics
  never touch the DB.
- **Restart recovery (§9):** daemon start loads projects/sessions/settings,
  reconstructs session layout, marks terminals non-resumable (OS processes
  died with the daemon — never pretend otherwise, §9.7). Worktree/agent
  resume arrives in later milestones.
- **Access pattern:** one connection pool, short writes. The realtime bus and
  PTY runtime stay fully in-memory.

## Consequences

- M3 acceptance: projects and session layout survive daemon restart.
- Event resume across daemon restart is explicitly out of scope: clients
  resnapshot on daemon restart (their `last_seq` predates the new process).
- Backups (§50) later read the SQLite file directly; no separate format.
