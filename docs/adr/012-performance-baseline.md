# ADR 012 - Performance Baseline

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §141; Milestone 18

## Decision

Ship a deterministic in-process realtime scale test as the first enforced
baseline. It exercises one stream with 100 subscribers, bounded replay, and a
time budget. Record current limits and production measurement requirements in
`docs/performance.md`.

## Consequences

The harness catches fan-out regressions early, but does not authorize v1.0
scale claims. Release claims require the end-to-end measurement matrix listed
in `docs/performance.md`.
