# ADR 021 - Herdr-Parity Web Workspaces and Crash-Safe Recovery

- Status: Accepted (implemented M21)
- Date: 2026-09-13
- Context: User direction; Plan §154 (new); Herdr docs (herdr.dev/docs)

## Decision

OrbyNode adopts Herdr's workspace model — workspaces, tabs, panes — as a
web-GUI-only surface. No terminal-multiplexer client is added; the browser is
the only pane interface, per the daemon-centric architecture (ADR 001).

Crash recovery is three-tier. After a hard OS crash, daemon restart restores:

1. **Shape** — workspaces, tabs, pane layout, cwd, env, focus, titles from
   SQLite (durable state; Plan rule 10).
2. **Scrollback** — pane output journaled durably through a batched,
   bounded writer into SQLite; replayed into pane scrollback before any
   client attaches. Journal output is captured once and fanned out together
   with live subscribers (Plan rule 2); per-pane and global byte caps keep
   the writer bounded under pressure (Plan rule 8).
3. **Agent sessions** — panes that ran agents with native integration
   session IDs (docs/agents.md Level 3) are resumed by typing the agent's
   documented `--resume`/`resume` command into the restored shell. Opt-in
   via configuration (`restore.resume_agents`, default off): typing into a
   fresh shell is a side effect enterprise deployments must control.

Agent state machine, Attention Center rollups, and remote-node identity are
unchanged; workspaces roll agent state up to tabs and workspaces the same
way tabs already roll it up to the sidebar.

Pane control (send input, read bounded output, wait) is exposed through the
existing REST/WebSocket API with JSON responses and server-side
authorization on every route and subscription (Plan rule 7). This preserves
Herdr's automation value without a socket protocol the daemon does not have.

Enterprise user management builds on the existing users/roles/audit tables:
admin invite links, password policy configuration, and session administration
in the web UI. Full SSO remains deferred (Plan §145).

## Consequences

- New database migrations for workspaces, tabs, panes, pane journal.
- `crates/terminal` gains a durable journal writer as a second fan-out
  subscriber; slow clients still cannot block agents.
- Daemon startup gains a restore phase before serving requests.
- xterm.js is added to the web workspace (justified: no viable
  terminal-in-browser alternative; VT parsing in the browser is required).
- Hard crash cannot restore live process state; the product never claims
  otherwise. Tier-2 replay and tier-3 agent resume are the documented
  recovery guarantees.
- Scale validation for the journal (multi-viewer, crash-mid-write,
  cap-eviction) becomes a release gate.
