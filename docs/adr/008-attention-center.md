# ADR 008 - Attention Center

## Status

Accepted

## Context

Attention is action-oriented runtime state. It must answer “what needs me
right now?” without polling and must remain visible after browser reconnects.
P0/P1 and critical P2 items must not be dropped under client backpressure.

## Decision

Use a daemon-owned in-memory aggregate on the existing realtime event bus.
Agent state and task transitions create, update, or resolve attention items.
Every item carries a stable resource identity so state changes cannot leak
stale warnings. The aggregate is exposed through authenticated REST for
startup snapshot and published on the `attention` stream for live updates.

In-memory is intentional while attention sources remain runtime-local. If
attention must survive daemon restart or aggregate across remote nodes, the
resource-based model can move to durable storage without changing the API or
event contract.

## Consequences

Attention follows realtime semantics: subscribe to `attention` for changes and
fetch `/attention` for current state. Restoring state on daemon restart is a
future persistence concern, not part of the realtime transport.
