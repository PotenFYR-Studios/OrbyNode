# ADR 009 - Realtime Protocol

- Status: Accepted
- Date: 2026-09-13 (Milestone 2)
- Context: Plan §56-§71, §88-§92, §115; Milestone 2 (§125)

## Decision

New crate `orbynode-realtime` owns the realtime core. The daemon exposes
**one WebSocket per browser tab** (`/ws`) carrying all streams, multiplexed
by topic (Plan §57). No polling anywhere.

### Event envelope (Plan §115)

```json
{
  "v": 1,                    // protocol_version
  "seq": 42101,              // global monotonic sequence
  "stream": "terminal:1",    // topic
  "id": "17",                // entity id within the stream
  "ver": 91,                 // entity version (Plan §68)
  "ts": 1739283941,          // unix millis
  "type": "terminal.output", // event type
  "data": { }                // payload (JSON for control events)
}
```

### Lifecycle

1. Client connects to `/ws`, sends `{"op":"hello","last_seq":N}` (or
   `{"op":"hello"}` for a fresh session).
2. Server replies `{"op":"snapshot","seq":S,"state":{...}}` - current durable
   state for the client's authorized scope. If `last_seq` is inside the
   replay ring, server instead replies `{"op":"resume","from":N+1}` followed
   by missed events - no full resnapshot on short interruptions (§62).
3. If `last_seq` is gone (ring wrapped): `{"op":"resync_required"}`, client
   re-snapshots (§62).
4. Client subscribes: `{"op":"sub","stream":"terminal:1"}`; unsubscribes with
   `{"op":"unsub","stream":"terminal:1"}`. Server only delivers events for
   subscribed streams.
5. Server→client events use the envelope above. Client→server control
   messages are small JSON ops.

### Backpressure and slow clients (§66)

- Each client has a **bounded** outgoing queue (`mpsc`, 1024 entries).
- The pump task sending frames awaits queue capacity; it never blocks
  producers beyond that bound.
- Terminal output streams are **replaceable**: when a client's queue overflows,
  the terminal stream is dropped and the client receives
  `{"op":"stream_overflow","stream":...}` and must resubscribe (replaying
  scrollback). Critical streams (attention, approvals) are never dropped.
- Coalescing (§64): high-frequency replaceable state (metrics, context
  meters) is coalesced per stream - only the newest value is kept when the
  queue backs up. Semantically significant events are never coalesced.

### Authorization (§60, §105)

Every `sub` is checked against the authorization layer before subscription.
M2 runs pre-auth (M4): all-local trust, every subscription allowed, but the
ACL hook exists in the gateway so M4/M11 plug in without protocol changes.
Revocation ends subscriptions immediately (tested with a stub ACL in M2; the
live revocation path is completed in M11).

### Terminal transport (§65)

Terminal bytes ride the multiplexed socket as binary frames prefixed with the
stream id: `[stream_id_varint][bytes]` - one PTY read still fans out to all
subscribed clients (§59); the bus copies to each subscriber's queue.

### In-process bus

Central `EventBus`: publishers call `publish(stream, event)`; a global
sequence counter stamps each event; a bounded replay ring (per stream +
global) retains recent events for resume. Nothing here touches the database
(§74): replay state is in-memory; resume across daemon restarts is out of
scope until M3 persists durable state, and clients resync then.

## Consequences

- Acceptance D (§125): 20 clients on one terminal → one PTY read, one bus
  publish, 20 queue pushes.
- Acceptance E: a stalled client's queue fills; the terminal stream is
  dropped for that client only. PTY and other clients are untouched.
- Frontend reconciler (§77) applies envelopes to TanStack Query caches;
  lands with the M2 web work.
