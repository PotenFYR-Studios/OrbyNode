# Herdr-Parity Web Workspaces and Crash-Safe Recovery — Design

Date: 2026-09-13
Status: Approved direction (1+2+3 recovery, web GUI only, all OS/arch)
ADR: docs/adr/021-herdr-parity-web-workspaces.md

## Goals

1. Herdr's full user-facing feature set, re-created web-GUI-only: no CLI
   multiplexer, no terminal client. Browser is the sole pane surface.
2. Robust, feature-rich surfaces beyond Herdr: enterprise user management,
   admin-invite flow, session administration, audit visibility.
3. Best-in-class crash recovery: after sudden OS shutdown, restore
   everything exactly — shape + scrollback + agent resume (1+2+3).
4. All first-class targets behave identically (six targets, Plan §3).

## Non-goals

- Terminal-multiplexer TUI client (Herdr parity means features, not client).
- Restoring live process state after power loss (physically impossible;
  tier-3 agent resume is the documented substitute).
- Full SSO suite (Plan §145 deferral stands).

## Concepts mapped to OrbyNode

| Herdr concept | OrbyNode equivalent | Notes |
| --- | --- | --- |
| Workspace | workspace row (new) | belongs to project or standalone |
| Tab | tab row (new) | layout container, agent state rolls up |
| Pane | pane row (new) + Terminal | real PTY from crates/terminal |
| Screen manifest | terminal analysis (Level 2) | existing detection |
| Lifecycle hooks | native integrations (Level 3) | existing crates/integrations |
| Mouse UI | xterm.js panes in React | drag splits, select, copy |
| Keyboard layer | web keybindings | optional, `ui.mouse_capture` analog |
| Socket API | REST + WebSocket fan-out | no new protocol |
| Named sessions | existing sessions table | reused, not duplicated |
| Machine profiles | existing remote nodes | pairing already exists |
| Marketplace | explicitly deferred | Plan §145 |

Agent states (blocked/working/done/idle) already exist as the richer
canonical set in docs/agents.md; rollups extend to tab and workspace.

## Data model (migration N+1, N+2)

workspaces(id, project_id NULL, name, position, created_at)
tabs(id, workspace_id, name, position, active_pane_id)
panes(id, tab_id, terminal_id, kind shell|agent, cwd, env_json, title,
      split_dir, split_ratio, position, closed_at NULL, last_session_id)
pane_journal(pane_id, seq INTEGER, chunk BLOB, created_at)
  PK (pane_id, seq); per-pane cap default 2 MiB, global cap default 256 MiB,
  eviction oldest-first per pane; chunks fsync-batched (default 250 ms or
  64 KiB, whichever first); settings keys journal.pane_cap_bytes,
  journal.global_cap_bytes, journal.batch_ms, restore.resume_agents.

## Recovery

Startup order: open DB → restore shape (workspaces/tabs/panes with rows)
→ create Terminals in saved cwd/env → replay pane_journal into scrollback
→ mark agent panes; if restore.resume_agents and last_session_id present,
write the agent's resume command + session ID into the fresh shell →
begin serving HTTP → realtime snapshot includes restored topology.

Partial-damage rule: a pane whose journal tail is torn (crash mid-batch)
keeps the last whole seq; restore never fails because one pane is damaged —
the pane is flagged degraded and surfaced in Attention Center.

## Web GUI

- Workspace sidebar with tab/pane tree; agent state dots roll up.
- Pane grid: splits (right/down), drag borders, rename, close, focus.
- xterm.js per pane; WebSocket attach with bounded reconnect.
- Command palette: create/split/focus/navigate; keyboard layer optional.
- Automation console (REST examples) instead of a socket API doc page.
- Enterprise admin: users list, role edit, invite-link creation, force
  logout, active sessions, audit log view. Uses existing auth crate
  permissions; every route server-authorized.

## API surface (all JSON, all authorized)

WS  /ws (existing) — adds workspace/tab/pane events (created, moved,
    resized, closed, restored, degraded)
GET/POST/PATCH/DELETE /workspaces, /workspaces/:id/tabs,
    /tabs/:id/panes, /panes/:id (move/resize/rename/close)
POST /panes/:id/input {data} — write to PTY
GET  /panes/:id/output?since_seq= — bounded journal read
GET  /panes/:id/wait?state= — agent-state wait (long-poll, bounded)
Admin: GET/POST /admin/invitations, GET/DELETE /admin/sessions/:id,
    GET /admin/audit (paginated)

## Testing

- Unit: journal batching/cap eviction, torn-tail recovery, restore order.
- Integration: crash simulation (kill -9 daemon mid-write; restart;
  verify shape+scrollback), split/resize/close flows over REST.
- Web: typecheck+build gate; pane tree and xterm attach covered by strict
  TS and manual checklist per docs/testing.md.
- Scale: 50 panes, 5 concurrent viewers, journal cap eviction live.
