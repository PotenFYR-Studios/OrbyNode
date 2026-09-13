# ADR 011 - API Interface Stability

> Canonical ADR 011. `011-platform-interfaces.md` is retained as a historical
> duplicate and points here.

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §140; Milestone 17

## Decision

Public REST surfaces are versioned under `/api/v1`. Webhooks, MCP manifests
and invocations, API tokens, plugin manifests, and event publication are
part of that contract. WebSocket envelopes remain versioned by the existing
`v` field.

## Consequences

Breaking changes require a new version and migration guidance. Experimental
fields must be additive, documented, and independently ignorable.
