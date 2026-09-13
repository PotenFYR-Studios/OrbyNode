# ADR 002 - PTY Abstraction

- Status: Accepted
- Date: 2026-09-13 (Milestone 1)
- Context: Plan §8, §59, §65, §66, §75; Milestone 1 (§124)

## Decision

New crate `crates/terminal` owns the PTY runtime.

- **Backend:** `portable-pty` (WezTerm) - cross-platform, ConPTY on Windows.
- **Concurrency:** one dedicated blocking OS thread per PTY reader (Plan §75
  permits platform PTY blocking workers). The thread reads raw bytes once and
  pushes into a `tokio::sync::broadcast` channel - **captured once, fanned out
  to N viewers** (§59). Writes go through the master handle under a mutex.
- **Scrollback:** bounded byte ring buffer (`TerminalConfig.scrollback_bytes`,
  default 1 MiB) per terminal. Reconnecting clients replay the buffer, then go
  live - reconnect is state re-attachment, never process re-creation.
- **Transport:** WebSocket per terminal for M1 (REST commands + WS stream).
  Binary frames carry raw PTY output; text frames carry JSON control messages
  (input, resize). M2 (ADR 009) multiplexes this into the per-tab gateway.
- **Lifecycle:** terminals are in-memory for M1 - a terminal lives as long as
  the daemon. Persistence/restart recovery is Milestone 3 (ADR 004). The M1
  acceptance - close browser, reopen, same shell still running - holds
  because the PTY is owned by the daemon, not the WebSocket.
- **Security:** M1 runs before the auth milestone (M4); localhost-only
  binding (ADR 001) is the containment. Terminal endpoints get auth the
  moment M4 lands, and RBAC in M11.

## Consequences

- A slow WS client cannot block the PTY: broadcast laggers are dropped with a
  resync signal; the reader thread never awaits a consumer.
- Shell detection is minimal for M1 (`$SHELL`/`/bin/sh` on Unix,
  PowerShell on Windows); working-directory tracking arrives with M6+.
- Killing the terminal kills the child process (explicit terminate; daemon
  exit handling refined in M3).
