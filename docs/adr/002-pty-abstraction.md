# ADR 002 — PTY Abstraction

- Status: Proposed
- Date: Milestone 1 (Real PTY Runtime)
- Context: Plan §8, §59, §66; Milestone 1 (§124)

## Decision

To be finalized at the start of Milestone 1. Expected shape:

- `portable-pty` as the cross-platform PTY backend (Windows ConPTY included).
- One dedicated blocking reader task per PTY (Plan §75 allows platform PTY
  blocking workers), fanning out into a bounded broadcast channel — captured
  once, fanned out to N viewers (§59).
- Bounded ring-buffer scrollback, per-client bounded output queues with
  slow-client policy (§66): batch → coalesce → drop replaceable → resync.
- Terminal data frames use binary WebSocket frames; control events use JSON
  envelopes (§65).

## Consequences

- Reconnect is client state re-attachment, not process re-creation: the PTY
  outlives every browser connection (M1 acceptance: refresh reconnects to the
  same running shell).
