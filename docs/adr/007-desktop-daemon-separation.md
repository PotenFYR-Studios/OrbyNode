# ADR 007 - Desktop / Daemon Separation

- Status: Accepted
- Date: 2026-09-13 (Milestone 5)
- Context: Plan §5, §18, §109; Milestone 5 (§128)

## Decision

The Tauri shell (`desktop/tauri`) is a **client**, never the runtime. It wraps
the web UI served by the daemon at `http://127.0.0.1:7676` and adds the tray.

- Tray menu ships with `Open Dashboard` and `Quit Tray` (Plan §18). Service
  Start/Stop entries land with the service-management API (M10); they call the
  daemon's HTTP surface, never manage processes directly.
- **Quit Tray exits the shell only.** The daemon is a separate OS process
  started by the user, the installer (M20), or an autostart entry.
- Window starts hidden (tray-first app); CSP allows localhost daemon origins
  only.
- The shell does not bundle a daemon binary at M5; packaging (M20) embeds and
  supervises it.

## Consequences

- Closing the desktop UI leaves daemon and agents running (M5 acceptance).
- No duplicate state: the shell renders what the daemon serves.
