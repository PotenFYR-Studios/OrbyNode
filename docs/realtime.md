# Realtime Protocol

OrbyNode uses one authenticated multiplexed WebSocket per client. REST handles
commands; realtime state uses snapshots, sequenced events and bounded replay.

## Transport

```text
GET /ws
```

The client must send a `hello` op before subscribing.

## Lifecycle

1. Client connects and sends:
   ```json
   {"op":"hello","last_seq":42101}
   ```
   or `{"op":"hello"}` for a fresh session.
2. Server sends a snapshot or resume acknowledgment.
3. Client subscribes:
   ```json
   {"op":"sub","stream":"terminal:1"}
   ```
4. Server replays retained events, then attaches the live stream.
5. Server publishes sequenced envelopes.
6. Client unsubscribes:
   ```json
   {"op":"unsub","stream":"terminal:1"}
   ```

## Event envelope

```json
{
  "v": 1,
  "seq": 42101,
  "stream": "terminal:1",
  "type": "terminal.output",
  "data": {},
  "priority": "droppable"
}
```

`seq` is a global monotonic sequence. `priority` is either `critical` or
`droppable`.

Critical events include attention, approvals and security state. Droppable
events include raw terminal output and replaceable metrics.

## Streams

Common stream shapes:

| Stream | Purpose |
| --- | --- |
| `terminal:<id>` | PTY output and terminal state. |
| `project:<id>` | Project-scoped task and worktree updates. |
| `attention` | Cross-project attention items. |
| `host` | Host and session metrics. |
| `notifications` | Notification deliveries. |
| `workflows` | Workflow run updates. |

Stream names are canonical strings, not scopes by themselves. Server-side
authorization decides access.

## Replay and convergence

Each stream has a bounded replay ring. On reconnect, the client sends its last
sequence. If retained replay can cover the gap, the server resumes from the
next event. If replay is unavailable, the client must request a fresh snapshot
and discard stale state.

Sequence gaps indicate lost droppable events and require resnapshot or
resubscribe. Critical events are protected ahead of droppable events.

## Backpressure

Each client has a bounded outgoing queue. Slow clients never block producers or
other clients. When droppable traffic overflows, that stream is marked for
resubscribe or resnapshot. Critical traffic can replace old droppable entries
when necessary.

## Binary terminal frames

PTY bytes use binary frames rather than JSON escaping:

```text
[u32 big-endian stream-length][stream bytes][PTY payload]
```

The daemon reads the PTY once and fans the bytes out to authorized subscribers.

## Authorization

Every subscription is checked server-side. A client cannot learn state by
guessing a stream name. Membership or role revocation ends access on the next
authorization check; the project RBAC path closes unauthorized subscriptions.

## Client implementation

The web client:

1. connects,
2. sends `hello`,
3. subscribes to authorized streams,
4. applies snapshots,
5. applies sequenced events,
6. resubscribes or resnapshots after overflow,
7. closes cleanly on teardown.

Do not poll REST endpoints to derive live state.
