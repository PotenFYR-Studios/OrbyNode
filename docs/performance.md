# Performance Profile

## Current baseline

- Realtime broadcast: 100 subscribers on one stream processed 32 burst events
  in the test profile in under five seconds; the assertion keeps regressions
  visible.
- Event replay rings remain bounded at 1,024 events per stream.
- Client queues remain bounded at 1,024 events, with droppable overflow and
  critical-event precedence.
- Release profiles use thin LTO and one codegen unit for production builds.

## Outstanding production measurements

The in-process broadcast harness validates bounded fan-out and prevents obvious
regressions, but is not a substitute for end-to-end production measurements.
Milestone 20 release engineering must add repeatable daemon-level captures for
CPU, RAM, event latency, terminal latency, database latency, bandwidth, browser
CPU/RAM, and queue depth on the target matrix.
