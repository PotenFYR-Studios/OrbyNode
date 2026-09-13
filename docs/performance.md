# Performance

Current performance baseline, limits and future measurement requirements.

## Priority

1. correctness
2. security
3. reliability
4. bounded resource use
5. latency
6. bandwidth
7. visual polish

Never optimize by sacrificing correctness or security.

## Current enforced baseline

`crates/realtime/tests/load.rs` publishes 32 events to 100 subscribers on one
stream and asserts:

- delivery reaches all subscribers,
- completion remains under five seconds in test profile,
- replay remains bounded.

This is a regression gate, not a v1.0 end-to-end claim.

## Bounds

| Resource | Bound |
| --- | --- |
| Realtime replay ring | 1,024 events per stream by default. |
| Client outgoing queue | 1,024 events by default. |
| Broadcast channel | 4,096 events per stream receiver channel. |
| Terminal output | Read once; fanned out through bounded queues. |
| Critical events | Protected over droppable events. |
| Workflow runs | SQLite-backed; current IDs are time-derived. |

Slow clients receive overflow or resync signals. They do not block producers or
other clients.

## Event behavior

- Critical attention, approval and security events are not dropped.
- Terminal bytes and replaceable metrics are droppable under pressure.
- Sequence gaps mean droppable loss and require resnapshot or resubscribe.
- Replay covers bounded short interruptions.

## Startup and database

SQLite runs in WAL mode with foreign keys enabled. Migrations are transactional
and versioned. Durable repositories avoid polling and are read on demand.

Startup cost is dominated by daemon initialization, SQLite open/migration and
static asset embedding.

## Release profile

The workspace release profile uses:

```toml
[profile.release]
lto = "thin"
codegen-units = 1
```

Release builds are locked.

## Measured requirements for v1.0

Before making scale claims, measure in clean target environments:

- 100 concurrent authenticated WebSocket clients,
- 100 persistent PTY terminals,
- 50 active agents,
- multiple tabs per user,
- heavy terminal output,
- metric and task streams,
- Git watchers,
- remote-node fan-in.

Capture:

| Metric | Target output |
| --- | --- |
| Daemon CPU | Percent by phase and steady state. |
| Daemon RSS | Steady and peak. |
| Event latency | Publish-to-receive distribution. |
| Terminal latency | Keystroke-to-output distribution. |
| Database latency | Read/write percentiles. |
| Bandwidth | Inbound/outbound per client and aggregate. |
| Browser CPU/RAM | Per tab under terminal and metric load. |
| Dropped/coalesced events | Count and stream distribution. |
| Queue depth | Percentiles and overflow counts. |
| SQLite WAL size | Steady and peak. |

## Profiling workflow

1. Build release binary.
2. Start clean data directory.
3. Generate deterministic load.
4. Sample daemon CPU, RSS and event latency.
5. Capture SQLite query timing.
6. Capture browser performance traces.
7. Repeat for target matrix.
8. Record machine specs, versions and commit.
9. Compare with previous profile.
10. File bottlenecks with reproducible artifacts.

## Optimization rules

- Fix correctness first.
- Do not remove authorization checks for speed.
- Do not make queues unbounded.
- Do not add polling.
- Prefer batched realtime updates to more client connections.
- Prefer indexes and prepared queries to duplicate repositories.
- Profile before redesigning architecture.

## Current known limits

- Browser CPU and RAM are not yet captured automatically.
- End-to-end terminal latency is not yet captured automatically.
- Production database latency is not yet benchmarked.
- Windows and macOS performance captures are outstanding.
- Desktop tray overhead is not measured in the baseline.
