# ADR 017 - Resource Efficiency

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §59, §66, §74, §125, §137, §146

## Decision

PTY bytes are read once and fanned out through bounded channels. Realtime
replay rings and client queues have explicit caps; droppable traffic overflows
before critical traffic. Durable state belongs in SQLite; high-frequency
transient state stays in memory. Metrics and terminal output may require
resnapshot or resync after overflow, while approvals and security events are
never dropped.

## Consequences

Slow clients cannot block producers or affect other clients. Bounded memory is
preferred to unbounded delivery, with explicit overflow and recovery signals.
